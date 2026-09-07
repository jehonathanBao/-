use crate::{
    contract_whale_monitor::{
        config::{contract_whale_runtime_config, ContractWhaleRetentionConfig},
        impact_baseline::{
            build_robust_impact_baseline, score_event_impact, ImpactBaselineKey,
            RobustImpactBaseline,
        },
        impact_episode::{aggregate_shock_episodes, ImpactBucketContribution, ImpactEventFragment},
        impact_grade::{
            assess_contract_impact_episode_for_event, AssessmentStatus,
            ContractEventImpactAssessment,
        },
        log_events,
        types::{
            ContractFlowBucket, ContractFundingSnapshot, ContractLiquidationBucket,
            ContractOiSnapshot, ContractReferencePriceSnapshot, ContractWhaleSignal,
        },
        LOG_PREFIX, LOG_TARGET,
    },
    normalizers::trade::now_ms,
    storage::{
        contract_event_grade_repo::ContractEventGradeRepo,
        contract_whale_repo::{
            ContractWhaleRepo, ContractWhaleRetentionPruneResult, ContractWhaleSignalQuery,
        },
        storage_health::{RetentionRunHealth, RetentionTableStatus, StorageHealthTracker},
        SqliteStore,
    },
};
use rusqlite::OptionalExtension;

const IMPACT_BASELINE_REUSE_MS: i64 = 5 * 60 * 1_000;
const IMPACT_BASELINE_POPULATION_VERSION: &str = "raw_market_windows_v1";
const MAX_IMPACT_EPISODE_FRAGMENTS: usize = 10_000;
const MAX_IMPACT_EPISODE_SPAN_MS: i64 = 24 * 60 * 60 * 1_000;
const MAX_IMPACT_BACKFILL_EPISODE_SPAN_MS: i64 = 8 * 24 * 60 * 60 * 1_000;
// A global share cutoff makes cross-venue confirmation impossible whenever a
// dominant venue contributes most of the flow. Use a small source-local floor
// instead; the episode still requires directional agreement before promotion.
const MIN_CONFIRMED_SOURCE_VOLUME_BTC: f64 = 0.1;
const MIN_CONFIRMED_SOURCE_TRADES: u64 = 10;
const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

/// Materialize the V3 assessment beside the legacy signal row. This is kept
/// separate from the legacy payload so shadow rollout and replay remain
/// backward compatible.
pub async fn materialize_contract_whale_impact_grades_nonblocking(
    store: Option<SqliteStore>,
    signals: Vec<ContractWhaleSignal>,
    now_ms: i64,
) -> anyhow::Result<Vec<ContractEventImpactAssessment>> {
    let Some(store) = store else {
        return Ok(Vec::new());
    };
    if signals.is_empty() {
        return Ok(Vec::new());
    }
    tokio::task::spawn_blocking(move || {
        materialize_contract_whale_impact_grades(
            &store,
            &signals,
            now_ms,
            MAX_IMPACT_EPISODE_SPAN_MS,
        )
    })
    .await
    .map_err(|error| anyhow::anyhow!("impact grade task failed: {error}"))?
}

/// Recompute the active grade version for a bounded historical signal range.
///
/// This is used once after a grade-version rollout so the event tape does not
/// show every retained event as unavailable merely because it was originally
/// materialized under the previous version. It only writes assessment rows;
/// notification/outbox delivery is intentionally not part of this path.
pub async fn backfill_contract_whale_impact_grades_nonblocking(
    store: Option<SqliteStore>,
    from_ts: i64,
    to_ts: i64,
    now_ms: i64,
) -> anyhow::Result<(usize, usize)> {
    let Some(store) = store else {
        return Ok((0, 0));
    };
    if from_ts > to_ts {
        return Ok((0, 0));
    }
    tokio::task::spawn_blocking(move || {
        let signals = query_all_contract_whale_signals(
            &store,
            ContractWhaleSignalQuery {
                from_ts: Some(from_ts),
                to_ts: Some(to_ts),
                ..ContractWhaleSignalQuery::default()
            },
        )?;
        let signal_count = signals.len();
        if signal_count == 0 {
            return Ok((0, 0));
        }
        let assessment_count = materialize_contract_whale_impact_grades(
            &store,
            &signals,
            now_ms,
            MAX_IMPACT_BACKFILL_EPISODE_SPAN_MS,
        )?
        .len();
        Ok((signal_count, assessment_count))
    })
    .await
    .map_err(|error| anyhow::anyhow!("impact grade backfill task failed: {error}"))?
}

