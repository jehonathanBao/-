//! Bounded public-flow observations. These identify persistent pressure, not accounts.
use super::{
    aggregator::{rolling_window_stats_with_config, RollingWindowStatsOptions},
    config::ContractWhaleRuntimeConfig,
    types::{ContractFlowBucket, ContractWhaleMarketContext, ContractWhaleSignal},
};
use std::collections::BTreeMap;

pub const VERSION: &str = "sustained_public_flow_v1";
pub const WINDOWS_SEC: [u64; 4] = [60, 300, 900, 3600];

/// All horizons use the same last closed minute and therefore one causal context.
pub fn closed_window_end(at: i64) -> i64 {
    at.div_euclid(60_000) * 60_000
}

fn valid_flow(row: &ContractFlowBucket) -> bool {
    row.trade_count > 0
        && [
            row.buy_notional_usd,
            row.sell_notional_usd,
            row.buy_volume_btc,
            row.sell_volume_btc,
        ]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SustainedFlowEvidence {
    pub version: String,
    pub window_sec: u64,
    pub observed_bins: usize,
    pub expected_bins: usize,
    pub baseline_bins: usize,
    pub coverage: f64,
    pub aligned_bin_fraction: f64,
    pub net_participation: f64,
    pub anomaly_percentile: f64,
    pub status: String,
    pub attribution: String,
}

#[derive(Default)]
struct Bin {
    buy: f64,
    sell: f64,
    seconds: usize,
}

pub fn observations(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    at: i64,
    config: &ContractWhaleRuntimeConfig,
) -> Vec<SustainedFlowEvidence> {
    // Database identity is exchange/symbol/second. Repeated inputs never grow volume.
    let mut unique = BTreeMap::new();
    for row in buckets.iter().filter(|row| {
        row.symbol.eq_ignore_ascii_case(symbol)
            && config.exchange_enabled(&row.exchange)
            && row.ts_bucket < at
            && row.ts_bucket >= at.saturating_sub(3 * 3_600_000)
            && valid_flow(row)
    }) {
        unique
            .entry((row.ts_bucket, row.exchange.as_str()))
            .or_insert(row);
    }
    WINDOWS_SEC
        .into_iter()
        .map(|window| {
            let bin_ms = if window == 60 { 10_000 } else { 60_000 };
            let end = closed_window_end(at);
            let start = end - window as i64 * 1000;
            let mut bins = BTreeMap::<i64, Bin>::new();
            let mut seconds = std::collections::BTreeSet::new();
            for row in unique.values().filter(|row| row.ts_bucket < end) {
                let key = row.ts_bucket.div_euclid(bin_ms) * bin_ms;
                let bin = bins.entry(key).or_default();
                bin.buy += row.buy_notional_usd;
                bin.sell += row.sell_notional_usd;
                if seconds.insert(row.ts_bucket) {
                    bin.seconds += 1;
                }
            }
            let complete = |bin: &&Bin| {
                bin.seconds as f64 >= bin_ms as f64 / 1000.0 * 0.8 && bin.buy + bin.sell > 0.0
            };
            let history = bins
                .range(..start)
                .map(|(_, bin)| bin)
                .filter(complete)
                .collect::<Vec<_>>();
            let current = bins
                .range(start..end)
                .map(|(_, bin)| bin)
                .filter(complete)
                .collect::<Vec<_>>();
            let expected = window as usize * 1000 / bin_ms as usize;
            let coverage = current.len() as f64 / expected as f64;
            let buy = current.iter().map(|bin| bin.buy).sum::<f64>();
            let sell = current.iter().map(|bin| bin.sell).sum::<f64>();
            let net = buy - sell;
            let participation = if buy + sell > 0.0 {
                net.abs() / (buy + sell)
            } else {
                0.0
            };
            let aligned = current
                .iter()
                .filter(|bin| (bin.buy - bin.sell) * net > 0.0)
                .count();
            let aligned_fraction = if current.is_empty() {
                0.0
            } else {
                aligned as f64 / current.len() as f64
            };
            let mean_abs_net = if current.is_empty() {
                0.0
            } else {
                net.abs() / current.len() as f64
            };
            let percentile = if history.is_empty() {
                0.0
            } else {
                history
                    .iter()
                    .filter(|bin| (bin.buy - bin.sell).abs() < mean_abs_net)
                    .count() as f64
                    / history.len() as f64
                    * 100.0
            };
            let candidate = history.len() >= 60
                && current.len() >= 3
                && coverage >= 0.9
                && aligned_fraction >= 0.8
                && participation >= 0.15
                && percentile >= 95.0;
            SustainedFlowEvidence {
                version: VERSION.into(),
                window_sec: window,
                observed_bins: current.len(),
                expected_bins: expected,
                baseline_bins: history.len(),
                coverage,
                aligned_bin_fraction: aligned_fraction,
                net_participation: participation,
                anomaly_percentile: percentile,
                status: if candidate {
                    "candidate"
                } else if history.len() < 60 || coverage < 0.9 {
                    "insufficient_evidence"
                } else {
                    "ordinary"
                }
                .into(),
                attribution: if net > 0.0 {
                    "persistent_buy_pressure_unattributed"
                } else if net < 0.0 {
                    "persistent_sell_pressure_unattributed"
                } else {
                    "unknown"
                }
                .into(),
            }
        })
        .collect()
}

pub fn candidates(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    at: i64,
    booted: i64,
    context: &ContractWhaleMarketContext,
    config: &ContractWhaleRuntimeConfig,
) -> Vec<ContractWhaleSignal> {
    let observations = observations(buckets, symbol, at, config);
    // One selected horizon per scan; overlapping horizons are evidence, not additive volume.
    let Some(evidence) = observations
        .into_iter()
        .rev()
        .find(|row| row.status == "candidate")
    else {
        return Vec::new();
    };
    let end = closed_window_end(at);
    // No cached historical alerts/signals immediately after restart; collect a full new window.
    if end - evidence.window_sec as i64 * 1000 < booted {
        return Vec::new();
    }
    let start = end - evidence.window_sec as i64 * 1000;
    let mut unique = BTreeMap::new();
    for row in buckets.iter().filter(|row| {
        row.symbol.eq_ignore_ascii_case(symbol)
            && config.exchange_enabled(&row.exchange)
            && row.ts_bucket >= start
            && row.ts_bucket < end
            && valid_flow(row)
    }) {
        unique
            .entry((row.ts_bucket, row.exchange.as_str()))
            .or_insert_with(|| row.clone());
    }
    let selected = unique.into_values().collect::<Vec<_>>();
    // Incomplete bins cannot reverse or wash out the direction admitted by covered bins.
    let buy = selected.iter().map(|row| row.buy_notional_usd).sum::<f64>();
    let sell = selected
        .iter()
        .map(|row| row.sell_notional_usd)
        .sum::<f64>();
    let net = buy - sell;
    let admitted_buy = evidence.attribution == "persistent_buy_pressure_unattributed";
    if !net.is_finite()
        || !((buy + sell).is_finite())
        || buy + sell <= 0.0
        || (net > 0.0) != admitted_buy
        || net.abs() / (buy + sell) < 0.15
    {
        return Vec::new();
    }
    // Never interpret a change of venue/basis as a price move.
    let mut venue_volume = BTreeMap::<&str, f64>::new();
    for row in &selected {
        *venue_volume.entry(&row.exchange).or_default() +=
            row.buy_notional_usd + row.sell_notional_usd;
    }
    let price_venue = venue_volume
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(venue, _)| venue);
    let prices = selected
        .iter()
        .filter(|row| Some(row.exchange.as_str()) == price_venue)
        .filter_map(|row| {
            row.vwap
                .filter(|p| p.is_finite() && *p > 0.0)
                .map(|p| (row.ts_bucket, p))
        })
        .collect::<Vec<_>>();
    let price_move = prices
        .first()
        .zip(prices.last())
        .filter(|(a, b)| a.0 < b.0 && a.0 <= start + 5_000 && b.0 >= end - 5_000)
        .map(|(a, b)| (b.1 / a.1 - 1.0) * 100.0);
    let Some(mut stats) = rolling_window_stats_with_config(
        &selected,
        symbol,
        evidence.window_sec,
        end - 1,
        RollingWindowStatsOptions {
            price_move_pct: price_move,
            dynamic_multiple: None,
            dynamic_baseline_btc: None,
            dynamic_threshold_level: "sustained_net_flow_candidate".into(),
            data_quality: (evidence.coverage * 95.0) as u8,
            config,
        },
    ) else {
        return Vec::new();
    };
    stats.market_context = context.clone();
    stats.startup_age_ms = Some(end - booted);
    let Some(mut signal) = super::detector::sustained_candidate_signal(&stats, config) else {
        return Vec::new();
    };
    signal = super::event_lifecycle::apply_contract_whale_event_lifecycle(
        vec![signal],
        super::event_lifecycle::ContractWhaleLifecycleClock::Live { now_ms: at },
    )
    .pop()
    .expect("one candidate");
    signal.sustained_flow = Some(evidence);
    set_episode_identity(&mut signal);
    signal.discord_eligible = false;
    signal.discord_would_send = false;
    signal.discord_reason = "sustained_candidate_display_only".into();
    vec![signal]
}

