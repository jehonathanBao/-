pub mod calc;
pub mod collector;
pub mod config;
pub mod discord;
pub mod engine;
pub mod types;

pub use calc::{compute_hourly_delta, should_alert};
pub use config::{load_hourly_delta_alert_config_from_settings, HourlyDeltaAlertConfig};
pub use discord::{
    build_hourly_delta_discord_content, build_hourly_delta_discord_payload,
    notify_hourly_delta_discord, HourlyDeltaDiscordOutcome, HourlyDeltaDiscordSettings,
};
pub use engine::HourlyDeltaAlertRuntime;
pub use types::{
    ClosedHourlyKline, HourlyDeltaAlertRecord, HourlyDeltaDataStatus, HourlyDeltaDirection,
    HourlyDeltaDiscordOutboxItem, HourlyDeltaDiscordOutboxStats, HourlyDeltaDiscordStatus,
    HourlyDeltaResult, HourlyDeltaRuntimeDiagnostics,
};

pub const LOG_EVENTS_PREFIX: &str = "cwm.hourly_delta";
