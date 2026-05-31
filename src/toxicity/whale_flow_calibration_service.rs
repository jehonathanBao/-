use crate::{
    toxicity::whale_flow_calibration::{
        build_whale_flow_calibration_report, build_whale_flow_calibration_status,
    },
    types::{
        toxic_markout::ToxicMarkoutRecentResponse,
        toxic_signal_history::ToxicSignalHistoryStatusResponse,
        whale_flow_calibration::{
            WhaleFlowCalibrationReportResponse, WhaleFlowCalibrationStatusResponse,
        },
        whale_flow_signal::WhaleFlowRecentResponse,
    },
};

pub fn build_whale_flow_threshold_calibration_report(
    selected_symbol: &str,
    whale_flow: &WhaleFlowRecentResponse,
    markout: &ToxicMarkoutRecentResponse,
    history_status: &ToxicSignalHistoryStatusResponse,
) -> WhaleFlowCalibrationReportResponse {
    build_whale_flow_calibration_report(selected_symbol, whale_flow, markout, history_status)
}

pub fn build_whale_flow_threshold_calibration_status(
    report: &WhaleFlowCalibrationReportResponse,
) -> WhaleFlowCalibrationStatusResponse {
    build_whale_flow_calibration_status(report)
}
