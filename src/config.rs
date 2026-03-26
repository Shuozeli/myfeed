use std::collections::HashMap;
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
    /// Optional keyword filter. Only items matching a keyword are sent to Telegram.
    /// All items are still saved to DB for snapshots/dump. Empty = no filter (send all).
    pub filter_keywords: Vec<String>,
    /// Digest mode: batch new items into a single Telegram message per crawl cycle
    /// instead of one message per item.
    pub digest_mode: bool,
    /// Default dedup window in hours. 0 = forever (never re-notify). Default: 0.
    pub dedup_window_hours: u64,
    /// Per-site dedup window overrides. Key: site name, Value: hours.
    pub dedup_overrides: HashMap<String, u64>,
}

impl Config {
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
            filter_keywords: env::var("FILTER_KEYWORDS")
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|k| k.trim().to_lowercase())
                        .filter(|k| !k.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            digest_mode: env::var("DIGEST_MODE")
                .ok()
                .is_some_and(|s| s == "true" || s == "1"),
            dedup_window_hours: env::var("DEDUP_WINDOW_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            dedup_overrides: env::var("DEDUP_OVERRIDES")
                .ok()
                .map(|s| {
                    s.split(',')
                        .filter_map(|pair| {
                            let parts: Vec<&str> = pair.trim().splitn(2, ':').collect();
                            if parts.len() == 2 {
                                Some((
                                    parts[0].trim().to_lowercase(),
                                    parts[1].trim().parse::<u64>().ok()?,
                                ))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Get the dedup window for a site in hours. 0 = forever.
    pub fn dedup_window_for(&self, site: &str) -> u64 {
        self.dedup_overrides
            .get(site)
            .copied()
            .unwrap_or(self.dedup_window_hours)
    }

    /// Check if an item matches the keyword filter.
    /// Returns true if no keywords configured (pass-through) or any keyword matches.
    pub fn matches_filter(&self, title: &str, preview: &str) -> bool {
        if self.filter_keywords.is_empty() {
            return true;
        }
        let title_lower = title.to_lowercase();
        let preview_lower = preview.to_lowercase();
        self.filter_keywords
            .iter()
            .any(|kw| title_lower.contains(kw) || preview_lower.contains(kw))
    }
}

fn required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("missing required env var: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(keywords: Vec<&str>) -> Config {
        Config {
            cdp_endpoint: String::new(),
            database_url: String::new(),
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
            crawl_interval_secs: 1800,
            enabled_sites: vec![],
            feed_output_path: None,
            feed_item_count: 100,
            filter_keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            digest_mode: false,
            dedup_window_hours: 0,
            dedup_overrides: HashMap::new(),
        }
    }

    #[test]
    fn filter_empty_keywords_passes_everything() {
        let config = test_config(vec![]);
        assert!(config.matches_filter("anything", "at all"));
    }

    #[test]
    fn filter_matches_title() {
        let config = test_config(vec!["rust", "ai"]);
        assert!(config.matches_filter("Learning Rust in 2026", ""));
        assert!(config.matches_filter("AI is changing everything", ""));
        assert!(!config.matches_filter("Python tutorial", "web dev"));
    }

    #[test]
    fn filter_matches_preview() {
        let config = test_config(vec!["tariff"]);
        assert!(config.matches_filter("Trade news", "New tariff on imports"));
        assert!(!config.matches_filter("Trade news", "Exports rise"));
    }

    #[test]
    fn filter_case_insensitive() {
        let config = test_config(vec!["rust"]);
        assert!(config.matches_filter("RUST 2026", ""));
        assert!(config.matches_filter("Rust Foundation", ""));
        assert!(config.matches_filter("rust-lang", ""));
    }

    #[test]
    fn dedup_window_default() {
        let config = test_config(vec![]);
        assert_eq!(config.dedup_window_for("reddit"), 0);
    }

    #[test]
    fn dedup_window_override() {
        let mut config = test_config(vec![]);
        config.dedup_window_hours = 0;
        config.dedup_overrides.insert("weibo-hot".to_string(), 24);
        config
            .dedup_overrides
            .insert("github-trending".to_string(), 48);
        assert_eq!(config.dedup_window_for("weibo-hot"), 24);
        assert_eq!(config.dedup_window_for("github-trending"), 48);
        assert_eq!(config.dedup_window_for("reddit"), 0);
    }
}
