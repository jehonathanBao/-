use crate::{
    toxicity::toxic_signal_group::{
        build_toxic_signal_group_detail, build_toxic_signal_group_recent,
        build_toxic_signal_group_status,
    },
    types::{
        toxic_signal_group::{
            ToxicSignalGroupDetailResponse, ToxicSignalGroupRecentResponse,
            ToxicSignalGroupStatusResponse,
        },
        toxic_signal_inbox::ToxicSignalInboxRecentResponse,
    },
};

pub fn toxic_signal_group_recent(
    requested_symbol: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalGroupRecentResponse {
    build_toxic_signal_group_recent(requested_symbol, inbox_recent)
}

pub fn toxic_signal_group_status(
    recent: &ToxicSignalGroupRecentResponse,
) -> ToxicSignalGroupStatusResponse {
    build_toxic_signal_group_status(recent)
}

pub fn toxic_signal_group_detail(
    requested_symbol: &str,
    group_id: &str,
    recent: &ToxicSignalGroupRecentResponse,
) -> ToxicSignalGroupDetailResponse {
    build_toxic_signal_group_detail(requested_symbol, group_id, recent)
}
