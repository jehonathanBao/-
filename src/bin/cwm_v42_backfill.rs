use std::env;

use anyhow::{anyhow, bail, Context};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::{
    impact_forecast::{
        evaluate_trade_plan_state, event_id, ContractWhaleOutcomeInputs,
        ContractWhaleV4DecisionState,
    },
    impact_v4_2::{
        build_hybrid_forecast, evaluate_v42_outcomes, CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
    },
    types::ContractWhaleSignal,
};
use btc_toxic_flow_monitor_rs::storage::{
    contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
    SqliteStore,
};

#[derive(Debug, Clone)]
struct Options {
    sqlite: String,
    symbol: Option<String>,
    from_ts: Option<i64>,
    to_ts: Option<i64>,
    limit: usize,
    write: bool,
    rebuild: bool,
}

#[derive(Debug, serde::Serialize)]
struct Report {
    forecast_version: &'static str,
    dry_run: bool,
    processed: usize,
    forecasts_written: usize,
    outcomes_written: usize,
    skipped: usize,
    failed: usize,
    checkpoint_key: &'static str,
}

const JOB_KEY: &str = "cwm_v4_2_hybrid_walk_forward_calibration_v2";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let options = parse_args(env::args().skip(1))?;
    let store = SqliteStore::open(&options.sqlite)?;
    let report = run(store, options).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.failed > 0 {
        bail!(
            "V4.2 backfill completed with {} failed events",
            report.failed
        );
    }
    Ok(())
}

async fn run(store: SqliteStore, options: Options) -> anyhow::Result<Report> {
    if options.rebuild && !options.write {
        bail!("--rebuild-v4-2 requires --write");
    }
    let now = chrono::Utc::now().timestamp_millis();
    let existing_checkpoint = if options.rebuild {
        None
    } else {
        store.get_contract_whale_v4_backfill_checkpoint(JOB_KEY)?
    };
    let resume_after = existing_checkpoint.as_ref().and_then(|checkpoint| {
        checkpoint
            .last_event_ts
            .zip(checkpoint.last_event_id.clone())
    });
    let mut signals = store.query_contract_whale_signals(&ContractWhaleSignalQuery {
        symbol: options.symbol.clone().map(|v| v.to_ascii_uppercase()),
        from_ts: options.from_ts,
        to_ts: options.to_ts.or(Some(now)),
        limit: options.limit.max(1),
        ..ContractWhaleSignalQuery::default()
    })?;
    signals.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.id.cmp(&b.id)));
    let before = signals.len();
    signals.retain(binance_only);
    let skipped = before.saturating_sub(signals.len());
    if let Some((resume_ts, resume_id)) = resume_after.as_ref() {
        signals.retain(|signal| {
            signal.ts > *resume_ts || (signal.ts == *resume_ts && signal.id > *resume_id)
        });
    }
    if options.rebuild && options.write {
        store.delete_contract_whale_impact_forecasts_for_version(
            CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
        )?;
        store.delete_contract_whale_horizon_outcomes_for_version(
            CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
        )?;
    }
    let mut history = std::collections::BTreeMap::<String, Vec<_>>::new();
    let mut forecasts = Vec::new();
    let mut outcomes = Vec::new();
    let mut states = Vec::new();
    let mut processed = 0;
    let mut failed = 0;
    let mut last_event_ts = existing_checkpoint
        .as_ref()
        .and_then(|checkpoint| checkpoint.last_event_ts);
    let mut last_event_id = existing_checkpoint
        .as_ref()
        .and_then(|checkpoint| checkpoint.last_event_id.clone());
    for signal in signals {
        let symbol = signal.symbol.clone();
        last_event_ts = Some(signal.ts);
        last_event_id = Some(signal.id.clone());
        if !history.contains_key(&symbol) {
            history.insert(
                symbol.clone(),
                store.list_contract_whale_horizon_outcomes_before(&symbol, signal.ts, 100_000)?,
            );
        }
        let from_ts = signal.ts.saturating_sub(4 * 60 * 60 * 1_000);
        let to_ts = now.min(signal.ts.saturating_add(86_400_000));
        let refs = store.list_contract_reference_prices_between(&symbol, from_ts, to_ts)?;
        let flow = store.list_contract_flow_buckets_between(&symbol, from_ts, to_ts)?;
        let oi = store.list_contract_oi_snapshots_between(&symbol, from_ts, to_ts)?;
        let funding = store.list_contract_funding_snapshots_between(&symbol, from_ts, to_ts)?;
        let liq = store.list_contract_liquidation_buckets_between(&symbol, signal.ts, to_ts)?;
        let Some(prior) = history.get_mut(&symbol) else {
            failed += 1;
            continue;
        };
        let forecast = build_hybrid_forecast(&signal, prior, &refs, signal.ts);
        let event_outcomes = evaluate_v42_outcomes(
            &signal,
            ContractWhaleOutcomeInputs {
                flow_buckets: &flow,
                reference_prices: &refs,
                oi_snapshots: &oi,
                funding_snapshots: &funding,
                liquidation_buckets: &liq,
            },
            now,
        );
        let (state, reason) = evaluate_trade_plan_state(&forecast, &event_outcomes, now);
        states.push(ContractWhaleV4DecisionState {
            event_id: event_id(&signal),
            forecast_version: CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION.to_string(),
            state,
            reason,
            updated_at_ms: now,
            decided_at_ms: Some(now),
        });
        if options.write {
            prior.extend(event_outcomes.iter().cloned());
            outcomes.extend(event_outcomes);
            forecasts.push(forecast);
        }
        processed += 1;
    }
    if options.write {
        let outcomes_written = store.upsert_contract_whale_horizon_outcomes(&outcomes)?;
        let forecasts_written = store.upsert_contract_whale_impact_forecasts(&forecasts)?;
        store.upsert_contract_whale_v4_decision_states(&states)?;
        store.upsert_contract_whale_v4_backfill_checkpoint(&btc_toxic_flow_monitor_rs::storage::contract_whale_repo::ContractWhaleV4BackfillCheckpoint {
            job_key: JOB_KEY.to_string(), status: "completed".to_string(), last_event_ts, last_event_id,
            processed_count: processed, forecast_count: forecasts_written, outcome_count: outcomes_written,
            skipped_count: skipped, degraded_count: 0, failed_count: failed, last_error: None, updated_at_ms: now,
        })?;
        Ok(Report {
            forecast_version: CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
            dry_run: false,
            processed,
            forecasts_written,
            outcomes_written,
            skipped,
            failed,
            checkpoint_key: JOB_KEY,
        })
    } else {
        Ok(Report {
            forecast_version: CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
            dry_run: true,
            processed,
            forecasts_written: 0,
            outcomes_written: 0,
            skipped,
            failed,
            checkpoint_key: JOB_KEY,
        })
    }
}

