use crate::types::market::Venue;

#[derive(Debug, Clone, Copy)]
pub struct VenueConfig {
    pub venue: Venue,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct VenueConfigs {
    pub binance: VenueConfig,
    pub bybit: VenueConfig,
    pub okx: VenueConfig,
}

impl VenueConfigs {
    pub fn all(&self) -> Vec<VenueConfig> {
        let mut venues = vec![self.binance, self.bybit, self.okx];
        venues.push(VenueConfig {
            venue: Venue::Bitfinex,
            enabled: bitfinex_enabled_from_env_or_config(),
        });
        venues
    }
}

fn bitfinex_enabled_from_env_or_config() -> bool {
    if let Ok(value) = std::env::var("ENABLE_BITFINEX") {
        return value.eq_ignore_ascii_case("true");
    }
    ::config::Config::builder()
        .add_source(::config::File::with_name("config/default").required(false))
        .add_source(
            ::config::File::with_name(
                &std::env::var("APP_CONFIG_FILE").unwrap_or_else(|_| "config/default".to_string()),
            )
            .required(false),
        )
        .build()
        .ok()
        .and_then(|settings| settings.get_bool("enable_bitfinex").ok())
        .unwrap_or(false)
}
