//! Regime-aware dynamic detector thresholds.
//!
//! Read-only monitoring path only: multipliers adjust candidate sensitivity and
//! never enable execution, Discord/Telegram gate bypass, or config mutation.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;

use crate::{
    config::thresholds::{LiqHuntParams, ToxicVolumeParams, VpinParams},
    market_regime_engine::{
        analyze_market_regime, MarketFeatureSet, MarketRegime, MarketRegimeEngineOutput,
    },
    normalizers::trade::now_ms,
    toxicity::sweep_detector::SweepParams,
    types::regime::{RegimeContext, RegimeMultiplier, RegimeThresholds},
};

static REGIME_THRESHOLDS_CONFIG: OnceLock<RwLock<RegimeThresholds>> = OnceLock::new();

pub fn set_regime_thresholds_config(config: RegimeThresholds) {
    match REGIME_THRESHOLDS_CONFIG.get() {
        Some(slot) => *slot.write() = config,
        None => {
            let _ = REGIME_THRESHOLDS_CONFIG.set(RwLock::new(config));
        }
    }
}

pub fn regime_thresholds_config() -> RegimeThresholds {
    REGIME_THRESHOLDS_CONFIG
        .get()
        .map(|slot| slot.read().clone())
        .unwrap_or_default()
}

pub fn load_regime_thresholds_from_settings(settings: &::config::Config) -> RegimeThresholds {
    match settings.get::<RegimeThresholds>("regime_thresholds") {
        Ok(mut loaded) => {
            if loaded.regime_adjust.is_empty() {
                loaded.regime_adjust = RegimeThresholds::default().regime_adjust;
            }
            if loaded.refresh_interval_ms == 0 {
                loaded.refresh_interval_ms = RegimeThresholds::default().refresh_interval_ms;
            }
            loaded
        }
        Err(_) => RegimeThresholds::default(),
    }
}

pub trait RegimeAdjustable {
    fn adjust(&self, factor: f64) -> Self;
}

pub trait RegimeAwareProvider {
    fn regime_manager(&self) -> &Arc<RegimeThresholdManager>;
}

#[derive(Clone)]
pub struct RegimeThresholdManager {
    current: Arc<RwLock<RegimeContext>>,
    thresholds: Arc<RwLock<RegimeThresholds>>,
}

impl RegimeThresholdManager {
    pub fn new(config: &RegimeThresholds) -> Self {
        let mut ctx = RegimeContext::default();
        ctx.timestamp_ms = now_ms();
        if config.enabled {
            ctx.multipliers = multipliers_from_adj(
                &config.multiplier_for(MarketRegime::Accumulation),
                ctx.confidence,
            );
        }
        Self {
            current: Arc::new(RwLock::new(ctx)),
            thresholds: Arc::new(RwLock::new(config.clone())),
        }
    }

    pub fn from_runtime_config() -> Self {
        Self::new(&regime_thresholds_config())
    }

    pub fn current(&self) -> RegimeContext {
        self.current.read().clone()
    }

    pub fn thresholds(&self) -> RegimeThresholds {
        self.thresholds.read().clone()
    }

    pub fn enabled(&self) -> bool {
        self.thresholds.read().enabled
    }

    pub fn update_from_engine(&self, output: &MarketRegimeEngineOutput) {
        if !self.enabled() {
            return;
        }

        let regime = parse_regime(&output.regime.regime);
        let confidence = output.regime.confidence.clamp(0.0, 1.0);
        let adj = self.thresholds.read().multiplier_for(regime);
        let multipliers = multipliers_from_adj(&adj, confidence);

        let mut ctx = self.current.write();
        ctx.regime = regime;
        ctx.confidence = confidence;
        ctx.multipliers = multipliers;
        ctx.timestamp_ms = now_ms();
        ctx.read_only = true;
    }

    pub fn analyze_and_update(&self, features: &MarketFeatureSet) -> MarketRegimeEngineOutput {
        let output = analyze_market_regime(features);
        self.update_from_engine(&output);
        output
    }

    pub fn factor(&self, key: &str) -> f64 {
        if !self.enabled() {
            return 1.0;
        }
        self.current
            .read()
            .multipliers
            .get(key)
            .copied()
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(1.0)
    }

    pub fn get_adjusted<T: RegimeAdjustable + Clone>(&self, base: &T, key: &str) -> T {
        base.adjust(self.factor(key))
    }

    pub fn adjusted_toxic_volume_params(&self, base: &ToxicVolumeParams) -> ToxicVolumeParams {
        self.get_adjusted(base, "toxic_volume_factor")
    }

    pub fn adjusted_sweep_params(&self, base: &SweepParams) -> SweepParams {
        self.get_adjusted(base, "sweep_min_notional_factor")
    }

    pub fn adjusted_vpin_params(&self, base: &VpinParams) -> VpinParams {
        self.get_adjusted(base, "vpin_z_threshold_factor")
    }

    pub fn adjusted_liq_hunt_params(&self, base: &LiqHuntParams) -> LiqHuntParams {
        self.get_adjusted(base, "liq_hunt_sensitivity_factor")
    }

    pub fn min_confidence_gate(&self) -> f64 {
        self.factor("min_confidence").clamp(0.0, 1.0)
    }

    pub fn passes_confidence_gate(&self) -> bool {
        let ctx = self.current.read();
        let min_confidence = ctx
            .multipliers
            .get("min_confidence")
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        ctx.confidence + f64::EPSILON >= min_confidence
    }
}

/// Blend regime multiplier toward `1.0` when classifier confidence is low.
///
/// `effective = 1.0 + (raw - 1.0) * confidence`
fn confidence_scaled(raw: f64, confidence: f64) -> f64 {
    let conf = confidence.clamp(0.0, 1.0);
    let raw = if raw.is_finite() && raw > 0.0 {
        raw
    } else {
        1.0
    };
    1.0 + (raw - 1.0) * conf
}

