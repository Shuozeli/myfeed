use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pwright_bridge::playwright::Page;
use pwright_script::executor::{self, ExecutionStatus};
use pwright_script::output::VecSink;
use pwright_script::parser;
use tracing::{error, info};

use crate::proto;

/// Run a feed recipe against a browser page and return typed `FeedItem`s.
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

/// Convert a raw JSON value from a recipe into a typed `FeedItem` proto.
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

#[allow(clippy::cast_possible_truncation)] // proto fields are i32; JSON values fit
fn int_field(v: &serde_json::Value, key: &str) -> i32 {
    v.get(key).and_then(serde_json::Value::as_i64).unwrap_or(0) as i32
}

/// Resolve the recipe file path for a given site name.
/// Checks private recipes first (recipes/private/), then public (recipes/).
/// Uses `RECIPES_DIR` env var if set, otherwise resolves relative to the executable.
pub fn recipe_path(site: &str) -> PathBuf {
    let base = match std::env::var("RECIPES_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("recipes")))
            .unwrap_or_else(|| PathBuf::from("recipes")),
    };
    let filename = format!("{site}-feed.yaml");

    // Check private recipes first
    let private_path = base.join("private").join(&filename);
    if private_path.exists() {
        return private_path;
    }

    base.join(filename)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_field_extracts_string() {
        let v = serde_json::json!({"name": "hello", "count": 42});
        assert_eq!(str_field(&v, "name"), "hello");
        assert_eq!(str_field(&v, "missing"), "");
        assert_eq!(str_field(&v, "count"), ""); // not a string
    }

    #[test]
    fn int_field_extracts_number() {
        let v = serde_json::json!({"score": 142, "name": "test"});
        assert_eq!(int_field(&v, "score"), 142);
        assert_eq!(int_field(&v, "missing"), 0);
        assert_eq!(int_field(&v, "name"), 0); // not a number
    }

    #[test]
    fn value_to_feed_item_hackernews() {
        let v = serde_json::json!({
            "id": "123",
            "title": "Test Story",
            "url": "https://example.com",
            "preview": "Some preview",
            "score": 200,
            "comments": 50,
            "age": "3 hours ago",
            "site_url": "https://news.ycombinator.com/item?id=123"
        });
        let item = value_to_feed_item("hackernews", &v).unwrap();
        assert_eq!(item.id, "123");
        assert_eq!(item.title, "Test Story");
        assert!(item.site_data.is_some());
        match item.site_data.unwrap() {
            proto::feed_item::SiteData::Hackernews(hn) => {
                assert_eq!(hn.score, 200);
                assert_eq!(hn.comments, 50);
            }
            _ => panic!("expected HackerNewsData"),
        }
    }

    #[test]
    fn value_to_feed_item_unknown_site_still_works() {
        let v = serde_json::json!({
            "id": "1",
            "title": "Test",
            "url": "https://example.com",
            "preview": ""
        });
        let item = value_to_feed_item("unknownsite", &v).unwrap();
        assert_eq!(item.id, "1");
        assert!(item.site_data.is_none());
    }

    #[test]
    fn value_to_feed_item_missing_id_returns_none() {
        let v = serde_json::json!({"title": "no id"});
        assert!(value_to_feed_item("hackernews", &v).is_none());
    }

    #[test]
    fn recipe_path_uses_site_name() {
        let path = recipe_path("hackernews");
        assert!(path.to_str().unwrap().contains("hackernews-feed.yaml"));
    }
}
