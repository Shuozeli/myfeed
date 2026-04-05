//! Pluggable notifier layer. Any notifier implementing this trait can be
//! swapped in without touching the scheduler.

use async_trait::async_trait;
use std::sync::Arc;

use crate::telegram::TelegramSender;

/// Trait for delivering feed items and messages to a destination.
/// Implement this to add new delivery channels (webhook, email, etc.).
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Deliver a single feed item.
    async fn notify_feed_item(&self, site: &str, title: &str, url: &str, preview: &str);

    /// Deliver a raw text message (e.g. digest summary, error notice).
    async fn notify_message(&self, text: &str);
}

#[async_trait]
impl Notifier for TelegramSender {
    async fn notify_feed_item(&self, site: &str, title: &str, url: &str, preview: &str) {
        TelegramSender::send_feed_item(self, site, title, url, preview).await;
    }

    async fn notify_message(&self, text: &str) {
        TelegramSender::send_message(self, text).await;
    }
}

/// A notifier that prints to stdout. Useful for testing and debugging.
pub struct StdoutNotifier;

#[async_trait]
impl Notifier for StdoutNotifier {
    async fn notify_feed_item(&self, site: &str, title: &str, url: &str, preview: &str) {
        if url.is_empty() {
            println!("[{site}] {title} | {preview}");
        } else {
            println!("[{site}] {title} | {url} | {preview}");
        }
    }

    async fn notify_message(&self, text: &str) {
        println!("{text}");
    }
}

/// Create a notifier from configuration. Falls back to StdoutNotifier if
/// no Telegram credentials are provided or if Telegram client creation fails.
pub fn create_notifier(config: &crate::config::Config) -> Arc<dyn Notifier> {
    if !config.telegram_bot_token.is_empty() && !config.telegram_chat_id.is_empty() {
        match crate::telegram::create_telegram_channel(
            config.telegram_bot_token.clone(),
            config.telegram_chat_id.clone(),
        ) {
            Ok((sender, consumer)) => {
                // Launch the consumer so queued messages are actually delivered.
                tokio::spawn(consumer.run());
                Arc::new(sender) as Arc<dyn Notifier>
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to create Telegram client, falling back to stdout");
                Arc::new(StdoutNotifier) as Arc<dyn Notifier>
            }
        }
    } else {
        Arc::new(StdoutNotifier) as Arc<dyn Notifier>
    }
}
