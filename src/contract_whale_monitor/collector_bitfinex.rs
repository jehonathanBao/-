pub const BITFINEX_BTC_PERP_TRADES_CHANNEL: &str = "trades:tBTCF0:USTF0";
pub const BITFINEX_ETH_PERP_TRADES_CHANNEL: &str = "trades:tETHF0:USTF0";
pub const BITFINEX_BTC_SPOT_TRADES_CHANNEL: &str = "trades:tBTCUSD";
pub const BITFINEX_ETH_SPOT_TRADES_CHANNEL: &str = "trades:tETHUSD";

pub const BITFINEX_PERP_TRADE_SYMBOLS: [&str; 2] = [
    BITFINEX_BTC_PERP_TRADES_CHANNEL,
    BITFINEX_ETH_PERP_TRADES_CHANNEL,
];
pub const BITFINEX_SPOT_TRADE_SYMBOLS: [&str; 2] = [
    BITFINEX_BTC_SPOT_TRADES_CHANNEL,
    BITFINEX_ETH_SPOT_TRADES_CHANNEL,
];

pub fn collector_status() -> &'static str {
    "wired_via_market_data_connector"
}
