//! V4.2 hybrid impact model.
//!
//! The model is deliberately conservative: the live event produces a
//! deterministic estimate immediately, while completed Binance outcomes are
//! blended in as evidence accumulates.  Historical rows are filtered by both
//! event time and outcome completion time, so a forecast can never see its
//! own future.

use super::{
    behavior_assessment::{build_detection_behavior, BehaviorDirectionBias},
    impact_forecast::{
        build_forecast, evaluate_horizon_outcomes, event_id, independent_calibration_outcomes,
        valid_calibration_outcome, ContractWhaleHorizonOutcome, ContractWhaleHorizonStats,
        ContractWhaleMultiHorizonImpactForecast, ContractWhaleOutcomeInputs, HORIZONS_SEC,
    },
    types::ContractWhaleSignal,
};

pub const CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION: &str =
    "cwm_impact_v4_2_hybrid_calibration_v2";

pub fn maturity_level(raw_samples: usize) -> &'static str {
    match raw_samples {
        0..=9 => "M0",
        10..=29 => "M1",
        30..=99 => "M2",
        100..=299 => "M3",
        _ => "M4",
    }
}

pub fn next_maturity_threshold(raw_samples: usize) -> usize {
    match raw_samples {
        0..=9 => 10,
        10..=29 => 30,
        30..=99 => 100,
        100..=299 => 300,
        _ => raw_samples,
    }
}

pub fn samples_until_next_maturity(raw_samples: usize) -> usize {
    next_maturity_threshold(raw_samples).saturating_sub(raw_samples)
}

pub fn effective_sample_size(weights: &[f64]) -> f64 {
    let sum = weights
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.0)
        .sum::<f64>();
    let square_sum = weights
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.0)
        .map(|v| v * v)
        .sum::<f64>();
    if square_sum <= 0.0 {
        0.0
    } else {
        (sum * sum / square_sum).max(0.0)
    }
}

fn direction_key(signal: &ContractWhaleSignal) -> &'static str {
    match build_detection_behavior(signal, None, signal.ts).direction_bias {
        BehaviorDirectionBias::Bullish => "bullish",
        BehaviorDirectionBias::Bearish => "bearish",
        BehaviorDirectionBias::Neutral => "neutral",
        BehaviorDirectionBias::Unknown => "unknown",
    }
}

fn behavior_key(signal: &ContractWhaleSignal) -> String {
    serde_json::to_string(&build_detection_behavior(signal, None, signal.ts).hypothesis)
        .unwrap_or_else(|_| "unclear".to_string())
        .trim_matches('"')
        .to_string()
}

fn regime_key(signal: &ContractWhaleSignal) -> String {
    super::impact_forecast::market_regime(&signal.market_driver.market_state)
}

fn intensity_key(signal: &ContractWhaleSignal) -> &'static str {
    match signal.score {
        0..=49 => "normal",
        50..=79 => "high",
        _ => "extreme",
    }
}

fn valid_prior(signal: &ContractWhaleSignal, outcome: &ContractWhaleHorizonOutcome) -> bool {
    let current = event_id(signal);
    valid_calibration_outcome(outcome, &signal.symbol, signal.ts, &current, &current)
}

fn cohort_weight(signal: &ContractWhaleSignal, outcome: &ContractWhaleHorizonOutcome) -> f64 {
    let behavior = behavior_key(signal);
    let direction = direction_key(signal);
    let regime = regime_key(signal);
    let intensity = intensity_key(signal);
    let exact = outcome.behavior == behavior
        && outcome.direction == direction
        && outcome.market_regime == regime
        && outcome.intensity_bucket == intensity;
    if exact {
        return 1.0;
    }
    if outcome.behavior == behavior
        && outcome.direction == direction
        && outcome.market_regime == regime
    {
        return 0.82;
    }
    if outcome.behavior == behavior && outcome.direction == direction {
        return 0.68;
    }
    if outcome.behavior == behavior {
        return 0.55;
    }
    if outcome.direction == direction && outcome.market_regime == regime {
        return 0.48;
    }
    if outcome.direction == direction {
        return 0.38;
    }
    0.20
}

