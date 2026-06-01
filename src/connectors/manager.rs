use std::{collections::BTreeMap, sync::Arc, time::Duration};

use parking_lot::RwLock;
use tokio::task::JoinHandle;

use crate::{
    config::AppConfig,
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    types::{
        market::{classify_network_error, Venue, VenueConnectionStatus, VenueHealth},
        status::VenueHealthMap,
    },
};

use super::{binance, bybit, okx};

#[derive(Clone)]
pub struct ConnectorManager {
    bus: MarketDataBus,
    health: Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    enabled: BTreeMap<Venue, bool>,
    symbol: String,
}

impl ConnectorManager {
    pub fn new(bus: MarketDataBus, config: &AppConfig) -> Self {
        let mut health = BTreeMap::new();
        let mut enabled = BTreeMap::new();
        for venue_config in config.venues.all() {
            enabled.insert(venue_config.venue, venue_config.enabled);
            health.insert(
                venue_config.venue.as_key().to_string(),
                VenueHealth::from_config_with_symbol(
                    venue_config.venue,
                    venue_config.enabled,
                    &config.symbol,
                ),
            );
        }
        Self {
            bus,
            health: Arc::new(RwLock::new(health)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            enabled,
            symbol: config.symbol.clone(),
        }
    }

    pub async fn start_all(&self) {
        for (venue, enabled) in self.enabled.clone() {
            if !enabled {
                self.update_health(VenueHealth::from_config_with_symbol(
                    venue,
                    false,
                    &self.symbol,
                ));
                continue;
            }
            let health = VenueHealth::from_config_with_symbol(venue, true, &self.symbol);
            if matches!(health.status, VenueConnectionStatus::ConfigurationError) {
                self.update_health(health);
                continue;
            }
            self.update_health(VenueHealth::start_attempted_with_symbol(
                venue,
                &self.symbol,
            ));
            let bus = self.bus.clone();
            let health = self.health.clone();
            let handle = match venue {
                Venue::Binance => tokio::spawn(async move {
                    binance::run(bus, health).await;
                }),
                Venue::Bybit => tokio::spawn(async move {
                    bybit::run(bus, health).await;
                }),
                Venue::Okx => tokio::spawn(async move {
                    okx::run(bus, health).await;
                }),
            };
            self.tasks.write().push(handle);
        }
    }

    pub async fn stop_all(&self) {
        let tasks = std::mem::take(&mut *self.tasks.write());
        for task in tasks {
            task.abort();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    pub fn get_venue_health(&self) -> VenueHealthMap {
        self.health.read().clone()
    }

    pub fn set_health_for_tests(&self, health: VenueHealth) {
        self.update_health(health);
    }

    pub fn ingest_trade_event_for_tests(&self, trade: crate::types::market::NormalizedTrade) {
        set_status(
            &self.bus,
            &self.health,
            trade.venue,
            VenueConnectionStatus::Connecting,
            None,
        );
        set_status(
            &self.bus,
            &self.health,
            trade.venue,
            VenueConnectionStatus::Connected,
            None,
        );
        mark_trade(&self.bus, &self.health, trade.venue, trade.ts);
        self.bus.publish(MarketDataEvent::Trade(trade));
    }

    fn update_health(&self, health: VenueHealth) {
        self.health
            .write()
            .insert(health.venue.as_key().to_string(), health.clone());
        self.bus.publish(MarketDataEvent::VenueHealth(health));
    }
}

pub(crate) fn publish_health(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    health: VenueHealth,
) {
    health_map
        .write()
        .insert(health.venue.as_key().to_string(), health.clone());
    bus.publish(MarketDataEvent::VenueHealth(health));
}

pub(crate) fn patch_health<F>(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
    patch: F,
) where
    F: FnOnce(&mut VenueHealth),
{
    let mut health = health_map
        .read()
        .get(venue.as_key())
        .cloned()
        .unwrap_or_else(|| VenueHealth::disconnected(venue, true));
    patch(&mut health);
    publish_health(bus, health_map, health);
}

pub(crate) fn mark_message(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
) {
    patch_health(bus, health_map, venue, |health| {
        health.last_message_ts = Some(crate::normalizers::trade::now_ms());
    });
}

pub(crate) fn mark_trade(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
    ts: i64,
) {
    patch_health(bus, health_map, venue, |health| {
        let now = crate::normalizers::trade::now_ms();
        health.last_trade_ts = Some(ts);
        health.last_message_ts = Some(now);
        health.last_trade_message_at_ms = Some(now);
        health.last_parsed_trade_at_ms = Some(now);
        health.trade_message_count += 1;
        health.trade_active = true;
        health.last_parse_error = None;
        health.ws_error_class = "none".to_string();
    });
}

pub(crate) fn mark_book(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
    ts: i64,
) {
    patch_health(bus, health_map, venue, |health| {
        let now = crate::normalizers::trade::now_ms();
        health.last_book_ts = Some(ts);
        health.last_message_ts = Some(now);
        health.last_book_message_at_ms = Some(now);
        health.last_parsed_book_at_ms = Some(now);
        health.book_message_count += 1;
        health.book_active = true;
        health.last_parse_error = None;
        health.ws_error_class = "none".to_string();
    });
}

pub(crate) fn mark_parse_error(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
    error: impl Into<String>,
) {
    patch_health(bus, health_map, venue, |health| {
        let error = error.into();
        health.last_parse_error = Some(error.clone());
        health.ws_error_class = classify_network_error(Some(&error)).to_string();
    });
}

pub(crate) fn mark_subscription_acked(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
    trade_acked: bool,
    book_acked: bool,
) {
    patch_health(bus, health_map, venue, |health| {
        health.trade_subscribe_acked = health.trade_subscribe_acked || trade_acked;
        health.book_subscribe_acked = health.book_subscribe_acked || book_acked;
    });
}

pub(crate) fn set_status(
    bus: &MarketDataBus,
    health_map: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    venue: Venue,
    status: VenueConnectionStatus,
    error: Option<String>,
) {
    patch_health(bus, health_map, venue, |health| {
        let now = crate::normalizers::trade::now_ms();
        let error_class = classify_network_error(error.as_deref());
        health.enabled = true;
        health.enable_flag_value = true;
        health.disabled_reason = None;
        health.connector_constructed = true;
        health.start_attempted = true;
        health.status = status;
        health.ws_configured = true;
        if matches!(status, VenueConnectionStatus::Connecting) {
            health.ws_connect_attempted = true;
        }
        if matches!(status, VenueConnectionStatus::Connected) {
            health.ws_connected = true;
            health.ws_last_connect_at_ms = Some(now);
            health.ws_error_class = "none".to_string();
            health.last_network_error_class = "none".to_string();
        }
        if matches!(
            status,
            VenueConnectionStatus::Reconnecting
                | VenueConnectionStatus::Disconnected
                | VenueConnectionStatus::Degraded
                | VenueConnectionStatus::Error
        ) {
            health.ws_connected = false;
            health.ws_last_disconnect_at_ms = Some(now);
        }
        if matches!(status, VenueConnectionStatus::Connecting) || health.ws_connect_attempted {
            health.trade_subscribe_attempted = health.trade_stream_configured;
            health.book_subscribe_attempted = health.book_stream_configured;
        }
        if let Some(error) = error {
            health.ws_last_error = Some(error.clone());
            health.last_error = Some(error);
            health.ws_error_class = error_class.to_string();
            health.last_network_error_class = error_class.to_string();
        } else {
            health.last_error = None;
        }
        if matches!(health.status, VenueConnectionStatus::Reconnecting) {
            health.reconnect_count += 1;
            health.ws_reconnect_count = health.reconnect_count;
        }
    });
}
