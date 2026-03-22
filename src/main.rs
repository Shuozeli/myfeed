mod config;
mod crawler;
mod db;
mod feed;
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

    let bot = Arc::new(telegram::TelegramBot::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id.clone(),
    ));

    match cli.command {
        Command::Run => {
            info!("starting myfeed daemon");
            scheduler::run(config, db, bot).await;
        }
        Command::Once => {
            info!("running single crawl cycle");
            for site in &config.enabled_sites {
                if let Err(e) = scheduler::crawl_site(&config, &db, &bot, site).await {
                    tracing::error!(site, error = %e, "crawl failed");
                }
            }
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
                other => {
                    eprintln!("unknown site: {other}");
                    eprintln!(
                        "supported: reddit, zhihu, xueqiu, x, linkedin, hackernews, 1point3acres"
                    );
                    std::process::exit(1);
                }
            };

            let browser = pwright_bridge::Browser::connect(config.browser_config())
                .await
                .expect("failed to connect to Chrome");
            let tab = browser.new_tab(url).await.expect("failed to open tab");

            println!("Browser tab opened to {url}");
            println!("Log in manually, then press Enter here when done.");
            println!("Your session cookies will be preserved for future crawls.");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();

            info!(site, "login session established");
            let _ = tab.close().await;
        }
        Command::List { site, limit } => {
            let items = db
                .recent_items(site.as_deref(), limit)
                .expect("failed to fetch items");
            for item in &items {
                let preview = if item.preview.len() > 60 {
                    format!("{}...", &item.preview[..60])
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
