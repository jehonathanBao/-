pub fn build_signal_dedupe_key(
    detector: &str,
    venue: &str,
    symbol: &str,
    side: Option<&str>,
    price_bucket: Option<f64>,
    window_start_ms: i64,
    window_ms: i64,
) -> String {
    let safe_window_ms = window_ms.max(1);
    let time_bucket = window_start_ms.div_euclid(safe_window_ms);
    let price = price_bucket
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{}:{}:{}:{}:{}:{}",
        detector,
        venue,
        symbol,
        side.unwrap_or("none"),
        price,
        time_bucket
    )
}
