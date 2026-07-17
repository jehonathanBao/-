use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::market_regime_engine::MarketRegime;

/// Live regime snapshot shared by detectors and V3 adaptive control.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeContext {
    pub regime: MarketRegime,
    /// Classifier confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Effective multipliers keyed by detector knob (e.g. `toxic_volume_factor`).
    pub multipliers: BTreeMap<String, f64>,
    pub timestamp_ms: i64,
    pub read_only: bool,
}

impl Default for RegimeContext {
    fn default() -> Self {
        Self {
            regime: MarketRegime::Accumulation,
            confidence: 0.6,
            multipliers: BTreeMap::from([
                ("toxic_volume_factor".to_string(), 1.0),
                ("sweep_min_notional_factor".to_string(), 1.0),
                ("vpin_z_threshold_factor".to_string(), 1.0),
                ("liq_hunt_sensitivity_factor".to_string(), 1.0),
                ("global_alert_factor".to_string(), 1.0),
                ("min_confidence".to_string(), 0.0),
            ]),
            timestamp_ms: 0,
            read_only: true,
        }
    }
}

/// Per-regime detector threshold multipliers.
///
/// Factor `< 1.0` lowers volume / score thresholds (more sensitive).
/// Factor `> 1.0` raises them (fewer false positives).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct RegimeMultiplier {
    #[serde(default = "default_one")]
    pub toxic_volume_factor: f64,
    #[serde(default = "default_one")]
    pub vpin_z_threshold_factor: f64,
    #[serde(default = "default_one")]
    pub wall_persistence_factor: f64,
    #[serde(default = "default_one")]
    pub liq_hunt_sensitivity_factor: f64,
    #[serde(default = "default_one")]
    pub sweep_min_notional_factor: f64,
    #[serde(default = "default_one")]
    pub global_alert_factor: f64,
    /// Soft gate: when live confidence is below this, dampen score / skip aggressive emits.
    #[serde(default)]
    pub min_confidence: f64,
}

impl Default for RegimeMultiplier {
    fn default() -> Self {
        Self {
            toxic_volume_factor: 1.0,
            vpin_z_threshold_factor: 1.0,
            wall_persistence_factor: 1.0,
            liq_hunt_sensitivity_factor: 1.0,
            sweep_min_notional_factor: 1.0,
            global_alert_factor: 1.0,
            min_confidence: 0.0,
        }
    }
}

/// Configurable regime-aware threshold table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct RegimeThresholds {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_refresh_interval_ms")]
    pub refresh_interval_ms: u64,
    /// Keys use `SCREAMING_SNAKE_CASE` regime names (`MANIPULATION`, ...).
    #[serde(default = "default_regime_adjust")]
    pub regime_adjust: BTreeMap<String, RegimeMultiplier>,
}

impl Default for RegimeThresholds {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_interval_ms: 15_000,
            regime_adjust: default_regime_adjust(),
        }
    }
}

impl RegimeThresholds {
    pub fn multiplier_for(&self, regime: MarketRegime) -> RegimeMultiplier {
        self.regime_adjust
            .get(regime.as_key())
            .cloned()
            .unwrap_or_default()
    }
}

fn default_one() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_refresh_interval_ms() -> u64 {
    15_000
}

fn default_regime_adjust() -> BTreeMap<String, RegimeMultiplier> {
    BTreeMap::from([
        (
            MarketRegime::Manipulation.as_key().to_string(),
            RegimeMultiplier {
                toxic_volume_factor: 0.60,
                vpin_z_threshold_factor: 0.85,
                wall_persistence_factor: 0.80,
                liq_hunt_sensitivity_factor: 0.85,
                sweep_min_notional_factor: 0.60,
                global_alert_factor: 0.95,
                min_confidence: 0.0,
            },
        ),
        (
            MarketRegime::Liquidation.as_key().to_string(),
            RegimeMultiplier {
                toxic_volume_factor: 1.40,
                vpin_z_threshold_factor: 1.25,
                wall_persistence_factor: 1.20,
                liq_hunt_sensitivity_factor: 1.25,
                sweep_min_notional_factor: 1.35,
                global_alert_factor: 1.10,
                min_confidence: 0.55,
            },
        ),
        (
            MarketRegime::Accumulation.as_key().to_string(),
            RegimeMultiplier {
                toxic_volume_factor: 0.85,
                vpin_z_threshold_factor: 0.95,
                wall_persistence_factor: 0.90,
                liq_hunt_sensitivity_factor: 0.90,
                sweep_min_notional_factor: 0.85,
                global_alert_factor: 0.98,
                min_confidence: 0.0,
            },
        ),
        (
            MarketRegime::Distribution.as_key().to_string(),
            RegimeMultiplier {
                toxic_volume_factor: 1.05,
                vpin_z_threshold_factor: 1.05,
                wall_persistence_factor: 1.05,
                liq_hunt_sensitivity_factor: 1.05,
                sweep_min_notional_factor: 1.05,
                global_alert_factor: 1.02,
                min_confidence: 0.0,
            },
        ),
    ])
}
