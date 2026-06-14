pub mod alerts;
pub mod api;
pub mod app;
pub mod binance_alt_contract_monitor;
pub mod calibration;
pub mod config;
pub mod connectors;
pub mod contract_whale_monitor;
pub mod market_data;
pub mod normalizers;
pub mod replay;
pub mod runtime;
pub mod safety;
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
