use std::fmt::Write;

use crate::types::toxic::{ToxicDirection, ToxicEvent, ToxicSeverity, ToxicState};

pub fn format_alert_message(event: &ToxicEvent, state: &ToxicState) -> String {
    let result = state.results.get(&event.window_ms.to_string());
    let active_venues = state.quality.active_venues.len();
    let leader_venue = event
        .leader_venue
        .map(|venue| venue.as_key().to_ascii_uppercase())
        .unwrap_or_else(|| "unknown".to_string());
    let direction = match event.direction {
        ToxicDirection::Buy => "BUY TOXIC",
        ToxicDirection::Sell => "SELL TOXIC",
        ToxicDirection::Neutral => "NEUTRAL",
    };

    let (aggressive_buy_btc, aggressive_sell_btc, net_aggressive_btc, toxic_ratio, liquidity) =
        result
            .map(|result| {
                (
                    result.aggressive_buy_btc,
                    result.aggressive_sell_btc,
                    result.net_aggressive_btc,
                    result.toxic_ratio,
                    result.liquidity.as_ref(),
                )
            })
            .unwrap_or((
                event.aggressive_buy_btc,
                event.aggressive_sell_btc,
                event.net_aggressive_btc,
                0.0,
                None,
            ));

    let imbalance_ratio = if (aggressive_buy_btc + aggressive_sell_btc).abs() > f64::EPSILON {
        (net_aggressive_btc.abs() / (aggressive_buy_btc + aggressive_sell_btc)) * 100.0
    } else {
        0.0
    };

    let markout_1s = format_bps(event.markout_1s_bps);
    let markout_5s = format_bps(event.markout_5s_bps);
    let ask_thin = liquidity.is_some_and(|liquidity| liquidity.ask_thin);
    let bid_thin = liquidity.is_some_and(|liquidity| liquidity.bid_thin);
    let spread_widened = liquidity.is_some_and(|liquidity| liquidity.spread_widened);

    let mut message = String::new();
    let _ = writeln!(message, "BTC Perp Toxic Flow Alert");
    let _ = writeln!(message);
    let _ = writeln!(message, "Severity: {}", event.severity.label());
    let _ = writeln!(message, "Direction: {direction}");
    let _ = writeln!(
        message,
        "Toxic Volume: {} BTC",
        format_btc(event.toxic_volume_btc)
    );
    let _ = writeln!(
        message,
        "Threshold: {} BTC",
        format_btc(event.threshold_btc)
    );
    let _ = writeln!(message, "Window: {}s", event.window_ms / 1000);
    let _ = writeln!(message, "Leader Venue: {leader_venue}");
    let _ = writeln!(message);
    let _ = writeln!(message, "Flow:");
    let _ = writeln!(
        message,
        "Aggressive Buy: {} BTC",
        format_btc(aggressive_buy_btc)
    );
    let _ = writeln!(
        message,
        "Aggressive Sell: {} BTC",
        format_btc(aggressive_sell_btc)
    );
    let _ = writeln!(
        message,
        "Net Flow: {} BTC",
        format_signed_btc(net_aggressive_btc)
    );
    let _ = writeln!(
        message,
        "Imbalance Ratio: {}%",
        format_percent(imbalance_ratio)
    );
    let _ = writeln!(
        message,
        "Toxic Ratio: {}%",
        format_percent(toxic_ratio * 100.0)
    );
    let _ = writeln!(message);
    let _ = writeln!(message, "Markout:");
    let _ = writeln!(message, "1s: {markout_1s} bps");
    let _ = writeln!(message, "5s: {markout_5s} bps");
    let _ = writeln!(message);
    let _ = writeln!(message, "Liquidity:");
    let _ = writeln!(message, "Sweep: {}", bool_text(event.sweep_detected));
    let _ = writeln!(message, "Ask Thin: {}", bool_text(ask_thin));
    let _ = writeln!(message, "Bid Thin: {}", bool_text(bid_thin));
    let _ = writeln!(message, "Spread Widened: {}", bool_text(spread_widened));
    let _ = writeln!(message);
    let _ = writeln!(message, "Cross Venue:");
    let _ = writeln!(
        message,
        "Confirmed: {}",
        bool_text(event.cross_venue_confirmed)
    );
    let _ = writeln!(message, "Active Venues: {active_venues}");
    let _ = writeln!(message);
    let _ = writeln!(message, "Reason:");
    for reason in event.reason_codes.iter().take(10) {
        let _ = writeln!(message, "- {reason}");
    }
    message
}

pub fn severity_label(severity: ToxicSeverity) -> &'static str {
    severity.label()
}

fn format_btc(value: f64) -> String {
    format_with_commas(value, 2)
}

fn format_signed_btc(value: f64) -> String {
    if value >= 0.0 {
        format!("+{}", format_btc(value))
    } else {
        format!("-{}", format_btc(value.abs()))
    }
}

fn format_bps(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{v:+.1}"))
}

fn format_percent(value: f64) -> String {
    format_with_commas(value, 1)
}

fn format_with_commas(value: f64, decimals: usize) -> String {
    let raw = format!("{value:.prec$}", prec = decimals);
    let (sign, digits) = if let Some(rest) = raw.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", raw.as_str())
    };

    let mut parts = digits.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();

    let mut grouped = String::new();
    for (idx, ch) in integer.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let integer_grouped: String = grouped.chars().rev().collect();
    let fraction_trimmed = fraction.trim_end_matches('0');
    if fraction_trimmed.is_empty() {
        format!("{sign}{integer_grouped}")
    } else {
        format!("{sign}{integer_grouped}.{fraction_trimmed}")
    }
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
