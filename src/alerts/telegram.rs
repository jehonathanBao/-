use std::sync::Arc;

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
            http: reqwest::Client::new(),
            mode: TelegramMode::Live,
            sent_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn mock_success(enabled: bool) -> Self {
        Self {
            enabled,
            bot_token: Some("mock-token".to_string()),
            chat_id: Some("mock-chat".to_string()),
            http: reqwest::Client::new(),
            mode: TelegramMode::MockSuccess,
            sent_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn mock_failure(enabled: bool) -> Self {
        Self {
            enabled,
            bot_token: Some("mock-token".to_string()),
            chat_id: Some("mock-chat".to_string()),
            http: reqwest::Client::new(),
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
