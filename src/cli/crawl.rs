//! `myfeed crawl` implementation.

use std::sync::Arc;

use chrono::Utc;
use pwright_bridge::Browser;
use tracing::{error, info};

use crate::config::Config;
use crate::crawler::{self, CrawlError};
use crate::db::FeedDb;
use crate::notifier::Notifier;
use crate::proto;

/// Output format for crawl results.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OutputFormat {
    #[default]
    Json,
    Jsonl,
    Table,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "jsonl" => Ok(OutputFormat::Jsonl),
            "table" => Ok(OutputFormat::Table),
            _ => Err(format!("unknown format: {s}")),
        }
    }
}

/// One site's worth of crawl results.
struct SiteResult {
    site: String,
    items: Vec<proto::FeedItem>,
}

/// Run crawl for one or more sites, output to stdout.
#[allow(clippy::too_many_arguments)]
pub async fn run_crawl(
    config: &Config,
    db: Option<&FeedDb>,
    notifier: Option<&Arc<dyn Notifier>>,
    sites: &[String],
    params: &[(String, String)],
    limit: Option<usize>,
    format: OutputFormat,
    compact: bool,
    save: bool,
    notify: bool,
) -> Result<(), CrawlError> {
    let browser = Browser::connect(pwright_bridge::BrowserConfig {
        cdp_url: config.cdp_endpoint.clone(),
        ..Default::default()
    })
    .await
    .map_err(|e| CrawlError::Execute(e.to_string()))?;

    let tab = browser
        .new_tab("about:blank")
        .await
        .map_err(|e| CrawlError::Execute(e.to_string()))?;

    let page = tab.page();
    let mut site_results: Vec<SiteResult> = Vec::new();

    for site in sites {
        let recipe_path = crawler::recipe_path(site);
        if !recipe_path.exists() {
            eprintln!("recipe not found for site: {site}");
            continue;
        }

        let params_map: std::collections::HashMap<String, String> = params
            .iter()
            .filter(|(k, _)| k.starts_with("param."))
            .map(|(k, v)| (k.trim_start_matches("param.").to_string(), v.clone()))
            .collect();

        info!(site, recipe = %recipe_path.display(), "running recipe");

        let items = crawler::run_recipe(&page, site, &recipe_path, &params_map).await?;

        if items.is_empty() {
            info!(site, "no items found");
            continue;
        }

        // Save to DB if requested
        if save
            && let Some(db) = db {
                let snapshot = proto::CrawlSnapshot {
                    site: site.clone(),
                    crawled_at: Utc::now().to_rfc3339(),
                    items: items.clone(),
                };
                if let Err(e) = db.save_snapshot(&snapshot) {
                    error!(site, error = %e, "failed to save snapshot");
                }
                for item in &items {
                    let raw =
                        serde_json::to_value(item).expect("serializing FeedItem is infallible");
                    if let Err(e) =
                        db.insert_item(site, &item.id, &item.title, &item.url, &item.preview, &raw)
                    {
                        error!(site, error = %e, "failed to insert item");
                    }
                }
            }

        // Send to notifier if requested
        if notify
            && let Some(notifier) = notifier {
                for item in &items {
                    notifier
                        .notify_feed_item(site, &item.title, &item.url, &item.preview)
                        .await;
                }
            }

        site_results.push(SiteResult {
            site: site.clone(),
            items,
        });
    }

    // Close tab
    let _ = tab.close().await;

    // Flatten and apply limit
    let flat: Vec<(&str, &proto::FeedItem)> = site_results
        .iter()
        .flat_map(|r| r.items.iter().map(move |i| (r.site.as_str(), i)))
        .collect();

    let to_show = if let Some(n) = limit {
        flat.into_iter().take(n).collect::<Vec<_>>()
    } else {
        flat
    };

    // Output
    match format {
        OutputFormat::Json => print_json(&to_show, compact),
        OutputFormat::Jsonl => print_jsonl(&to_show, compact),
        OutputFormat::Table => print_table(&to_show, compact),
    }

    Ok(())
}

fn print_json(items: &[(&str, &proto::FeedItem)], compact: bool) {
    let objects: Vec<serde_json::Value> = items
        .iter()
        .map(|(site, item)| {
            let mut obj = serde_json::json!({
                "id": item.id,
                "site": site,
                "title": item.title,
                "url": item.url,
            });
            if !compact {
                obj["preview"] = serde_json::json!(item.preview);
                if let Some(ref sd) = item.site_data {
                    obj["site_data"] = site_data_to_json(sd);
                }
            }
            obj
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&objects).unwrap_or_default()
    );
}

fn print_jsonl(items: &[(&str, &proto::FeedItem)], compact: bool) {
    for (site, item) in items {
        let mut obj = serde_json::json!({
            "id": item.id,
            "site": site,
            "title": item.title,
            "url": item.url,
        });
        if !compact {
            obj["preview"] = serde_json::json!(item.preview);
            if let Some(ref sd) = item.site_data {
                obj["site_data"] = site_data_to_json(sd);
            }
        }
        println!("{}", serde_json::to_string(&obj).unwrap_or_default());
    }
}

fn print_table(items: &[(&str, &proto::FeedItem)], _compact: bool) {
    for (site, item) in items {
        let preview = if item.preview.len() > 50 {
            format!("{}...", &item.preview[..50])
        } else {
            item.preview.clone()
        };
        println!("[{}] {} | {} | {}", site, item.title, item.url, preview);
    }
}

fn site_data_to_json(sd: &proto::feed_item::SiteData) -> serde_json::Value {
    match sd {
        proto::feed_item::SiteData::Hackernews(h) => serde_json::json!({
            "score": h.score,
            "comments": h.comments,
            "age": h.age,
        }),
        proto::feed_item::SiteData::Reddit(r) => serde_json::json!({
            "subreddit": r.subreddit,
            "upvotes": r.upvotes,
            "comments": r.comments,
            "author": r.author,
        }),
        proto::feed_item::SiteData::Xueqiu(x) => serde_json::json!({
            "author": x.author,
            "replies": x.replies,
            "likes": x.likes,
            "symbol": x.symbol,
        }),
        proto::feed_item::SiteData::Zhihu(z) => serde_json::json!({
            "upvotes": z.upvotes,
            "answers": z.answers,
            "author": z.author,
            "topic": z.topic,
        }),
        proto::feed_item::SiteData::Linkedin(l) => serde_json::json!({
            "author": l.author,
            "likes": l.likes,
            "comments": l.comments,
            "company": l.company,
        }),
        proto::feed_item::SiteData::X(x) => serde_json::json!({
            "author": x.author,
            "likes": x.likes,
            "retweets": x.retweets,
            "replies": x.replies,
        }),
        _ => serde_json::json!({}),
    }
}
