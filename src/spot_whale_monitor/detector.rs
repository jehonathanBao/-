use super::{
    config::SpotWhaleRuntimeConfig,
    types::{
        SpotWhaleDirection, SpotWhaleSeverity, SpotWhaleSignal, SpotWhaleSignalType,
        SpotWhaleWindowStats,
    },
    LOG_PREFIX, LOG_TARGET,
};

pub fn detect_spot_whale_signal_with_config(
    stats: &SpotWhaleWindowStats,
    config: &SpotWhaleRuntimeConfig,
) -> Option<SpotWhaleSignal> {
    if !config.symbol_enabled(&stats.symbol) || stats.total_volume_base <= 0.0 {
        return None;
    }
    let signal_type = classify_signal_type(stats)?;
    let severity = classify_severity(stats, signal_type, config);
    if severity == SpotWhaleSeverity::Calm {
        return None;
    }
    let score = score_signal(stats, signal_type, severity, config);
    let (discord_eligible, discord_reason) = discord_gate(
        severity,
        score,
        stats.multi_exchange_confirmed,
        stats.data_quality,
    );
    let direction = direction_for(signal_type);
    let signal = SpotWhaleSignal {
        id: format!(
            "spot-whale:{}:{}:{}:{}",
            stats.symbol,
            stats.window_sec,
            stats.ts,
            direction_label(direction)
        ),
        ts: stats.ts,
        symbol: stats.symbol.clone(),
        window_sec: stats.window_sec,
        signal_type,
        direction,
        severity,
        score,
        total_volume_base: round(stats.total_volume_base, 4),
        net_volume_base: round(stats.net_volume_base, 4),
        total_notional_usd: round(stats.total_notional_usd, 2),
        dominance: round(stats.dominance, 4),
        price_move_pct: stats.price_move_pct.map(|value| round(value, 4)),
        coinbase_premium_pct: stats.coinbase_premium_pct.map(|value| round(value, 4)),
        main_exchange: stats.main_exchange.clone(),
        exchanges: stats.exchanges.clone(),
        dynamic_multiple: stats.dynamic_multiple.map(|value| round(value, 3)),
        multi_exchange_confirmed: stats.multi_exchange_confirmed,
        data_quality: stats.data_quality,
        discord_eligible,
        discord_sent: false,
        discord_sent_at: None,
        discord_reason,
        final_result: final_result_text(signal_type),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
    };
    if signal.severity.rank() >= SpotWhaleSeverity::High.rank() {
        tracing::info!(
            target: LOG_TARGET,
            symbol = signal.symbol.as_str(),
            severity = ?signal.severity,
            score = signal.score,
            "{} signal generated",
            LOG_PREFIX
        );
    } else {
        tracing::debug!(
            target: LOG_TARGET,
            symbol = signal.symbol.as_str(),
            severity = ?signal.severity,
            score = signal.score,
            "{} signal generated",
            LOG_PREFIX
        );
    }
    Some(signal)
}

fn classify_signal_type(stats: &SpotWhaleWindowStats) -> Option<SpotWhaleSignalType> {
    if stats
        .coinbase_premium_pct
        .is_some_and(|premium| premium.abs() >= 0.18)
        && stats.exchange_count >= 2
    {
        return Some(SpotWhaleSignalType::SpotExchangeDislocation);
    }
    let price_move_pct = stats.price_move_pct.unwrap_or(0.0);
    if stats.net_volume_base > 0.0 {
        if stats.dominance >= 0.60 && price_move_pct < 0.05 {
            Some(SpotWhaleSignalType::SpotUpsideSuppression)
        } else {
            Some(SpotWhaleSignalType::SpotAggressiveBuy)
        }
    } else if stats.net_volume_base < 0.0 {
        if stats.dominance >= 0.60 && price_move_pct > -0.05 {
            Some(SpotWhaleSignalType::SpotDownsideAbsorption)
        } else {
            Some(SpotWhaleSignalType::SpotAggressiveSell)
        }
    } else {
        None
    }
}

