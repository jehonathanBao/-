use crate::{
    toxicity::orderbook_wall_interpretation::analyze_orderbook_wall_interpretation,
    types::{
        liquidation::LiquidationToxicityRecentResponse,
        orderbook_wall::{
            OrderbookWallInterpretationReport, OrderbookWallInterpretationStatusResponse,
            OrderbookWallLifecycleReport,
        },
        toxic_flow::ActiveTradeToxicityRecentResponse,
    },
};

pub fn build_orderbook_wall_interpretation_recent(
    requested_symbol: &str,
    lifecycle_report: &OrderbookWallLifecycleReport,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
) -> OrderbookWallInterpretationReport {
    analyze_orderbook_wall_interpretation(
        requested_symbol,
        lifecycle_report,
        active_trade_recent,
        liquidation_recent,
    )
}

pub fn build_orderbook_wall_interpretation_status(
    requested_symbol: &str,
    lifecycle_report: &OrderbookWallLifecycleReport,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
) -> OrderbookWallInterpretationStatusResponse {
    let recent = build_orderbook_wall_interpretation_recent(
        requested_symbol,
        lifecycle_report,
        active_trade_recent,
        liquidation_recent,
    );
    OrderbookWallInterpretationStatusResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        enabled: true,
        signal_count: recent.signals.len(),
        last_signal_at_ms: recent.signals.iter().map(|signal| signal.ts_ms).max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No cancel/amend".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }
}