fn materialize_contract_whale_impact_grades(
    store: &SqliteStore,
    signals: &[ContractWhaleSignal],
    now_ms: i64,
    _max_episode_span_ms: i64,
) -> anyhow::Result<Vec<ContractEventImpactAssessment>> {
    let config = contract_whale_runtime_config();
    let grade_repo = ContractEventGradeRepo::new(store.clone());
    let mut fragments_by_baseline =
        std::collections::BTreeMap::<String, Vec<ImpactEventFragment>>::new();
    let mut history_ranges =
        std::collections::BTreeMap::<String, (String, u64, String, i64, i64)>::new();
    for signal in signals {
        let profile = if signal.threshold_profile.trim().is_empty() {
            "default".to_string()
        } else {
            signal.threshold_profile.clone()
        };
        let key_string = format!(
            "{}:{}:{}",
            signal.symbol.to_ascii_uppercase(),
            signal.window_sec,
            profile
        );
        let current_fragment = impact_fragment_from_signal(signal);
        let entry = fragments_by_baseline.entry(key_string.clone()).or_default();
        if !entry
            .iter()
            .any(|fragment| fragment.event_id == current_fragment.event_id)
        {
            entry.push(current_fragment);
        }
        history_ranges
            .entry(key_string)
            .and_modify(|range| {
                range.3 = range.3.min(signal.ts);
                range.4 = range.4.max(signal.ts);
            })
            .or_insert((
                signal.symbol.clone(),
                signal.window_sec,
                profile,
                signal.ts,
                signal.ts,
            ));
    }

    // Materialization runs on a scan batch, while an episode can span several
    // scans. Pull one bounded neighbor range into each batch so the configured
    // gap is effective even when input signals arrive out of order. Do not
    // recursively expand this range: a long stream of unrelated events would
    // otherwise make one bad episode abort grading for the entire batch.
    let gap_ms = config
        .impact_grade_v3
        .episode_gap_seconds
        .saturating_mul(1_000);
    for (key, (symbol, window_sec, profile, min_ts, max_ts)) in &history_ranges {
        let entry = fragments_by_baseline.entry(key.clone()).or_default();
        let mut known_event_ids = entry
            .iter()
            .map(|fragment| fragment.event_id.clone())
            .collect::<std::collections::HashSet<_>>();
        let query_from = min_ts.saturating_sub(gap_ms);
        let query_to = max_ts.saturating_add(gap_ms);
        let history = query_all_contract_whale_signals(
            store,
            ContractWhaleSignalQuery {
                symbol: Some(symbol.clone()),
                window_sec: Some(*window_sec),
                threshold_profile: Some(profile.clone()),
                from_ts: Some(query_from),
                to_ts: Some(query_to),
                ..ContractWhaleSignalQuery::default()
            },
        )?;
        for historical_signal in history {
            let historical_fragment = impact_fragment_from_signal(&historical_signal);
            if known_event_ids.insert(historical_fragment.event_id.clone()) {
                entry.push(historical_fragment);
            }
        }
        if entry.len() > MAX_IMPACT_EPISODE_FRAGMENTS {
            tracing::warn!(
                target: LOG_TARGET,
                key,
                count = entry.len(),
                max = MAX_IMPACT_EPISODE_FRAGMENTS,
                "impact grade neighbor range truncated"
            );
            entry.truncate(MAX_IMPACT_EPISODE_FRAGMENTS);
        }
    }

    for (key, fragments) in &mut fragments_by_baseline {
        let Some((symbol, window_sec, _, _, _)) = history_ranges.get(key) else {
            continue;
        };
        enrich_impact_fragments_with_raw_evidence(store, symbol, *window_sec, fragments)?;
    }

    // Build each baseline strictly from observations that predate the entire
    // candidate episode. Using `now_ms` here would leak future observations
    // into replayed or delayed events and make the same event grade change
    // depending on when it was processed.
    let mut baselines = std::collections::BTreeMap::<String, Option<RobustImpactBaseline>>::new();
    for (key, fragments) in &fragments_by_baseline {
        let Some((symbol, window_sec, profile, _, _)) = history_ranges.get(key) else {
            continue;
        };
        // Baselines use every positive raw market window, not only rows that
        // already triggered the detector. Raw-bucket retention is therefore
        // the effective lookback and is recorded in the baseline range.
        let baseline_lookback_days = config
            .impact_grade_v3
            .baseline_lookback_days
            .min(config.retention.flow_1s_days.max(1));
        let (baseline_from_ms, baseline_to_ms) =
            impact_baseline_time_range(baseline_lookback_days, *window_sec, fragments, now_ms);
        let baseline_key = ImpactBaselineKey {
            symbol: symbol.to_ascii_uppercase(),
            window_sec: *window_sec,
            // Keep the population definition in the key so baselines made
            // from the former event-triggered population cannot be reused.
            threshold_profile: format!("{profile}:{IMPACT_BASELINE_POPULATION_VERSION}"),
        };
        let baseline = match load_reusable_impact_baseline(
            store,
            &baseline_key,
            baseline_from_ms,
            baseline_to_ms,
            config.impact_grade_v3.baseline_min_samples,
        )? {
            Some(baseline) => Some(baseline),
            None => {
                let samples = store.list_contract_flow_window_volumes_between(
                    symbol,
                    *window_sec,
                    baseline_from_ms,
                    baseline_to_ms,
                )?;
                let baseline = build_robust_impact_baseline(
                    baseline_key.clone(),
                    samples.clone(),
                    config.impact_grade_v3.baseline_min_samples,
                );
                persist_impact_baseline_progress(
                    store,
                    &baseline_key,
                    samples.len(),
                    config.impact_grade_v3.baseline_min_samples,
                    baseline_from_ms,
                    baseline_to_ms,
                    baseline.is_some(),
                    now_ms,
                )?;
                if let Some(baseline) = baseline.as_ref() {
                    persist_impact_baseline(
                        store,
                        baseline,
                        now_ms,
                        baseline_from_ms,
                        baseline_to_ms,
                    )?;
                }
                baseline
            }
        };
        baselines.insert(key.clone(), baseline);
    }
    let mut assessments = Vec::with_capacity(signals.len());
    for (key_string, fragments) in fragments_by_baseline {
        let baseline = baselines.get(&key_string).and_then(Option::as_ref);
        let episodes =
            aggregate_shock_episodes(fragments, config.impact_grade_v3.episode_gap_seconds);
        for mut episode in episodes {
            // The baseline population contains per-window signal volumes, so
            // rank the comparable peak-window statistic. Episode-wide unique
            // turnover remains the absolute materiality/hard-evidence input.
            let robust_score = baseline
                .and_then(|baseline| score_event_impact(episode.peak_window_volume_btc, baseline));
            episode.robust_percentile = robust_score.map(|score| score.percentile);
            episode.robust_z = robust_score.map(|score| score.robust_z);
            episode.baseline_sample_count =
                robust_score.map(|score| score.sample_count).unwrap_or(0);
            let mut episode_assessment = assess_contract_impact_episode_for_event(
                &episode,
                &episode.episode_id,
                &config,
                now_ms,
            );
            // A missing baseline has two materially different meanings. A
            // recent event can still become gradable as the raw window
            // population grows; an event older than the retained raw history
            // can never be safely re-scored without future-data leakage.
            if episode_assessment.status == AssessmentStatus::BaselineInsufficient {
                let historical_cutoff = now_ms
                    .saturating_sub(config.retention.flow_1s_days.max(1).saturating_mul(DAY_MS));
                let (status, reason) = if episode.start_time_ms < historical_cutoff {
                    (
                        AssessmentStatus::HistoricalBaselineUnavailable,
                        "historical_baseline_unavailable",
                    )
                } else {
                    (AssessmentStatus::BaselineWarmingUp, "baseline_warming_up")
                };
                episode_assessment.status = status;
                if !episode_assessment
                    .reason_codes
                    .iter()
                    .any(|code| code == reason)
                {
                    episode_assessment
                        .reason_codes
                        .insert(0, reason.to_string());
                }
            }
            let mut alias_specs: Vec<(String, Option<String>, Option<String>)> = Vec::new();
            alias_specs.push((episode.episode_id.clone(), None, None));
            for event_id in &episode.source_event_ids {
                alias_specs.push((
                    event_id.clone(),
                    Some(episode.episode_id.clone()),
                    Some(event_id.clone()),
                ));
            }
            let borrowed_specs = alias_specs
                .iter()
                .map(|(alias, projection, source)| {
                    (alias.as_str(), projection.as_deref(), source.as_deref())
                })
                .collect::<Vec<_>>();
            grade_repo.upsert_assessment_with_aliases(
                &episode_assessment,
                &borrowed_specs,
                now_ms,
            )?;
            for event_id in episode.source_event_ids.clone() {
                let mut assessment = episode_assessment.clone();
                assessment.event_id = event_id;
                assessments.push(assessment);
            }
            // Keep the canonical episode assessment in the return value when
            // no source lifecycle ID was available. Normal production rows
            // always have at least one source ID, but this preserves a stable
            // fallback for replay/import callers.
            if episode.source_event_ids.is_empty() {
                assessments.push(episode_assessment);
            }
        }
    }
    Ok(assessments)
}

