use crate::{
    toxicity::toxic_signal_alert_preview::{
        build_toxic_signal_alert_explain, build_toxic_signal_alert_preview,
        build_toxic_signal_alert_preview_status,
    },
    types::{
        toxic_signal_alert_preview::{
            ToxicSignalAlertPreviewExplainResponse, ToxicSignalAlertPreviewGate,
            ToxicSignalAlertPreviewResponse, ToxicSignalAlertPreviewStatusResponse,
        },
        toxic_signal_inbox::ToxicSignalInboxRecentResponse,
    },
};

pub fn toxic_signal_alert_preview(
    requested_symbol: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    gate: ToxicSignalAlertPreviewGate,
) -> ToxicSignalAlertPreviewResponse {
    build_toxic_signal_alert_preview(requested_symbol, inbox_recent, gate)
}

pub fn toxic_signal_alert_preview_status(
    preview: &ToxicSignalAlertPreviewResponse,
) -> ToxicSignalAlertPreviewStatusResponse {
    build_toxic_signal_alert_preview_status(preview)
}

pub fn toxic_signal_alert_explain(
    signal_id: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    gate: &ToxicSignalAlertPreviewGate,
) -> ToxicSignalAlertPreviewExplainResponse {
    build_toxic_signal_alert_explain(signal_id, inbox_recent, gate)
}