fn weighted_quantile(values: &[(f64, f64)], q: f64) -> Option<f64> {
    let mut values = values
        .iter()
        .copied()
        .filter(|(value, weight)| value.is_finite() && weight.is_finite() && *weight > 0.0)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.0.total_cmp(&right.0));
    let total = values.iter().map(|(_, weight)| *weight).sum::<f64>();
    if total <= 0.0 {
        return None;
    }
    let target = total * q.clamp(0.0, 1.0);
    let mut cumulative = 0.0;
    for (value, weight) in &values {
        cumulative += *weight;
        if cumulative >= target {
            return Some(*value);
        }
    }
    values.last().map(|(value, _)| *value)
}

fn model_estimate(
    signal: &ContractWhaleSignal,
    horizon_sec: u64,
) -> (f64, f64, f64, f64, f64, f64) {
    let horizon_factor = match horizon_sec {
        900 => 0.70,
        3_600 => 1.0,
        14_400 => 1.35,
        _ => 1.80,
    };
    let price_move = signal.price_move_pct.unwrap_or(0.0).abs().min(5.0) * 8.0;
    let dominance = signal.dominance.clamp(0.0, 1.0) * 8.0;
    let oi = signal.oi_change_pct.unwrap_or(0.0).abs().min(5.0) * 1.5;
    let liquidation = (signal.liquidation_ratio.unwrap_or(0.0).abs()).min(1.0) * 8.0;
    let base = (2.0 + signal.score as f64 * 0.22 + price_move + dominance + oi + liquidation)
        * horizon_factor;
    let quality = (signal.data_quality as f64 / 100.0).clamp(0.0, 1.0);
    // Historical markouts are already signed into the hypothesis direction.
    // The prior must use that same coordinate system for both long and short.
    let median = base.max(1.0);
    let width = (base * (1.65 - quality * 0.65) + 4.0).max(4.0);
    let p25 = median - width;
    let p75 = median + width;
    let direction_probability =
        (0.50 + signal.score as f64 / 420.0 + quality * 0.06).clamp(0.50, 0.92);
    let reversal_probability = (1.0 - direction_probability).clamp(0.08, 0.50);
    let structure_probability = (direction_probability * (0.45 + quality * 0.45)).clamp(0.0, 0.95);
    (
        median,
        p25.min(p75),
        p25.max(p75),
        direction_probability,
        reversal_probability,
        structure_probability,
    )
}

