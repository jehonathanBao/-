use crate::types::{
    flow::{FlowState, FlowWindow},
    sweep::SweepState,
    toxic_flow::{ActiveTradeToxicityFeatures, ActiveTradeToxicityReport},
};

const DEFAULT_WINDOW_MS: u64 = 5_000;

pub fn analyze_active_trade_toxicity(
    requested_symbol: &str,
    flow_state: &FlowState,
    sweep_state: &SweepState,
) -> ActiveTradeToxicityReport {
    if !flow_state.symbol.eq_ignore_ascii_case(requested_symbol) {
        return insufficient_data_report(
            requested_symbol,
            DEFAULT_WINDOW_MS,
            flow_state.updated_at.max(0) as u64,
            vec!["Requested symbol is not active in the current runtime.".to_string()],
            vec!["No active trade flow is available for the requested symbol.".to_string()],
        );
    }

    let generated_at_ms = flow_state.updated_at.max(0) as u64;
    let Some(window) = select_window(flow_state) else {
        return insufficient_data_report(
            requested_symbol,
            DEFAULT_WINDOW_MS,
            generated_at_ms,
            vec!["No flow windows are currently populated.".to_string()],
            vec!["Not enough recent aggressive trades to classify toxicity.".to_string()],
        );
    };

    if window.trade_count == 0 || window.abs_aggressive_btc <= f64::EPSILON {
        return insufficient_data_report(
            requested_symbol,
            window.window_ms,
            generated_at_ms,
            vec!["Recent flow window has no aggressive trade activity.".to_string()],
            vec!["Not enough recent aggressive trades to classify toxicity.".to_string()],
        );
    }

    let buy_volume = window.aggressive_buy_usd.max(0.0);
    let sell_volume = window.aggressive_sell_usd.max(0.0);
    let total_volume = buy_volume + sell_volume;
    let net_aggressive_volume = buy_volume - sell_volume;
    let imbalance_ratio = if total_volume > f64::EPSILON {
        (net_aggressive_volume.abs() / total_volume).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let large_trade_count = estimate_large_trade_count(window);
    let burst_score = compute_burst_score(window);
    let volume_spike_score = compute_volume_spike_score(flow_state, window);
    let sweep_watch = sweep_state
        .results
        .get(&window.window_ms.to_string())
        .map(|result| result.sweep_detected)
        .unwrap_or(false);

    let notional_pressure_score = if total_volume >= 5_000_000.0 {
        20.0
    } else if total_volume >= 1_000_000.0 {
        12.0
    } else if total_volume >= 250_000.0 {
        6.0
    } else {
        0.0
    };
    let sweep_bonus = if sweep_watch { 14.0 } else { 0.0 };
    let score = (imbalance_ratio * 45.0)
        + (burst_score * 0.15)
        + (volume_spike_score * 0.15)
        + ((large_trade_count as f64) * 12.0)
        + notional_pressure_score
        + sweep_bonus;
    let score = score.clamp(0.0, 100.0);

    let side_bias = if imbalance_ratio < 0.10 {
        "neutral".to_string()
    } else if net_aggressive_volume > 0.0 {
        "buy".to_string()
    } else if net_aggressive_volume < 0.0 {
        "sell".to_string()
    } else {
        "neutral".to_string()
    };

    let mut warnings = Vec::new();
    let mut no_trade_reasons = Vec::new();

    if !window.data_quality.has_books {
        warnings.push("Order book context is unavailable; sweep watch is flow-only.".to_string());
    }
    if volume_spike_score == 0.0 {
        warnings.push("Volume spike baseline is limited across current flow windows.".to_string());
    }
    if large_trade_count > 0 {
        warnings.push("Large trade concentration is elevated in the active window.".to_string());
    }
    if sweep_watch {
        warnings
            .push("Potential toxic sweep watch detected in the matching short window.".to_string());
    }

    let status = if window.trade_count < 3 || total_volume <= f64::EPSILON {
        no_trade_reasons
            .push("Not enough recent aggressive trades to classify toxicity.".to_string());
        "insufficient_data".to_string()
    } else if score < 35.0 && imbalance_ratio < 0.25 && !sweep_watch {
        no_trade_reasons.push("Flow is balanced or lacks directional stress.".to_string());
        "neutral".to_string()
    } else if sweep_watch && score >= 75.0 {
        "high_toxicity_watch".to_string()
    } else if side_bias == "buy" {
        "buy_toxicity_watch".to_string()
    } else if side_bias == "sell" {
        "sell_toxicity_watch".to_string()
    } else {
        no_trade_reasons
            .push("Directional bias is too weak for a toxicity watch classification.".to_string());
        "neutral".to_string()
    };

    if status == "neutral" {
        no_trade_reasons
            .push("This layer is analysis-only and does not emit trading actions.".to_string());
    }

    ActiveTradeToxicityReport {
        read_only: true,
        runtime_modified: false,
        symbol: requested_symbol.to_string(),
        window_ms: window.window_ms,
        generated_at_ms,
        status,
        score: round2(score),
        side_bias,
        features: ActiveTradeToxicityFeatures {
            trade_count: window.trade_count as usize,
            buy_volume: round2(buy_volume),
            sell_volume: round2(sell_volume),
            net_aggressive_volume: round2(net_aggressive_volume),
            imbalance_ratio: round4(imbalance_ratio),
            large_trade_count,
            burst_score: round2(burst_score),
            volume_spike_score: round2(volume_spike_score),
        },
        warnings,
        no_trade_reasons,
    }
}

fn select_window(flow_state: &FlowState) -> Option<&FlowWindow> {
    if let Some(window) = flow_state.windows.get(&DEFAULT_WINDOW_MS.to_string()) {
        if window.trade_count > 0 {
            return Some(window);
        }
    }

    flow_state
        .windows
        .values()
        .filter(|window| window.trade_count > 0)
        .max_by(|left, right| {
            left.trade_count
                .cmp(&right.trade_count)
                .then_with(|| right.window_ms.cmp(&left.window_ms))
        })
}

fn insufficient_data_report(
    symbol: &str,
    window_ms: u64,
    generated_at_ms: u64,
    warnings: Vec<String>,
    no_trade_reasons: Vec<String>,
) -> ActiveTradeToxicityReport {
    ActiveTradeToxicityReport {
        read_only: true,
        runtime_modified: false,
        symbol: symbol.to_string(),
        window_ms,
        generated_at_ms,
        status: "insufficient_data".to_string(),
        score: 0.0,
        side_bias: "neutral".to_string(),
        features: ActiveTradeToxicityFeatures {
            trade_count: 0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            net_aggressive_volume: 0.0,
            imbalance_ratio: 0.0,
            large_trade_count: 0,
            burst_score: 0.0,
            volume_spike_score: 0.0,
        },
        warnings,
        no_trade_reasons,
    }
}

fn estimate_large_trade_count(window: &FlowWindow) -> usize {
    if window.trade_count == 0 || window.avg_trade_size_btc <= f64::EPSILON {
        return 0;
    }

    let mut count = 0;
    if window.max_trade_size_btc >= 25.0
        && window.max_trade_size_btc >= window.avg_trade_size_btc * 2.0
    {
        count += 1;
    }
    if window.max_trade_size_btc >= 100.0
        && window.max_trade_size_btc >= window.avg_trade_size_btc * 4.0
    {
        count += 1;
    }
    count
}

fn compute_burst_score(window: &FlowWindow) -> f64 {
    if window.window_ms == 0 {
        return 0.0;
    }
    let trades_per_second = window.trade_count as f64 / (window.window_ms as f64 / 1000.0);
    (trades_per_second / 3.0 * 100.0).clamp(0.0, 100.0)
}

fn compute_volume_spike_score(flow_state: &FlowState, selected: &FlowWindow) -> f64 {
    let baseline = flow_state
        .windows
        .values()
        .filter(|window| {
            window.window_ms != selected.window_ms && window.abs_aggressive_btc > f64::EPSILON
        })
        .map(|window| window.aggressive_buy_usd + window.aggressive_sell_usd)
        .collect::<Vec<_>>();

    if baseline.is_empty() {
        return 0.0;
    }

    let baseline_avg = baseline.iter().sum::<f64>() / baseline.len() as f64;
    if baseline_avg <= f64::EPSILON {
        return 0.0;
    }

    let selected_total = selected.aggressive_buy_usd + selected.aggressive_sell_usd;
    ((selected_total / baseline_avg) * 25.0).clamp(0.0, 100.0)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
