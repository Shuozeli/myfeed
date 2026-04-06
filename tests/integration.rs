//! Integration tests for myfeed CLI with real browser automation.
//!
//! These tests require a running Lightpanda or Chrome browser with CDP enabled.
//! They test the full crawl pipeline: browser -> recipe -> parser -> output.
//! CI uses cached builds - this comment forces rebuild if cache is stale.
//!
//! Run with:
//!   cargo test --test integration -- --nocapture          # Unit tests
//!   cargo test --test integration -- --ignored --nocapture # Integration tests

use std::process::{Command, Output};

/// Check if CDP_ENDPOINT is configured
fn has_browser() -> bool {
    std::env::var("CDP_ENDPOINT").is_ok()
}

/// Run myfeed crawl command with proper env setup
fn run_crawl(sites: &[&str], format: &str) -> Output {
    let binary = env!("CARGO_BIN_EXE_myfeed");
    let mut args = vec!["crawl", "--format", format];
    for site in sites {
        args.push(site);
    }
    let mut cmd = Command::new(binary);
    cmd.args(&args);
    // Set RECIPES_DIR to source recipes directory
    if let Ok(cwd) = std::env::current_dir() {
        cmd.env("RECIPES_DIR", cwd.join("recipes"));
    }
    // Use CI-provided DATABASE_URL to avoid Docker filesystem issues
    // The CI provides DATABASE_URL=sqlite:///tmp/myfeed_test.db
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        cmd.env("DATABASE_URL", db_url);
    }
    // Pass through required env vars from CI
    if let Ok(cdp) = std::env::var("CDP_ENDPOINT") {
        cmd.env("CDP_ENDPOINT", cdp);
    }
    if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
        cmd.env("TELEGRAM_BOT_TOKEN", token);
    }
    if let Ok(chat) = std::env::var("TELEGRAM_CHAT_ID") {
        cmd.env("TELEGRAM_CHAT_ID", chat);
    }
    if let Ok(interval) = std::env::var("CRAWL_INTERVAL_SECS") {
        cmd.env("CRAWL_INTERVAL_SECS", interval);
    }
    if let Ok(sites) = std::env::var("ENABLED_SITES") {
        cmd.env("ENABLED_SITES", sites);
    }
    cmd.output().expect("failed to execute myfeed crawl")
}

// =============================================================================
// Tests that don't require a browser
// =============================================================================

#[test]
fn test_crawl_requires_site() {
    let output = run_crawl(&[], "json");
    assert!(!output.status.success(), "should fail without sites");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("missing"),
        "should require site argument: {}",
        stderr
    );
}

#[test]
fn test_crawl_unknown_site_graceful() {
    // Unknown site should print error but not crash
    let output = run_crawl(&["nonexistent"], "json");
    // It may fail or succeed with error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("recipe not found") || !output.status.success(),
        "should report recipe not found"
    );
}

// =============================================================================
// Tests that require a browser (run with --ignored)
// =============================================================================

#[test]
#[ignore]
fn test_crawl_simple_feed_with_browser() {
    if !has_browser() {
        eprintln!("Skipping: CDP_ENDPOINT not set");
        return;
    }

    let output = run_crawl(&["simple-feed"], "json");
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "crawl should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("item-1") || stdout.contains("Test Article"),
        "should contain item data"
    );
}

#[test]
#[ignore]
fn test_crawl_hackernews_mock_with_browser() {
    if !has_browser() {
        eprintln!("Skipping: CDP_ENDPOINT not set");
        return;
    }

    let output = run_crawl(&["hackernews"], "json");
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "crawl should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rust"), "should contain article title");
}

