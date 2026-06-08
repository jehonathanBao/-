use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{app::AppState, spot_whale_monitor::service::SpotWhaleQuery};

type ApiJsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Deserialize, Default)]
pub struct SpotWhaleApiQuery {
    pub symbol: Option<String>,
    pub limit: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub discord_sent: Option<String>,
}

pub async fn spot_whale_summary_route(
    State(state): State<AppState>,
    Query(query): Query<SpotWhaleApiQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol(query.symbol.as_deref())?;
    Ok(Json(serde_json::json!(state
        .spot_whale_service()
        .summary(&symbol))))
}

pub async fn spot_whale_latest_route(
    State(state): State<AppState>,
    Query(query): Query<SpotWhaleApiQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref())?;
    Ok(Json(serde_json::json!(state
        .spot_whale_service()
        .latest(&symbol, limit))))
}

pub async fn spot_whale_history_route(
    State(state): State<AppState>,
    Query(query): Query<SpotWhaleApiQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref())?;
    let discord_sent = match query.discord_sent.as_deref() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        Some("all") | None => None,
        Some(_) => {
            return Err(bad_request(
                "invalid_discord_sent",
                "discord_sent must be true, false, or all",
            ));
        }
    };
    Ok(Json(serde_json::json!(state.spot_whale_service().history(
        SpotWhaleQuery {
            symbol: Some(symbol),
            severity: query
                .severity
                .filter(|value| !value.eq_ignore_ascii_case("all")),
            signal_type: query
                .signal_type
                .filter(|value| !value.eq_ignore_ascii_case("all")),
            discord_sent,
            limit: Some(limit),
        }
    ))))
}

fn parse_symbol(value: Option<&str>) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let symbol = value.unwrap_or("BTC").trim().to_ascii_uppercase();
    match symbol.as_str() {
        "BTC" | "ETH" => Ok(symbol),
        _ => Err(bad_request(
            "invalid_symbol",
            "symbol must be BTC or ETH for spot whale monitor",
        )),
    }
}

fn parse_limit(value: Option<&str>) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    match value {
        Some(raw) => raw
            .parse::<usize>()
            .map(|limit| limit.clamp(1, 200))
            .map_err(|_| bad_request("invalid_limit", "limit must be a positive integer")),
        None => Ok(50),
    }
}

fn bad_request(reason: &str, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "ok": false,
            "reason": reason,
            "message": message,
        })),
    )
}