fn query_all_contract_whale_signals(
    store: &SqliteStore,
    mut query: ContractWhaleSignalQuery,
) -> anyhow::Result<Vec<ContractWhaleSignal>> {
    const PAGE_SIZE: usize = 500;
    let mut signals = Vec::new();
    query.limit = PAGE_SIZE;
    query.offset = 0;
    loop {
        let page = store.query_contract_whale_signals(&query)?;
        let page_len = page.len();
        let next_cursor = page.last().map(|signal| (signal.ts, signal.id.clone()));
        signals.extend(page);
        if page_len < PAGE_SIZE {
            break;
        }
        let Some((cursor_ts, cursor_signal_id)) = next_cursor else {
            break;
        };
        query.cursor_ts = Some(cursor_ts);
        query.cursor_signal_id = Some(cursor_signal_id);
    }
    Ok(signals)
}

fn impact_baseline_time_range(
    lookback_days: i64,
    window_sec: u64,
    fragments: &[ImpactEventFragment],
    fallback_now_ms: i64,
) -> (i64, i64) {
    let earliest_start_ms = fragments
        .iter()
        .map(|fragment| fragment.start_time_ms)
        .min()
        .unwrap_or(fallback_now_ms);
    let to_ms = earliest_start_ms.saturating_sub((window_sec as i64).saturating_mul(1_000));
    let from_ms = to_ms.saturating_sub(lookback_days.saturating_mul(24 * 60 * 60 * 1_000));
    (from_ms, to_ms)
}

fn enrich_impact_fragments_with_raw_evidence(
    store: &SqliteStore,
    symbol: &str,
    window_sec: u64,
    fragments: &mut [ImpactEventFragment],
) -> anyhow::Result<()> {
    let window_ms = (window_sec as i64).saturating_mul(1_000);
    let Some(from_ts) = fragments
        .iter()
        .map(|fragment| fragment.start_time_ms.saturating_sub(window_ms))
        .min()
    else {
        return Ok(());
    };
    let to_ts = fragments
        .iter()
        .map(|fragment| fragment.end_time_ms)
        .max()
        .unwrap_or(from_ts);
    let flow_buckets = store.list_contract_flow_buckets_between(symbol, from_ts, to_ts)?;
    let liquidation_buckets =
        store.list_contract_liquidation_buckets_between(symbol, from_ts, to_ts)?;

    for fragment in fragments {
        let fragment_from = fragment.start_time_ms.saturating_sub(window_ms);
        let fresh_sources = flow_buckets
            .iter()
            .filter(|bucket| {
                bucket.ts_bucket >= fragment.end_time_ms.saturating_sub(5_000)
                    && bucket.ts_bucket.saturating_add(999) <= fragment.end_time_ms
                    && bucket.trade_count > 0
                    && bucket.buy_notional_usd.is_finite()
                    && bucket.sell_notional_usd.is_finite()
                    && bucket.buy_notional_usd + bucket.sell_notional_usd > 0.0
            })
            .map(|bucket| bucket.exchange.to_ascii_lowercase())
            .collect::<std::collections::BTreeSet<_>>();
        fragment
            .confirmed_sources
            .retain(|source| fresh_sources.contains(source));
        let fragment_flow_buckets = flow_buckets
            .iter()
            .filter(|bucket| {
                bucket.ts_bucket >= fragment_from
                    && bucket.ts_bucket.saturating_add(999) <= fragment.end_time_ms
            })
            .map(|bucket| ImpactBucketContribution {
                identity: format!(
                    "{}:{}:{}",
                    bucket.symbol.to_ascii_uppercase(),
                    bucket.exchange.to_ascii_lowercase(),
                    bucket.ts_bucket
                ),
                source: bucket.exchange.to_ascii_lowercase(),
                volume_btc: bucket.buy_volume_btc.max(0.0) + bucket.sell_volume_btc.max(0.0),
                notional_usd: bucket.buy_notional_usd.max(0.0) + bucket.sell_notional_usd.max(0.0),
            })
            .collect::<Vec<_>>();
        if !fragment_flow_buckets.is_empty() {
            let mut volume_by_source = std::collections::BTreeMap::<String, f64>::new();
            for bucket in &fragment_flow_buckets {
                *volume_by_source.entry(bucket.source.clone()).or_default() += bucket.volume_btc;
            }
            fragment.confirmed_sources = volume_by_source
                .into_iter()
                .filter(|(source, volume)| {
                    *volume >= MIN_CONFIRMED_SOURCE_VOLUME_BTC && fresh_sources.contains(source)
                })
                .map(|(source, _)| source)
                .collect();
            // Persist the deduplicated market-window turnover whenever the
            // detector did not provide trader-level turnover.  Keeping this
            // value explicit prevents the production path from silently
            // dropping all turnover evidence before grading.
            if fragment.unique_turnover_btc.is_none() {
                let turnover_btc = fragment_flow_buckets
                    .iter()
                    .map(|bucket| bucket.volume_btc.max(0.0))
                    .sum::<f64>();
                if turnover_btc > 0.0 {
                    fragment.unique_turnover_btc = Some(turnover_btc);
                }
            }
            if fragment.unique_turnover_notional_usd.is_none() {
                let turnover_usd = fragment_flow_buckets
                    .iter()
                    .map(|bucket| bucket.notional_usd.max(0.0))
                    .sum::<f64>();
                if turnover_usd > 0.0 {
                    fragment.unique_turnover_notional_usd = Some(turnover_usd);
                }
            }
        }
        fragment.flow_buckets = fragment_flow_buckets;
        fragment.liquidation_buckets = liquidation_buckets
            .iter()
            .filter(|bucket| {
                bucket.ts_bucket >= fragment_from
                    && bucket.ts_bucket.saturating_add(999) <= fragment.end_time_ms
            })
            .map(|bucket| ImpactBucketContribution {
                identity: format!(
                    "{}:{}:{}",
                    bucket.symbol.to_ascii_uppercase(),
                    bucket.exchange.to_ascii_lowercase(),
                    bucket.ts_bucket
                ),
                source: bucket.exchange.to_ascii_lowercase(),
                volume_btc: bucket.long_liq_btc.max(0.0) + bucket.short_liq_btc.max(0.0),
                notional_usd: bucket.liq_notional_usd.max(0.0),
            })
            .collect();
    }
    Ok(())
}