/// Reapply after event-aligned enrichment so the final hypothesis owns the key.
pub fn set_episode_identity(signal: &mut ContractWhaleSignal) {
    let Some(evidence) = signal.sustained_flow.as_ref() else {
        return;
    };
    let end = signal.ts.saturating_add(1);
    let behavior = super::behavior_assessment::build_detection_behavior(&signal, None, signal.ts);
    let hypothesis = serde_json::to_value(behavior.hypothesis).expect("serializable hypothesis");
    let episode = format!(
        "cwm-sustained:{}:{}:{}:{}:{}",
        signal.symbol.to_ascii_uppercase(),
        evidence.window_sec,
        end.div_euclid(evidence.window_sec as i64 * 1000),
        if evidence.attribution == "persistent_buy_pressure_unattributed" {
            "buy"
        } else {
            "sell"
        },
        hypothesis.as_str().unwrap_or("unclear")
    );
    signal.id = format!("{episode}:{end}");
    signal.event_lifecycle.event_id = episode;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows() -> Vec<ContractFlowBucket> {
        (0..10800)
            .map(|second| {
                let active = second >= 7200;
                ContractFlowBucket {
                    ts_bucket: second * 1000,
                    exchange: "binance".into(),
                    symbol: "BTC".into(),
                    buy_volume_btc: if active { 0.02 } else { 0.0101 },
                    sell_volume_btc: 0.01,
                    buy_notional_usd: if active { 1200.0 } else { 606.0 },
                    sell_notional_usd: 600.0,
                    trade_count: 10,
                    vwap: Some(60000.0),
                    ..Default::default()
                }
            })
            .collect()
    }
    #[test]
    fn split_small_trades_form_sustained_candidate_without_double_counting() {
        let config = ContractWhaleRuntimeConfig::default();
        let mut input = rows();
        let first = observations(&input, "BTC", 10800000, &config);
        assert_eq!(first.last().unwrap().status, "candidate");
        assert_eq!(first.last().unwrap().observed_bins, 60);
        input.extend(rows());
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(observations(&input, "BTC", 10800000, &config)).unwrap()
        );
    }
    #[test]
    fn missing_history_and_future_data_cannot_create_candidate() {
        let config = ContractWhaleRuntimeConfig::default();
        assert!(observations(&[], "BTC", 10800000, &config)
            .iter()
            .all(|row| row.status == "insufficient_evidence"));
        let mut input = rows();
        for row in &mut input {
            row.ts_bucket += 10800000;
        }
        assert!(observations(&input, "BTC", 10800000, &config)
            .iter()
            .all(|row| row.status != "candidate"));
    }
    #[test]
    fn restart_does_not_emit_cached_sustained_candidates() {
        assert!(candidates(
            &rows(),
            "BTC",
            10800000,
            10700000,
            &Default::default(),
            &ContractWhaleRuntimeConfig::default()
        )
        .is_empty());
    }

    #[test]
    fn full_live_window_has_stable_identity_and_stays_display_only() {
        let config = ContractWhaleRuntimeConfig::default();
        let result = candidates(&rows(), "BTC", 10800000, 0, &Default::default(), &config);
        assert_eq!(result.len(), 1);
        let signal = &result[0];
        assert_eq!(signal.sustained_flow.as_ref().unwrap().window_sec, 3600);
        assert!(!signal.discord_eligible);
        assert!(signal.read_only && signal.analysis_only && !signal.execution_enabled);
        assert_eq!(signal.price_move_pct, Some(0.0));
        assert_eq!(
            signal.event_lifecycle.latest_window_volume_btc,
            signal.total_volume_btc
        );
        let repeat = candidates(&rows(), "BTC", 10800000, 0, &Default::default(), &config);
        assert_eq!(signal.id, repeat[0].id);
        assert_eq!(
            signal.event_lifecycle.event_id,
            repeat[0].event_lifecycle.event_id
        );
    }

    #[test]
    fn ordinary_balanced_and_invalid_flow_never_becomes_sustained_evidence() {
        let config = ContractWhaleRuntimeConfig::default();
        let mut input = rows();
        for row in &mut input {
            row.buy_notional_usd = row.sell_notional_usd;
        }
        assert!(observations(&input, "BTC", 10800000, &config)
            .iter()
            .all(|row| row.status != "candidate"));
        for row in &mut input {
            row.buy_notional_usd = f64::INFINITY;
        }
        assert!(candidates(&input, "BTC", 10800000, 0, &Default::default(), &config).is_empty());
    }

    #[test]
    fn opposing_sustained_hypotheses_never_share_episode_identity() {
        let config = ContractWhaleRuntimeConfig::default();
        let buy = candidates(&rows(), "BTC", 10800000, 0, &Default::default(), &config);
        let mut sell_rows = rows();
        for row in &mut sell_rows {
            std::mem::swap(&mut row.buy_notional_usd, &mut row.sell_notional_usd);
            std::mem::swap(&mut row.buy_volume_btc, &mut row.sell_volume_btc);
        }
        let sell = candidates(&sell_rows, "BTC", 10800000, 0, &Default::default(), &config);
        assert_eq!(buy.len(), 1);
        assert_eq!(sell.len(), 1);
        assert_ne!(
            buy[0].event_lifecycle.event_id,
            sell[0].event_lifecycle.event_id
        );
    }

    #[test]
    fn incomplete_bin_reversal_cannot_relabel_persistent_buy_flow_as_sell() {
        let config = ContractWhaleRuntimeConfig::default();
        let mut input = rows();
        // Only the final fifteen minutes are anomalous; fourteen complete buy bins qualify.
        for row in input.iter_mut().filter(|row| row.ts_bucket < 9_900_000) {
            row.buy_notional_usd = 606.0;
            row.buy_volume_btc = 0.0101;
        }
        input.retain(|row| row.ts_bucket < 10_740_000 || row.ts_bucket == 10_740_000);
        let last = input.last_mut().unwrap();
        last.sell_notional_usd = 100_000_000.0;
        last.sell_volume_btc = last.sell_notional_usd / 60_000.0;
        let evidence = observations(&input, "BTC", 10800000, &config);
        let admitted = evidence.iter().find(|row| row.window_sec == 900).unwrap();
        assert_eq!(admitted.status, "candidate");
        assert_eq!(admitted.attribution, "persistent_buy_pressure_unattributed");
        assert!(candidates(&input, "BTC", 10800000, 0, &Default::default(), &config).is_empty());
    }

    #[test]
    fn unsupported_price_window_is_unavailable_not_flat() {
        let config = ContractWhaleRuntimeConfig::default();
        for valid_seconds in [vec![9_000], vec![8_990, 9_010]] {
            let mut input = rows();
            for row in &mut input {
                if !valid_seconds.contains(&(row.ts_bucket / 1000)) {
                    row.vwap = None;
                }
            }
            let result = candidates(&input, "BTC", 10800000, 0, &Default::default(), &config);
            assert!(result.iter().all(|signal| signal.price_move_pct.is_none()));
        }
    }
}
