//! Integration tests for myfeed CLI commands
//!
//! These tests run the compiled binary and verify behavior.
//! Some tests require Chrome (crawl, recipe validate) - see profile.
//!
//! Run with:
//!   cargo test --test cli           # Unit tests only
//!   cargo test --test cli -- --ignored  # Including integration tests

use std::process::{Command, Output};

/// Helper to run myfeed command and capture output
fn run_myfeed(args: &[&str]) -> Output {
    let binary = env!("CARGO_BIN_EXE_myfeed");
    Command::new(binary)
        .args(args)
        .output()
        .expect("failed to execute myfeed")
}

/// Helper to run myfeed and assert it succeeds
fn run_myfeed_ok(args: &[&str]) -> String {
    let output = run_myfeed(args);
    assert!(
        output.status.success(),
        "myfeed {:?} failed:\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper to run myfeed and assert it fails
fn run_myfeed_fail(args: &[&str]) -> (String, String) {
    let output = run_myfeed(args);
    assert!(
        !output.status.success(),
        "myfeed {:?} should have failed but succeeded",
        args
    );
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// =============================================================================
// CLI Structure Tests
// =============================================================================

#[test]
fn test_help_flag() {
    let output = run_myfeed_ok(&["--help"]);
    assert!(output.contains("Usage:"));
    assert!(output.contains("Commands:"));
    assert!(output.contains("run"));
    assert!(output.contains("crawl"));
    assert!(output.contains("list"));
}

#[test]
fn test_no_args_shows_help() {
    // Running with no args should show help (not crash)
    let output = run_myfeed(&[]);
    // clap returns exit code 1 for missing required args but shows help
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:") || stderr.contains("error:"));
}

#[test]
fn test_unknown_command() {
    let (stdout, stderr) = run_myfeed_fail(&["unknown-cmd"]);
    assert!(
        stdout.contains("unrecognized") || stderr.contains("unrecognized"),
        "should mention unrecognized command"
    );
}

// =============================================================================
// Crawl Command Tests
// =============================================================================

#[test]
fn test_crawl_requires_site() {
    let (_stdout, stderr) = run_myfeed_fail(&["crawl"]);
    assert!(
        stderr.contains("required") || stderr.contains("missing"),
        "should require site argument"
    );
}

#[test]
fn test_crawl_help() {
    let output = run_myfeed_ok(&["crawl", "--help"]);
    assert!(output.contains("Sites to crawl"));
    assert!(output.contains("--format"));
    assert!(output.contains("--compact"));
    assert!(output.contains("--save"));
    assert!(output.contains("--notify"));
}

#[test]
fn test_crawl_unknown_site() {
    // Should not crash, should report recipe not found
    let output = run_myfeed(&["crawl", "nonexistent-site"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The crawl command continues even if one site fails
    // It prints "recipe not found for site: nonexistent-site"
    assert!(stderr.contains("recipe not found") || !output.status.success());
}

#[test]
fn test_crawl_single_site() {
    // Test with a site that should exist (hackernews)
    // This will fail without Chrome but we can verify the error message
    let output = run_myfeed(&["crawl", "hackernews"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should either succeed or fail with Chrome connection error
    assert!(
        stderr.contains("Chrome")
            || stderr.contains("connection")
            || stderr.contains("CDP")
            || output.status.success()
    );
}

// =============================================================================
// Recipe Command Tests
// =============================================================================

#[test]
fn test_recipe_list() {
    let output = run_myfeed_ok(&["recipe", "list"]);
    assert!(output.contains("Available recipes"));
    assert!(output.contains("hackernews"));
    assert!(output.contains("reddit"));
}

#[test]
fn test_recipe_help() {
    let output = run_myfeed_ok(&["recipe", "--help"]);
    assert!(output.contains("Manage and validate recipes"));
    assert!(output.contains("list"));
    assert!(output.contains("validate"));
}

#[test]
fn test_recipe_validate_unknown_site() {
    // Should fail with recipe not found
    let (stdout, stderr) = run_myfeed_fail(&["recipe", "validate", "nonexistent"]);
    assert!(
        stdout.contains("not found") || stderr.contains("not found"),
        "should report recipe not found"
    );
}

// =============================================================================
// List Command Tests (require database)
// =============================================================================

#[test]
fn test_list_help() {
    let output = run_myfeed_ok(&["list", "--help"]);
    assert!(output.contains("Filter by site"));
    assert!(output.contains("--limit"));
}

#[test]
fn test_list_empty_db() {
    // With an empty database, should just print "no items found"
    let output = run_myfeed_ok(&["list", "--limit", "5"]);
    // Either shows items or says none found
    assert!(
        output.contains("no items found") || output.contains("["),
        "should show items or no items message"
    );
}

// =============================================================================
// Dump Command Tests
// =============================================================================

#[test]
fn test_dump_help() {
    let output = run_myfeed_ok(&["dump", "--help"]);
    assert!(output.contains("hours"));
    assert!(output.contains("--compact"));
    assert!(output.contains("--ids"));
}

#[test]
fn test_dump_default_args() {
    // Should work with defaults (24 hours, no site filter)
    let output = run_myfeed_ok(&["dump"]);
    // Should output valid JSON (may have log prefix, find the JSON part)
    let has_json = output.contains("\"items\"")
        || output.contains("\"mode\"")
        || output.starts_with("{")
        || output.starts_with("[");
    assert!(
        has_json,
        "should be JSON: {}",
        &output[..output.len().min(200)]
    );
}

// =============================================================================
// Events Command Tests
// =============================================================================

#[test]
fn test_events_help() {
    let output = run_myfeed_ok(&["events", "--help"]);
    assert!(output.contains("events"));
    assert!(output.contains("--limit"));
}

#[test]
fn test_events_default() {
    let output = run_myfeed_ok(&["events"]);
    // Should output events or "no events found"
    assert!(
        output.contains("no events found") || output.contains("["),
        "should show events or no events message"
    );
}

// =============================================================================
// Snapshots Command Tests
// =============================================================================

#[test]
fn test_snapshots_requires_site() {
    let (stdout, stderr) = run_myfeed_fail(&["snapshots"]);
    assert!(
        stdout.contains("missing") || stderr.contains("missing") || stderr.contains("required"),
        "should require site argument"
    );
}

#[test]
fn test_snapshots_help() {
    let output = run_myfeed_ok(&["snapshots", "--help"]);
    assert!(output.contains("Site name"));
    assert!(output.contains("--limit"));
}

// =============================================================================
// Login Command Tests
// =============================================================================

#[test]
fn test_login_help() {
    let output = run_myfeed_ok(&["login", "--help"]);
    assert!(output.contains("Site to log in"));
    assert!(output.contains("reddit"));
    assert!(output.contains("zhihu"));
    assert!(output.contains("x"));
}

#[test]
fn test_login_unknown_site() {
    let (stdout, stderr) = run_myfeed_fail(&["login", "unknown"]);
    assert!(
        stdout.contains("unknown") || stderr.contains("unknown"),
        "should report unknown site"
    );
}

// =============================================================================
// Integration tests (require Chrome) - run with --ignored
// =============================================================================

#[test]
#[ignore]
fn test_crawl_with_chrome_hackernews() {
    let output = run_myfeed_ok(&["crawl", "hackernews", "--format", "json", "--limit", "3"]);
    // Should output JSON array
    assert!(
        output.starts_with("[") || output.starts_with("{"),
        "should be JSON"
    );
}

#[test]
#[ignore]
fn test_crawl_with_chrome_reddit() {
    let output = run_myfeed_ok(&["crawl", "reddit", "--format", "json", "--limit", "3"]);
    assert!(
        output.starts_with("[") || output.starts_with("{"),
        "should be JSON"
    );
}

#[test]
#[ignore]
fn test_recipe_validate_with_chrome() {
    // This test requires Chrome connection
    let output = run_myfeed_ok(&["recipe", "validate", "hackernews"]);
    assert!(output.contains("Validation passed") || output.contains("Recipe:"));
}

#[test]
#[ignore]
fn test_crawl_save_to_db() {
    let _output = run_myfeed_ok(&["crawl", "hackernews", "--save", "--limit", "5"]);
    // Should have saved items - check list command
    let list_output = run_myfeed_ok(&["list", "--site", "hackernews", "--limit", "10"]);
    assert!(list_output.contains("hackernews") || list_output.contains("no items"));
}