fn impact_fragment_from_signal(signal: &ContractWhaleSignal) -> ImpactEventFragment {
    let event_id = if signal.event_lifecycle.event_id.trim().is_empty() {
        signal.id.clone()
    } else {
        signal.event_lifecycle.event_id.clone()
    };
    let mut active_sources = signal.active_contract_sources.clone();
    if active_sources.is_empty() {
        active_sources = signal
            .active_sources
            .contract
            .iter()
            .filter(|source| {
                source.enabled
                    && source.status.eq_ignore_ascii_case("active")
                    && source.market_type
                        == crate::contract_whale_monitor::types::ContractWhaleMarketType::Perp
            })
            .map(|source| source.exchange.clone())
            .collect();
    }
    let active_sources = active_sources
        .into_iter()
        .map(|source| source.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let mut confirmed_sources = signal
        .exchanges
        .iter()
        .filter(|source| active_sources.contains(&source.exchange.to_ascii_lowercase()))
        .filter(|source| {
            source.total_volume_btc >= MIN_CONFIRMED_SOURCE_VOLUME_BTC
                && source.trade_count >= MIN_CONFIRMED_SOURCE_TRADES
        })
        .map(|source| source.exchange.to_ascii_lowercase())
        .collect::<Vec<_>>();
    confirmed_sources.sort();
    confirmed_sources.dedup();
    let liquidation_btc =
        signal.liquidation_long_btc.max(0.0) + signal.liquidation_short_btc.max(0.0);
    let liquidation_btc = (liquidation_btc > 0.0).then_some(liquidation_btc);
    let liquidation_usd =
        (signal.liquidation_notional_usd > 0.0).then_some(signal.liquidation_notional_usd);
    let unique_turnover_btc = signal.event_lifecycle.unique_turnover_btc;
    let unique_turnover_usd = unique_turnover_btc.and_then(|volume| {
        signal
            .current_market_price_usd
            .or(signal.order_price_usd)
            .filter(|price| price.is_finite() && *price > 0.0)
            .map(|price| volume * price)
    });
    let start_time_ms = if signal.event_lifecycle.start_time > 0 {
        signal.event_lifecycle.start_time
    } else {
        signal.ts
    };
    let end_time_ms = signal
        .event_lifecycle
        .last_update_time
        .max(signal.ts)
        .max(start_time_ms);
    ImpactEventFragment {
        event_id,
        symbol: signal.symbol.clone(),
        start_time_ms,
        end_time_ms,
        total_volume_btc: signal
            .event_lifecycle
            .peak_window_volume_btc
            .max(signal.total_volume_btc),
        total_notional_usd: signal.total_notional_usd.max(0.0),
        net_volume_btc: signal.net_volume_btc,
        unique_turnover_btc,
        unique_turnover_notional_usd: unique_turnover_usd,
        live_liquidation_btc: liquidation_btc,
        live_liquidation_notional_usd: liquidation_usd,
        peak_abs_price_move_pct: signal.price_move_pct.map(f64::abs),
        peak_abs_oi_change_pct: signal.oi_change_pct.map(f64::abs),
        confirmed_sources,
        data_quality: signal.data_quality,
        robust_percentile: None,
        robust_z: None,
        baseline_sample_count: 0,
        anchor_price: signal
            .current_market_price_usd
            .or(signal.order_price_usd)
            .or_else(|| {
                (signal.total_volume_btc > 0.0)
                    .then_some(signal.total_notional_usd / signal.total_volume_btc)
            }),
        flow_buckets: Vec::new(),
        liquidation_buckets: Vec::new(),
    }
}

fn persist_impact_baseline(
    store: &SqliteStore,
    baseline: &RobustImpactBaseline,
    computed_at_ms: i64,
    lookback_from_ms: i64,
    lookback_to_ms: i64,
) -> anyhow::Result<()> {
    let sorted_samples_json = serde_json::to_string(&baseline.sorted_log_samples)?;
    store.with_write_connection(|conn| {
        conn.execute(
            "INSERT INTO contract_event_impact_baselines
             (symbol, window_sec, threshold_profile, computed_at_ms, lookback_from_ms,
              lookback_to_ms, sample_count, median_log_volume, mad_log_volume, sorted_samples_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(symbol, window_sec, threshold_profile) DO UPDATE SET
               computed_at_ms = excluded.computed_at_ms,
               lookback_from_ms = excluded.lookback_from_ms,
               lookback_to_ms = excluded.lookback_to_ms,
               sample_count = excluded.sample_count,
               median_log_volume = excluded.median_log_volume,
               mad_log_volume = excluded.mad_log_volume,
               sorted_samples_json = excluded.sorted_samples_json",
            rusqlite::params![
                baseline.key.symbol.as_str(),
                baseline.key.window_sec as i64,
                baseline.key.threshold_profile.as_str(),
                computed_at_ms,
                lookback_from_ms,
                lookback_to_ms,
                baseline.sample_count as i64,
                baseline.median_log_volume,
                baseline.mad_log_volume,
                sorted_samples_json,
            ],
        )?;
        Ok(())
    })
}

fn persist_impact_baseline_progress(
    store: &SqliteStore,
    key: &ImpactBaselineKey,
    sample_count: usize,
    required_samples: usize,
    lookback_from_ms: i64,
    lookback_to_ms: i64,
    ready: bool,
    updated_at_ms: i64,
) -> anyhow::Result<()> {
    store.with_write_connection(|conn| {
        conn.execute(
            "INSERT INTO contract_event_impact_baseline_progress
             (symbol, window_sec, threshold_profile, sample_count, required_samples,
              lookback_from_ms, lookback_to_ms, ready, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(symbol, window_sec, threshold_profile) DO UPDATE SET
               sample_count = excluded.sample_count,
               required_samples = excluded.required_samples,
               lookback_from_ms = excluded.lookback_from_ms,
               lookback_to_ms = excluded.lookback_to_ms,
               ready = excluded.ready,
               updated_at_ms = excluded.updated_at_ms",
            rusqlite::params![
                key.symbol,
                key.window_sec as i64,
                key.threshold_profile,
                sample_count as i64,
                required_samples as i64,
                lookback_from_ms,
                lookback_to_ms,
                i64::from(ready),
                updated_at_ms,
            ],
        )?;
        Ok(())
    })
}

fn load_reusable_impact_baseline(
    store: &SqliteStore,
    key: &ImpactBaselineKey,
    requested_from_ms: i64,
    requested_to_ms: i64,
    min_samples: usize,
) -> anyhow::Result<Option<RobustImpactBaseline>> {
    store.with_connection(|conn| {
        let row = conn
            .query_row(
                "SELECT lookback_from_ms, lookback_to_ms, sample_count,
                        median_log_volume, mad_log_volume, sorted_samples_json
                   FROM contract_event_impact_baselines
                  WHERE symbol = ?1 AND window_sec = ?2 AND threshold_profile = ?3",
                rusqlite::params![key.symbol, key.window_sec as i64, key.threshold_profile],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?;
        let Some((stored_from_ms, stored_to_ms, sample_count, median, mad, samples_json)) = row
        else {
            return Ok(None);
        };
        if stored_to_ms > requested_to_ms
            || requested_to_ms.saturating_sub(stored_to_ms) > IMPACT_BASELINE_REUSE_MS
            || stored_from_ms.abs_diff(requested_from_ms) > IMPACT_BASELINE_REUSE_MS as u64
            || sample_count < min_samples as i64
            || !median.is_finite()
            || !mad.is_finite()
            || mad <= 0.0
        {
            return Ok(None);
        }
        let sorted_log_samples: Vec<f64> = serde_json::from_str(&samples_json)?;
        if sorted_log_samples.len() != sample_count as usize
            || sorted_log_samples.iter().any(|sample| !sample.is_finite())
            || !sorted_log_samples
                .windows(2)
                .all(|window| window[0] <= window[1])
        {
            return Ok(None);
        }
        Ok(Some(RobustImpactBaseline {
            key: key.clone(),
            sample_count: sample_count as usize,
            median_log_volume: median,
            mad_log_volume: mad,
            sorted_log_samples,
        }))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractWhalePersistenceOutcome {
    pub attempted: bool,
    pub succeeded: bool,
    pub written: usize,
}

impl ContractWhalePersistenceOutcome {
    fn skipped() -> Self {
        Self {
            attempted: false,
            succeeded: false,
            written: 0,
        }
    }

    fn success(written: usize) -> Self {
        Self {
            attempted: true,
            succeeded: true,
            written,
        }
    }

    fn failed() -> Self {
        Self {
            attempted: true,
            succeeded: false,
            written: 0,
        }
    }
}

pub async fn flush_contract_flow_buckets_nonblocking(
    store: Option<SqliteStore>,
    buckets: Vec<ContractFlowBucket>,
) -> ContractWhalePersistenceOutcome {
    if buckets.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} bucket flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = buckets.len();
    let mut per_symbol = std::collections::BTreeMap::<String, usize>::new();
    for bucket in &buckets {
        *per_symbol.entry(bucket.symbol.clone()).or_default() += 1;
    }
    let symbol_breakdown = per_symbol
        .into_iter()
        .map(|(symbol, rows)| format!("{symbol}:{rows}"))
        .collect::<Vec<_>>()
        .join(",");
    match tokio::task::spawn_blocking(move || store.upsert_contract_flow_buckets(&buckets)).await {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                bucket_count = count,
                written = written,
                symbols = symbol_breakdown.as_str(),
                "{} bucket flushed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} bucket flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} bucket flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn flush_contract_liquidation_buckets_nonblocking(
    store: Option<SqliteStore>,
    buckets: Vec<ContractLiquidationBucket>,
) -> ContractWhalePersistenceOutcome {
    if buckets.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} liquidation bucket flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = buckets.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_liquidation_buckets(&buckets))
        .await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                bucket_count = count,
                written = written,
                "{} liquidation bucket flushed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} liquidation bucket flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} liquidation bucket flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_oi_snapshots_nonblocking(
    store: Option<SqliteStore>,
    snapshots: Vec<ContractOiSnapshot>,
) -> ContractWhalePersistenceOutcome {
    if snapshots.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} oi snapshot flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = snapshots.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_oi_snapshots(&snapshots)).await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                snapshot_count = count,
                written,
                "{} oi snapshot flush success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} oi snapshot flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} oi snapshot flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_funding_snapshots_nonblocking(
    store: Option<SqliteStore>,
    snapshots: Vec<ContractFundingSnapshot>,
) -> ContractWhalePersistenceOutcome {
    if snapshots.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} funding snapshot flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = snapshots.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_funding_snapshots(&snapshots))
        .await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                snapshot_count = count,
                written,
                "{} funding snapshot flush success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} funding snapshot flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} funding snapshot flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_reference_prices_nonblocking(
    store: Option<SqliteStore>,
    snapshots: Vec<ContractReferencePriceSnapshot>,
) -> ContractWhalePersistenceOutcome {
    if snapshots.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        return ContractWhalePersistenceOutcome::skipped();
    };
    let count = snapshots.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_reference_prices(&snapshots))
        .await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                snapshot_count = count,
                written,
                "{} Binance reference-price snapshot flush success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} Binance reference-price snapshot flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} Binance reference-price snapshot task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_whale_signal_nonblocking(
    store: Option<SqliteStore>,
    signal: ContractWhaleSignal,
) -> ContractWhalePersistenceOutcome {
    tracing::info!(
        target: LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        signal_id = signal.id.as_str(),
        persist_attempt = true,
        "{} signal persistence attempt",
        LOG_PREFIX
    );
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::SIGNAL_GENERATED,
            signal_id = signal.id.as_str(),
            persist_attempt = false,
            persist_skip_reason = "sqlite_store_unavailable",
            "{} signal persistence skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let signal_id = signal.id.clone();
    match tokio::task::spawn_blocking(move || store.upsert_contract_whale_signal(&signal)).await {
        Ok(Ok(())) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::SIGNAL_GENERATED,
                signal_id = signal_id.as_str(),
                persist_success = true,
                "{} signal persistence success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(1)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_id = signal_id.as_str(),
                persist_success = false,
                error = %error,
                "{} signal persistence failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_id = signal_id.as_str(),
                persist_success = false,
                error = %error,
                "{} signal persistence task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_whale_signals_nonblocking(
    store: Option<SqliteStore>,
    signals: Vec<ContractWhaleSignal>,
) -> ContractWhalePersistenceOutcome {
    if signals.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let signal_count = signals.len();
    let symbols = signals
        .iter()
        .map(|signal| signal.symbol.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    tracing::info!(
        target: LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        signal_count,
        symbols = symbols.as_str(),
        persist_attempt = true,
        "{} signal batch persistence attempt",
        LOG_PREFIX
    );
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::SIGNAL_GENERATED,
            signal_count,
            symbols = symbols.as_str(),
            persist_attempt = false,
            persist_skip_reason = "sqlite_store_unavailable",
            "{} signal batch persistence skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    match tokio::task::spawn_blocking(move || store.upsert_contract_whale_signals(&signals)).await {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::SIGNAL_GENERATED,
                signal_count,
                symbols = symbols.as_str(),
                persist_success = true,
                written,
                "{} signal batch persistence success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_count,
                symbols = symbols.as_str(),
                persist_success = false,
                error = %error,
                "{} signal batch persistence failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_count,
                symbols = symbols.as_str(),
                persist_success = false,
                error = %error,
                "{} signal batch persistence task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub fn spawn_contract_whale_retention_task(
    store: Option<SqliteStore>,
    retention: ContractWhaleRetentionConfig,
    storage_health: StorageHealthTracker,
) {
    const INITIAL_RETENTION_DELAY_SECS: u64 = 30;
    let Some(store) = store else {
        return;
    };
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::RETENTION_PRUNED,
            flow_1s_days = retention.flow_1s_days,
            oi_raw_days = retention.oi_raw_days,
            funding_raw_days = retention.funding_raw_days,
            liquidation_days = retention.liquidation_days,
            reference_price_days = retention.reference_price_days,
            aggregate_context_days = retention.aggregate_context_days,
            signals_days = retention.signals_days,
            impact_b_days = retention.impact_b_days,
            initial_delay_seconds = INITIAL_RETENTION_DELAY_SECS,
            "{} retention task scheduled",
            LOG_PREFIX
        );
        tokio::time::sleep(std::time::Duration::from_secs(INITIAL_RETENTION_DELAY_SECS)).await;
        prune_contract_whale_retention_nonblocking(
            store.clone(),
            retention.clone(),
            now_ms(),
            storage_health.clone(),
        )
        .await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        interval.tick().await;
        loop {
            interval.tick().await;
            prune_contract_whale_retention_nonblocking(
                store.clone(),
                retention.clone(),
                now_ms(),
                storage_health.clone(),
            )
            .await;
        }
    });
}

