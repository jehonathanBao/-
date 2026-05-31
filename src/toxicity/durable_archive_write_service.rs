use crate::{
    toxicity::durable_archive_write::{
        build_durable_archive_write_status, empty_durable_archive_write_request,
        reject_durable_archive_write,
    },
    types::durable_archive_write::{
        DurableArchiveWriteRejectedResponse, DurableArchiveWriteRequest,
        DurableArchiveWriteStatusResponse,
    },
};

pub fn durable_archive_write_status() -> DurableArchiveWriteStatusResponse {
    build_durable_archive_write_status()
}

pub fn durable_archive_write_reject(
    request_contract: Option<DurableArchiveWriteRequest>,
) -> DurableArchiveWriteRejectedResponse {
    reject_durable_archive_write(
        request_contract.unwrap_or_else(empty_durable_archive_write_request),
    )
}
