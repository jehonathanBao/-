use std::{fs, path::Path};

#[test]
fn toxic_signal_operator_runbook_exists_and_states_safety_boundary() {
    let path = Path::new("docs/toxic-signal-operator-runbook.md");
    assert!(path.exists(), "runbook should exist at {:?}", path);

    let markdown = fs::read_to_string(path).expect("read runbook");
    assert!(markdown.contains("# Toxic Signal Operator Runbook"));
    assert!(markdown.contains("This system is signal-only."));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
    assert!(markdown.contains("notificationSent=false"));
    assert!(markdown.contains("executionTriggered=false"));
    assert!(markdown.contains("retentionMode=in_memory_bounded"));
    assert!(markdown.contains("databaseWriteEnabled=false"));
    assert!(markdown.contains("## 4. S1 — Signal Inbox"));
    assert!(markdown.contains("## 10. S7 — Signal History"));
    assert!(markdown.contains("## 11. Common Status Meanings"));
    assert!(markdown.contains("## 12. Recommended Operator Workflow"));
    assert!(markdown.contains("## 13. What Not To Do"));
}

#[test]
fn durable_signal_archive_design_exists_and_stays_design_only() {
    let path = Path::new("docs/durable-signal-archive-design.md");
    assert!(path.exists(), "archive design should exist at {:?}", path);

    let markdown = fs::read_to_string(path).expect("read archive design");
    assert!(markdown.contains("# Durable Signal Archive Design / Write-audit Plan"));
    assert!(markdown.contains("This document is a design plan only."));
    assert!(markdown.contains("retentionMode=in_memory_bounded"));
    assert!(markdown.contains("durableStorageEnabled=false"));
    assert!(markdown.contains("databaseWriteEnabled=false"));
    assert!(markdown.contains("executionEnabled=false"));
    assert!(markdown.contains("notificationSent=false"));
    assert!(markdown.contains("executionTriggered=false"));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
    assert!(markdown.contains("No auto weight update"));
    assert!(markdown.contains("no DB implementation"));
    assert!(markdown.contains("no JSONL writer"));
    assert!(markdown.contains("no SQLite writer"));
    assert!(markdown.contains("no API behavior change"));
    assert!(markdown.contains("no dashboard behavior change"));
    assert!(markdown
        .to_ascii_lowercase()
        .contains("archive failure must never affect realtime signal generation"));
}

#[test]
fn durable_signal_archive_readiness_checklist_exists_and_blocks_mvp_until_schema_contract() {
    let path = Path::new("docs/durable-signal-archive-readiness-checklist.md");
    assert!(
        path.exists(),
        "archive readiness checklist should exist at {:?}",
        path
    );

    let markdown = fs::read_to_string(path).expect("read archive readiness checklist");
    assert!(markdown.contains("# Durable Signal Archive MVP Readiness Checklist"));
    assert!(markdown.contains("ready_for_archive_mvp = false"));
    assert!(markdown.contains("blocked_reasons = ["));
    assert!(markdown
        .contains("recommended_next_card = \"S15A - Durable Archive Schema Contract Draft\""));
    assert!(markdown.contains("archiveWriteEnabled=false"));
    assert!(markdown.contains("durableStorageEnabled=false"));
    assert!(markdown.contains("databaseWriteEnabled=false"));
    assert!(markdown.contains("executionEnabled=false"));
    assert!(markdown.contains("notificationSent=false"));
    assert!(markdown.contains("executionTriggered=false"));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
    assert!(markdown.contains("No auto weight update"));
    assert!(markdown.contains("no actual DB by default"));
    assert!(markdown.contains("archive failure must not affect S7 in-memory history"));
    assert!(markdown.contains("S15 does not add these config fields"));
}

