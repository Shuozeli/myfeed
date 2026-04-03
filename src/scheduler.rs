use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use pwright_bridge::Browser;
use tracing::{error, info, warn};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::crawler;
use crate::db::FeedDb;
use crate::notifier::Notifier;
use crate::proto;

/// Typed event detail structs for `event_log` entries.
/// These ensure compile-time consistency between writers (scheduler) and readers (db).
#[derive(Serialize, Deserialize)]
pub struct CrawlCompleteEvent {
    pub items_found: usize,
    pub new_items: u32,
}

#[derive(Serialize, Deserialize)]
pub struct CrawlErrorEvent {
    pub error: String,
}

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
pub async fn run(config: Arc<Config>, db: Arc<FeedDb>, notifier: Arc<dyn Notifier>) -> ! {
    let interval = Duration::from_secs(config.crawl_interval_secs);

    info!(
        interval_secs = config.crawl_interval_secs,
        sites = ?config.enabled_sites,
        digest_mode = config.digest_mode,
        filter_keywords = ?config.filter_keywords,
        "scheduler starting"
    );

    loop {
        // Cleanup old event log entries (>30 days)
        if let Err(e) = db_blocking(&db, |db| db.cleanup_old_events(30)).await {
            warn!(error = %e, "failed to cleanup old events");
        }

        for site in &config.enabled_sites {
            let mut last_err = None;
            for attempt in 1..=3 {
                // Per-site timeout: 2 minutes max per attempt
                let result = tokio::time::timeout(
                    Duration::from_secs(120),
                    crawl_site(&config, &db, &notifier, site),
                )
                .await;

                match result {
                    Ok(Ok(())) => {
                        last_err = None;
                        break;
                    }
                    Ok(Err(e)) => {
                        if attempt < 3 {
                            warn!(site, attempt, error = %e, "crawl failed, retrying in 5s");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                        last_err = Some(e);
                    }
                    Err(_) => {
                        let msg: Box<dyn std::error::Error + Send + Sync> =
                            "crawl timed out after 120s".into();
                        if attempt < 3 {
                            warn!(site, attempt, "crawl timed out, retrying in 5s");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                        last_err = Some(msg);
                    }
                }
            }
            if let Some(e) = last_err {
                error!(site, error = %e, "crawl failed after 3 attempts");
                let site = site.clone();
                let event = CrawlErrorEvent {
                    error: e.to_string(),
                };
                let details = serde_json::to_value(&event)
                    .expect("serializing CrawlErrorEvent is infallible");
                if let Err(log_err) =
                    db_blocking(&db, move |db| db.log_event("crawl_error", &site, &details)).await
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
    notifier: &Arc<dyn Notifier>,
    site: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let recipe = crawler::recipe_path(site);
    if !recipe.exists() {
        warn!(site, path = %recipe.display(), "recipe not found, skipping");
        return Ok(());
    }

    let owned_site = site.to_string();

    info!(site, "starting crawl");
    {
        let s = owned_site.clone();
        db_blocking(db, move |db| {
            db.log_event("crawl_start", &s, &serde_json::json!({}))
        })
        .await?;
    }

    let browser = Browser::connect(pwright_bridge::BrowserConfig {
        cdp_url: config.cdp_endpoint.clone(),
        ..Default::default()
    })
    .await?;
    let tab = browser.new_tab("about:blank").await?;
    let page = tab.page();

    // Run recipe and process results. Always close the tab afterwards.
    let result = crawl_with_page(config, db, notifier, site, &page).await;
    if let Err(e) = tab.close().await {
        warn!(site, error = %e, "failed to close tab");
    }
    let (items_len, new_count) = result?;

    {
        let s = owned_site.clone();
        let event = CrawlCompleteEvent {
            items_found: items_len,
            new_items: new_count,
        };
        let details =
            serde_json::to_value(&event).expect("serializing CrawlCompleteEvent is infallible");
        db_blocking(db, move |db| db.log_event("crawl_complete", &s, &details)).await?;
    }

    info!(site, found = items_len, new = new_count, "crawl complete");

    // Check for stale recipe: warn after 3 consecutive crawls with 0 items
    if items_len == 0 {
        let s = owned_site.clone();
        let streak = db_blocking(db, move |db| db.consecutive_empty_crawls(&s)).await?;
        if streak >= 3 {
            let msg = format!(
                "Warning: {site} has returned 0 items for {streak} consecutive crawls. \
                 The recipe may need updating."
            );
            warn!(site, streak, "stale recipe detected");
            notifier.notify_message(&msg).await;
        }
    }

    Ok(())
}

/// Inner crawl logic separated so `tab.close()` always runs regardless of errors.
async fn crawl_with_page(
    config: &Config,
    db: &Arc<FeedDb>,
    notifier: &Arc<dyn crate::notifier::Notifier>,
    site: &str,
    page: &pwright_bridge::playwright::Page,
) -> Result<(usize, u32), Box<dyn std::error::Error + Send + Sync>> {
    let owned_site = site.to_string();
    let params = HashMap::new();
    let recipe = crawler::recipe_path(site);
    let items = crawler::run_recipe(page, site, &recipe, &params).await?;

    info!(site, items_found = items.len(), "recipe produced items");

    // Save snapshot (all items, regardless of filter)
    {
        let snapshot = proto::CrawlSnapshot {
            site: owned_site.clone(),
            crawled_at: Utc::now().to_rfc3339(),
            items: items.clone(),
        };
        db_blocking(db, move |db| db.save_snapshot(&snapshot)).await?;
    }

    // Dedup check + insert: check first, then insert
    let dedup_window = config.dedup_window_for(site);
    let mut new_items: Vec<proto::FeedItem> = Vec::new();
    for item in &items {
        // Check dedup before inserting
        let s = owned_site.clone();
        let eid = item.id.clone();
        let was_seen =
            db_blocking(db, move |db| db.was_seen_recently(&s, &eid, dedup_window)).await?;

        // Always insert (for snapshots/history)
        let raw = serde_json::to_value(item).expect("serializing proto FeedItem is infallible");
        let s = owned_site.clone();
        let db_item = item.clone();
        db_blocking(db, move |db| {
            db.insert_item(
                &s,
                &db_item.id,
                &db_item.title,
                &db_item.url,
                &db_item.preview,
                &raw,
            )
        })
        .await?;

        if !was_seen {
            new_items.push(item.clone());
        }
    }

    // Apply keyword filter (items are already saved to DB regardless)
    let filtered: Vec<&proto::FeedItem> = new_items
        .iter()
        .filter(|item| config.matches_filter(&item.title, &item.preview))
        .collect();

    #[allow(clippy::cast_possible_truncation)] // filtered items count is small (< 100)
    let new_count = filtered.len() as u32;

    if filtered.is_empty() {
        return Ok((items.len(), 0));
    }

    // Enqueue notifications (consumer drains at 1 msg/sec)
    if config.digest_mode {
        let digest = format_digest(site, &filtered);
        notifier.notify_message(&digest).await;
    } else {
        for item in &filtered {
            notifier
                .notify_feed_item(site, &item.title, &item.url, &item.preview)
                .await;
        }
    }

    Ok((items.len(), new_count))
}

/// Format a digest message for Telegram (HTML).
fn format_digest(site: &str, items: &[&proto::FeedItem]) -> String {
    use std::fmt::Write;
    let mut msg = format!("<b>[{}] {} new items</b>\n\n", site, items.len());
    for item in items {
        let title = crate::telegram::escape_html(&item.title);
        if item.url.is_empty() {
            let _ = writeln!(msg, "- {title}");
        } else {
            let _ = writeln!(msg, "- <a href=\"{}\">{title}</a>", item.url);
        }
    }
    msg
}
