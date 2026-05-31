use crate::{
    toxicity::durable_archive_write_audit::{
        build_durable_archive_write_audit_latest, build_durable_archive_write_audit_recent,
        build_durable_archive_write_audit_status,
    },
    types::durable_archive_write_audit::{
        DurableArchiveWriteAuditLatestResponse, DurableArchiveWriteAuditRecentResponse,
        DurableArchiveWriteAuditStatusResponse,
    },
};

pub fn durable_archive_write_audit_status() -> DurableArchiveWriteAuditStatusResponse {
    build_durable_archive_write_audit_status()
}

pub fn durable_archive_write_audit_recent() -> DurableArchiveWriteAuditRecentResponse {
    build_durable_archive_write_audit_recent()
}

pub fn durable_archive_write_audit_latest() -> DurableArchiveWriteAuditLatestResponse {
    build_durable_archive_write_audit_latest()
}