fn multipliers_from_adj(adj: &RegimeMultiplier, confidence: f64) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (
            "toxic_volume_factor".to_string(),
            confidence_scaled(adj.toxic_volume_factor, confidence),
        ),
        (
            "sweep_min_notional_factor".to_string(),
            confidence_scaled(adj.sweep_min_notional_factor, confidence),
        ),
        (
            "vpin_z_threshold_factor".to_string(),
            confidence_scaled(adj.vpin_z_threshold_factor, confidence),
        ),
        (
            "wall_persistence_factor".to_string(),
            confidence_scaled(adj.wall_persistence_factor, confidence),
        ),
        (
            "liq_hunt_sensitivity_factor".to_string(),
            confidence_scaled(adj.liq_hunt_sensitivity_factor, confidence),
        ),
        (
            "global_alert_factor".to_string(),
            confidence_scaled(adj.global_alert_factor, confidence),
        ),
        (
            "min_confidence".to_string(),
            adj.min_confidence.clamp(0.0, 1.0),
        ),
    ])
}

pub fn parse_regime(value: &str) -> MarketRegime {
    match value.trim().to_ascii_uppercase().as_str() {
        "LIQUIDATION" => MarketRegime::Liquidation,
        "MANIPULATION" => MarketRegime::Manipulation,
        "DISTRIBUTION" => MarketRegime::Distribution,
        _ => MarketRegime::Accumulation,
    }
}

impl RegimeAdjustable for ToxicVolumeParams {
    fn adjust(&self, factor: f64) -> Self {
        let factor = sanitize_factor(factor);
        let mut params = self.clone();
        params.threshold_btc = (params.threshold_btc * factor).max(1.0);
        params.min_large_flow_btc = (params.min_large_flow_btc * factor).max(1.0);
        params
    }
}

impl RegimeAdjustable for SweepParams {
    fn adjust(&self, factor: f64) -> Self {
        let factor = sanitize_factor(factor);
        let mut params = self.clone();
        params.min_swept_volume_btc = (params.min_swept_volume_btc * factor).max(1.0);
        params
    }
}

impl RegimeAdjustable for VpinParams {
    fn adjust(&self, factor: f64) -> Self {
        let factor = sanitize_factor(factor);
        let mut params = self.clone();
        params.spike_zscore = (params.spike_zscore * factor).clamp(0.5, 8.0);
        params.high_threshold = (params.high_threshold * factor).clamp(0.20, 0.98);
        params.extreme_threshold = (params.extreme_threshold * factor)
            .max(params.high_threshold + 0.01)
            .clamp(0.25, 0.99);
        params
    }
}

impl RegimeAdjustable for LiqHuntParams {
    fn adjust(&self, factor: f64) -> Self {
        let factor = sanitize_factor(factor);
        let mut params = self.clone();
        // Higher factor → higher score gates → fewer emits (lower false positives).
        params.active_score = (params.active_score * factor).clamp(10.0, 99.0);
        params.likely_score = (params.likely_score * factor).clamp(5.0, 95.0);
        params.watch_score = (params.watch_score * factor).clamp(1.0, 90.0);
        params
    }
}

fn sanitize_factor(factor: f64) -> f64 {
    if factor.is_finite() && factor > 0.0 {
        factor.clamp(0.25, 3.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_regime_engine::MarketFeatureSet;

    #[test]
    fn manipulation_lowers_volume_thresholds() {
        let manager = RegimeThresholdManager::new(&RegimeThresholds::default());
        let output = analyze_market_regime(&MarketFeatureSet {
            symbol: "BTC".to_string(),
            price_change_5m_pct: Some(0.08),
            oi_change_pct: Some(-0.7),
            volume_spike_multiple: Some(4.6),
            funding_rate: Some(0.0008),
            spot_futures_divergence_pct: Some(0.35),
            liquidation_ratio: Some(0.05),
            price_impact_efficiency: Some(0.02),
            data_quality: Some(1.0),
            ..MarketFeatureSet::default()
        });
        manager.update_from_engine(&output);
        assert_eq!(manager.current().regime, MarketRegime::Manipulation);
        assert!(manager.factor("toxic_volume_factor") < 0.85);

        let base = ToxicVolumeParams::default();
        let adjusted = manager.adjusted_toxic_volume_params(&base);
        assert!(adjusted.threshold_btc < base.threshold_btc);
    }

    #[test]
    fn liquidation_raises_thresholds_and_sets_confidence_gate() {
        let manager = RegimeThresholdManager::new(&RegimeThresholds::default());
        let output = analyze_market_regime(&MarketFeatureSet {
            symbol: "BTC".to_string(),
            price_change_5m_pct: Some(-1.2),
            oi_change_pct: Some(-1.8),
            volume_spike_multiple: Some(3.2),
            funding_rate: Some(-0.001),
            liquidation_ratio: Some(0.42),
            data_quality: Some(0.9),
            ..MarketFeatureSet::default()
        });
        manager.update_from_engine(&output);
        assert_eq!(manager.current().regime, MarketRegime::Liquidation);
        assert!(manager.factor("toxic_volume_factor") > 1.1);
        assert!(manager.min_confidence_gate() >= 0.50);
    }

    #[test]
    fn disabled_manager_keeps_identity_factors() {
        let mut config = RegimeThresholds::default();
        config.enabled = false;
        let manager = RegimeThresholdManager::new(&config);
        let base = SweepParams::default();
        let adjusted = manager.adjusted_sweep_params(&base);
        assert!((adjusted.min_swept_volume_btc - base.min_swept_volume_btc).abs() < f64::EPSILON);
    }
}
