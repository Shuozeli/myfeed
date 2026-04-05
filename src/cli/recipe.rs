//! `myfeed recipe` subcommands.

use std::path::PathBuf;

use pwright_bridge::Browser;
use pwright_script::parser;

fn recipes_base_dir() -> PathBuf {
    match std::env::var("RECIPES_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("recipes")))
            .unwrap_or_else(|| PathBuf::from("recipes")),
    }
}

/// List all available recipes.
pub fn list_recipes() {
    let recipes_dir = recipes_base_dir();
    let mut sites = std::collections::HashSet::new();

    // Scan recipes/ for *-feed.yaml files
    if let Ok(entries) = std::fs::read_dir(&recipes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.to_string_lossy().ends_with("-feed.yaml") {
                let stem = path.file_stem().unwrap().to_string_lossy();
                let name = stem.trim_end_matches("-feed");
                sites.insert(name.to_string());
            }
        }
    }

    let mut sites: Vec<String> = sites.into_iter().collect();
    sites.sort();

    println!("Available recipes ({}):", sites.len());
    for site in sites {
        println!("  {}", site);
    }
}

/// Validate a recipe by parsing it and doing a dry-run in headless Chrome.
pub async fn validate(site: &str, config: &crate::config::Config) -> Result<(), String> {
    let recipe_path = crate::crawler::recipe_path(site);

    if !recipe_path.exists() {
        return Err(format!("recipe not found for site: {}", site));
    }

    // Parse the YAML
    let script = parser::parse_yaml_file(&recipe_path).map_err(|e| format!("parse error: {e}"))?;

    println!("Recipe: {}", script.name);
    println!("Path: {}", recipe_path.display());
    println!("Steps: {}", script.steps.len());
    println!("Scripts: {}", script.scripts.len());

    // Try to connect to Chrome and do a dry-run
    println!("\nConnecting to Chrome...");

    let browser = Browser::connect(pwright_bridge::BrowserConfig {
        cdp_url: config.cdp_endpoint.clone(),
        ..Default::default()
    })
    .await
    .map_err(|e| format!("Chrome connection failed: {e}"))?;

    let tab = browser
        .new_tab("about:blank")
        .await
        .map_err(|e| format!("failed to open tab: {e}"))?;

    println!("Running recipe against blank page...");

    let params: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut sink = pwright_script::output::VecSink::default();

    let result = pwright_script::executor::execute(&script, &tab.page(), &params, &mut sink)
        .await
        .map_err(|e| format!("execution error: {e}"))?;

    let _ = tab.close().await;

    println!("\nResult:");
    println!("  Status: {:?}", result.status);
    println!(
        "  Steps: {} total, {} succeeded",
        result.total_steps, result.succeeded
    );
    println!("  Outputs: {}", result.outputs.len());

    if result.status == pwright_script::executor::ExecutionStatus::Error {
        if let Some(e) = result.error {
            return Err(format!("recipe error: {e}"));
        }
    }

    // Show outputs
    for (i, output) in result.outputs.iter().enumerate() {
        println!(
            "  Output {}: {:?}",
            i + 1,
            output.keys().collect::<Vec<_>>()
        );
    }

    println!("\nValidation passed.");
    Ok(())
}