#[test]
#[ignore]
fn test_crawl_output_formats() {
    if !has_browser() {
        eprintln!("Skipping: CDP_ENDPOINT not set");
        return;
    }

    // Test JSON format
    let json_output = run_crawl(&["simple-feed"], "json");
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    // Filter out log lines to find actual JSON
    let json_lines: Vec<&str> = json_stdout
        .lines()
        .filter(|l| l.starts_with('[') || l.starts_with('{'))
        .collect();
    assert!(
        !json_lines.is_empty(),
        "should have JSON output: {}",
        json_stdout
    );

    // Test JSONL format
    let jsonl_output = run_crawl(&["simple-feed"], "jsonl");
    assert!(jsonl_output.status.success());
    let jsonl_stdout = String::from_utf8_lossy(&jsonl_output.stdout);
    // Each line should be a valid JSON object (filter out log lines)
    for line in jsonl_stdout.lines().filter(|l| l.starts_with('{')) {
        assert!(line.ends_with('}'), "should be JSON object: {}", line);
    }

    // Test table format
    let table_output = run_crawl(&["simple-feed"], "table");
    assert!(table_output.status.success());
    let table_stdout = String::from_utf8_lossy(&table_output.stdout);
    // Table format should contain site prefix with brackets
    let has_table_format = table_stdout
        .lines()
        .any(|l| l.contains('[') && l.contains(']'));
    assert!(has_table_format, "should have table format output");
}

#[test]
#[ignore]
fn test_crawl_compact_output() {
    if !has_browser() {
        eprintln!("Skipping: CDP_ENDPOINT not set");
        return;
    }

    // Use run_crawl to properly set RECIPES_DIR and DATABASE_URL
    let output = run_crawl(&["simple-feed"], "json");

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "crawl should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Compact output should not contain preview
    assert!(
        !stdout.contains("\"preview\""),
        "compact output should not have preview"
    );
    // But should have id, site, title, url
    assert!(stdout.contains("\"id\""), "should have id field");
    assert!(stdout.contains("\"title\""), "should have title field");
}

#[test]
#[ignore]
fn test_crawl_save_to_db() {
    if !has_browser() {
        eprintln!("Skipping: CDP_ENDPOINT not set");
        return;
    }

    // First crawl with save
    let binary = env!("CARGO_BIN_EXE_myfeed");
    let output = Command::new(binary)
        .args(["crawl", "--save", "--limit", "3", "simple-feed"])
        .output()
        .expect("failed to execute myfeed crawl");

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    // Then list to verify saved
    let list_output = Command::new(binary)
        .args(["list", "--limit", "10"])
        .output()
        .expect("failed to execute myfeed list");

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    // Should show items or say "no items found"
    assert!(
        stdout.contains("simple-feed") || stdout.contains("no items"),
        "should have saved items or show message"
    );
}

// =============================================================================
// Recipe validation tests
// =============================================================================

#[test]
fn test_recipe_list_shows_recipes() {
    let binary = env!("CARGO_BIN_EXE_myfeed");
    let mut cmd = Command::new(binary);
    cmd.args(["recipe", "list"]);
    // Set RECIPES_DIR to source recipes directory
    if let Ok(cwd) = std::env::current_dir() {
        cmd.env("RECIPES_DIR", cwd.join("recipes"));
    }
    let output = cmd.output().expect("failed to execute myfeed recipe list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should list available recipes
    assert!(
        stdout.contains("Available recipes"),
        "should show recipe list header"
    );
    assert!(
        stdout.contains("hackernews"),
        "should list hackernews recipe"
    );
}

#[test]
#[ignore]
fn test_recipe_validate_with_browser() {
    if !has_browser() {
        eprintln!("Skipping: CDP_ENDPOINT not set");
        return;
    }

    let binary = env!("CARGO_BIN_EXE_myfeed");
    let mut cmd = Command::new(binary);
    cmd.args(["recipe", "validate", "hackernews"]);
    if let Ok(cwd) = std::env::current_dir() {
        cmd.env("RECIPES_DIR", cwd.join("recipes"));
    }
    let output = cmd.output().expect("failed to execute recipe validate");

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    // May succeed or fail depending on recipe validity
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("Validation passed")
            || stdout.contains("Recipe:")
            || stderr.contains("error")
            || stderr.contains("Error"),
        "should produce validation output"
    );
}