fn binance_only(signal: &ContractWhaleSignal) -> bool {
    let active = signal
        .active_contract_sources
        .iter()
        .all(|s| s.eq_ignore_ascii_case("binance"))
        && signal
            .active_contract_sources
            .iter()
            .any(|s| s.eq_ignore_ascii_case("binance"));
    let contributions = signal
        .exchanges
        .iter()
        .all(|v| v.exchange.eq_ignore_ascii_case("binance"))
        && signal
            .exchanges
            .iter()
            .any(|v| v.exchange.eq_ignore_ascii_case("binance"));
    active || contributions
}

fn parse_args(mut args: impl Iterator<Item = String>) -> anyhow::Result<Options> {
    let mut options = Options {
        sqlite: env::var("SQLITE_PATH")
            .unwrap_or_else(|_| ".runtime/btc-toxic-flow.sqlite".to_string()),
        symbol: None,
        from_ts: None,
        to_ts: None,
        limit: 100_000,
        write: false,
        rebuild: false,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sqlite" => options.sqlite = next(&mut args, "--sqlite")?,
            "--symbol" => options.symbol = Some(next(&mut args, "--symbol")?),
            "--from-ts" => {
                options.from_ts = Some(
                    next(&mut args, "--from-ts")?
                        .parse()
                        .context("invalid --from-ts")?,
                )
            }
            "--to-ts" => {
                options.to_ts = Some(
                    next(&mut args, "--to-ts")?
                        .parse()
                        .context("invalid --to-ts")?,
                )
            }
            "--limit" => {
                options.limit = next(&mut args, "--limit")?
                    .parse()
                    .context("invalid --limit")?
            }
            "--write" => options.write = true,
            "--dry-run" => options.write = false,
            "--rebuild-v4-2" => options.rebuild = true,
            "-h" | "--help" => bail!(usage()),
            other => bail!("unknown argument `{other}`\n{}", usage()),
        }
    }
    Ok(options)
}

fn next(args: &mut impl Iterator<Item = String>, name: &str) -> anyhow::Result<String> {
    args.next()
        .ok_or_else(|| anyhow!("{name} requires a value"))
}

fn usage() -> &'static str {
    "usage: cwm_v42_backfill [--sqlite <path>] [--symbol BTC] [--from-ts <ms>] [--to-ts <ms>] [--limit <n>] [--write|--dry-run] [--rebuild-v4-2]"
}
