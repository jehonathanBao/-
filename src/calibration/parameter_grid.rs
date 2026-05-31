use crate::config::AppConfig;

use super::calibration_types::CalibrationScenario;

#[derive(Debug, Clone)]
pub struct CalibrationGrid {
    pub baseline: CalibrationScenario,
    pub threshold_comparison: Vec<CalibrationScenario>,
    pub toxic_ratio_comparison: Vec<CalibrationScenario>,
    pub vpin_parameter_comparison: Vec<CalibrationScenario>,
    pub liq_hunt_score_comparison: Vec<CalibrationScenario>,
}

impl CalibrationGrid {
    pub fn default_for_config(config: &AppConfig) -> Self {
        let baseline = CalibrationScenario {
            group: "baseline",
            label: "baseline".to_string(),
            toxic_threshold_btc: config.toxic_volume_alert_btc,
            min_toxic_ratio: 0.60,
            vpin_bucket_size_btc: config.vpin_bucket_size_btc,
            vpin_lookback_buckets: config.vpin_lookback_buckets,
            vpin_spike_zscore: config.vpin_spike_zscore,
            liq_hunt_likely_score: config.liq_hunt_likely_score,
            liq_hunt_active_score: config.liq_hunt_active_score,
        };

        Self {
            baseline: baseline.clone(),
            threshold_comparison: [300.0, 600.0, 1000.0, 1500.0, 2000.0]
                .into_iter()
                .map(|threshold| CalibrationScenario {
                    group: "threshold",
                    label: format!("threshold_{threshold:.0}"),
                    toxic_threshold_btc: threshold,
                    ..baseline.clone()
                })
                .collect(),
            toxic_ratio_comparison: [0.50, 0.60, 0.70, 0.80]
                .into_iter()
                .map(|ratio| CalibrationScenario {
                    group: "toxic_ratio",
                    label: format!("min_toxic_ratio_{ratio:.2}"),
                    min_toxic_ratio: ratio,
                    ..baseline.clone()
                })
                .collect(),
            vpin_parameter_comparison: [50.0, 100.0, 250.0]
                .into_iter()
                .flat_map(|bucket_size| {
                    let baseline_for_bucket = baseline.clone();
                    [20_usize, 50, 100].into_iter().flat_map(move |lookback| {
                        let baseline_for_lookback = baseline_for_bucket.clone();
                        [2.0, 2.5, 3.0]
                            .into_iter()
                            .map(move |zscore| {
                                let baseline = baseline_for_lookback.clone();
                                CalibrationScenario {
                                    group: "vpin",
                                    label: format!(
                                        "vpin_bucket_{bucket_size:.0}_lookback_{lookback}_z_{zscore:.1}"
                                    ),
                                    vpin_bucket_size_btc: bucket_size,
                                    vpin_lookback_buckets: lookback,
                                    vpin_spike_zscore: zscore,
                                    ..baseline
                                }
                            })
                    })
                })
                .collect(),
            liq_hunt_score_comparison: [50.0, 60.0]
                .into_iter()
                .flat_map(|likely| {
                    let baseline_for_likely = baseline.clone();
                    [75.0, 80.0]
                        .into_iter()
                        .map(move |active| {
                            let baseline = baseline_for_likely.clone();
                            CalibrationScenario {
                                group: "liq_hunt",
                                label: format!("liq_hunt_likely_{likely:.0}_active_{active:.0}"),
                                liq_hunt_likely_score: likely,
                                liq_hunt_active_score: active,
                                ..baseline
                            }
                        })
                })
                .collect(),
        }
    }
}
