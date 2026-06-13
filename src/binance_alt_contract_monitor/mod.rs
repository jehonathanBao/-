//! Read-only Binance altcoin perpetual anomaly monitoring.
//!
//! This module consumes public market-data streams only. It never places
//! orders, cancels orders, blocks orders, transfers funds, reads private
//! account state, or modifies exchange account state.

pub mod aggregator;
pub mod atca;
pub mod collector;
pub mod config;
pub mod context;
pub mod detector;
pub mod discord;
pub mod mcss;
pub mod regime;
pub mod scoring;
pub mod service;
pub mod smaf;
pub mod smle;
pub mod smll;
pub mod smp;
pub mod symbol_universe;
pub mod types;

pub const LOG_TARGET: &str = "binance_alt_contract_monitor";
pub const LOG_PREFIX: &str = "[bacm]";
