use std::path::PathBuf;

use crate::{
    toxicity::toxic_governance_ledger::{
        build_toxic_governance_ledger_export, build_toxic_governance_ledger_status,
        build_toxic_governance_ledger_summary, load_toxic_governance_decisions,
    },
    types::toxic_governance_ledger::{
        ToxicGovernanceLedgerExportResponse, ToxicGovernanceLedgerStatusResponse,
        ToxicGovernanceLedgerSummaryResponse,
    },
};

const DEFAULT_GOVERNANCE_LEDGER_PATH: &str = "data/governance/toxic_governance_ledger.jsonl";

pub fn toxic_governance_ledger_summary(
    selected_symbol: Option<&str>,
) -> ToxicGovernanceLedgerSummaryResponse {
    let path = PathBuf::from(DEFAULT_GOVERNANCE_LEDGER_PATH);
    match load_toxic_governance_decisions(&path) {
        Ok(decisions) => build_toxic_governance_ledger_summary(
            selected_symbol.unwrap_or("ALL"),
            &decisions,
            if path.exists() {
                Vec::new()
            } else {
                vec!["governance_ledger_file_missing".to_string()]
            },
        ),
        Err(error) => build_toxic_governance_ledger_summary(
            selected_symbol.unwrap_or("ALL"),
            &[],
            vec![format!("governance_ledger_load_error: {error}")],
        ),
    }
}

pub fn toxic_governance_ledger_status(
    selected_symbol: Option<&str>,
) -> ToxicGovernanceLedgerStatusResponse {
    let summary = toxic_governance_ledger_summary(selected_symbol);
    build_toxic_governance_ledger_status(&summary)
}

pub fn toxic_governance_ledger_export(
    selected_symbol: Option<&str>,
) -> ToxicGovernanceLedgerExportResponse {
    let summary = toxic_governance_ledger_summary(selected_symbol);
    build_toxic_governance_ledger_export(&summary)
}