fn classify_severity(
    stats: &SpotWhaleWindowStats,
    signal_type: SpotWhaleSignalType,
    config: &SpotWhaleRuntimeConfig,
) -> SpotWhaleSeverity {
    let thresholds = config.thresholds_for_symbol_window(&stats.symbol, stats.window_sec);
    let same_direction_price_move = same_direction_price_move(stats, signal_type);
    let muted = matches!(
        signal_type,
        SpotWhaleSignalType::SpotDownsideAbsorption
            | SpotWhaleSignalType::SpotUpsideSuppression
            | SpotWhaleSignalType::SpotExchangeDislocation
    );
    let dynamic_multiple = stats.dynamic_multiple.unwrap_or(0.0);
    let warmup = stats
        .startup_age_ms
        .is_some_and(|age| age < config.data_quality.warmup_ms);

    if stats.total_volume_base >= thresholds.s_base
        && stats.total_notional_usd >= thresholds.s_notional_usd
        && stats.dominance >= 0.65
        && stats.data_quality >= 70
        && stats.multi_exchange_confirmed
        && !warmup
        && (same_direction_price_move >= 0.25 || muted)
    {
        return SpotWhaleSeverity::S;
    }
    if stats.total_volume_base >= thresholds.critical_base
        && stats.total_notional_usd >= thresholds.critical_notional_usd
        && stats.dominance >= 0.60
        && stats.data_quality >= 70
        && !warmup
        && (stats.multi_exchange_confirmed || stats.exchange_count >= 1)
        && (same_direction_price_move >= 0.15 || muted)
    {
        return SpotWhaleSeverity::Critical;
    }
    if stats.total_volume_base >= thresholds.high_base
        && stats.total_notional_usd >= thresholds.high_notional_usd
        && stats.dominance >= 0.55
        && stats.data_quality >= 60
    {
        return SpotWhaleSeverity::High;
    }
    if stats.total_volume_base >= thresholds.high_base * 0.5 || dynamic_multiple >= 4.0 {
        return SpotWhaleSeverity::Medium;
    }
    SpotWhaleSeverity::Calm
}

fn score_signal(
    stats: &SpotWhaleWindowStats,
    signal_type: SpotWhaleSignalType,
    severity: SpotWhaleSeverity,
    config: &SpotWhaleRuntimeConfig,
) -> u8 {
    let thresholds = config.thresholds_for_symbol_window(&stats.symbol, stats.window_sec);
    let volume_score = (stats.total_volume_base / thresholds.s_base * 35.0).clamp(0.0, 35.0);
    let dynamic_score = stats
        .dynamic_multiple
        .map(|multiple| (multiple / 10.0 * 20.0).clamp(0.0, 20.0))
        .unwrap_or(0.0);
    let dominance_score = ((stats.dominance - 0.50) / 0.25 * 15.0).clamp(0.0, 15.0);
    let price_score = price_impact_score(stats, signal_type);
    let exchange_score = if stats.multi_exchange_confirmed {
        10.0
    } else if stats.exchange_count == 1 {
        4.0
    } else {
        0.0
    };
    let data_quality_score = stats.data_quality as f64 / 100.0 * 5.0;
    let mut score = volume_score
        + dynamic_score
        + dominance_score
        + price_score
        + exchange_score
        + data_quality_score;
    if stats.exchange_count == 1 && severity.rank() >= SpotWhaleSeverity::Critical.rank() {
        score -= 10.0;
    }
    if stats
        .startup_age_ms
        .is_some_and(|age| age < config.data_quality.warmup_ms)
    {
        score -= 20.0;
    }
    score.round().clamp(0.0, 100.0) as u8
}

