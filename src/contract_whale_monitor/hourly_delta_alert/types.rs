use serde::{Deserialize, Serialize};

pub const RECORD_KEY_PREFIX: &str = "binance:BTCUSDT:1h";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HourlyDeltaDirection {
    NetBuy,
    NetSell,
    Flat,
}

impl HourlyDeltaDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NetBuy => "net_buy",
            Self::NetSell => "net_sell",
            Self::Flat => "flat",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "net_buy" => Some(Self::NetBuy),
            "net_sell" => Some(Self::NetSell),
            "flat" => Some(Self::Flat),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HourlyDeltaDataStatus {
    Closed,
    Pending,
    Failed,
}

impl HourlyDeltaDataStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Pending => "pending",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "closed" => Some(Self::Closed),
            "pending" => Some(Self::Pending),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HourlyDeltaDiscordStatus {
    None,
    Pending,
    DryRun,
    Sent,
    Retry,
    Dead,
    Sending,
}

impl HourlyDeltaDiscordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Pending => "pending",
            Self::DryRun => "dry_run",
            Self::Sent => "sent",
            Self::Retry => "retry",
            Self::Dead => "dead",
            Self::Sending => "sending",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "pending" => Some(Self::Pending),
            "dry_run" => Some(Self::DryRun),
            "sent" => Some(Self::Sent),
            "retry" => Some(Self::Retry),
            "dead" => Some(Self::Dead),
            "sending" => Some(Self::Sending),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedHourlyKline {
    pub exchange: String,
    pub symbol: String,
    pub interval: String,
    pub open_time_ms: i64,
    pub close_time_ms: i64,
    pub volume_btc: f64,
    pub taker_buy_btc: f64,
    pub is_closed: bool,
}

impl ClosedHourlyKline {
    pub fn record_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.exchange.to_ascii_lowercase(),
            self.symbol.to_ascii_uppercase(),
            self.interval.to_ascii_lowercase(),
            self.open_time_ms
        )
    }

    pub fn taker_sell_btc(&self) -> f64 {
        (self.volume_btc - self.taker_buy_btc).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyDeltaResult {
    pub record_key: String,
    pub exchange: String,
    pub symbol: String,
    pub interval: String,
    pub kline_open_time_ms: i64,
    pub kline_close_time_ms: i64,
    pub taker_buy_btc: f64,
    pub taker_sell_btc: f64,
    pub delta_btc: f64,
    pub volume_btc: f64,
    pub direction: HourlyDeltaDirection,
    pub above_threshold: bool,
    pub threshold_btc: f64,
    pub data_status: HourlyDeltaDataStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyDeltaAlertRecord {
    pub record_key: String,
    pub exchange: String,
    pub symbol: String,
    pub interval: String,
    pub kline_open_time_ms: i64,
    pub kline_close_time_ms: i64,
    pub taker_buy_btc: f64,
    pub taker_sell_btc: f64,
    pub delta_btc: f64,
    pub volume_btc: f64,
    pub direction: HourlyDeltaDirection,
    pub above_threshold: bool,
    pub data_status: HourlyDeltaDataStatus,
    pub discord_status: HourlyDeltaDiscordStatus,
    pub discord_sent_at_ms: Option<i64>,
    pub attempts: usize,
    pub last_error: Option<String>,
    pub payload_json: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HourlyDeltaDiscordOutboxItem {
    pub record_key: String,
    pub record: HourlyDeltaAlertRecord,
    pub attempts: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HourlyDeltaDiscordOutboxStats {
    pub pending: usize,
    pub retrying: usize,
    pub failed: usize,
    pub oldest_pending_age_sec: i64,
}

#[derive(Debug, Clone, Default)]
pub struct HourlyDeltaRuntimeDiagnostics {
    pub ws_connected: bool,
    pub last_ws_event_at_ms: Option<i64>,
    pub last_closed_open_time_ms: Option<i64>,
    pub closed_processed: u64,
    pub alerts_enqueued: u64,
    pub backfill_ok: u64,
    pub backfill_fail: u64,
    pub discord_sent: u64,
    pub discord_dry_run: u64,
    pub outbox_polls: u64,
    pub outbox_claimed: u64,
    pub outbox_errors: u64,
    pub last_outbox_poll_at_ms: Option<i64>,
    pub last_error: Option<String>,
}