pub async fn prune_contract_whale_retention_nonblocking(
    store: SqliteStore,
    retention: ContractWhaleRetentionConfig,
    now_ms: i64,
    storage_health: StorageHealthTracker,
) -> Option<ContractWhaleRetentionPruneResult> {
    let flow_cutoff = retention_cutoff_ms(now_ms, retention.flow_1s_days);
    let oi_raw_cutoff = retention_cutoff_ms(now_ms, retention.oi_raw_days);
    let funding_raw_cutoff = retention_cutoff_ms(now_ms, retention.funding_raw_days);
    let liquidation_cutoff = retention_cutoff_ms(now_ms, retention.liquidation_days);
    let reference_price_cutoff = retention_cutoff_ms(now_ms, retention.reference_price_days);
    let aggregate_context_cutoff = retention_cutoff_ms(now_ms, retention.aggregate_context_days);
    let signal_cutoff = retention_cutoff_ms(now_ms, retention.signals_days);
    let impact_b_cutoff = retention_cutoff_ms(now_ms, retention.impact_b_days);
    let started_at = std::time::Instant::now();
    match tokio::task::spawn_blocking(move || {
        store.prune_contract_whale_retention(
            flow_cutoff,
            oi_raw_cutoff,
            funding_raw_cutoff,
            liquidation_cutoff,
            reference_price_cutoff,
            aggregate_context_cutoff,
            signal_cutoff,
            impact_b_cutoff,
        )
    })
    .await
    {
        Ok(Ok(result)) => {
            for table_result in &result.table_results {
                match table_result.status {
                    RetentionTableStatus::Ok => tracing::info!(
                        target: LOG_TARGET,
                        table = table_result.table.as_str(),
                        time_column = table_result.time_column.as_str(),
                        status = table_result.status.as_str(),
                        deleted = table_result.deleted_rows,
                        duration_ms = table_result.duration_ms,
                        "{} retention table result",
                        LOG_PREFIX
                    ),
                    RetentionTableStatus::Skipped => tracing::warn!(
                        target: LOG_TARGET,
                        table = table_result.table.as_str(),
                        time_column = table_result.time_column.as_str(),
                        status = table_result.status.as_str(),
                        reason = table_result.reason.as_deref().unwrap_or("unknown"),
                        duration_ms = table_result.duration_ms,
                        "{} retention table skipped",
                        LOG_PREFIX
                    ),
                    RetentionTableStatus::Error => tracing::warn!(
                        target: LOG_TARGET,
                        table = table_result.table.as_str(),
                        time_column = table_result.time_column.as_str(),
                        status = table_result.status.as_str(),
                        error_kind = table_result.error_kind.as_deref().unwrap_or("unknown"),
                        error = table_result.error.as_deref().unwrap_or("unknown"),
                        duration_ms = table_result.duration_ms,
                        "{} retention table failed",
                        LOG_PREFIX
                    ),
                }
            }
            let total_deleted_rows = result.flow_1s_deleted
                + result.liquidation_deleted
                + result.oi_deleted
                + result.funding_deleted
                + result.reference_price_deleted
                + result.oi_1m_deleted
                + result.funding_1m_deleted
                + result.percentile_deleted
                + result.signal_deleted
                + result.impact_grade_deleted
                + result.v4_outcome_deleted
                + result.v4_forecast_deleted
                + result.ordinary_archive_deleted;
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::RETENTION_PRUNED,
                flow_cutoff_ts = result.flow_cutoff_ts,
                oi_raw_cutoff_ts = result.oi_raw_cutoff_ts,
                funding_raw_cutoff_ts = result.funding_raw_cutoff_ts,
                liquidation_cutoff_ts = result.liquidation_cutoff_ts,
                reference_price_cutoff_ts = result.reference_price_cutoff_ts,
                aggregate_context_cutoff_ts = result.aggregate_context_cutoff_ts,
                signal_cutoff_ts = result.signal_cutoff_ts,
                flow_1s_deleted = result.flow_1s_deleted,
                liquidation_deleted = result.liquidation_deleted,
                oi_deleted = result.oi_deleted,
                funding_deleted = result.funding_deleted,
                reference_price_deleted = result.reference_price_deleted,
                oi_1m_deleted = result.oi_1m_deleted,
                funding_1m_deleted = result.funding_1m_deleted,
                percentile_deleted = result.percentile_deleted,
                signal_deleted = result.signal_deleted,
                impact_grade_deleted = result.impact_grade_deleted,
                v4_outcome_deleted = result.v4_outcome_deleted,
                v4_forecast_deleted = result.v4_forecast_deleted,
                ordinary_archive_deleted = result.ordinary_archive_deleted,
                ordinary_signals_archived = result.ordinary_signals_archived,
                permanent_signals_archived = result.permanent_signals_archived,
                total_deleted_rows = total_deleted_rows,
                failed_tables = result
                    .table_results
                    .iter()
                    .filter(|entry| entry.status == RetentionTableStatus::Error)
                    .count(),
                skipped_tables = result
                    .table_results
                    .iter()
                    .filter(|entry| entry.status == RetentionTableStatus::Skipped)
                    .count(),
                protected_impact_a_s_count = result.protected_impact_a_s_count,
                duration_ms = started_at.elapsed().as_millis() as u64,
                "{} retention pruned",
                LOG_PREFIX
            );
            storage_health.record_contract_whale_retention(
                RetentionRunHealth {
                    ok: result
                        .table_results
                        .iter()
                        .all(|entry| entry.status != RetentionTableStatus::Error),
                    total_deleted_rows,
                    failed_tables: result
                        .table_results
                        .iter()
                        .filter(|entry| entry.status == RetentionTableStatus::Error)
                        .map(|entry| entry.table.clone())
                        .collect(),
                    skipped_tables: result
                        .table_results
                        .iter()
                        .filter(|entry| entry.status == RetentionTableStatus::Skipped)
                        .map(|entry| entry.table.clone())
                        .collect(),
                    error: None,
                    duration_ms: Some(started_at.elapsed().as_millis() as u64),
                    finished_at_ms: Some(now_ms),
                },
                result.wal_checkpoint.clone(),
            );
            storage_health.refresh_now();
            Some(result)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} retention prune failed",
                LOG_PREFIX
            );
            storage_health.record_contract_whale_retention(
                RetentionRunHealth {
                    ok: false,
                    total_deleted_rows: 0,
                    failed_tables: Vec::new(),
                    skipped_tables: Vec::new(),
                    error: Some(error.to_string()),
                    duration_ms: Some(started_at.elapsed().as_millis() as u64),
                    finished_at_ms: Some(now_ms),
                },
                None,
            );
            storage_health.refresh_now();
            None
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} retention prune task failed",
                LOG_PREFIX
            );
            storage_health.record_contract_whale_retention(
                RetentionRunHealth {
                    ok: false,
                    total_deleted_rows: 0,
                    failed_tables: Vec::new(),
                    skipped_tables: Vec::new(),
                    error: Some(error.to_string()),
                    duration_ms: Some(started_at.elapsed().as_millis() as u64),
                    finished_at_ms: Some(now_ms),
                },
                None,
            );
            storage_health.refresh_now();
            None
        }
    }
}

