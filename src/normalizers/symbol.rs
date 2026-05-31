use crate::types::market::Venue;

pub fn normalize_symbol(venue: Venue, symbol: &str) -> Option<&'static str> {
    match (venue, symbol) {
        (Venue::Binance, "BTCUSDT" | "btcusdt") => Some("BTC-PERP"),
        (Venue::Bybit, "BTCUSDT") => Some("BTC-PERP"),
        (Venue::Okx, "BTC-USDT-SWAP") => Some("BTC-PERP"),
        _ => None,
    }
}

pub fn require_symbol(venue: Venue, symbol: &str) -> anyhow::Result<String> {
    normalize_symbol(venue, symbol)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("unsupported {venue} symbol: {symbol}"))
}