fn blended_horizon(
    signal: &ContractWhaleSignal,
    horizon_sec: u64,
    outcomes: &[ContractWhaleHorizonOutcome],
    transaction_cost_bps: f64,
    prior_strength: f64,
) -> (ContractWhaleHorizonStats, usize, f64) {
    let mut weighted = Vec::new();
    let mut weights = Vec::new();
    let mut follow = Vec::new();
    let mut structure = Vec::new();
    let mut mfe = Vec::new();
    let mut mae = Vec::new();
    let mut exact = 0;
    let mut behavior_direction_regime = 0;
    let mut behavior_direction = 0;
    let mut direction_regime = 0;
    let mut direction = 0;
    let mut global = 0;
    let priors = independent_calibration_outcomes(
        outcomes
            .iter()
            .filter(|outcome| outcome.horizon_sec == horizon_sec && valid_prior(signal, outcome)),
    );
    for outcome in &priors {
        let weight = cohort_weight(signal, outcome);
        let markout = outcome.signed_markout_bps.unwrap_or(0.0);
        weighted.push((markout, weight));
        weights.push(weight);
        if outcome.follow_through.is_some() {
            follow.push((outcome.follow_through == Some(true), weight));
        }
        if outcome.structure_break.is_some() {
            structure.push((outcome.structure_break == Some(true), weight));
        }
        if let Some(value) = outcome.mfe_bps {
            mfe.push((value, weight));
        }
        if let Some(value) = outcome.mae_bps {
            mae.push((value, weight));
        }
        let behavior = behavior_key(signal);
        let dir = direction_key(signal);
        let regime = regime_key(signal);
        let intensity = intensity_key(signal);
        if outcome.behavior == behavior
            && outcome.direction == dir
            && outcome.market_regime == regime
            && outcome.intensity_bucket == intensity
        {
            exact += 1;
        } else if outcome.behavior == behavior
            && outcome.direction == dir
            && outcome.market_regime == regime
        {
            behavior_direction_regime += 1;
        } else if outcome.behavior == behavior && outcome.direction == dir {
            behavior_direction += 1;
        } else if outcome.direction == dir && outcome.market_regime == regime {
            direction_regime += 1;
        } else if outcome.direction == dir {
            direction += 1;
        } else {
            global += 1;
        }
    }
    let raw = weighted.len();
    let n_eff = effective_sample_size(&weights);
    let (model_median, model_p25, model_p75, model_direction, model_reversal, model_structure) =
        model_estimate(signal, horizon_sec);
    let historical_median = weighted_quantile(&weighted, 0.50);
    let historical_p25 = weighted_quantile(&weighted, 0.25);
    let historical_p75 = weighted_quantile(&weighted, 0.75);
    let sample_weight = if n_eff <= 0.0 {
        0.0
    } else {
        n_eff / (n_eff + prior_strength.max(1.0))
    };
    let model_weight = 1.0 - sample_weight;
    let blend = |historical: Option<f64>, model: f64| {
        historical
            .map(|v| v * sample_weight + model * model_weight)
            .unwrap_or(model)
    };
    let median = blend(historical_median, model_median);
    let p25 = blend(historical_p25, model_p25);
    let p75 = blend(historical_p75, model_p75);
    let follow_rate = if follow.is_empty() {
        None
    } else {
        Some(
            follow
                .iter()
                .map(|(v, w)| if *v { *w } else { 0.0 })
                .sum::<f64>()
                / follow.iter().map(|(_, w)| *w).sum::<f64>(),
        )
    };
    let structure_rate = if structure.is_empty() {
        None
    } else {
        Some(
            structure
                .iter()
                .map(|(v, w)| if *v { *w } else { 0.0 })
                .sum::<f64>()
                / structure.iter().map(|(_, w)| *w).sum::<f64>(),
        )
    };
    let expected_low_price = None;
    let expected_high_price = None;
    let rating = match signal.score {
        score if score >= 85 && signal.data_quality >= 75 => "S",
        score if score >= 70 && signal.data_quality >= 70 => "A",
        score if score >= 55 && signal.data_quality >= 60 => "B",
        _ => "C",
    };
    let stats = ContractWhaleHorizonStats {
        horizon: match horizon_sec {
            900 => "15m",
            3_600 => "1h",
            14_400 => "4h",
            _ => "1d",
        }
        .to_string(),
        horizon_sec,
        sample_count: raw,
        median_bps: Some(median),
        net_median_bps: Some(median - transaction_cost_bps),
        p25_bps: Some(p25),
        p75_bps: Some(p75),
        mfe_bps: Some(blend(
            weighted_quantile(&mfe, 0.50),
            model_median.abs() * 1.4,
        )),
        mae_bps: Some(blend(
            weighted_quantile(&mae, 0.50),
            -model_median.abs() * 0.9,
        )),
        follow_through_rate: follow_rate.or(Some(model_direction)),
        structure_break_rate: structure_rate.or(Some(model_structure)),
        state: maturity_level(raw).to_string(),
        baseline_level: if exact > 0 {
            "exact"
        } else if raw > 0 {
            "hierarchical"
        } else {
            "model"
        }
        .to_string(),
        cohort_sample_count: exact,
        exact_sample_count: exact,
        behavior_regime_sample_count: behavior_direction_regime,
        behavior_sample_count: behavior_direction,
        direction_sample_count: direction + direction_regime,
        global_sample_count: global,
        fallback_sample_count: raw.saturating_sub(exact),
        sample_from_ts: priors.iter().map(|o| o.event_ts).min(),
        sample_to_ts: priors.iter().map(|o| o.event_ts).max(),
        source_policy: "binance_only".to_string(),
        expected_low_price,
        expected_high_price,
        rating: rating.to_string(),
        model_median_bps: Some(model_median),
        model_p25_bps: Some(model_p25),
        model_p75_bps: Some(model_p75),
        model_weight,
        sample_weight,
        raw_sample_count: raw,
        effective_sample_count: n_eff,
        direction_probability: Some(model_direction),
        reversal_probability: Some(model_reversal),
        structure_break_probability: Some(model_structure),
        confirmation_rule: "同向价格延续并伴随 OI/主动流确认".to_string(),
        invalidation_rule: "反向签名 markout 穿越失效阈值或结构回收".to_string(),
        data_quality: signal.data_quality,
    };
    (stats, raw, n_eff)
}

