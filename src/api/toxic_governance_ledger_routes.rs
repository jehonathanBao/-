use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    app::AppState,
    toxicity::toxic_governance_ledger_service::{
        toxic_governance_ledger_export, toxic_governance_ledger_status,
        toxic_governance_ledger_summary,
    },
};

use self::SymbolSelection::{All, Exact};

enum SymbolSelection<'a> {
    All,
    Exact(&'a str),
}

pub async fn toxic_governance_ledger_status_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(toxic_governance_ledger_status(
        selection_symbol(All)
    )))
}

pub async fn toxic_governance_ledger_summary_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(toxic_governance_ledger_summary(
        selection_symbol(All)
    )))
}

pub async fn toxic_governance_ledger_recent_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(toxic_governance_ledger_summary(
        selection_symbol(All)
    )))
}

pub async fn toxic_governance_ledger_for_symbol(
    State(_state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(toxic_governance_ledger_summary(
        selection_symbol(Exact(&symbol),)
    )))
}

pub async fn toxic_governance_ledger_export_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(toxic_governance_ledger_export(
        selection_symbol(All)
    )))
}

fn selection_symbol(selection: SymbolSelection<'_>) -> Option<&str> {
    match selection {
        All => None,
        SymbolSelection::Exact(symbol) => Some(symbol),
    }
}
