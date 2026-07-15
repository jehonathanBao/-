use serde::Serialize;

use crate::runtime::{
    cwm_risk_fusion::{
        CwmRiskContribution, MainForceStructureRisk, ShortTermToxicRisk, SplitRiskSystems,
    },
    tof_metrics::TofMetrics,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitRiskSystemsTransport {
    pub short_term_toxic: ShortTermToxicRisk,
    pub market_structure_score: Option<MainForceStructureRisk>,
    pub main_force_structure: Option<MainForceStructureRisk>,
}

impl SplitRiskSystemsTransport {
    pub fn from_core(systems: &SplitRiskSystems, market_structure_available: bool) -> Self {
        let main_force_structure =
            market_structure_available.then(|| systems.main_force_structure.clone());
        Self {
            short_term_toxic: systems.short_term_toxic.clone(),
            market_structure_score: main_force_structure.clone(),
            main_force_structure,
        }
    }
}

pub fn market_structure_evidence_available(
    tof_metrics: &TofMetrics,
    cwm_contribution: &CwmRiskContribution,
) -> bool {
    tof_metrics.lineage.alert_eligible && cwm_contribution.available && cwm_contribution.fresh
}

#[cfg(test)]
mod tests {
    use super::{market_structure_evidence_available, SplitRiskSystemsTransport};
    use crate::runtime::{
        cwm_risk_fusion::{build_split_risk_systems, CwmRiskContribution, SplitRiskSystemsInput},
        metric_provenance::MetricLineage,
        tof_metrics::{enhance_signal_summary, TofDirection, TofMetrics, TofSummaryInput},
    };

    #[test]
    fn transport_requires_bilateral_tof_and_fresh_cwm_evidence() {
        for (name, tof_available, cwm_available, expected) in [
            ("neither", false, false, false),
            ("tof_only", true, false, false),
            ("cwm_only", false, true, false),
            ("both", true, true, true),
        ] {
            let tof = tof_metrics(tof_available);
            let cwm = cwm_contribution(cwm_available);
            let available = market_structure_evidence_available(&tof, &cwm);
            assert_eq!(available, expected, "availability for {name}");

            let systems = build_split_risk_systems(SplitRiskSystemsInput {
                ts_ms: 1_700_000_000_000,
                symbol: "BTC-PERP",
                short_toxic_score: 82,
                short_tof_score: 72.0,
                short_direction: TofDirection::Bearish,
                toxic_type: "spoofing_candidate",
                data_quality: 82.0,
                detector_confidence: 82.0,
                direction_confidence: 82.0,
                direction_source: "detector",
                tof_metrics: &tof,
                advanced_score: None,
                perp_score: None,
                metrics_direction: TofDirection::Bearish,
                cwm_contribution: cwm,
            });
            let transport = SplitRiskSystemsTransport::from_core(&systems, available);
            assert_eq!(
                transport.market_structure_score.is_some(),
                expected,
                "marketStructureScore for {name}"
            );
            assert_eq!(
                transport.main_force_structure.is_some(),
                expected,
                "mainForceStructure for {name}"
            );
        }
    }

    fn tof_metrics(available: bool) -> TofMetrics {
        let mut metrics = enhance_signal_summary(&TofSummaryInput {
            signal_kind: "spoofing_candidate",
            direction_bias: "short_bias",
            severity: "high",
            confidence: 0.82,
            quality_bucket: "good",
            summary: "large ask wall removed",
            existing_risk_score: 82,
            existing_data_quality: 82.0,
        })
        .tof_metrics;
        if available {
            metrics.lineage = MetricLineage::calculated("test_tof", 1_700_000_000_000, true);
        }
        metrics
    }

    fn cwm_contribution(available: bool) -> CwmRiskContribution {
        let mut contribution = CwmRiskContribution::unavailable("BTC-PERP");
        if available {
            contribution.available = true;
            contribution.fresh = true;
            contribution.observed_at_ms = Some(1_700_000_000_000);
            contribution.score = Some(90);
            contribution.data_quality = Some(90);
        }
        contribution
    }
}
