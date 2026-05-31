use crate::{
    toxicity::structural_toxicity::analyze_structural_toxicity,
    types::{
        liquidation::LiquidationToxicityRecentResponse,
        orderbook_wall::{OrderbookWallInterpretationReport, OrderbookWallLifecycleReport},
        structural_toxicity::{StructuralToxicityRecentResponse, StructuralToxicityStatusResponse},
        toxic_flow::ActiveTradeToxicityRecentResponse,
    },
};

pub fn build_structural_toxicity_recent(
    requested_symbol: &str,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
) -> StructuralToxicityRecentResponse {
    analyze_structural_toxicity(
        requested_symbol,
        active_trade_recent,
        liquidation_recent,
        wall_lifecycle_report,
        wall_interpretation_report,
    )
}

pub fn build_structural_toxicity_status(
    requested_symbol: &str,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
) -> StructuralToxicityStatusResponse {
    let recent = build_structural_toxicity_recent(
        requested_symbol,
        active_trade_recent,
        liquidation_recent,
        wall_lifecycle_report,
        wall_interpretation_report,
    );
    StructuralToxicityStatusResponse {
        read_only: true,
        runtime_modified: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        signal_count: recent.signals.len(),
        last_signal_at_ms: recent.signals.iter().map(|signal| signal.ts_ms).max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }
}