#[test]
fn durable_signal_archive_schema_contract_exists_and_keeps_archive_write_disabled() {
    let path = Path::new("docs/durable-signal-archive-schema-contract.md");
    assert!(
        path.exists(),
        "archive schema contract should exist at {:?}",
        path
    );

    let markdown = fs::read_to_string(path).expect("read archive schema contract");
    assert!(markdown.contains("# Durable Signal Archive Schema Contract"));
    assert!(markdown.contains("schemaVersion"));
    assert!(markdown.contains("archiveWriteEnabled=false"));
    assert!(markdown.contains("durableStorageEnabled=false"));
    assert!(markdown.contains("databaseWriteEnabled=false"));
    assert!(markdown.contains("jsonlWriteEnabled=false"));
    assert!(markdown.contains("sqliteWriteEnabled=false"));
    assert!(markdown.contains("executionEnabled=false"));
    assert!(markdown.contains("notificationSent=false"));
    assert!(markdown.contains("executionTriggered=false"));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
    assert!(markdown.contains("ready_for_archive_mvp=false"));
    assert!(markdown.contains("source snapshots describe what the operator saw"));
    assert!(markdown.contains("The next recommended card remains:"));
    assert!(markdown.contains("S15B - Durable Archive Write Plan / Dry-run Writer Contract"));
}

#[test]
fn durable_signal_archive_dryrun_review_pack_doc_exists_and_keeps_review_pack_read_only() {
    let path = Path::new("docs/durable_archive_dryrun_review_pack.md");
    assert!(
        path.exists(),
        "dry-run review pack doc should exist at {:?}",
        path
    );

    let markdown = fs::read_to_string(path).expect("read dry-run review pack doc");
    assert!(markdown.contains("# Durable Archive Dry-run Review Pack"));
    assert!(markdown.contains("readOnly=true"));
    assert!(markdown.contains("analysisOnly=true"));
    assert!(markdown.contains("manualReviewRequired=true"));
    assert!(markdown.contains("archiveWriteEnabled=false"));
    assert!(markdown.contains("durableStorageEnabled=false"));
    assert!(markdown.contains("databaseWriteEnabled=false"));
    assert!(markdown.contains("jsonlWriteEnabled=false"));
    assert!(markdown.contains("sqliteWriteEnabled=false"));
    assert!(markdown.contains("executionEnabled=false"));
    assert!(markdown.contains("notificationSent=false"));
    assert!(markdown.contains("executionTriggered=false"));
    assert!(markdown.contains("Validation Errors"));
    assert!(markdown.contains("Validation Warnings"));
    assert!(markdown.contains("Unsafe Fields"));
    assert!(markdown.contains("Field Contract"));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
}

#[test]
fn durable_archive_write_disabled_by_default_doc_exists_and_keeps_write_gate_closed() {
    let path = Path::new("docs/durable_archive_write_disabled_by_default.md");
    assert!(path.exists(), "write gate doc should exist at {:?}", path);

    let markdown = fs::read_to_string(path).expect("read write gate doc");
    assert!(markdown.contains("# Durable Archive Write Disabled-by-default"));
    assert!(markdown.contains("This is a disabled-by-default write gate."));
    assert!(markdown.contains("It does not enable durable archive writes."));
    assert!(markdown.contains("It does not write DB, JSONL, SQLite, or files."));
    assert!(markdown.contains("archiveWriteEnabled=false"));
    assert!(markdown.contains("databaseWriteEnabled=false"));
    assert!(markdown.contains("jsonlWriteEnabled=false"));
    assert!(markdown.contains("sqliteWriteEnabled=false"));
    assert!(markdown.contains("fileArchiveWriteEnabled=false"));
    assert!(markdown.contains("recordsWritten=0"));
    assert!(markdown.contains("bytesWritten=0"));
    assert!(markdown.contains("writeRejected=true"));
    assert!(markdown.contains("archive_write_disabled_by_default"));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
    assert!(markdown.contains("POST /api/archive/dry-run/write"));
    assert!(markdown.contains("GET /api/archive/dry-run/review-pack/latest"));
}

