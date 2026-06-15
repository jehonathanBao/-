use std::sync::{OnceLock, RwLock};

static GLOBAL_SYSTEM_MODE: OnceLock<RwLock<SystemModeConfig>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketSystemMode {
    BearMarket,
    Normal,
}

impl MarketSystemMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BearMarket => "bear_market",
            Self::Normal => "normal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemFeatureConfig {
    pub altcoin_monitoring: bool,
    pub memecoin_tracking: bool,
    pub low_cap_scanner: bool,
}

impl Default for SystemFeatureConfig {
    fn default() -> Self {
        Self {
            altcoin_monitoring: false,
            memecoin_tracking: false,
            low_cap_scanner: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemModeConfig {
    pub mode: MarketSystemMode,
    pub features: SystemFeatureConfig,
}

impl Default for SystemModeConfig {
    fn default() -> Self {
        Self {
            mode: MarketSystemMode::BearMarket,
            features: SystemFeatureConfig::default(),
        }
    }
}

impl SystemModeConfig {
    pub fn altcoin_monitoring_enabled(self) -> bool {
        self.mode != MarketSystemMode::BearMarket && self.features.altcoin_monitoring
    }

    pub fn altcoin_disabled_reason(self) -> Option<&'static str> {
        if self.mode == MarketSystemMode::BearMarket {
            Some("bear_market_mode")
        } else if !self.features.altcoin_monitoring {
            Some("altcoin_monitoring_feature_disabled")
        } else {
            None
        }
    }
}

pub fn system_mode_config() -> SystemModeConfig {
    global_config()
        .read()
        .expect("system mode config lock poisoned")
        .to_owned()
}

pub fn set_system_mode_config(config: SystemModeConfig) {
    *global_config()
        .write()
        .expect("system mode config lock poisoned") = config;
}

pub fn reset_system_mode_config() {
    set_system_mode_config(SystemModeConfig::default());
}

pub fn load_system_mode_config_from_settings(settings: &::config::Config) -> SystemModeConfig {
    let fallback = SystemModeConfig::default();
    SystemModeConfig {
        mode: market_system_mode_setting(settings, "SYSTEM_MODE", "system.mode", fallback.mode),
        features: SystemFeatureConfig {
            altcoin_monitoring: bool_setting(
                settings,
                "ENABLE_ALTCOIN_MONITORING",
                "features.altcoin_monitoring",
                fallback.features.altcoin_monitoring,
            ),
            memecoin_tracking: bool_setting(
                settings,
                "ENABLE_MEMECOIN_TRACKING",
                "features.memecoin_tracking",
                fallback.features.memecoin_tracking,
            ),
            low_cap_scanner: bool_setting(
                settings,
                "ENABLE_LOW_CAP_SCANNER",
                "features.low_cap_scanner",
                fallback.features.low_cap_scanner,
            ),
        },
    }
}

fn global_config() -> &'static RwLock<SystemModeConfig> {
    GLOBAL_SYSTEM_MODE.get_or_init(|| RwLock::new(SystemModeConfig::default()))
}

fn bool_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: bool) -> bool {
    std::env::var(env_key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| settings.get_bool(toml_key).ok())
        .unwrap_or(default)
}

fn market_system_mode_setting(
    settings: &::config::Config,
    env_key: &str,
    toml_key: &str,
    default: MarketSystemMode,
) -> MarketSystemMode {
    let value = std::env::var(env_key)
        .ok()
        .or_else(|| settings.get_string(toml_key).ok());
    match value
        .unwrap_or_else(|| default.as_str().to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "normal" | "default" | "active" | "bull" | "bull_market" => MarketSystemMode::Normal,
        "bear" | "bear_market" | "bear_market_mode" => MarketSystemMode::BearMarket,
        _ => default,
    }
}
