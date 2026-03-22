use std::time::Duration;

use reqwest::Client;
use serde::Serialize;

/// Telegram Bot API client for sending feed updates.
pub struct TelegramBot {
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
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            token,
            chat_id,
        }
    }

    /// Send a text message to the configured chat.
    pub async fn send_message(&self, text: &str) -> Result<(), TelegramError> {
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

    /// Format and send a feed item as a Telegram message.
    pub async fn send_feed_item(
        &self,
        site: &str,
        title: &str,
        url: &str,
        preview: &str,
    ) -> Result<(), TelegramError> {
        let escaped_title = escape_html(title);
        let escaped_preview = escape_html(preview);
        let text = if url.is_empty() {
            format!("<b>[{site}]</b> {escaped_title}\n{escaped_preview}")
        } else {
            format!("<b>[{site}]</b> <a href=\"{url}\">{escaped_title}</a>\n{escaped_preview}")
        };
        self.send_message(&text).await
    }
}

/// Escape special HTML characters for Telegram's HTML parse mode.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[derive(Debug)]
pub enum TelegramError {
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
