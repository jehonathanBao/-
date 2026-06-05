use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Context};
use parking_lot::RwLock;
use serde::Serialize;

#[derive(Debug, Clone)]
enum TelegramMode {
    Live,
    MockSuccess,
    MockFailure,
}

#[derive(Debug, Clone)]
pub struct TelegramClient {
    enabled: bool,
    bot_token: Option<String>,
    chat_id: Option<String>,
    http: reqwest::Client,
    mode: TelegramMode,
    sent_messages: Arc<RwLock<Vec<String>>>,
}

#[derive(Debug, Serialize)]
struct TelegramSendMessagePayload<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
    disable_web_page_preview: bool,
}

impl TelegramClient {
    pub fn new(enabled: bool, bot_token: Option<String>, chat_id: Option<String>) -> Self {
        Self {
            enabled,
            bot_token,
            chat_id,
            http: alert_http_client(),
            mode: TelegramMode::Live,
            sent_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn mock_success(enabled: bool) -> Self {
        Self {
            enabled,
            bot_token: Some("mock-token".to_string()),
            chat_id: Some("mock-chat".to_string()),
            http: alert_http_client(),
            mode: TelegramMode::MockSuccess,
            sent_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn mock_failure(enabled: bool) -> Self {
        Self {
            enabled,
            bot_token: Some("mock-token".to_string()),
            chat_id: Some("mock-chat".to_string()),
            http: alert_http_client(),
            mode: TelegramMode::MockFailure,
            sent_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn sent_messages(&self) -> Vec<String> {
        self.sent_messages.read().clone()
    }

    pub async fn send_message(&self, text: &str) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let token = self
            .bot_token
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("telegram bot token missing"))?;
        let chat_id = self
            .chat_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("telegram chat id missing"))?;

        match self.mode {
            TelegramMode::MockSuccess => {
                self.sent_messages.write().push(text.to_string());
                Ok(())
            }
            TelegramMode::MockFailure => Err(anyhow!("mock telegram failure")),
            TelegramMode::Live => {
                let url = format!("https://api.telegram.org/bot{token}/sendMessage");
                let payload = TelegramSendMessagePayload {
                    chat_id,
                    text,
                    parse_mode: "HTML",
                    disable_web_page_preview: true,
                };
                self.http
                    .post(url)
                    .json(&payload)
                    .send()
                    .await
                    .context("failed to send telegram message")?
                    .error_for_status()
                    .context("telegram returned an error")?;
                self.sent_messages.write().push(text.to_string());
                Ok(())
            }
        }
    }
}

fn alert_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(alert_http_timeout_secs()))
        .build()
        .expect("alert http client")
}

fn alert_http_timeout_secs() -> u64 {
    std::env::var("ALERT_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn alert_http_timeout_uses_env_or_safe_default() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::remove_var("ALERT_HTTP_TIMEOUT_SECS");
        assert_eq!(super::alert_http_timeout_secs(), 5);

        std::env::set_var("ALERT_HTTP_TIMEOUT_SECS", "9");
        assert_eq!(super::alert_http_timeout_secs(), 9);

        std::env::remove_var("ALERT_HTTP_TIMEOUT_SECS");
    }
}
