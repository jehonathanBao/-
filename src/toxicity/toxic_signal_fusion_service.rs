use crate::{
    toxicity::toxic_signal_fusion::analyze_toxic_signal_fusion,
    types::{
        liquidation::LiquidationToxicityRecentResponse,
        orderbook_wall::{OrderbookWallInterpretationReport, OrderbookWallLifecycleReport},
        structural_toxicity::StructuralToxicityRecentResponse,
        toxic_flow::ActiveTradeToxicityRecentResponse,
        toxic_signal::{ToxicSignalRecentResponse, ToxicSignalStatusResponse},
    },
};

pub fn build_toxic_signal_fusion_recent(
    requested_symbol: &str,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicSignalRecentResponse {
    analyze_toxic_signal_fusion(
        requested_symbol,
        active_trade_recent,
        liquidation_recent,
        wall_lifecycle_report,
        wall_interpretation_report,
        structural_recent,
    )
}

pub fn build_toxic_signal_fusion_status(
    requested_symbol: &str,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicSignalStatusResponse {
    let recent = build_toxic_signal_fusion_recent(
        requested_symbol,
        active_trade_recent,
        liquidation_recent,
        wall_lifecycle_report,
        wall_interpretation_report,
        structural_recent,
    );
    ToxicSignalStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        signal_count: recent.signals.len(),
        last_signal_at_ms: recent.signals.iter().map(|signal| signal.ts_ms).max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No cancel/amend".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }
}