pub fn discord_gate(
    severity: SpotWhaleSeverity,
    score: u8,
    multi_exchange_confirmed: bool,
    data_quality: u8,
) -> (bool, String) {
    if data_quality < 70 {
        return (false, "data_quality_display_only".to_string());
    }
    match severity {
        SpotWhaleSeverity::S | SpotWhaleSeverity::Critical => {
            (score >= 80, "critical_or_s_gate".to_string())
        }
        SpotWhaleSeverity::High if score >= 85 && multi_exchange_confirmed => {
            (true, "high_score_multi_exchange".to_string())
        }
        SpotWhaleSeverity::High => (false, "high_without_discord_confirmation".to_string()),
        SpotWhaleSeverity::Medium | SpotWhaleSeverity::Calm => {
            (false, "medium_or_low_display_only".to_string())
        }
    }
}

fn price_impact_score(stats: &SpotWhaleWindowStats, signal_type: SpotWhaleSignalType) -> f64 {
    let price_move = stats.price_move_pct.unwrap_or(0.0).abs();
    match signal_type {
        SpotWhaleSignalType::SpotAggressiveBuy | SpotWhaleSignalType::SpotAggressiveSell => {
            (price_move / 0.25 * 15.0).clamp(0.0, 15.0)
        }
        SpotWhaleSignalType::SpotDownsideAbsorption
        | SpotWhaleSignalType::SpotUpsideSuppression => {
            if price_move <= 0.05 {
                12.0
            } else {
                6.0
            }
        }
        SpotWhaleSignalType::SpotExchangeDislocation => 10.0,
    }
}

fn same_direction_price_move(
    stats: &SpotWhaleWindowStats,
    signal_type: SpotWhaleSignalType,
) -> f64 {
    let price_move_pct = stats.price_move_pct.unwrap_or(0.0);
    match signal_type {
        SpotWhaleSignalType::SpotAggressiveBuy => price_move_pct.max(0.0),
        SpotWhaleSignalType::SpotAggressiveSell => (-price_move_pct).max(0.0),
        _ => 0.0,
    }
}

fn direction_for(signal_type: SpotWhaleSignalType) -> SpotWhaleDirection {
    match signal_type {
        SpotWhaleSignalType::SpotAggressiveBuy => SpotWhaleDirection::Buy,
        SpotWhaleSignalType::SpotAggressiveSell => SpotWhaleDirection::Sell,
        SpotWhaleSignalType::SpotDownsideAbsorption => SpotWhaleDirection::Absorption,
        SpotWhaleSignalType::SpotUpsideSuppression => SpotWhaleDirection::Suppression,
        SpotWhaleSignalType::SpotExchangeDislocation => SpotWhaleDirection::Dislocation,
    }
}

fn direction_label(direction: SpotWhaleDirection) -> &'static str {
    match direction {
        SpotWhaleDirection::Buy => "buy",
        SpotWhaleDirection::Sell => "sell",
        SpotWhaleDirection::Absorption => "absorption",
        SpotWhaleDirection::Suppression => "suppression",
        SpotWhaleDirection::Dislocation => "dislocation",
    }
}

fn final_result_text(signal_type: SpotWhaleSignalType) -> String {
    match signal_type {
        SpotWhaleSignalType::SpotAggressiveBuy => {
            "Binance / Coinbase 现货主动买入同步放大，疑似现货资金主动推升".to_string()
        }
        SpotWhaleSignalType::SpotAggressiveSell => {
            "Binance / Coinbase 现货主动卖出同步放大，疑似现货资金主动砸盘".to_string()
        }
        SpotWhaleSignalType::SpotDownsideAbsorption => {
            "现货主动卖出放大但价格未明显下跌，疑似下方现货承接吸收".to_string()
        }
        SpotWhaleSignalType::SpotUpsideSuppression => {
            "现货主动买入放大但价格未明显上涨，疑似上方现货卖盘压制".to_string()
        }
        SpotWhaleSignalType::SpotExchangeDislocation => {
            "Binance 与 Coinbase 现货价格出现异常偏离，疑似跨交易所现货错位".to_string()
        }
    }
}

fn round(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}
