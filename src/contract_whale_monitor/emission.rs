use super::{
    config::ContractWhaleEmissionConfig,
    types::{
        ContractWhaleDirection, ContractWhaleEmissionFingerprint, ContractWhaleSeverity,
        ContractWhaleSignal,
    },
};

pub fn emission_key(signal: &ContractWhaleSignal) -> String {
    format!(
        "{}:{}:{:?}:{:?}",
        signal.symbol.to_ascii_uppercase(),
        signal.window_sec,
        signal.signal_type,
        signal.direction
    )
}

pub fn fingerprint(
    signal: &ContractWhaleSignal,
    emitted_at: i64,
) -> ContractWhaleEmissionFingerprint {
    ContractWhaleEmissionFingerprint {
        source_window_end_ts: signal.ts,
        severity: signal.severity,
        impact_level: signal.impact_level.clone(),
        classification: signal.classification_v2.structure_interpretation,
        score: signal.score,
        total_volume_btc: signal.total_volume_btc,
        net_volume_btc: signal.net_volume_btc,
        last_emitted_at: emitted_at,
    }
}

pub fn should_emit(
    signal: &ContractWhaleSignal,
    previous: Option<&ContractWhaleEmissionFingerprint>,
    now_ms: i64,
    config: &ContractWhaleEmissionConfig,
) -> bool {
    if !config.enabled || previous.is_none() {
        return true;
    }
    let previous = previous.expect("checked above");
    if signal.severity != previous.severity
        || signal.impact_level != previous.impact_level
        || signal.classification_v2.structure_interpretation != previous.classification
        || net_direction_flipped(
            signal.direction,
            previous.net_volume_btc,
            signal.net_volume_btc,
        )
    {
        return true;
    }
    if signal.score.abs_diff(previous.score) >= config.score_delta_min {
        return true;
    }
    let baseline = previous.total_volume_btc.abs().max(1.0);
    if (signal.total_volume_btc - previous.total_volume_btc).abs() / baseline
        >= config.volume_delta_ratio_min
    {
        return true;
    }
    if now_ms.saturating_sub(previous.last_emitted_at)
        >= config.force_refresh_seconds.saturating_mul(1000)
    {
        return true;
    }

    // A window end advancing by a full window is fresh, non-overlapping evidence.
    signal.ts.saturating_sub(previous.source_window_end_ts)
        >= (signal.window_sec as i64).saturating_mul(1000)
}

fn net_direction_flipped(
    direction: ContractWhaleDirection,
    previous_net_volume_btc: f64,
    net_volume_btc: f64,
) -> bool {
    let previous_sign = previous_net_volume_btc.signum();
    let next_sign = net_volume_btc.signum();
    (previous_sign != 0.0 && next_sign != 0.0 && previous_sign != next_sign)
        || matches!(direction, ContractWhaleDirection::Buy) && previous_net_volume_btc < 0.0
        || matches!(direction, ContractWhaleDirection::Sell) && previous_net_volume_btc > 0.0
}

pub fn severity_is_critical_or_s(severity: ContractWhaleSeverity) -> bool {
    matches!(
        severity,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S
    )
}

#[cfg(test)]
mod tests {
    use crate::contract_whale_monitor::{
        aggregator::{aggregate_1s_buckets, rolling_window_stats},
        detector::detect_contract_whale_signal,
        normalizer::normalize_binance_agg_trade,
    };

    use super::*;

    fn sample_signal() -> ContractWhaleSignal {
        let now = 1_700_000_015_000;
        let buckets = aggregate_1s_buckets(&[normalize_binance_agg_trade(
            now - 1_000,
            70_000.0,
            1_800.0,
            false,
        )
        .unwrap()]);
        let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.4), 94)
            .expect("window stats");
        stats.percentile_level = Some(99.9);
        detect_contract_whale_signal(&stats).expect("signal")
    }

    #[test]
    fn watermark_suppresses_near_duplicate_but_keeps_material_change() {
        let signal = sample_signal();
        let config = ContractWhaleEmissionConfig::default();
        let now = signal.ts;
        let previous = fingerprint(&signal, now);

        assert!(!should_emit(&signal, Some(&previous), now + 2_000, &config));

        let mut upgraded = signal.clone();
        upgraded.score = upgraded.score.saturating_add(config.score_delta_min);
        assert!(should_emit(
            &upgraded,
            Some(&previous),
            now + 2_000,
            &config
        ));
    }

    #[test]
    fn watermark_always_allows_forced_refresh_and_disabled_mode() {
        let signal = sample_signal();
        let config = ContractWhaleEmissionConfig::default();
        let previous = fingerprint(&signal, signal.ts);
        assert!(should_emit(
            &signal,
            Some(&previous),
            signal.ts + config.force_refresh_seconds * 1_000,
            &config,
        ));

        let disabled = ContractWhaleEmissionConfig {
            enabled: false,
            ..config
        };
        assert!(should_emit(
            &signal,
            Some(&previous),
            signal.ts + 1,
            &disabled
        ));
    }
}
