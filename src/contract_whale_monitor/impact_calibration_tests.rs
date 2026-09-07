use super::super::types::{ContractExchange, ContractWhaleActiveFlowDirection};
use super::*;

pub(crate) fn signal(id: &str, ts: i64, bearish: bool) -> ContractWhaleSignal {
    let mut signal: ContractWhaleSignal = serde_json::from_value(serde_json::json!({
        "id":id, "ts":ts, "symbol":"BTC", "windowSec":15,
        "signalType":"aggressive_sell", "direction":if bearish { "sell" } else { "buy" },
        "severity":"high", "score":80, "totalVolumeBtc":100,
        "netVolumeBtc":if bearish { -80 } else { 80 }, "totalNotionalUsd":8000000,
        "dominance":0.8, "mainExchange":"binance", "exchanges":[], "dataQuality":90,
        "discordEligible":false, "discordSent":false, "discordReason":"", "finalResult":"",
        "readOnly":true, "analysisOnly":true, "executionEnabled":false
    }))
    .unwrap();
    signal.classification_v2.flow_direction = if bearish {
        ContractWhaleActiveFlowDirection::SellDominant
    } else {
        ContractWhaleActiveFlowDirection::BuyDominant
    };
    signal.price_move_pct = Some(if bearish { -0.2 } else { 0.2 });
    signal.oi_change_pct = Some(0.3);
    signal
}

pub(crate) fn reference(ts: i64, source: &str, price: f64) -> ContractReferencePriceSnapshot {
    ContractReferencePriceSnapshot {
        ts_bucket: ts.div_euclid(60_000) * 60_000,
        exchange: ContractExchange::Binance,
        symbol: "BTC".into(),
        price_source: source.into(),
        price,
        premium_bps: None,
        event_time_ms: ts,
        received_at_ms: ts,
    }
}

pub(crate) fn outcomes(
    signal: &ContractWhaleSignal,
    refs: &[ContractReferencePriceSnapshot],
    now: i64,
) -> Vec<ContractWhaleHorizonOutcome> {
    evaluate_horizon_outcomes(
        signal,
        ContractWhaleOutcomeInputs {
            flow_buckets: &[],
            reference_prices: refs,
            oi_snapshots: &[],
            funding_snapshots: &[],
            liquidation_buckets: &[],
        },
        now,
    )
}

pub(crate) fn prior(id: &str, ts: i64, bearish: bool) -> ContractWhaleHorizonOutcome {
    let signal = signal(id, ts, bearish);
    let refs = (0..=15)
        .map(|minute| {
            reference(
                ts + minute * 60_000,
                "index",
                100.0
                    + if bearish {
                        -(minute as f64) / 15.0
                    } else {
                        minute as f64 / 15.0
                    },
            )
        })
        .collect::<Vec<_>>();
    outcomes(&signal, &refs, ts + 900_000).remove(0)
}

#[test]
fn calibration_bullish_control_and_exact_bearish_signed_outcome() {
    for bearish in [false, true] {
        let signal = signal("signed", 60_000, bearish);
        let entry = 77329.07249373439;
        let end = if bearish {
            77190.32824074072
        } else {
            entry + (entry - 77190.32824074072)
        };
        let values = outcomes(
            &signal,
            &[
                reference(60_000, "index", entry),
                reference(960_000, "index", end),
            ],
            960_000,
        );
        assert!((values[0].signed_markout_bps.unwrap() - 17.942055752).abs() < 1e-8);
        assert_eq!(values[0].follow_through, Some(true));
    }
}

#[test]
fn calibration_future_close_is_not_available_at_bucket_open() {
    let signal = signal("reference", 90_000, false);
    let refs = [reference(119_999, "index", 120.0)];
    let path = preferred_reference_path(&signal, &refs, "index", &[]);
    assert!(reference_price_at_or_before(&path, 90_000, 120_000).is_none());
    assert!(reference_for_forecast(&signal, &refs).is_none());
    assert_eq!(prior_structure_levels(&refs, 90_000), (None, None));
    let health = forecast_data_stream_health(&signal, &refs);
    let reference_health = health
        .iter()
        .find(|row| row.stream == "binance_mark_index")
        .unwrap();
    assert_eq!(reference_health.status, "unavailable");
    assert_eq!(reference_health.last_event_ts, None);
}

#[test]
fn calibration_entry_never_uses_future_nearest_price() {
    let signal = signal("reference", 90_000, false);
    let refs = [
        reference(120_000, "index", 120.0),
        reference(990_000, "index", 125.0),
    ];
    assert!(reference_for_forecast(&signal, &refs).is_none());
    assert!(outcomes(&signal, &refs, 990_000)[0].entry_price.is_none());
}

#[test]
fn calibration_preferred_reference_beats_newer_vwap() {
    let signal = signal("reference", 90_000, false);
    for source in ["index", "mark"] {
        let refs = [reference(60_000, source, 100.0)];
        let path = preferred_reference_path(&signal, &refs, source, &[(89_000, 110.0)]);
        assert_eq!(
            reference_price_at_or_before(&path, 90_000, 120_000),
            Some((100.0, source.into()))
        );
        assert_eq!(
            reference_price_at_or_before(&path, 200_001, 120_000),
            Some((110.0, "perp_vwap".into()))
        );
    }
}

