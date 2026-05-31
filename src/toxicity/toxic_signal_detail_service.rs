use crate::{
    toxicity::toxic_signal_detail::{
        build_toxic_signal_detail, build_toxic_signal_detail_status,
        build_toxic_signal_group_detail, ToxicSignalDetailContext,
    },
    types::toxic_signal_detail::{
        ToxicSignalDetailGroupResponse, ToxicSignalDetailResponse, ToxicSignalDetailStatusResponse,
    },
};

pub fn toxic_signal_detail_status(
    requested_symbol: &str,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailStatusResponse {
    build_toxic_signal_detail_status(requested_symbol, context)
}

pub fn toxic_signal_detail_by_signal_id(
    requested_symbol: &str,
    signal_id: &str,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailResponse {
    build_toxic_signal_detail(requested_symbol, signal_id, context)
}

pub fn toxic_signal_detail_by_group_id(
    requested_symbol: &str,
    group_id: &str,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailGroupResponse {
    build_toxic_signal_group_detail(requested_symbol, group_id, context)
}