pub fn build_hybrid_forecast(
    signal: &ContractWhaleSignal,
    historical: &[ContractWhaleHorizonOutcome],
    reference_prices: &[super::types::ContractReferencePriceSnapshot],
    computed_at_ms: i64,
) -> ContractWhaleMultiHorizonImpactForecast {
    let mut forecast = build_forecast(signal, historical, reference_prices, computed_at_ms);
    let config = super::config::contract_whale_runtime_config().impact_v4_2;
    let transaction_cost = forecast.transaction_cost_bps;
    let mut raw_total = 0usize;
    let mut effective_total: f64 = 0.0;
    let horizons = HORIZONS_SEC
        .into_iter()
        .map(|horizon_sec| {
            let (stats, raw, effective) = blended_horizon(
                signal,
                horizon_sec,
                historical,
                transaction_cost,
                config.prior_strength,
            );
            raw_total = raw_total.max(raw);
            effective_total = effective_total.max(effective);
            stats
        })
        .collect::<Vec<_>>();
    let maturity = maturity_level(raw_total).to_string();
    let severity = match signal.score {
        score if score >= config.s_score && signal.data_quality >= config.s_min_data_quality => "S",
        score if score >= config.a_score && signal.data_quality >= config.a_min_data_quality => "A",
        score if score >= config.b_score && signal.data_quality >= config.b_min_data_quality => "B",
        _ => "C",
    };
    let sample_weight = if effective_total <= 0.0 {
        0.0
    } else {
        effective_total / (effective_total + config.prior_strength.max(1.0))
    };
    forecast.forecast_version = CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION.to_string();
    forecast.impact_grade = severity.to_string();
    forecast.signal_severity = severity.to_string();
    forecast.maturity_state = maturity.clone();
    forecast.status = "realtime_ready".to_string();
    forecast.horizons = horizons;
    forecast.raw_sample_count = raw_total;
    forecast.exact_sample_count = forecast
        .horizons
        .iter()
        .map(|h| h.exact_sample_count)
        .max()
        .unwrap_or(0);
    forecast.effective_sample_count = effective_total.round() as usize;
    forecast.maturity_level = maturity;
    forecast.prediction_source = if raw_total == 0 {
        "model_estimate"
    } else if effective_total < 300.0 {
        "hybrid"
    } else {
        "historical_validated"
    }
    .to_string();
    forecast.model_weight = 1.0 - sample_weight;
    forecast.sample_weight = sample_weight;
    forecast.next_maturity_threshold = next_maturity_threshold(raw_total);
    forecast.samples_until_next_maturity = samples_until_next_maturity(raw_total);
    forecast.direction_probability = forecast
        .horizons
        .first()
        .and_then(|h| h.direction_probability);
    forecast.early_warning = config.dashboard_early_warning_enabled;
    forecast.external_alert_enabled =
        config.external_directional_alerts_enabled && !config.shadow_mode;
    forecast.prior_strength = config.prior_strength;
    forecast.baseline_level = forecast
        .horizons
        .iter()
        .find(|h| h.sample_count > 0)
        .map(|h| h.baseline_level.clone())
        .unwrap_or_else(|| "model".to_string());
    forecast.strategy_mode = if config.shadow_mode {
        "v4_2_hybrid_shadow"
    } else {
        "v4_2_auto_gate"
    }
    .to_string();
    forecast.production_ready = false;
    forecast
}

