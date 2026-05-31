use std::fmt;

pub const VENUE_ACTIVE_WINDOW_MS: i64 = 30_000;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Venue {
    Binance,
    Bybit,
    Okx,
}

impl Venue {
    pub const ALL: [Venue; 3] = [Venue::Binance, Venue::Bybit, Venue::Okx];

    pub fn as_key(self) -> &'static str {
        match self {
            Venue::Binance => "binance",
            Venue::Bybit => "bybit",
            Venue::Okx => "okx",
        }
    }
}

impl fmt::Display for Venue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_key())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggressorSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedTrade {
    pub venue: Venue,
    pub symbol: String,
    pub ts: i64,
    pub price: f64,
    pub size_btc: f64,
    pub size_usd: f64,
    pub aggressor_side: AggressorSide,
    pub trade_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedBook {
    pub venue: Venue,
    pub symbol: String,
    pub ts: i64,
    pub best_bid: f64,
    pub best_ask: f64,
    #[serde(default)]
    pub bids: Vec<(f64, f64)>,
    #[serde(default)]
    pub asks: Vec<(f64, f64)>,
    pub mid: f64,
    pub spread_bps: f64,
    pub bid_depth_btc_10bps: f64,
    pub ask_depth_btc_10bps: f64,
    pub bid_depth_usd_10bps: f64,
    pub ask_depth_usd_10bps: f64,
    pub imbalance_10bps: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VenueConnectionStatus {
    Disabled,
    #[serde(rename = "configuration_error")]
    ConfigurationError,
    Connecting,
    Connected,
    Reconnecting,
    Degraded,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueHealth {
    pub venue: Venue,
    pub enabled: bool,
    pub enable_flag_name: String,
    pub enable_flag_value: bool,
    pub enable_source: String,
    pub disabled_reason: Option<String>,
    pub requested_symbol: String,
    pub venue_symbol: Option<String>,
    pub venue_market_type: Option<String>,
    pub symbol_mapping_status: String,
    pub symbol_mapping_error: Option<String>,
    pub connector_constructed: bool,
    pub start_attempted: bool,
    pub status: VenueConnectionStatus,
    pub last_trade_ts: Option<i64>,
    pub last_book_ts: Option<i64>,
    pub last_message_ts: Option<i64>,
    pub reconnect_count: u64,
    pub last_error: Option<String>,
    pub ws_configured: bool,
    pub ws_connect_attempted: bool,
    pub ws_connected: bool,
    pub ws_last_connect_at_ms: Option<i64>,
    pub ws_last_disconnect_at_ms: Option<i64>,
    pub ws_reconnect_count: u64,
    pub ws_last_error: Option<String>,
    pub ws_error_class: String,
    pub trade_stream_configured: bool,
    pub book_stream_configured: bool,
    pub trade_subscribe_attempted: bool,
    pub book_subscribe_attempted: bool,
    pub trade_subscribe_acked: bool,
    pub book_subscribe_acked: bool,
    pub ack_mode: String,
    pub last_trade_message_at_ms: Option<i64>,
    pub last_book_message_at_ms: Option<i64>,
    pub trade_message_count: u64,
    pub book_message_count: u64,
    pub last_parsed_trade_at_ms: Option<i64>,
    pub last_parsed_book_at_ms: Option<i64>,
    pub last_parse_error: Option<String>,
    pub active_window_ms: i64,
    pub trade_active: bool,
    pub book_active: bool,
    pub activity_status: String,
    pub proxy_enabled: bool,
    pub proxy_supported: bool,
    pub proxy_source: Option<String>,
    pub proxy_scheme: Option<String>,
    pub proxy_host_masked: Option<String>,
    pub proxy_port_masked: Option<String>,
    pub proxy_configured_for_ws: bool,
    pub network_probe_enabled: bool,
    pub last_network_error_class: String,
}

impl VenueHealth {
    pub fn from_config(venue: Venue, enabled: bool) -> Self {
        Self::from_config_with_symbol(venue, enabled, "BTC-PERP")
    }

    pub fn from_config_with_symbol(venue: Venue, enabled: bool, requested_symbol: &str) -> Self {
        let mapping = venue_symbol_mapping(venue, requested_symbol);
        let proxy = proxy_diagnostics_from_env();
        let venue_symbol = mapping.venue_symbol;
        let mapping_missing = enabled && venue_symbol.is_none();
        Self {
            venue,
            enabled,
            enable_flag_name: venue_enable_flag_name(venue).to_string(),
            enable_flag_value: enabled,
            enable_source: venue_enable_source(venue).to_string(),
            disabled_reason: if !enabled {
                Some("env_or_config_flag_false".to_string())
            } else if mapping_missing {
                Some("symbol_mapping_missing".to_string())
            } else {
                None
            },
            requested_symbol: requested_symbol.to_string(),
            venue_symbol,
            venue_market_type: mapping.venue_market_type.map(str::to_string),
            symbol_mapping_status: mapping.status.to_string(),
            symbol_mapping_error: mapping.error,
            connector_constructed: false,
            start_attempted: false,
            status: if mapping_missing {
                VenueConnectionStatus::ConfigurationError
            } else if enabled {
                VenueConnectionStatus::Disconnected
            } else {
                VenueConnectionStatus::Disabled
            },
            last_trade_ts: None,
            last_book_ts: None,
            last_message_ts: None,
            reconnect_count: 0,
            last_error: None,
            ws_configured: enabled,
            ws_connect_attempted: false,
            ws_connected: false,
            ws_last_connect_at_ms: None,
            ws_last_disconnect_at_ms: None,
            ws_reconnect_count: 0,
            ws_last_error: None,
            ws_error_class: "none".to_string(),
            trade_stream_configured: enabled && !mapping_missing,
            book_stream_configured: enabled && !mapping_missing,
            trade_subscribe_attempted: false,
            book_subscribe_attempted: false,
            trade_subscribe_acked: false,
            book_subscribe_acked: false,
            ack_mode: venue_ack_mode(venue).to_string(),
            last_trade_message_at_ms: None,
            last_book_message_at_ms: None,
            trade_message_count: 0,
            book_message_count: 0,
            last_parsed_trade_at_ms: None,
            last_parsed_book_at_ms: None,
            last_parse_error: None,
            active_window_ms: VENUE_ACTIVE_WINDOW_MS,
            trade_active: false,
            book_active: false,
            activity_status: if enabled {
                "not_started".to_string()
            } else {
                "disabled".to_string()
            },
            proxy_enabled: proxy.enabled,
            proxy_supported: false,
            proxy_source: proxy.source,
            proxy_scheme: proxy.scheme,
            proxy_host_masked: proxy.host_masked,
            proxy_port_masked: proxy.port_masked,
            proxy_configured_for_ws: false,
            network_probe_enabled: false,
            last_network_error_class: "none".to_string(),
        }
    }

    pub fn disabled(venue: Venue) -> Self {
        Self::from_config(venue, false)
    }

    pub fn disconnected(venue: Venue, enabled: bool) -> Self {
        Self::from_config(venue, enabled)
    }

    pub fn start_attempted(venue: Venue) -> Self {
        let mut health = Self::from_config(venue, true);
        health.connector_constructed = true;
        health.start_attempted = true;
        health
    }

    pub fn start_attempted_with_symbol(venue: Venue, requested_symbol: &str) -> Self {
        let mut health = Self::from_config_with_symbol(venue, true, requested_symbol);
        health.connector_constructed = true;
        health.start_attempted = true;
        health.ws_connect_attempted = true;
        health.trade_subscribe_attempted = health.trade_stream_configured;
        health.book_subscribe_attempted = health.book_stream_configured;
        health
    }
}

pub struct VenueSymbolMapping {
    pub venue_symbol: Option<String>,
    pub venue_market_type: Option<&'static str>,
    pub status: &'static str,
    pub error: Option<String>,
}

pub fn venue_enable_source(venue: Venue) -> &'static str {
    let env_key = venue_enable_flag_name(venue);
    if std::env::var(env_key).is_ok() {
        return "env";
    }
    let toml_key = match venue {
        Venue::Binance => "enable_binance",
        Venue::Bybit => "enable_bybit",
        Venue::Okx => "enable_okx",
    };
    let has_toml_value = ::config::Config::builder()
        .add_source(::config::File::with_name("config/default").required(false))
        .build()
        .ok()
        .and_then(|settings| settings.get_bool(toml_key).ok())
        .is_some();
    if has_toml_value {
        "toml"
    } else {
        "hardcoded_default"
    }
}

pub fn venue_enable_flag_name(venue: Venue) -> &'static str {
    match venue {
        Venue::Binance => "ENABLE_BINANCE",
        Venue::Bybit => "ENABLE_BYBIT",
        Venue::Okx => "ENABLE_OKX",
    }
}

pub fn venue_symbol_mapping(venue: Venue, requested_symbol: &str) -> VenueSymbolMapping {
    let normalized = requested_symbol
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    let supported_btc_perp = matches!(normalized.as_str(), "BTCPERP" | "BTCUSDT" | "BTCUSDTPERP");
    if !supported_btc_perp {
        return VenueSymbolMapping {
            venue_symbol: None,
            venue_market_type: None,
            status: "missing",
            error: Some(format!(
                "{requested_symbol} has no {} public BTC perpetual mapping",
                venue.as_key()
            )),
        };
    }
    let (venue_symbol, venue_market_type) = match venue {
        Venue::Binance | Venue::Bybit => ("BTCUSDT", "linear_perpetual"),
        Venue::Okx => ("BTC-USDT-SWAP", "swap"),
    };
    VenueSymbolMapping {
        venue_symbol: Some(venue_symbol.to_string()),
        venue_market_type: Some(venue_market_type),
        status: "ok",
        error: None,
    }
}

pub fn venue_ack_mode(venue: Venue) -> &'static str {
    match venue {
        Venue::Binance => "not_supported",
        Venue::Bybit | Venue::Okx => "exchange_ack",
    }
}

