use crate::types::market::Venue;

pub fn canonical_base_asset(symbol: &str) -> Option<String> {
    let upper = symbol.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return None;
    }
    let without_bitfinex_prefix = if upper.starts_with('T') && upper.contains("F0") {
        &upper[1..]
    } else {
        &upper
    };
    let first = without_bitfinex_prefix
        .split([':', '/', '_'])
        .next()
        .unwrap_or(without_bitfinex_prefix);
    let base = first.split('-').next().unwrap_or(first);
    let base = base
        .trim_end_matches("PERP")
        .trim_end_matches("SWAP")
        .trim_end_matches("USDT")
        .trim_end_matches("USDC")
        .trim_end_matches("USD")
        .trim_end_matches("F0");
    (!base.is_empty()).then(|| base.to_string())
}

pub fn canonical_perp_symbol(symbol: &str) -> Option<String> {
    canonical_base_asset(symbol).map(|base| format!("{base}-PERP"))
}

pub fn normalize_symbol(venue: Venue, symbol: &str) -> Option<&'static str> {
    match (venue, symbol) {
        (Venue::Binance, "BTCUSDT" | "btcusdt") => Some("BTC-PERP"),
        (Venue::Binance, "ETHUSDT" | "ethusdt") => Some("ETH-PERP"),
        (Venue::Bybit, "BTCUSDT") => Some("BTC-PERP"),
        (Venue::Bybit, "ETHUSDT" | "ethusdt") => Some("ETH-PERP"),
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
