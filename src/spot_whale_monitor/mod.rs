//! Read-only BTC/ETH spot whale monitoring.
//!
//! This module consumes public market-data WebSockets only. It never places
//! orders, cancels orders, blocks orders, transfers funds, reads private
//! account state, or modifies exchange account state.

pub mod collector_binance;
pub mod collector_bitfinex;
pub mod collector_coinbase;
pub mod config;
pub mod detector;
pub mod discord_notifier;
pub mod normalizer;
pub mod service;
pub mod types;

pub const LOG_TARGET: &str = "spot_whale_monitor";
pub const LOG_PREFIX: &str = "[swm]";
