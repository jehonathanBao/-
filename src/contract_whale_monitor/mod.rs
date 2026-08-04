//! This module is read-only. It never places orders, cancels orders,
//! blocks orders, transfers funds, or modifies exchange account state.

pub mod aggregator;
pub mod classification;
pub mod cluster;
pub mod collector_binance;
pub mod collector_bitfinex;
pub mod collector_okx;
pub mod config;
pub mod context;
pub mod detector;
pub mod discord;
pub mod discord_gate;
pub mod discord_notifier;
pub mod emission;
pub mod event_lifecycle;
pub mod event_quality;
pub mod hourly_delta_alert;
pub mod impact_baseline;
pub mod impact_episode;
pub mod impact_grade;
pub mod intelligence;
pub mod merge;
pub mod normalizer;
pub mod outcome_calibration;
pub mod persistence;
pub mod replay;
pub mod scoring;
pub mod trading;
pub mod trajectory;
pub mod types;

pub use config::ContractWhaleRuntimeConfig;

pub const LOG_TARGET: &str = "contract_whale_monitor";
pub const LOG_PREFIX: &str = "[cwm]";

pub mod log_events {
    pub const CONFIG_LOADED: &str = "cwm.config.loaded";
    pub const RUNTIME_STARTED: &str = "cwm.runtime.started";
    pub const RUNTIME_STOPPED: &str = "cwm.runtime.stopped";
    pub const RUNTIME_DISABLED: &str = "cwm.runtime.disabled";
    pub const WS_CONNECTED: &str = "cwm.ws.connected";
    pub const WS_DISCONNECTED: &str = "cwm.ws.disconnected";
    pub const TRADE_NORMALIZED: &str = "cwm.trade.normalized";
    pub const BUCKET_FLUSHED: &str = "cwm.bucket.flushed";
    pub const SIGNAL_GENERATED: &str = "cwm.signal.generated";
    pub const HEALTH_EVALUATED: &str = "cwm.health.evaluated";
    pub const DISCORD_ELIGIBLE: &str = "cwm.discord.eligible";
    pub const DISCORD_SENT: &str = "cwm.discord.sent";
    pub const DISCORD_SKIPPED: &str = "cwm.discord.skipped";
    pub const DISCORD_WOULD_SEND: &str = "cwm.discord.would_send";
    pub const RETENTION_PRUNED: &str = "cwm.retention.pruned";
    pub const ERROR: &str = "cwm.error";
    pub const HOURLY_DELTA_CLOSED: &str = "cwm.hourly_delta.closed";
    pub const HOURLY_DELTA_BACKFILL_OK: &str = "cwm.hourly_delta.backfill.ok";
    pub const HOURLY_DELTA_DISCORD_SENT: &str = "cwm.hourly_delta.discord.sent";
}
