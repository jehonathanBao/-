use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanLogItem {
    pub id: u64,
    pub ts_ms: i64,
    pub ts: String,
    pub level: String,
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<String>,
}

#[derive(Clone)]
pub struct ScanLogStore {
    inner: Arc<ScanLogStoreInner>,
}

struct ScanLogStoreInner {
    capacity: usize,
    next_id: AtomicU64,
    items: RwLock<VecDeque<ScanLogItem>>,
    tx: broadcast::Sender<ScanLogItem>,
}

impl ScanLogStore {
    pub fn new_from_env() -> Self {
        Self::new(scan_log_capacity_from_env())
    }

    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.clamp(50, 2_000);
        let (tx, _) = broadcast::channel(capacity);
        Self {
            inner: Arc::new(ScanLogStoreInner {
                capacity,
                next_id: AtomicU64::new(1),
                items: RwLock::new(VecDeque::with_capacity(capacity)),
                tx,
            }),
        }
    }

    pub fn push(
        &self,
        level: impl AsRef<str>,
        kind: impl AsRef<str>,
        message: impl AsRef<str>,
        symbol: Option<String>,
        candidate_id: Option<String>,
    ) -> ScanLogItem {
        let ts_ms = crate::normalizers::trade::now_ms();
        let item = ScanLogItem {
            id: self.inner.next_id.fetch_add(1, Ordering::SeqCst),
            ts_ms,
            ts: rfc3339_from_ms(ts_ms),
            level: sanitize_token(level.as_ref()),
            kind: sanitize_token(kind.as_ref()),
            message: sanitize_text(message.as_ref()),
            symbol: symbol.map(|value| sanitize_text(&value)),
            candidate_id: candidate_id.map(|value| sanitize_text(&value)),
        };
        {
            let mut items = self.inner.items.write();
            while items.len() >= self.inner.capacity {
                items.pop_front();
            }
            items.push_back(item.clone());
        }
        let _ = self.inner.tx.send(item.clone());
        item
    }

    pub fn recent(&self, limit: usize) -> Vec<ScanLogItem> {
        let limit = limit.clamp(1, self.inner.capacity);
        let items = self.inner.items.read();
        let skip = items.len().saturating_sub(limit);
        items.iter().skip(skip).cloned().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ScanLogItem> {
        self.inner.tx.subscribe()
    }
}

fn scan_log_capacity_from_env() -> usize {
    std::env::var("SCAN_LOG_BUFFER_SIZE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200)
        .clamp(50, 2_000)
}

fn rfc3339_from_ms(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn sanitize_token(value: &str) -> String {
    let sanitized = sanitize_text(value);
    sanitized
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .take(64)
        .collect::<String>()
}

pub fn sanitize_text(value: &str) -> String {
    let mut sanitized = value.trim().chars().take(500).collect::<String>();
    for secret in configured_secret_values() {
        sanitized = sanitized.replace(&secret, "[redacted]");
    }
    for forbidden in [
        "discord.com/api/webhooks",
        "discordapp.com/api/webhooks",
        "OPERATOR_TOKEN",
        "OPERATOR_API_TOKEN",
        "DISCORD_WEBHOOK_URL",
        "TELEGRAM_BOT_TOKEN",
        "Authorization",
        "authorization",
        "Bearer ",
        "rawPayload",
        "raw_payload",
        "raw payload",
        "markout",
        "evidence",
        "webhook",
        "token",
        "apiKey",
        "api key",
    ] {
        sanitized = replace_case_insensitive(&sanitized, forbidden, "[redacted]");
    }
    sanitized
}

fn configured_secret_values() -> Vec<String> {
    [
        "OPERATOR_TOKEN",
        "OPERATOR_API_TOKEN",
        "DISCORD_WEBHOOK_URL",
        "TELEGRAM_BOT_TOKEN",
        "TELEGRAM_CHAT_ID",
    ]
    .into_iter()
    .filter_map(|key| std::env::var(key).ok())
    .map(|value| value.trim().to_string())
    .filter(|value| value.len() >= 4)
    .collect()
}

fn replace_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return input.to_string();
    }
    let lower_input = input.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(offset) = lower_input[cursor..].find(&lower_needle) {
        let start = cursor + offset;
        let end = start + needle.len();
        result.push_str(&input[cursor..start]);
        result.push_str(replacement);
        cursor = end;
    }
    result.push_str(&input[cursor..]);
    result
}

#[cfg(test)]
mod tests {
    use super::{sanitize_text, ScanLogStore};

    #[test]
    fn scan_log_store_keeps_bounded_recent_items() {
        let store = ScanLogStore::new(50);
        for index in 0..55 {
            store.push("info", "tick", format!("scan tick {index}"), None, None);
        }

        let items = store.recent(200);
        assert_eq!(items.len(), 50);
        assert!(items.first().expect("first item").message.contains("5"));
        assert!(items.last().expect("last item").message.contains("54"));
    }

    #[test]
    fn scan_log_redacts_secret_labels_and_values() {
        std::env::set_var("OPERATOR_TOKEN", "secret-operator-token");
        std::env::set_var(
            "DISCORD_WEBHOOK_URL",
            "https://discord.com/api/webhooks/123/secret",
        );

        let text = sanitize_text(
            "Authorization Bearer secret-operator-token rawPayload markout evidence https://discord.com/api/webhooks/123/secret",
        );

        assert!(!text.contains("secret-operator-token"));
        assert!(!text.contains("discord.com/api/webhooks"));
        assert!(!text.contains("Authorization"));
        assert!(!text.contains("rawPayload"));
        assert!(!text.contains("markout"));
        assert!(!text.contains("evidence"));

        std::env::remove_var("OPERATOR_TOKEN");
        std::env::remove_var("DISCORD_WEBHOOK_URL");
    }
}
