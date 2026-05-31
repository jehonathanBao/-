use std::{
    collections::BTreeMap,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::{
    replay::liq_hunt_replay_report::LiqHuntReplaySummary,
    replay::liquidation_replay_report::LiquidationReplaySummary,
    replay::vpin_replay_report::VpinReplaySummary,
    types::toxic::{ToxicEvent, ToxicSeverity},
};

#[derive(Debug, Clone)]
pub struct ReplayMarkerOutcome {
    pub matched: usize,
    pub missed: usize,
    pub unexpected: usize,
}

#[derive(Debug, Clone)]
pub struct ReplayReport {
    pub input_path: String,
    pub event_count: usize,
    pub trade_count: usize,
    pub book_count: usize,
    pub detected_events: Vec<ToxicEvent>,
    pub threshold_buckets: BTreeMap<String, usize>,
    pub reason_code_frequency: BTreeMap<String, usize>,
    pub markers: ReplayMarkerOutcome,
    pub vpin_summary: Option<VpinReplaySummary>,
    pub liquidation_summary: Option<LiquidationReplaySummary>,
    pub liq_hunt_summary: Option<LiqHuntReplaySummary>,
}

impl ReplayReport {
    pub fn max_toxic_volume_btc(&self) -> f64 {
        self.detected_events
            .iter()
            .map(|event| event.toxic_volume_btc)
            .fold(0.0, f64::max)
    }

    pub fn max_severity(&self) -> ToxicSeverity {
        self.detected_events
            .iter()
            .map(|event| event.severity)
            .max()
            .unwrap_or(ToxicSeverity::Normal)
    }

    pub fn buy_count(&self) -> usize {
        self.detected_events
            .iter()
            .filter(|event| matches!(event.direction, crate::types::toxic::ToxicDirection::Buy))
            .count()
    }

    pub fn sell_count(&self) -> usize {
        self.detected_events
            .iter()
            .filter(|event| matches!(event.direction, crate::types::toxic::ToxicDirection::Sell))
            .count()
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# BTC Toxic Flow Replay Report");
        let _ = writeln!(out);
        let _ = writeln!(out, "Input:");
        let _ = writeln!(out, "- file path: {}", self.input_path);
        let _ = writeln!(out, "- event count: {}", self.event_count);
        let _ = writeln!(out, "- trade count: {}", self.trade_count);
        let _ = writeln!(out, "- book count: {}", self.book_count);
        let _ = writeln!(out);
        let _ = writeln!(out, "Detected Toxic Events:");
        let _ = writeln!(out, "- count: {}", self.detected_events.len());
        let _ = writeln!(
            out,
            "- max toxic_volume_btc: {:.1}",
            self.max_toxic_volume_btc()
        );
        let _ = writeln!(out, "- max severity: {:?}", self.max_severity());
        let _ = writeln!(out, "- buy count: {}", self.buy_count());
        let _ = writeln!(out, "- sell count: {}", self.sell_count());
        if !self.detected_events.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "Detected Toxic Event Details:");
            for (index, event) in self.detected_events.iter().enumerate() {
                let _ = writeln!(
                    out,
                    "- Event #{} ts={} direction={:?} severity={:?} toxic_volume_btc={:.1} window_ms={}",
                    index + 1,
                    event.ts,
                    event.direction,
                    event.severity,
                    event.toxic_volume_btc,
                    event.window_ms
                );
                let _ = writeln!(
                    out,
                    "  - liquidation: side={:?} cluster_distance_bps={:?} cluster_notional_usd={:?} liq_hunt_pressure={:.2} nearby={} possible={}",
                    event.nearest_cluster_side,
                    event.cluster_distance_bps,
                    event.cluster_notional_usd,
                    event.liq_hunt_pressure,
                    event.liq_cluster_nearby,
                    event.possible_liq_hunt_setup
                );
                let _ = writeln!(out, "  - reasons: {}", event.reason_codes.join(", "));
            }
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "Threshold Buckets:");
        for (bucket, count) in &self.threshold_buckets {
            let _ = writeln!(out, "- {bucket}: {count}");
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "Reason Code Frequency:");
        for (code, count) in &self.reason_code_frequency {
            let _ = writeln!(out, "- {code}: {count}");
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "Expected Markers:");
        let _ = writeln!(out, "- matched: {}", self.markers.matched);
        let _ = writeln!(out, "- missed: {}", self.markers.missed);
        let _ = writeln!(out, "- unexpected: {}", self.markers.unexpected);
        if let Some(vpin) = &self.vpin_summary {
            let _ = writeln!(out);
            let _ = writeln!(out, "## VPIN Summary");
            let _ = writeln!(out, "- Bucket size BTC: {:.1}", vpin.bucket_size_btc);
            let _ = writeln!(out, "- Lookback buckets: {}", vpin.lookback_buckets);
            let _ = writeln!(out, "- Completed buckets: {}", vpin.completed_buckets);
            let _ = writeln!(out, "- Max VPIN: {:?}", vpin.max_vpin);
            let _ = writeln!(out, "- Max VPIN z-score: {:?}", vpin.max_vpin_zscore);
            let _ = writeln!(out, "- VPIN high count: {}", vpin.vpin_high_count);
            let _ = writeln!(out, "- VPIN spike count: {}", vpin.vpin_spike_count);
            let _ = writeln!(out, "- VPIN extreme count: {}", vpin.vpin_extreme_count);
            let _ = writeln!(out, "- Dominant direction: {:?}", vpin.dominant_direction);
            let _ = writeln!(out);
            let _ = writeln!(out, "Top VPIN Buckets:");
            for bucket in &vpin.top_buckets {
                let _ = writeln!(
                    out,
                    "- ts={} imbalance_ratio={:.3} direction={:?} buy_btc={:.1} sell_btc={:.1}",
                    bucket.end_ts,
                    bucket.imbalance_ratio,
                    bucket.direction,
                    bucket.buy_btc,
                    bucket.sell_btc
                );
            }
        }
        if let Some(liquidation) = &self.liquidation_summary {
            let _ = writeln!(out);
            let _ = writeln!(out, "## Liquidation Cluster Summary");
            let _ = writeln!(out, "| Metric | Value |");
            let _ = writeln!(out, "|---|---:|");
            let _ = writeln!(out, "| Snapshots Seen | {} |", liquidation.snapshots_seen);
            let _ = writeln!(
                out,
                "| Clusters Detected | {} |",
                liquidation.clusters_detected
            );
            let _ = writeln!(
                out,
                "| Nearby Cluster Events | {} |",
                liquidation.nearby_cluster_events
            );
            let _ = writeln!(
                out,
                "| Possible Hunt Setups | {} |",
                liquidation.possible_hunt_setups
            );
            let _ = writeln!(
                out,
                "| Max Cluster Intensity | {:.2} |",
                liquidation.max_cluster_intensity
            );
            let _ = writeln!(out, "| Strongest Side | {} |", liquidation.strongest_side);
            let _ = writeln!(out);
            let _ = writeln!(out, "## Liquidation Evidence Events");
            let _ = writeln!(
                out,
                "| Time | Price | Side | Cluster Price | Distance bps | Intensity | Flags |"
            );
            let _ = writeln!(out, "|---|---:|---|---:|---:|---:|---|");
            for evidence in &liquidation.evidence {
                let flags = evidence.explanation.join(", ");
                let _ = writeln!(
                    out,
                    "| {} | {:.1} | {:?} | {} | {} | {:.2} | {} |",
                    evidence.ts_ms,
                    evidence.mark_price,
                    evidence.nearest_cluster_side,
                    evidence
                        .nearest_cluster_price
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "-".to_string()),
                    evidence
                        .nearest_cluster_distance_bps
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "-".to_string()),
                    evidence.cluster_intensity,
                    flags
                );
            }
        }
        if let Some(liq_hunt) = &self.liq_hunt_summary {
            let _ = writeln!(out);
            let _ = writeln!(out, "## Liq Hunt Detector Summary");
            let _ = writeln!(out, "- Max score: {:.1}", liq_hunt.max_score);
            let _ = writeln!(out, "- Active count: {}", liq_hunt.active_count);
            let _ = writeln!(out, "- Likely count: {}", liq_hunt.likely_count);
            let _ = writeln!(out, "- Watch count: {}", liq_hunt.watch_count);
            let _ = writeln!(out);
            let _ = writeln!(out, "Direction Count:");
            let _ = writeln!(out, "- short_squeeze: {}", liq_hunt.short_squeeze_count);
            let _ = writeln!(out, "- long_squeeze: {}", liq_hunt.long_squeeze_count);
            let _ = writeln!(out);
            let _ = writeln!(out, "Top Liq Hunt Signals:");
            let _ = writeln!(
                out,
                "| ts | level | direction | score | toxic_volume_btc | cluster_side | distance_bps | cluster_notional_usd | reason_codes |"
            );
            let _ = writeln!(out, "|---|---|---|---:|---:|---|---:|---:|---|");
            for signal in &liq_hunt.top_signals {
                let _ = writeln!(
                    out,
                    "| {} | {:?} | {:?} | {:.1} | {} | {} | {} | {} | {} |",
                    signal.ts,
                    signal.level,
                    signal.direction,
                    signal.score,
                    signal
                        .toxic_volume_btc
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "-".to_string()),
                    signal
                        .nearest_cluster_side
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                    signal
                        .nearest_cluster_distance_bps
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "-".to_string()),
                    signal
                        .nearest_cluster_notional_usd
                        .map(|value| format!("{value:.0}"))
                        .unwrap_or_else(|| "-".to_string()),
                    signal.reason_codes.join(", ")
                );
            }
        }
        out
    }

    pub fn write_to_dir(&self, dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(dir.as_ref())
            .with_context(|| format!("failed to create report dir {}", dir.as_ref().display()))?;
        let path = dir.as_ref().join(format!(
            "replay-{}.md",
            chrono::Utc::now().timestamp_millis()
        ));
        fs::write(&path, self.to_markdown())
            .with_context(|| format!("failed to write report {}", path.display()))?;
        Ok(path)
    }
}
