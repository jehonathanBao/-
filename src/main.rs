use anyhow::{anyhow, Context};
use btc_toxic_flow_monitor_rs::{
    api, app::AppState, calibration::calibration_runner::CalibrationRunner, config::AppConfig,
    replay::replay_runner::ReplayRunner, safety,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    safety::assert_read_only_runtime()?;
    let config = AppConfig::from_env().context("failed to load config")?;

    let mut args = std::env::args().skip(1);
    if let Some(mode) = args.next() {
        if mode == "serve" {
            let state = AppState::new(config.clone());
            return api::server::serve(config, state).await;
        }
        if mode == "replay" {
            let input = args
                .next()
                .ok_or_else(|| anyhow!("usage: cargo run -- replay <path>"))?;
            let mut runner = ReplayRunner::new(config.clone());
            let report = runner.run_file(&input)?;
            let path = report.write_to_dir(&config.replay_report_dir)?;
            println!("{}", path.display());
            return Ok(());
        }
        if mode == "calibrate" {
            let input = args
                .next()
                .ok_or_else(|| anyhow!("usage: cargo run -- calibrate <path>"))?;
            let runner = CalibrationRunner::new(config.clone());
            let report = runner.run_file(&input)?;
            let (md_path, json_path) = runner.write_report(&report, &config.replay_report_dir)?;
            println!("{}", md_path.display());
            println!("{}", json_path.display());
            return Ok(());
        }
    }

    let state = AppState::new(config.clone());
    state.start().await;

    api::server::serve(config, state).await
}
