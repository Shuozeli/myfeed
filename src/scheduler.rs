use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use pwright_bridge::Browser;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::crawler;
use crate::db::FeedDb;
use crate::proto;
use crate::telegram::TelegramBot;

/// Run a blocking database operation on the tokio blocking thread pool.
async fn db_blocking<F, T>(
    db: &Arc<FeedDb>,
    f: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(&FeedDb) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let db = db.clone();
    Ok(tokio::task::spawn_blocking(move || f(&db)).await??)
}

/// Main scheduler loop. Connects to Chrome, runs feed recipes on a timer.
pub async fn run(config: Arc<Config>, db: Arc<FeedDb>, bot: Arc<TelegramBot>) -> ! {
    let interval = Duration::from_secs(config.crawl_interval_secs);

    info!(
        interval_secs = config.crawl_interval_secs,
        sites = ?config.enabled_sites,
        "scheduler starting"
    );

    loop {
        for site in &config.enabled_sites {
            if let Err(e) = crawl_site(&config, &db, &bot, site).await {
                error!(site, error = %e, "crawl failed");
                let site = site.clone();
                if let Err(log_err) = db_blocking(&db, move |db| {
                    db.log_event(
                        "crawl_error",
                        &site,
                        &serde_json::json!({ "error": e.to_string() }),
                    )
                })
                .await
                {
                    warn!(error = %log_err, "failed to log crawl error to database");
                }
            }
        }

        // Generate Atom feed if configured
        if let Some(ref path) = config.feed_output_path {
            let item_count = config.feed_item_count;
            let path = path.clone();
            match db_blocking(&db, move |db| db.recent_items(None, item_count)).await {
                Ok(items) => {
                    let xml = crate::feed::generate_atom(&items);
                    if let Err(e) = std::fs::write(&path, &xml) {
                        error!(path, error = %e, "failed to write atom feed");
                    } else {
                        info!(path, items = items.len(), "atom feed updated");
                    }
                }
                Err(e) => warn!(error = %e, "failed to query items for atom feed"),
            }
        }

        info!(
            next_in_secs = interval.as_secs(),
            "sleeping until next crawl"
        );
        tokio::time::sleep(interval).await;
    }
}

/// Crawl a single site: connect to Chrome, run recipe, dedup, notify, snapshot.
pub async fn crawl_site(
    config: &Config,
    db: &Arc<FeedDb>,
    bot: &TelegramBot,
    site: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let recipe = crawler::recipe_path(site);
    if !recipe.exists() {
        warn!(site, path = %recipe.display(), "recipe not found, skipping");
        return Ok(());
    }

    info!(site, "starting crawl");
    {
        let site = site.to_string();
        db_blocking(db, move |db| {
            db.log_event("crawl_start", &site, &serde_json::json!({}))
        })
        .await?;
    }

    let browser = Browser::connect(config.browser_config()).await?;
    let tab = browser.new_tab("about:blank").await?;
    let page = tab.page();

    // Run recipe and process results. Always close the tab afterwards.
    let result = crawl_with_page(db, bot, site, &page).await;
    if let Err(e) = tab.close().await {
        warn!(site, error = %e, "failed to close tab");
    }
    let (items_len, new_count) = result?;

    {
        let site = site.to_string();
        db_blocking(db, move |db| {
            db.log_event(
                "crawl_complete",
                &site,
                &serde_json::json!({
                    "items_found": items_len,
                    "new_items": new_count,
                }),
            )
        })
        .await?;
    }

    info!(site, found = items_len, new = new_count, "crawl complete");
    Ok(())
}

/// Inner crawl logic separated so tab.close() always runs regardless of errors.
async fn crawl_with_page(
    db: &Arc<FeedDb>,
    bot: &TelegramBot,
    site: &str,
    page: &pwright_bridge::playwright::Page,
) -> Result<(usize, u32), Box<dyn std::error::Error + Send + Sync>> {
    let params = HashMap::new();
    let recipe = crawler::recipe_path(site);
    let items = crawler::run_recipe(page, site, &recipe, &params).await?;

    info!(site, items_found = items.len(), "recipe produced items");

    // Save snapshot
    {
        let snapshot = proto::CrawlSnapshot {
            site: site.to_string(),
            crawled_at: Utc::now().to_rfc3339(),
            items: items.clone(),
        };
        db_blocking(db, move |db| db.save_snapshot(&snapshot)).await?;
    }

    // Dedup + notify using atomic INSERT OR IGNORE
    let mut new_count = 0u32;
    for item in &items {
        let raw = serde_json::to_value(item).unwrap_or_default();
        let db_site = site.to_string();
        let db_item = item.clone();
        let is_new = db_blocking(db, move |db| {
            db.insert_if_new(
                &db_site,
                &db_item.id,
                &db_item.title,
                &db_item.url,
                &db_item.preview,
                &raw,
            )
        })
        .await?;

        if !is_new {
            continue;
        }

        if let Err(e) = bot
            .send_feed_item(site, &item.title, &item.url, &item.preview)
            .await
        {
            error!(site, title = %item.title, error = %e, "telegram send failed");
        }

        new_count += 1;
    }

    Ok((items.len(), new_count))
}
