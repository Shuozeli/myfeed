mod config;
mod crawler;
mod db;
mod feed;
pub mod notifier;
pub mod proto;
mod scheduler;
mod schema;
mod telegram;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "myfeed", about = "Personal feed bot powered by pwright")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the feed crawler daemon.
    Run,

    /// Run a single crawl cycle and exit (useful for testing).
    Once,

    /// Open a browser tab for manual login. The user logs in,
    /// then the session cookies persist for future crawls.
    Login {
        /// Site to log in to (reddit, zhihu, weibo, x).
        site: String,
    },

    /// List recent feed items from the database.
    List {
        /// Filter by site name (e.g., reddit).
        #[arg(short, long)]
        site: Option<String>,

        /// Number of items to show.
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },

    /// List crawl snapshots for a site.
    Snapshots {
        /// Site name (required).
        site: String,

        /// Number of snapshots to show.
        #[arg(short, long, default_value = "10")]
        limit: i64,
    },

    /// Show a specific snapshot by ID.
    Snapshot {
        /// Snapshot ID.
        id: i32,
    },

    /// Dump recent feed items as JSON for agent consumption.
    Dump {
        /// Hours of history to include (default: 24).
        #[arg(long, default_value = "24")]
        hours: u64,

        /// Filter by site (repeatable).
        #[arg(long)]
        site: Vec<String>,

        /// Compact mode: title + url only, no preview (default for index).
        #[arg(long)]
        compact: bool,

        /// Fetch full details for specific item IDs (comma-separated).
        #[arg(long, value_delimiter = ',')]
        ids: Vec<i32>,
    },

    /// Show recent events from the event log.
    Events {
        /// Number of recent events to show.
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "myfeed=info".parse().expect("valid filter")),
        )
        .init();

    let cli = Cli::parse();
    let config = Arc::new(config::Config::from_env());

    let db = Arc::new(db::FeedDb::new(&config.database_url));
    db.migrate();

    let notifier = notifier::create_notifier(&config);

    match cli.command {
        Command::Run => {
            info!("starting myfeed daemon");
            scheduler::run(config, db, notifier).await;
        }
        Command::Once => {
            info!("running single crawl cycle");
            for site in &config.enabled_sites {
                if let Err(e) = scheduler::crawl_site(&config, &db, &notifier, site).await {
                    tracing::error!(site, error = %e, "crawl failed");
                }
            }
            // Wait a bit for queued messages to drain
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            info!("single crawl cycle complete");
        }
        Command::Login { site } => {
            info!(site, "opening browser for manual login");
            let url = match site.as_str() {
                "reddit" => "https://www.reddit.com/login",
                "zhihu" => "https://www.zhihu.com/signin",
                "xueqiu" => "https://xueqiu.com/",
                "x" => "https://x.com/i/flow/login",
                "linkedin" => "https://www.linkedin.com/login",
                "hackernews" => "https://news.ycombinator.com/login",
                "1point3acres" => {
                    "https://www.1point3acres.com/bbs/member.php?mod=logging&action=login"
                }
                "wsj" => "https://accounts.wsj.com/login",
                "nyt" => "https://myaccount.nytimes.com/auth/login",
                "businessinsider" => "https://www.businessinsider.com/login",
                "cls" => "https://www.cls.cn",
                "peoplesdaily" => "https://paper.people.com.cn",
                "latepost" => "https://www.latepost.com",
                other => {
                    eprintln!("unknown site: {other}");
                    eprintln!(
                        "supported: reddit, zhihu, xueqiu, x, linkedin, hackernews, \
                         1point3acres, wsj, nyt, businessinsider, cls, peoplesdaily, latepost"
                    );
                    std::process::exit(1);
                }
            };

            let browser = pwright_bridge::Browser::connect(pwright_bridge::BrowserConfig {
                cdp_url: config.cdp_endpoint.clone(),
                ..Default::default()
            })
            .await
            .expect("failed to connect to browser");
            let tab = browser.new_tab(url).await.expect("failed to open tab");

            println!("Browser tab opened to {url}");
            println!("Log in manually, then press Enter here when done.");
            println!("Your session cookies will be preserved for future crawls.");

            let mut input = String::new();
            if let Err(e) = std::io::stdin().read_line(&mut input) {
                eprintln!("failed to read from stdin: {e}");
                eprintln!("run this command in an interactive terminal");
                std::process::exit(1);
            }

            info!(site, "login session established");
            let _ = tab.close().await;
        }
        Command::List { site, limit } => {
            let items = db
                .recent_items(site.as_deref(), limit)
                .expect("failed to fetch items");
            for item in &items {
                let preview = if item.preview.chars().count() > 60 {
                    format!("{}...", item.preview.chars().take(60).collect::<String>())
                } else {
                    item.preview.clone()
                };
                println!(
                    "[{}] {} | {} | {}",
                    item.site, item.title, item.url, preview
                );
            }
            if items.is_empty() {
                println!("no items found");
            }
        }
        Command::Snapshots { site, limit } => {
            let snapshots = db
                .recent_snapshots(&site, limit)
                .expect("failed to fetch snapshots");
            for s in &snapshots {
                println!(
                    "[{}] #{} | {} | {} items",
                    s.crawled_at, s.id, s.site, s.item_count
                );
            }
            if snapshots.is_empty() {
                println!("no snapshots found for {site}");
            }
        }
        Command::Snapshot { id } => {
            match db.get_snapshot(id).expect("failed to fetch snapshot") {
                Some(s) => {
                    println!("Snapshot #{} | {} | {}", s.id, s.site, s.crawled_at);
                    println!("Items: {}", s.item_count);
                    println!("---");
                    // Pretty-print the items JSON
                    match serde_json::from_str::<serde_json::Value>(&s.items_json) {
                        Ok(v) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&v).unwrap_or(s.items_json)
                            );
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to parse snapshot JSON, showing raw");
                            println!("{}", s.items_json);
                        }
                    }
                }
                None => println!("snapshot #{id} not found"),
            }
        }
        Command::Dump {
            hours,
            site,
            compact,
            ids,
        } => handle_dump(&db, hours, &site, compact, &ids),
        Command::Events { limit } => {
            let events = db.recent_events(limit).expect("failed to fetch events");
            for event in &events {
                println!(
                    "[{}] {} | {} | {}",
                    event.created_at, event.event_type, event.site, event.details
                );
            }
            if events.is_empty() {
                println!("no events found");
            }
        }
    }
}

