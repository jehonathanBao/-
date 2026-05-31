use crate::{
    toxicity::toxic_replay::{
        build_toxic_replay_by_signal_id, build_toxic_replay_latest, build_toxic_replay_recent,
        build_toxic_replay_status,
    },
    types::{
        liquidation::LiquidationToxicityRecentResponse,
        orderbook_wall::{OrderbookWallInterpretationReport, OrderbookWallLifecycleReport},
        structural_toxicity::StructuralToxicityRecentResponse,
        toxic_flow::ActiveTradeToxicityRecentResponse,
        toxic_replay::{
            ToxicReplayDetailResponse, ToxicReplayRecentResponse, ToxicReplayStatusResponse,
        },
        toxic_signal::ToxicSignalRecentResponse,
    },
};

pub fn replay_recent(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
) -> ToxicReplayRecentResponse {
    build_toxic_replay_recent(requested_symbol, fusion_recent)
}

pub fn replay_status(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
) -> ToxicReplayStatusResponse {
    build_toxic_replay_status(requested_symbol, fusion_recent)
}

#[allow(clippy::too_many_arguments)]
pub fn replay_latest(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicReplayDetailResponse {
    build_toxic_replay_latest(
        requested_symbol,
        fusion_recent,
        active_trade_recent,
        liquidation_recent,
        wall_lifecycle_report,
        wall_interpretation_report,
        structural_recent,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn replay_by_signal_id(
    requested_symbol: &str,
    signal_id: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicReplayDetailResponse {
    build_toxic_replay_by_signal_id(
        requested_symbol,
        signal_id,
        fusion_recent,
        active_trade_recent,
        liquidation_recent,
        wall_lifecycle_report,
        wall_interpretation_report,
        structural_recent,
    )
}
