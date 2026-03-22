use std::env;

/// Application configuration. All fields are required -- missing values
/// cause an immediate panic at startup (fail-fast).
#[derive(Debug, Clone)]
pub struct Config {
    pub cdp_endpoint: String,
    pub database_url: String,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub crawl_interval_secs: u64,
    pub enabled_sites: Vec<String>,
    /// Optional: path to write Atom feed XML. Empty/unset = disabled.
    pub feed_output_path: Option<String>,
    /// Number of items to include in the Atom feed. Default: 100.
    pub feed_item_count: i64,
}

impl Config {
    pub fn browser_config(&self) -> pwright_bridge::BrowserConfig {
        pwright_bridge::BrowserConfig {
            cdp_url: self.cdp_endpoint.clone(),
            ..Default::default()
        }
    }

    pub fn from_env() -> Self {
        Self {
            cdp_endpoint: required("CDP_ENDPOINT"),
            database_url: required("DATABASE_URL"),
            telegram_bot_token: required("TELEGRAM_BOT_TOKEN"),
            telegram_chat_id: required("TELEGRAM_CHAT_ID"),
            crawl_interval_secs: required("CRAWL_INTERVAL_SECS")
                .parse::<u64>()
                .expect("CRAWL_INTERVAL_SECS must be a positive integer"),
            enabled_sites: required("ENABLED_SITES")
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            feed_output_path: env::var("FEED_OUTPUT_PATH").ok().filter(|s| !s.is_empty()),
            feed_item_count: env::var("FEED_ITEM_COUNT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
        }
    }
}

fn required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("missing required env var: {key}"))
}
