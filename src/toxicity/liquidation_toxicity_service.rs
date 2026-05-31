use crate::{
    toxicity::{
        active_trade_toxicity_service::build_active_trade_toxicity_recent,
        liquidation_toxicity::{
            analyze_liquidation_toxicity, build_liquidation_toxicity_recent_response,
        },
    },
    types::{
        flow::FlowState,
        liquidation::{
            LiquidationState, LiquidationToxicityRecentResponse, LiquidationToxicityStatusResponse,
        },
        markout::MarkoutState,
        sweep::SweepState,
    },
};

pub fn build_liquidation_toxicity_recent(
    requested_symbol: &str,
    liquidation_state: &LiquidationState,
    flow_state: &FlowState,
    sweep_state: &SweepState,
    markout_state: &MarkoutState,
) -> LiquidationToxicityRecentResponse {
    let active_trade_recent = build_active_trade_toxicity_recent(
        requested_symbol,
        flow_state,
        sweep_state,
        markout_state,
    );
    let assessment =
        analyze_liquidation_toxicity(requested_symbol, liquidation_state, &active_trade_recent);
    build_liquidation_toxicity_recent_response(requested_symbol, assessment)
}

pub fn build_liquidation_toxicity_status(
    requested_symbol: &str,
    liquidation_state: &LiquidationState,
    flow_state: &FlowState,
    sweep_state: &SweepState,
    markout_state: &MarkoutState,
) -> LiquidationToxicityStatusResponse {
    let recent = build_liquidation_toxicity_recent(
        requested_symbol,
        liquidation_state,
        flow_state,
        sweep_state,
        markout_state,
    );
    LiquidationToxicityStatusResponse {
        read_only: true,
        runtime_modified: false,
        enabled: liquidation_state.metrics.enabled,
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
