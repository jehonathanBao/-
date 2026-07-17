pub mod alerts;
pub mod api;
pub mod app;
pub mod binance_alt_contract_monitor;
pub mod btc_structure_engine;
pub mod calibration;
pub mod config;
pub mod connectors;
pub mod contract_whale_monitor;
pub mod core_event;
pub mod liquidation_cascade_predictor;
pub mod market_data;
pub mod market_domain;
pub mod market_regime_engine;
pub mod multi_timeframe_orderflow_fusion;
pub mod normalization;
pub mod normalizers;
pub mod regime_thresholds;
pub mod replay;
pub mod runtime;
pub mod safety;
pub mod semantic;
pub mod signal_semantics;
pub mod spot_whale_monitor;
pub mod storage;
pub mod toxic_v3;
pub mod toxicity;
pub mod types;

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, OnceLock};

    pub(crate) fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
}
