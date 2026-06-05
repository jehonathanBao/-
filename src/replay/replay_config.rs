use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductionReplayConfig {
    pub input: ReplayInputConfig,
    pub replay: ReplayRuntimeConfig,
    pub markout: MarkoutConfig,
    pub alert_gate: AlertGateConfig,
    pub output: ReplayOutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayInputConfig {
    pub path: PathBuf,
    pub format: String,
    pub venue: Option<String>,
    pub symbol: Option<String>,
    pub timezone: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayRuntimeConfig {
    pub sort_by_ts: bool,
    pub max_events: usize,
    pub start_ts_ms: i64,
    pub end_ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MarkoutConfig {
    pub horizons_ms: Vec<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertGateConfig {
    pub min_score: u8,
    pub min_data_quality: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayOutputConfig {
    pub report_dir: PathBuf,
    pub write_json: bool,
    pub write_markdown: bool,
    pub write_csv: bool,
}

impl ProductionReplayConfig {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()
            .with_context(|| {
                format!(
                    "failed to read production replay config {}",
                    path.as_ref().display()
                )
            })?
            .try_deserialize::<Self>()
            .context("failed to parse production replay config")
    }

    pub fn input_path(&self) -> &Path {
        &self.input.path
    }

    pub fn output_root(&self) -> &Path {
        &self.output.report_dir
    }
}

impl Default for ReplayInputConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/production_replay/BTCUSDT_2025-01-01.jsonl"),
            format: "auto".to_string(),
            venue: None,
            symbol: None,
            timezone: "UTC".to_string(),
        }
    }
}

impl Default for ReplayRuntimeConfig {
    fn default() -> Self {
        Self {
            sort_by_ts: true,
            max_events: 0,
            start_ts_ms: 0,
            end_ts_ms: 0,
        }
    }
}

impl Default for MarkoutConfig {
    fn default() -> Self {
        Self {
            horizons_ms: vec![1_000, 5_000, 30_000],
        }
    }
}

impl Default for AlertGateConfig {
    fn default() -> Self {
        Self {
            min_score: 80,
            min_data_quality: 70.0,
        }
    }
}

impl Default for ReplayOutputConfig {
    fn default() -> Self {
        Self {
            report_dir: PathBuf::from("data/production_replay/reports"),
            write_json: true,
            write_markdown: true,
            write_csv: true,
        }
    }
}