pub fn evaluate_v42_outcomes(
    signal: &ContractWhaleSignal,
    inputs: ContractWhaleOutcomeInputs<'_>,
    now_ms: i64,
) -> Vec<ContractWhaleHorizonOutcome> {
    evaluate_horizon_outcomes(signal, inputs, now_ms)
        .into_iter()
        .map(|mut outcome| {
            outcome.outcome_version = CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION.to_string();
            outcome
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::impact_forecast::calibration_tests::{prior, signal};
    use super::*;

    #[test]
    fn calibration_mirrored_prior_quantiles_and_net_returns_match() {
        let long = blended_horizon(&signal("long", 2_000_000, false), 900, &[], 15.0, 30.0).0;
        let short = blended_horizon(&signal("short", 2_000_000, true), 900, &[], 15.0, 30.0).0;
        assert_eq!(long.model_median_bps, short.model_median_bps);
        assert_eq!(long.p25_bps, short.p25_bps);
        assert_eq!(long.p75_bps, short.p75_bps);
        assert_eq!(long.net_median_bps, short.net_median_bps);
    }

    #[test]
    fn calibration_duplicate_versions_and_repeated_history_are_stable() {
        let signal = signal("current", 2_000_000, false);
        let base = prior("prior", 60_000, false);
        let mut hybrid = base.clone();
        hybrid.outcome_version = CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION.into();
        hybrid.signed_markout_bps = Some(20.0);
        let one = build_hybrid_forecast(&signal, &[hybrid.clone()], &[], signal.ts);
        let duplicate = build_hybrid_forecast(
            &signal,
            &[base.clone(), hybrid.clone(), base, hybrid],
            &[],
            signal.ts,
        );
        assert_eq!(duplicate.horizons[0].sample_count, 1);
        assert_eq!(
            serde_json::to_value(one).unwrap(),
            serde_json::to_value(duplicate).unwrap()
        );
    }

    #[test]
    fn calibration_invalid_current_and_unmatured_priors_are_excluded() {
        let signal = signal("current", 2_000_000, false);
        let valid = prior("prior", 60_000, false);
        assert!(valid_prior(&signal, &valid));
        for case in 0..9 {
            let mut invalid = valid.clone();
            match case {
                0 => invalid.state = "closed".into(),
                1 => invalid.signed_markout_bps = Some(f64::NAN),
                2 => invalid.price_coverage = 0.79,
                3 => invalid.data_quality = 69,
                4 => invalid.episode_id = "current".into(),
                5 => invalid.event_ts = signal.ts - 1,
                6 => invalid.outcome_version = "cwm_impact_v4_2_hybrid".into(),
                7 => invalid.price_data_degraded = true,
                _ => invalid.reference_price_available = false,
            }
            assert!(!valid_prior(&signal, &invalid), "accepted case {case}");
        }
    }

    #[test]
    fn maturity_thresholds_are_reachable_without_three_hundred_s_signals() {
        assert_eq!(maturity_level(0), "M0");
        assert_eq!(maturity_level(9), "M0");
        assert_eq!(maturity_level(10), "M1");
        assert_eq!(maturity_level(30), "M2");
        assert_eq!(maturity_level(100), "M3");
        assert_eq!(maturity_level(300), "M4");
    }

    #[test]
    fn effective_n_is_monotonic_and_zero_safe() {
        assert_eq!(effective_sample_size(&[]), 0.0);
        assert!((effective_sample_size(&[1.0, 1.0, 1.0]) - 3.0).abs() < 1e-9);
        assert!(effective_sample_size(&[1.0, 0.82, 0.68]) > 2.0);
    }
}
