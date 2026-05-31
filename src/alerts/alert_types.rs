use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertState {
    pub telegram_enabled: bool,
    pub last_checked_ts: Option<i64>,
    pub last_sent_ts: Option<i64>,
    pub last_suppressed_ts: Option<i64>,
    pub sent_count: u64,
    pub suppressed_count: u64,
    pub last_error: Option<String>,
}

impl AlertState {
    pub fn new(telegram_enabled: bool) -> Self {
        Self {
            telegram_enabled,
            last_checked_ts: None,
            last_sent_ts: None,
            last_suppressed_ts: None,
            sent_count: 0,
            suppressed_count: 0,
            last_error: None,
        }
    }
}