fn retention_cutoff_ms(now_ms: i64, retention_days: i64) -> i64 {
    let safe_days = retention_days.max(1);
    now_ms.saturating_sub(safe_days.saturating_mul(24 * 60 * 60 * 1000))
}

#[cfg(test)]
mod impact_materialization_tests {
    use super::*;
    use crate::contract_whale_monitor::types::{
        ContractLiquidationBucket, ContractWhaleMarketType, ContractWhaleSourceRole,
    };

    #[test]
    fn raw_flow_and_liquidation_buckets_are_loaded_and_deduplicated_per_episode() {
        let path = std::env::temp_dir().join(format!(
            "cwm-impact-raw-evidence-{}-{}.sqlite",
            std::process::id(),
            now_ms()
        ));
        let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
        store.migrate().unwrap();
        store
            .upsert_contract_flow_buckets(&[
                ContractFlowBucket {
                    ts_bucket: 1_000_000,
                    exchange: "binance".to_string(),
                    symbol: "BTC".to_string(),
                    market_type: ContractWhaleMarketType::Perp,
                    source_role: ContractWhaleSourceRole::Primary,
                    product_id: Some("BTCUSDT".to_string()),
                    buy_volume_btc: 10.0,
                    sell_volume_btc: 20.0,
                    buy_notional_usd: 1_000_000.0,
                    sell_notional_usd: 2_000_000.0,
                    trade_count: 2,
                    max_single_trade_btc: 20.0,
                    vwap: Some(100_000.0),
                },
                ContractFlowBucket {
                    ts_bucket: 1_000_000,
                    exchange: "bitfinex".to_string(),
                    symbol: "BTC".to_string(),
                    market_type: ContractWhaleMarketType::Perp,
                    source_role: ContractWhaleSourceRole::Confirmation,
                    product_id: Some("tBTCF0:USTF0".to_string()),
                    buy_volume_btc: 5.0,
                    sell_volume_btc: 5.0,
                    buy_notional_usd: 500_000.0,
                    sell_notional_usd: 500_000.0,
                    trade_count: 2,
                    max_single_trade_btc: 5.0,
                    vwap: Some(100_000.0),
                },
            ])
            .unwrap();
        store
            .upsert_contract_liquidation_buckets(&[ContractLiquidationBucket {
                ts_bucket: 1_000_000,
                exchange: "binance".to_string(),
                symbol: "BTC".to_string(),
                long_liq_btc: 2.0,
                short_liq_btc: 1.0,
                liq_notional_usd: 300_000.0,
                order_count: 2,
                max_single_liq_btc: 2.0,
                vwap: Some(100_000.0),
            }])
            .unwrap();
        let window_volumes = store
            .list_contract_flow_window_volumes_between("BTC", 15, 900_000, 1_010_000)
            .unwrap();
        assert_eq!(window_volumes, vec![40.0]);

        let mut fragments = vec![fragment("event-1"), fragment("event-2")];
        enrich_impact_fragments_with_raw_evidence(&store, "BTC", 15, &mut fragments).unwrap();
        assert_eq!(fragments[0].flow_buckets.len(), 2);
        assert_eq!(fragments[0].liquidation_buckets.len(), 1);

        let episodes = aggregate_shock_episodes(fragments, 900);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].total_volume_btc, 40.0);
        assert_eq!(episodes[0].total_notional_usd, 4_000_000.0);
        assert_eq!(episodes[0].unique_turnover_btc, Some(40.0));
        assert_eq!(
            episodes[0].confirmed_sources,
            vec!["binance".to_string(), "bitfinex".to_string()]
        );
        assert_eq!(episodes[0].live_liquidation_btc, Some(3.0));
        assert_eq!(episodes[0].live_liquidation_notional_usd, Some(300_000.0));

        let mut stale = fragment("stale-evidence");
        stale.end_time_ms += 10_000;
        enrich_impact_fragments_with_raw_evidence(
            &store,
            "BTC",
            15,
            std::slice::from_mut(&mut stale),
        )
        .unwrap();
        assert!(
            stale.confirmed_sources.is_empty(),
            "old volume is not a fresh source confirmation"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn baseline_cutoff_precedes_the_earliest_episode_fragment() {
        let fragments = vec![fragment("later"), {
            let mut earlier = fragment("earlier");
            earlier.start_time_ms = 900_000;
            earlier
        }];
        let (from_ms, to_ms) = impact_baseline_time_range(90, 15, &fragments, 9_000_000);
        assert_eq!(to_ms, 885_000);
        assert_eq!(to_ms.saturating_sub(from_ms), 90 * 24 * 60 * 60 * 1_000);
    }

    #[test]
    fn recent_persisted_baseline_is_reused_without_future_leakage() {
        let path = std::env::temp_dir().join(format!(
            "cwm-impact-baseline-reuse-{}-{}.sqlite",
            std::process::id(),
            now_ms()
        ));
        let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
        store.migrate().unwrap();
        let key = ImpactBaselineKey {
            symbol: "BTC".to_string(),
            window_sec: 15,
            threshold_profile: "binance_bitfinex".to_string(),
        };
        let baseline =
            build_robust_impact_baseline(key.clone(), (1..=10).map(f64::from), 10).unwrap();
        let stored_from = 1_000_000;
        let stored_to = 2_000_000;
        persist_impact_baseline(&store, &baseline, 2_000_100, stored_from, stored_to).unwrap();

        let reused = load_reusable_impact_baseline(
            &store,
            &key,
            stored_from + 60_000,
            stored_to + 60_000,
            10,
        )
        .unwrap();
        assert_eq!(reused.unwrap().sample_count, 10);
        assert!(
            load_reusable_impact_baseline(&store, &key, stored_from - 1, stored_to - 1, 10,)
                .unwrap()
                .is_none()
        );

        let _ = std::fs::remove_file(path);
    }

    fn fragment(event_id: &str) -> ImpactEventFragment {
        ImpactEventFragment {
            event_id: event_id.to_string(),
            symbol: "BTC".to_string(),
            start_time_ms: 1_000_000,
            end_time_ms: 1_005_000,
            total_volume_btc: 999.0,
            total_notional_usd: 99_900_000.0,
            net_volume_btc: 100.0,
            unique_turnover_btc: None,
            unique_turnover_notional_usd: None,
            live_liquidation_btc: None,
            live_liquidation_notional_usd: None,
            peak_abs_price_move_pct: Some(1.0),
            peak_abs_oi_change_pct: Some(1.0),
            confirmed_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            data_quality: 90,
            robust_percentile: None,
            robust_z: None,
            baseline_sample_count: 0,
            anchor_price: Some(100_000.0),
            flow_buckets: Vec::new(),
            liquidation_buckets: Vec::new(),
        }
    }
}