#[test]
fn calibration_coverage_counts_unique_minutes_and_excludes_future_endpoint() {
    let signal = signal("coverage", 60_000, false);
    let mut refs = (0..16)
        .map(|second| reference(60_000 + second * 1_000, "index", 100.0))
        .collect::<Vec<_>>();
    refs.push(reference(960_000, "index", 101.0));
    let first = outcomes(&signal, &refs, 960_000).remove(0);
    assert!(
        (first.price_coverage - 2.0 / 15.0).abs() < 1e-9,
        "{}",
        first.price_coverage
    );
    assert!(first.price_data_degraded);
    refs.pop();
    refs.push(reference(960_001, "index", 110.0));
    assert!(outcomes(&signal, &refs, 960_000)[0].end_price.is_none());
}

#[test]
fn calibration_base_forecast_deduplicates_and_rejects_bad_history() {
    let signal = signal("current", 2_000_000, false);
    let valid = prior("prior", 60_000, false);
    let once = build_forecast(&signal, &[valid.clone()], &[], signal.ts);
    let twice = build_forecast(&signal, &[valid.clone(), valid.clone()], &[], signal.ts);
    assert_eq!(
        serde_json::to_value(once).unwrap(),
        serde_json::to_value(twice).unwrap()
    );
    let mut invalid = valid;
    invalid.state = "closed".into();
    assert_eq!(
        build_forecast(&signal, &[invalid], &[], signal.ts).horizons[0].sample_count,
        0
    );
}

#[test]
fn calibration_flow_bucket_is_only_known_after_its_second_closes() {
    let signal = signal("flow", 90_000, false);
    let bucket = ContractFlowBucket {
        ts_bucket: 90_000,
        exchange: "binance".into(),
        symbol: "BTC".into(),
        buy_volume_btc: 1.0,
        vwap: Some(100.0),
        ..Default::default()
    };
    assert!(weighted_prices(&signal, &[bucket.clone()], 90_000).is_empty());
    assert_eq!(
        weighted_prices(&signal, &[bucket], 91_000),
        vec![(90_999, 100.0)]
    );
}

#[test]
fn calibration_semantic_versions_do_not_reuse_corrupt_rows() {
    assert_ne!(CONTRACT_WHALE_IMPACT_FORECAST_VERSION, "cwm_impact_v4_1");
    assert_ne!(
        super::super::impact_v4_2::CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
        "cwm_impact_v4_2_hybrid"
    );
    assert_ne!(
        super::super::impact_v4_2_gate::GATE_VERSION,
        "cwm_v4_2_auto_gate_v1"
    );
}

#[test]
fn calibration_secondary_evidence_never_uses_future_observations() {
    let oi = [ContractOiSnapshot {
        ts: 90_001,
        exchange: ContractExchange::Binance,
        symbol: "BTC".into(),
        oi_btc: 100.0,
        oi_notional_usd: None,
        ct_val_available: true,
        evidence_degraded_reason: None,
    }];
    assert!(nearest_oi(&oi, "BTC", 90_000, 120_000).is_none());
    let funding = [ContractFundingSnapshot {
        ts: 90_001,
        exchange: ContractExchange::Binance,
        symbol: "BTC".into(),
        funding_rate: 0.0001,
    }];
    assert!(funding_change(&funding, "BTC", 90_000, 990_000).is_none());
    let liquidation = [ContractLiquidationBucket {
        ts_bucket: 990_000,
        exchange: "binance".into(),
        symbol: "BTC".into(),
        long_liq_btc: 5.0,
        ..Default::default()
    }];
    assert_eq!(
        liquidation_sum(&liquidation, "BTC", 90_000, 990_000),
        (None, None)
    );
}

#[test]
fn calibration_dense_flow_cannot_inflate_minute_coverage() {
    let signal = signal("dense", 60_000, false);
    let buckets = (0..16)
        .map(|second| ContractFlowBucket {
            ts_bucket: 60_000 + second * 1_000,
            exchange: "binance".into(),
            symbol: "BTC".into(),
            buy_volume_btc: 1.0,
            vwap: Some(500.0),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let refs = [
        reference(60_000, "index", 100.0),
        reference(960_000, "index", 101.0),
    ];
    let outcome = evaluate_horizon_outcomes(
        &signal,
        ContractWhaleOutcomeInputs {
            flow_buckets: &buckets,
            reference_prices: &refs,
            oi_snapshots: &[],
            funding_snapshots: &[],
            liquidation_buckets: &[],
        },
        960_000,
    )
    .remove(0);
    // The entry is not a future path slot, and the valid index suppresses VWAP.
    assert!((outcome.price_coverage - 1.0 / 15.0).abs() < 1e-9);
    assert!(outcome.mfe_bps.unwrap() < 101.0);
}
