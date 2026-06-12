use crate::types::market::Venue;

pub fn normalize_symbol(venue: Venue, symbol: &str) -> Option<&'static str> {
    match (venue, symbol) {
        (Venue::Binance, "BTCUSDT" | "btcusdt") => Some("BTC-PERP"),
        (Venue::Binance, "ETHUSDT" | "ethusdt") => Some("ETH-PERP"),
        (Venue::Bybit, "BTCUSDT") => Some("BTC-PERP"),
        (Venue::Okx, "BTC-USDT-SWAP") => Some("BTC-PERP"),
        (Venue::Okx, "ETH-USDT-SWAP") => Some("ETH-PERP"),
        (Venue::Bitfinex, "tBTCF0:USTF0" | "BTCF0:USTF0") => Some("BTC-PERP"),
        (Venue::Bitfinex, "tETHF0:USTF0" | "ETHF0:USTF0") => Some("ETH-PERP"),
        _ => None,
    }
}

pub fn require_symbol(venue: Venue, symbol: &str) -> anyhow::Result<String> {
    normalize_symbol(venue, symbol)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("unsupported {venue} symbol: {symbol}"))
}
