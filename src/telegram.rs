use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// A message queued for delivery to Telegram.
#[derive(Debug)]
pub struct QueuedMessage {
    pub text: String,
}

/// Handle for enqueueing Telegram messages. Cheap to clone.
#[derive(Clone)]
pub struct TelegramSender {
    tx: mpsc::Sender<QueuedMessage>,
}

impl TelegramSender {
    /// Enqueue a formatted text message. Non-blocking, returns immediately.
    pub async fn send_message(&self, text: &str) {
        let msg = QueuedMessage {
            text: text.to_string(),
        };
        if let Err(e) = self.tx.send(msg).await {
            warn!(error = %e, "telegram queue full or closed, message dropped");
        }
    }

    /// Format and enqueue a feed item message.
    pub async fn send_feed_item(&self, site: &str, title: &str, url: &str, preview: &str) {
        let escaped_title = escape_html(title);
        let escaped_preview = escape_html(preview);
        let text = if url.is_empty() {
            format!("<b>[{site}]</b> {escaped_title}\n{escaped_preview}")
        } else {
            format!("<b>[{site}]</b> <a href=\"{url}\">{escaped_title}</a>\n{escaped_preview}")
        };
        self.send_message(&text).await;
    }
}

/// Create a Telegram sender + background consumer pair.
/// The consumer task drains the queue at a steady rate (1 msg/sec)
/// and respects 429 `retry_after` from the API.
pub fn create_telegram_channel(
    token: String,
    chat_id: String,
) -> (TelegramSender, TelegramConsumer) {
    let (tx, rx) = mpsc::channel::<QueuedMessage>(2000);
    let sender = TelegramSender { tx };
    let consumer = TelegramConsumer {
        rx,
        bot: TelegramBot::new(token, chat_id),
    };
    (sender, consumer)
}

/// Background consumer that drains the message queue at a steady rate.
pub struct TelegramConsumer {
    rx: mpsc::Receiver<QueuedMessage>,
    bot: TelegramBot,
}

impl TelegramConsumer {
    /// Run the consumer loop. Call this in a spawned task.
    pub async fn run(mut self) {
        info!("telegram consumer started, draining at 1 msg/sec");
        while let Some(msg) = self.rx.recv().await {
            loop {
                match self.bot.send_message(&msg.text).await {
                    Ok(()) => break,
                    Err(TelegramError::Api { status: 429, body }) => {
                        // Parse retry_after from response
                        let retry_after = parse_retry_after(&body).unwrap_or(10);
                        warn!(
                            retry_after,
                            queue_len = self.rx.len(),
                            "telegram rate limited, backing off"
                        );
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                        // Retry the same message
                    }
                    Err(e) => {
                        error!(error = %e, "telegram send failed, skipping message");
                        break;
                    }
                }
            }
            // Steady drain: 1 message per second to respect Telegram rate limits
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        info!("telegram consumer stopped (channel closed)");
    }
}

/// Parse `retry_after` seconds from Telegram 429 response body.
fn parse_retry_after(body: &str) -> Option<u64> {
    #[derive(Deserialize)]
    struct ErrorResp {
        parameters: Option<RetryParams>,
    }
    #[derive(Deserialize)]
    struct RetryParams {
        retry_after: Option<u64>,
    }
    serde_json::from_str::<ErrorResp>(body)
        .ok()?
        .parameters?
        .retry_after
}

/// Low-level Telegram Bot API client. Used by the consumer only.
struct TelegramBot {
    client: Client,
    token: String,
    chat_id: String,
}

#[derive(Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
    disable_web_page_preview: bool,
}

impl TelegramBot {
    fn new(token: String, chat_id: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            token,
            chat_id,
        }
    }

    async fn send_message(&self, text: &str) -> Result<(), TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);

        let body = SendMessageRequest {
            chat_id: &self.chat_id,
            text,
            parse_mode: "HTML",
            disable_web_page_preview: false,
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| TelegramError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(TelegramError::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(())
    }
}

/// Escape special HTML characters for Telegram's HTML parse mode.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[derive(Debug)]
enum TelegramError {
    Http(String),
    Api { status: u16, body: String },
}

impl std::fmt::Display for TelegramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelegramError::Http(e) => write!(f, "telegram HTTP error: {e}"),
            TelegramError::Api { status, body } => {
                write!(f, "telegram API error ({status}): {body}")
            }
        }
    }
}

impl std::error::Error for TelegramError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_special_chars() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("normal text"), "normal text");
        assert_eq!(escape_html(""), "");
    }

    #[test]
    fn escape_html_combined() {
        assert_eq!(escape_html("1 < 2 & 3 > 0"), "1 &lt; 2 &amp; 3 &gt; 0");
    }

    #[test]
    fn parse_retry_after_valid() {
        let body = r#"{"ok":false,"error_code":429,"description":"Too Many Requests: retry after 9","parameters":{"retry_after":9}}"#;
        assert_eq!(parse_retry_after(body), Some(9));
    }

    #[test]
    fn parse_retry_after_missing() {
        assert_eq!(parse_retry_after("{}"), None);
        assert_eq!(parse_retry_after("not json"), None);
        assert_eq!(parse_retry_after(r#"{"parameters":{}}"#), None);
    }
}
