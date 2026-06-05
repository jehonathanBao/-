use anyhow::{anyhow, Context};
use btc_toxic_flow_monitor_rs::replay::{
    production_report::{run_production_replay, write_production_report},
    replay_config::ProductionReplayConfig,
};

fn main() -> anyhow::Result<()> {
    let config_path = parse_config_path()?;
    let config = ProductionReplayConfig::from_file(&config_path)?;
    let report = run_production_replay(&config).with_context(|| {
        format!(
            "production replay failed for input {}",
            config.input_path().display()
        )
    })?;
    let output = write_production_report(&report, &config)?;
    println!("{}", output.report_dir.display());
    Ok(())
}

fn parse_config_path() -> anyhow::Result<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            return args.next().ok_or_else(|| {
                anyhow!("usage: cargo run --bin replay_production -- --config <path>")
            });
        }
    }
    Err(anyhow!(
        "usage: cargo run --bin replay_production -- --config <path>"
    ))
}
