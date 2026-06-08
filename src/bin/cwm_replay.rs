use std::{env, path::PathBuf};

use anyhow::{anyhow, bail};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::replay::{
    format_contract_whale_replay_report, run_contract_whale_replay,
};

fn main() -> anyhow::Result<()> {
    let args = parse_args(env::args().skip(1))?;
    let report = run_contract_whale_replay(&args.input)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", format_contract_whale_replay_report(&report));
    }
    Ok(())
}

struct ReplayArgs {
    input: PathBuf,
    json: bool,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> anyhow::Result<ReplayArgs> {
    let mut input = None;
    let mut json = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                input = Some(
                    args.next()
                        .map(PathBuf::from)
                        .ok_or_else(|| anyhow!("--input requires a file path"))?,
                );
            }
            "--json" => json = true,
            "-h" | "--help" => bail!(usage()),
            other => bail!("unknown argument `{other}`\n{}", usage()),
        }
    }
    Ok(ReplayArgs {
        input: input.ok_or_else(|| anyhow!(usage()))?,
        json,
    })
}

fn usage() -> &'static str {
    "usage: cargo run --bin cwm_replay -- --input <path> [--json]"
}