pub fn classify_network_error(error: Option<&str>) -> &'static str {
    let Some(error) = error else {
        return "none";
    };
    let error = error.to_ascii_lowercase();
    if error.contains("403") || error.contains("forbidden") {
        "http_403"
    } else if error.contains("429") {
        "http_429"
    } else if error.contains("rate limit") || error.contains("too many request") {
        "rate_limited"
    } else if error.contains("proxy") {
        "proxy_error"
    } else if error.contains("timed out") || error.contains("timeout") {
        "timeout"
    } else if error.contains("tls")
        || error.contains("certificate")
        || error.contains("handshake")
        || error.contains("schannel")
    {
        "tls_error"
    } else if error.contains("dns")
        || error.contains("name resolution")
        || error.contains("failed to lookup")
        || error.contains("nodename")
    {
        "dns_error"
    } else if error.contains("tcp")
        || error.contains("connection refused")
        || error.contains("connection reset")
        || error.contains("connect error")
    {
        "tcp_connect_error"
    } else if error.contains("subscription") || error.contains("subscribe") {
        "subscription_rejected"
    } else if error.contains("schema") {
        "schema_error"
    } else if error.contains("parse") || error.contains("json") {
        "message_parse_error"
    } else {
        "unknown"
    }
}

#[derive(Debug, Clone)]
pub struct ProxyDiagnostics {
    pub enabled: bool,
    pub source: Option<String>,
    pub scheme: Option<String>,
    pub host_masked: Option<String>,
    pub port_masked: Option<String>,
}

pub fn proxy_diagnostics_from_env() -> ProxyDiagnostics {
    for key in ["WSS_PROXY", "HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"] {
        let Ok(value) = std::env::var(key) else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        let parsed = url::Url::parse(&value).ok();
        return ProxyDiagnostics {
            enabled: true,
            source: Some(key.to_string()),
            scheme: parsed.as_ref().map(|url| url.scheme().to_string()),
            host_masked: parsed
                .as_ref()
                .and_then(|url| url.host_str())
                .map(mask_host),
            port_masked: parsed
                .as_ref()
                .and_then(|url| url.port())
                .map(|port| mask_port(port).to_string()),
        };
    }

    ProxyDiagnostics {
        enabled: false,
        source: None,
        scheme: None,
        host_masked: None,
        port_masked: None,
    }
}

fn mask_host(host: &str) -> String {
    let parts = host.split('.').collect::<Vec<_>>();
    if parts.len() >= 2 {
        format!("***.{}", parts[parts.len() - 1])
    } else {
        "***".to_string()
    }
}

fn mask_port(port: u16) -> &'static str {
    match port {
        80 => "80",
        443 => "443",
        _ => "***",
    }
}
