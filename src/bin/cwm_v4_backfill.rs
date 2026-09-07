use std::env;

use anyhow::{anyhow, bail};
use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::v4_backfill::{
        run_contract_whale_v4_backfill, ContractWhaleV4BackfillOptions,
    },
    storage::SqliteStore,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let (sqlite_path, options) = parse_args(env::args().skip(1))?;
    let store = SqliteStore::open(&sqlite_path)?;
    let report = run_contract_whale_v4_backfill(store, options).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.failed > 0 {
        bail!("V4.1 backfill completed with {} failed events", report.failed);
    }
    Ok(())
}

fn parse_args(
    mut args: impl Iterator<Item = String>,
) -> anyhow::Result<(String, ContractWhaleV4BackfillOptions)> {
    let mut sqlite_path = env::var("SQLITE_PATH")
        .unwrap_or_else(|_| ".runtime/btc-toxic-flow.sqlite".to_string());
    let mut options = ContractWhaleV4BackfillOptions::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sqlite" => sqlite_path = next_value(&mut args, "--sqlite")?,
            "--job-key" => options.job_key = next_value(&mut args, "--job-key")?,
            "--symbol" => options.symbol = Some(next_value(&mut args, "--symbol")?),
            "--from-ts" => options.from_ts = Some(parse_i64(next_value(&mut args, "--from-ts")?)?),
            "--to-ts" => options.to_ts = Some(parse_i64(next_value(&mut args, "--to-ts")?)?),
            "--limit" => {
                options.limit = next_value(&mut args, "--limit")?
                    .parse::<usize>()
                    .map_err(|error| anyhow!("invalid --limit: {error}"))?;
            }
            "--write" => options.dry_run = false,
            "--dry-run" => options.dry_run = true,
            "--rebuild-v4-1" => options.rebuild_v4_1 = true,
            "--no-fetch-reference-history" => options.fetch_reference_history = false,
            "-h" | "--help" => bail!(usage()),
            other => bail!("unknown argument `{other}`\n{}", usage()),
        }
    }
    if options.rebuild_v4_1 && options.dry_run {
        bail!("--rebuild-v4-1 requires --write");
    }
    Ok((sqlite_path, options))
}

fn next_value(
    args: &mut impl Iterator<Item = String>,
    name: &str,
) -> anyhow::Result<String> {
    args.next()
        .ok_or_else(|| anyhow!("{name} requires a value"))
}

fn parse_i64(value: String) -> anyhow::Result<i64> {
    value
        .parse::<i64>()
        .map_err(|error| anyhow!("invalid timestamp `{value}`: {error}"))
}

fn usage() -> &'static str {
    "usage: cwm_v4_backfill [--sqlite <path>] [--symbol BTC] [--from-ts <ms>] [--to-ts <ms>] [--limit <n>] [--write] [--rebuild-v4-1] [--no-fetch-reference-history]"
}