#[test]
fn whale_flow_operator_runbook_exists_and_keeps_signal_only_boundary() {
    let path = Path::new("docs/whale-flow-operator-runbook.md");
    assert!(
        path.exists(),
        "whale flow runbook should exist at {:?}",
        path
    );

    let markdown = fs::read_to_string(path).expect("read whale flow runbook");
    assert!(markdown.contains("# Whale Flow Operator Runbook"));
    assert!(markdown.contains("Whale Flow is a signal-only monitoring layer."));
    assert!(markdown.contains("It does not place orders."));
    assert!(markdown.contains("It does not cancel or amend orders."));
    assert!(markdown.contains("It does not manage wallets or signing."));
    assert!(markdown.contains("It does not trigger live trading."));
    assert!(markdown.contains("It does not auto-apply threshold changes."));
    assert!(markdown.contains("1s  >= 100 BTC"));
    assert!(markdown.contains("5s  >= 300 BTC"));
    assert!(markdown.contains("15s >= 800 BTC"));
    assert!(markdown.contains("60s >= 2000 BTC"));
    assert!(markdown.contains("directionRatio >= 70%"));
    assert!(markdown.contains("relativeVolumeMultiple >= 5x"));
    assert!(markdown.contains("minVenueConfirmations >= 2"));
    assert!(markdown.contains("aggressive_buy"));
    assert!(markdown.contains("aggressive_sell"));
    assert!(markdown.contains("absorption"));
    assert!(markdown.contains("liquidation_sweep"));
    assert!(markdown.contains("trap"));
    assert!(markdown.contains("candidate count is not evidence count"));
    assert!(markdown.contains("resolved markout evidence is required"));
    assert!(markdown.contains("needs_more_data"));
    assert!(markdown.contains("retentionMode=in_memory_bounded"));
    assert!(markdown.contains("currentCandidates"));
    assert!(markdown.contains("recordedCount"));
    assert!(markdown.contains("calibrationReady"));
    assert!(markdown.contains("`READY` does not mean auto-apply"));
    assert!(markdown.contains("`NOT READY` means collect more evidence"));
    assert!(markdown.contains("`not_enough_data` must not be treated as `aligned`"));
    assert!(markdown.contains("Do not trade directly from whale flow."));
    assert!(markdown.contains("Do not treat `directionBias` as an order instruction."));
    assert!(markdown.contains("Do not send alerts, webhooks, or Telegram from this layer."));
}

#[test]
fn whale_flow_overnight_soak_readiness_doc_exists_and_stays_signal_only() {
    let path = Path::new("docs/whale-flow-overnight-soak-readiness.md");
    assert!(
        path.exists(),
        "whale flow overnight soak doc should exist at {:?}",
        path
    );

    let markdown = fs::read_to_string(path).expect("read whale flow overnight soak doc");
    assert!(markdown.contains("# Whale Flow Overnight Soak Readiness Checklist"));
    assert!(markdown.contains("This checklist is for local overnight signal-only data collection."));
    assert!(markdown.contains("It does not enable trading."));
    assert!(markdown.contains("It does not enable order placement."));
    assert!(markdown.contains("It does not enable wallet/signing."));
    assert!(markdown.contains("It does not enable notification sending."));
    assert!(markdown.contains("It does not enable durable archive writes."));
    assert!(markdown.contains("`readOnly=true`"));
    assert!(markdown.contains("`analysisOnly=true`"));
    assert!(markdown.contains("`executionEnabled=false`"));
    assert!(markdown.contains("`runtimeModified=false`"));
    assert!(markdown.contains("`notificationSent=false`"));
    assert!(markdown.contains("`executionTriggered=false`"));
    assert!(markdown.contains("`archiveWriteEnabled=false`"));
    assert!(markdown.contains("`databaseWriteEnabled=false`"));
    assert!(markdown.contains("`jsonlWriteEnabled=false`"));
    assert!(markdown.contains("`sqliteWriteEnabled=false`"));
    assert!(markdown.contains("`monitoringStarted=false`"));
    assert!(
        markdown.contains("`monitoringStarted=true` does not guarantee venue streams are active")
    );
    assert!(markdown.contains("No API keys"));
    assert!(markdown.contains("`retentionMode=in_memory_bounded`"));
    assert!(markdown.contains("`durableStorageEnabled=false`"));
    assert!(markdown.contains("`databaseWriteEnabled=false`"));
    assert!(markdown.contains("candidate count is not evidence count"));
    assert!(markdown.contains("resolved markout evidence is required"));
    assert!(markdown.contains("`READY` does not mean auto-apply"));
    assert!(markdown.contains("`NOT READY` means collect more evidence"));
    assert!(markdown.contains("No order placement"));
    assert!(markdown.contains("No wallet/signing"));
    assert!(markdown.contains("No live trading"));
    assert!(markdown.contains(
        "Do not enable archive writes during a soak unless a separate approved task exists."
    ));
}