/// Handle the `dump` subcommand: output feed items as JSON for agent consumption.
fn handle_dump(db: &db::FeedDb, hours: u64, sites: &[String], compact: bool, ids: &[i32]) {
    // Mode 1: Fetch specific items by ID (full details)
    if !ids.is_empty() {
        let items = db.items_by_ids(ids).expect("failed to fetch items");
        let items_json: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "site": item.site,
                    "title": item.title,
                    "url": item.url,
                    "preview": item.preview,
                    "raw_json": item.raw_json,
                    "created_at": item.created_at,
                })
            })
            .collect();
        let output = serde_json::json!({
            "mode": "detail",
            "total_items": items.len(),
            "items": items_json,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("JSON serialization is infallible")
        );
        return;
    }

    // Mode 2: Time-range index (compact by default)
    #[allow(clippy::cast_possible_wrap)] // hours is small (< 1000)
    let since = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
    let since_str = since.to_rfc3339();
    let items = db
        .items_since(&since_str, sites)
        .expect("failed to fetch items");

    let mut site_counts = std::collections::HashMap::<&str, usize>::new();
    for item in &items {
        *site_counts.entry(&item.site).or_default() += 1;
    }

    let items_json: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            let mut obj = serde_json::json!({
                "id": item.id,
                "site": item.site,
                "title": item.title,
                "url": item.url,
            });
            if !compact {
                obj["preview"] = serde_json::json!(item.preview);
                obj["created_at"] = serde_json::json!(item.created_at);
            }
            obj
        })
        .collect();

    let output = serde_json::json!({
        "mode": if compact { "index" } else { "full" },
        "period": format!("{} to {}", since_str, chrono::Utc::now().to_rfc3339()),
        "total_items": items.len(),
        "sites": site_counts,
        "items": items_json,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("JSON serialization is infallible")
    );
}
