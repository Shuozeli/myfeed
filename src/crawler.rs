use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pwright_bridge::playwright::Page;
use pwright_script::executor::{self, ExecutionStatus};
use pwright_script::output::VecSink;
use pwright_script::parser;
use tracing::{error, info};

use crate::proto;

/// Run a feed recipe against a browser page and return typed FeedItems.
pub async fn run_recipe(
    page: &Page,
    site: &str,
    recipe_path: &Path,
    params: &HashMap<String, String>,
) -> Result<Vec<proto::FeedItem>, CrawlError> {
    let script =
        parser::parse_yaml_file(recipe_path).map_err(|e| CrawlError::Parse(e.to_string()))?;

    let mut sink = VecSink::default();
    let result = executor::execute(&script, page, params, &mut sink)
        .await
        .map_err(|e| CrawlError::Execute(e.to_string()))?;

    if result.status == ExecutionStatus::Error {
        return Err(CrawlError::Execute(
            result.error.unwrap_or_else(|| "unknown error".to_string()),
        ));
    }

    info!(
        recipe = %recipe_path.display(),
        steps = result.total_steps,
        succeeded = result.succeeded,
        outputs = result.outputs.len(),
        "recipe execution complete"
    );

    let mut items = Vec::new();
    for output in &result.outputs {
        if let Some(items_json) = output.get("items") {
            match serde_json::from_str::<Vec<serde_json::Value>>(items_json) {
                Ok(arr) => {
                    for obj in &arr {
                        if let Some(item) = value_to_feed_item(site, obj) {
                            items.push(item);
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "failed to parse items JSON from recipe output");
                }
            }
        }
    }

    Ok(items)
}

/// Convert a raw JSON value from a recipe into a typed FeedItem proto.
fn value_to_feed_item(site: &str, v: &serde_json::Value) -> Option<proto::FeedItem> {
    let id = v.get("id").and_then(|v| v.as_str())?.to_string();
    let title = v.get("title").and_then(|v| v.as_str())?.to_string();
    let url = str_field(v, "url");
    let preview = str_field(v, "preview");

    let site_data = match site {
        "hackernews" => Some(proto::feed_item::SiteData::Hackernews(
            proto::HackerNewsData {
                score: int_field(v, "score"),
                comments: int_field(v, "comments"),
                age: str_field(v, "age"),
                site_url: str_field(v, "site_url"),
            },
        )),
        "reddit" => Some(proto::feed_item::SiteData::Reddit(proto::RedditData {
            subreddit: str_field(v, "subreddit"),
            upvotes: int_field(v, "upvotes"),
            comments: int_field(v, "comments"),
            author: str_field(v, "author"),
        })),
        "1point3acres" => Some(proto::feed_item::SiteData::OnePoint3Acres(
            proto::OnePoint3AcresData {
                forum: str_field(v, "forum"),
                author: str_field(v, "author"),
                post_content: str_field(v, "post_content"),
            },
        )),
        "zhihu" => Some(proto::feed_item::SiteData::Zhihu(proto::ZhihuData {
            upvotes: int_field(v, "upvotes"),
            answers: int_field(v, "answers"),
            author: str_field(v, "author"),
            topic: str_field(v, "topic"),
        })),
        "xueqiu" => Some(proto::feed_item::SiteData::Xueqiu(proto::XueqiuData {
            author: str_field(v, "author"),
            replies: int_field(v, "replies"),
            likes: int_field(v, "likes"),
            symbol: str_field(v, "symbol"),
        })),
        "linkedin" => Some(proto::feed_item::SiteData::Linkedin(proto::LinkedInData {
            author: str_field(v, "author"),
            likes: int_field(v, "likes"),
            comments: int_field(v, "comments"),
            company: str_field(v, "company"),
        })),
        "x" => Some(proto::feed_item::SiteData::X(proto::XData {
            author: str_field(v, "author"),
            likes: int_field(v, "likes"),
            retweets: int_field(v, "retweets"),
            replies: int_field(v, "replies"),
            datetime: str_field(v, "datetime"),
        })),
        other => {
            tracing::warn!(
                site = other,
                "unknown site, no typed site_data in proto oneof"
            );
            None
        }
    };

    Some(proto::FeedItem {
        id,
        title,
        url,
        preview,
        site_data,
    })
}

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn int_field(v: &serde_json::Value, key: &str) -> i32 {
    v.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}

/// Resolve the recipe file path for a given site name.
/// Uses RECIPES_DIR env var if set, otherwise resolves relative to the executable.
pub fn recipe_path(site: &str) -> PathBuf {
    let base = match std::env::var("RECIPES_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            // Fall back to recipes/ relative to the executable's directory
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("recipes")))
                .unwrap_or_else(|| PathBuf::from("recipes"))
        }
    };
    base.join(format!("{site}-feed.yaml"))
}

#[derive(Debug)]
pub enum CrawlError {
    Parse(String),
    Execute(String),
}

impl std::fmt::Display for CrawlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrawlError::Parse(e) => write!(f, "recipe parse error: {e}"),
            CrawlError::Execute(e) => write!(f, "recipe execution error: {e}"),
        }
    }
}

impl std::error::Error for CrawlError {}
