//! Resumable chronological V4.1 walk-forward backfill.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::storage::{
    contract_whale_repo::{
        ContractWhaleRepo, ContractWhaleSignalQuery, ContractWhaleV4BackfillCheckpoint,
    },
    SqliteStore,
};

use super::{
    collector_binance::fetch_binance_reference_history_for_symbol,
    impact_forecast::{
        build_forecast, evaluate_horizon_outcomes, event_id, ContractWhaleHorizonOutcome,
        evaluate_trade_plan_state, ContractWhaleOutcomeInputs, ContractWhaleV4DecisionState,
        CONTRACT_WHALE_IMPACT_FORECAST_VERSION,
    },
};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;
const STRUCTURE_LOOKBACK_MS: i64 = 4 * 60 * 60 * 1_000;

#[derive(Debug, Clone)]
pub struct ContractWhaleV4BackfillOptions {
    pub job_key: String,
    pub symbol: Option<String>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub limit: usize,
    pub dry_run: bool,
    pub rebuild_v4_1: bool,
    pub fetch_reference_history: bool,
}

impl Default for ContractWhaleV4BackfillOptions {
    fn default() -> Self {
        Self {
            job_key: "cwm_v4_1_walk_forward".to_string(),
            symbol: None,
            from_ts: None,
            to_ts: None,
            limit: 100_000,
            dry_run: true,
            rebuild_v4_1: false,
            fetch_reference_history: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleV4BackfillReport {
    pub job_key: String,
    pub status: String,
    pub dry_run: bool,
    pub processed: usize,
    pub forecasts_written: usize,
    pub outcomes_written: usize,
    pub skipped: usize,
    pub degraded: usize,
    pub failed: usize,
    pub reference_rows_fetched: usize,
    pub archived_forecasts: usize,
    pub removed_invalid_outcomes: usize,
    pub first_event_ts: Option<i64>,
    pub last_event_ts: Option<i64>,
    pub last_event_id: Option<String>,
    pub errors: Vec<String>,
}

pub async fn run_contract_whale_v4_backfill(
    store: SqliteStore,
    options: ContractWhaleV4BackfillOptions,
) -> anyhow::Result<ContractWhaleV4BackfillReport> {
    store.migrate()?;
    let now_ms = crate::normalizers::trade::now_ms();
    let mut report = ContractWhaleV4BackfillReport {
        job_key: options.job_key.clone(),
        status: "running".to_string(),
        dry_run: options.dry_run,
        ..ContractWhaleV4BackfillReport::default()
    };

    if options.rebuild_v4_1 && !options.dry_run {
        let (archived, outcomes) =
            archive_and_clear_invalid_v4_1(&store, &options.job_key, now_ms)?;
        report.archived_forecasts = archived;
        report.removed_invalid_outcomes = outcomes;
    }

    let existing_checkpoint = if options.rebuild_v4_1 {
        None
    } else {
        store.get_contract_whale_v4_backfill_checkpoint(&options.job_key)?
    };
    let resume_after = existing_checkpoint
        .as_ref()
        .and_then(|checkpoint| checkpoint.last_event_ts.zip(checkpoint.last_event_id.clone()));
    if let Some(checkpoint) = existing_checkpoint.as_ref() {
        report.processed = checkpoint.processed_count;
        report.forecasts_written = checkpoint.forecast_count;
        report.outcomes_written = checkpoint.outcome_count;
        report.skipped = checkpoint.skipped_count;
        report.degraded = checkpoint.degraded_count;
        report.failed = checkpoint.failed_count;
    }

    let mut signals = store.query_contract_whale_signals(&ContractWhaleSignalQuery {
        symbol: options.symbol.as_ref().map(|value| value.to_ascii_uppercase()),
        from_ts: options.from_ts,
        to_ts: options.to_ts.or(Some(now_ms)),
        limit: options.limit.max(1),
        ..ContractWhaleSignalQuery::default()
    })?;
    signals.sort_by(|left, right| left.ts.cmp(&right.ts).then_with(|| left.id.cmp(&right.id)));

    let before_source_policy_filter = signals.len();
    signals.retain(|signal| {
        let perp_binance_only = signal.active_contract_sources.iter().all(|source| {
            source.eq_ignore_ascii_case("binance")
        }) && signal.active_contract_sources.iter().any(|source| {
            source.eq_ignore_ascii_case("binance")
        });
        let contribution_binance_only = signal.exchanges.iter().all(|contribution| {
            contribution.exchange.eq_ignore_ascii_case("binance")
        }) && signal.exchanges.iter().any(|contribution| {
            contribution.exchange.eq_ignore_ascii_case("binance")
        });
        perp_binance_only || contribution_binance_only
    });
    report.skipped += before_source_policy_filter.saturating_sub(signals.len());

    // Lifecycle updates can create several signal rows for one event. V4.1
    // owns one immutable T0 forecast per lifecycle event.
    let mut seen_events = BTreeSet::new();
    signals.retain(|signal| seen_events.insert(event_id(signal)));
    if let Some((checkpoint_ts, checkpoint_id)) = resume_after.as_ref() {
        signals.retain(|signal| {
            signal.ts > *checkpoint_ts
                || (signal.ts == *checkpoint_ts && event_id(signal) > *checkpoint_id)
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("cwm-v4-1-backfill/1")
        .build()?;
    let mut history_by_symbol = BTreeMap::<String, Vec<ContractWhaleHorizonOutcome>>::new();

    for signal in signals {
        let current_event_id = event_id(&signal);
        report.first_event_ts.get_or_insert(signal.ts);
        report.last_event_ts = Some(signal.ts);
        report.last_event_id = Some(current_event_id.clone());
        let from_ts = signal.ts.saturating_sub(STRUCTURE_LOOKBACK_MS);
        let to_ts = now_ms.min(signal.ts.saturating_add(DAY_MS));

        if !history_by_symbol.contains_key(&signal.symbol) {
            let prior = store.list_contract_whale_horizon_outcomes_before(
                &signal.symbol,
                signal.ts,
                100_000,
            )?;
            history_by_symbol.insert(signal.symbol.clone(), prior);
        }

        let event_result = async {
            let mut reference_prices =
                store.list_contract_reference_prices_between(&signal.symbol, from_ts, to_ts)?;
            let has_event_reference = reference_prices.iter().any(|row| {
                matches!(row.price_source.as_str(), "mark" | "index")
                    && row.ts_bucket.abs_diff(signal.ts) <= 120_000
            });
            if !has_event_reference && options.fetch_reference_history {
                let fetched = fetch_binance_reference_history_for_symbol(
                    &client,
                    &signal.symbol,
                    from_ts,
                    to_ts,
                )
                .await?;
                report.reference_rows_fetched += fetched.len();
                if !options.dry_run && !fetched.is_empty() {
                    store.upsert_contract_reference_prices(&fetched)?;
                }
                reference_prices.extend(fetched);
                reference_prices.sort_by(|left, right| {
                    left.ts_bucket
                        .cmp(&right.ts_bucket)
                        .then_with(|| left.price_source.cmp(&right.price_source))
                });
                reference_prices.dedup_by(|left, right| {
                    left.ts_bucket == right.ts_bucket
                        && left.price_source == right.price_source
                        && left.symbol == right.symbol
                });
            }

            let flow_buckets =
                store.list_contract_flow_buckets_between(&signal.symbol, from_ts, to_ts)?;
            let oi_snapshots =
                store.list_contract_oi_snapshots_between(&signal.symbol, from_ts, to_ts)?;
            let funding_snapshots =
                store.list_contract_funding_snapshots_between(&signal.symbol, from_ts, to_ts)?;
            let liquidation_buckets = store.list_contract_liquidation_buckets_between(
                &signal.symbol,
                signal.ts,
                to_ts,
            )?;
            let history = history_by_symbol
                .get_mut(&signal.symbol)
                .context("walk-forward history missing")?;

            // This order is intentional: forecast first from prior completed
            // outcomes, then evaluate this event and expose it to later events.
            let forecast = build_forecast(&signal, history, &reference_prices, signal.ts);
            let outcomes = evaluate_horizon_outcomes(
                &signal,
                ContractWhaleOutcomeInputs {
                    flow_buckets: &flow_buckets,
                    reference_prices: &reference_prices,
                    oi_snapshots: &oi_snapshots,
                    funding_snapshots: &funding_snapshots,
                    liquidation_buckets: &liquidation_buckets,
                },
                now_ms,
            );
            let degraded = !forecast.degraded_reasons.is_empty()
                || !forecast.missing_evidence.is_empty()
                || outcomes.iter().any(|outcome| outcome.price_data_degraded);
            let (forecast_written, outcomes_written) = if options.dry_run {
                (0, 0)
            } else {
                let (decision_state, decision_reason) =
                    evaluate_trade_plan_state(&forecast, &outcomes, now_ms);
                store.upsert_contract_whale_v4_decision_states(&[
                    ContractWhaleV4DecisionState {
                        event_id: forecast.event_id.clone(),
                        forecast_version: forecast.forecast_version.clone(),
                        state: decision_state,
                        reason: decision_reason,
                        updated_at_ms: now_ms,
                        decided_at_ms: Some(now_ms),
                    },
                ])?;
                (
                    store.upsert_contract_whale_impact_forecasts(&[forecast])?,
                    store.upsert_contract_whale_horizon_outcomes(&outcomes)?,
                )
            };
            history.extend(outcomes.iter().cloned());
            Ok::<_, anyhow::Error>((forecast_written, outcomes_written, degraded))
        }
        .await;

        report.processed += 1;
        match event_result {
            Ok((forecast_written, outcomes_written, degraded)) => {
                report.forecasts_written += forecast_written;
                report.outcomes_written += outcomes_written;
                if forecast_written == 0 && !options.dry_run {
                    report.skipped += 1;
                }
                if degraded {
                    report.degraded += 1;
                }
            }
            Err(error) => {
                report.failed += 1;
                report.errors.push(format!("{current_event_id}: {error}"));
            }
        }

        if !options.dry_run {
            store.upsert_contract_whale_v4_backfill_checkpoint(
                &checkpoint_from_report(&report, "running", now_ms),
            )?;
        }
    }

    report.status = if report.failed == 0 {
        "complete".to_string()
    } else {
        "complete_with_errors".to_string()
    };
    if !options.dry_run {
        store.upsert_contract_whale_v4_backfill_checkpoint(&checkpoint_from_report(
            &report,
            &report.status,
            crate::normalizers::trade::now_ms(),
        ))?;
    }
    Ok(report)
}

fn checkpoint_from_report(
    report: &ContractWhaleV4BackfillReport,
    status: &str,
    updated_at_ms: i64,
) -> ContractWhaleV4BackfillCheckpoint {
    ContractWhaleV4BackfillCheckpoint {
        job_key: report.job_key.clone(),
        status: status.to_string(),
        last_event_ts: report.last_event_ts,
        last_event_id: report.last_event_id.clone(),
        processed_count: report.processed,
        forecast_count: report.forecasts_written,
        outcome_count: report.outcomes_written,
        skipped_count: report.skipped,
        degraded_count: report.degraded,
        failed_count: report.failed,
        last_error: report.errors.last().cloned(),
        updated_at_ms,
    }
}

fn archive_and_clear_invalid_v4_1(
    store: &SqliteStore,
    job_key: &str,
    archived_at_ms: i64,
) -> anyhow::Result<(usize, usize)> {
    store.with_write_connection(|conn| {
        let tx = conn.unchecked_transaction()?;
        let archived = tx.execute(
            r#"
            INSERT OR IGNORE INTO contract_whale_impact_forecast_audit (
              event_id, forecast_version, archived_at_ms, archive_reason, payload_json
            )
            SELECT event_id, forecast_version, ?1, 'v4_1_walk_forward_rebuild', payload_json
              FROM contract_whale_impact_forecasts
             WHERE forecast_version = ?2
            "#,
            rusqlite::params![archived_at_ms, CONTRACT_WHALE_IMPACT_FORECAST_VERSION],
        )?;
        tx.execute(
            "DELETE FROM contract_whale_impact_forecasts WHERE forecast_version = ?1",
            [CONTRACT_WHALE_IMPACT_FORECAST_VERSION],
        )?;
        let outcomes = tx.execute(
            "DELETE FROM contract_whale_behavior_horizon_outcomes WHERE outcome_version = ?1",
            [CONTRACT_WHALE_IMPACT_FORECAST_VERSION],
        )?;
        tx.execute(
            "DELETE FROM contract_whale_v4_decision_states WHERE forecast_version = ?1",
            [CONTRACT_WHALE_IMPACT_FORECAST_VERSION],
        )?;
        tx.execute(
            "DELETE FROM contract_whale_v4_backfill_checkpoint WHERE job_key = ?1",
            [job_key],
        )?;
        tx.commit()?;
        Ok((archived, outcomes))
    })
}
