use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{app::AppState, binance_alt_contract_monitor::service::BinanceAltContractQuery};

type ApiJsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Deserialize, Default)]
pub struct BinanceAltContractApiQuery {
    pub symbol: Option<String>,
    pub limit: Option<String>,
    pub severity: Option<String>,
    #[serde(rename = "type")]
    pub signal_type_alias: Option<String>,
    pub signal_type: Option<String>,
    pub direction: Option<String>,
    pub would_send: Option<String>,
    pub liquidation: Option<String>,
    #[serde(rename = "liquidationDriven")]
    pub liquidation_driven: Option<String>,
    pub tier: Option<String>,
    pub min_build_score: Option<String>,
}

pub async fn binance_alt_contract_summary_route(
    State(state): State<AppState>,
    Query(query): Query<BinanceAltContractApiQuery>,
) -> ApiJsonResult {
    let symbol = parse_optional_symbol(query.symbol.as_deref())?;
    Ok(Json(serde_json::json!(state
        .binance_alt_contract_service()
        .summary(symbol.as_deref()))))
}

pub async fn binance_alt_contract_latest_route(
    State(state): State<AppState>,
    Query(query): Query<BinanceAltContractApiQuery>,
) -> ApiJsonResult {
    let symbol = parse_optional_symbol(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref())?;
    Ok(Json(serde_json::json!(state
        .binance_alt_contract_service()
        .latest(symbol.as_deref(), limit))))
}

pub async fn binance_alt_contract_history_route(
    State(state): State<AppState>,
    Query(query): Query<BinanceAltContractApiQuery>,
) -> ApiJsonResult {
    let symbol = parse_optional_symbol(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref())?;
    let signal_type = query
        .signal_type
        .or(query.signal_type_alias)
        .filter(|value| !value.eq_ignore_ascii_case("all"));
    let would_send = parse_optional_bool(query.would_send.as_deref())?;
    let liquidation_filter = query.liquidation_driven.or(query.liquidation);
    let liquidation = parse_optional_bool(liquidation_filter.as_deref())?;
    let min_build_score = parse_optional_u8(query.min_build_score.as_deref(), "min_build_score")?;
    Ok(Json(serde_json::json!(state
        .binance_alt_contract_service()
        .history(BinanceAltContractQuery {
            symbol,
            severity: query
                .severity
                .filter(|value| !value.eq_ignore_ascii_case("all")),
            signal_type,
            direction: query
                .direction
                .filter(|value| !value.eq_ignore_ascii_case("all")),
            would_send,
            liquidation,
            tier: query
                .tier
                .filter(|value| !value.eq_ignore_ascii_case("all")),
            min_build_score,
            limit: Some(limit),
        }))))
}

fn parse_optional_symbol(
    value: Option<&str>,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let symbol = raw.trim().to_ascii_uppercase();
    if symbol.is_empty() || symbol == "ALL" {
        return Ok(None);
    }
    if symbol == "BTC" || symbol == "BTCUSDT" || symbol == "ETH" || symbol == "ETHUSDT" {
        return Err(bad_request(
            "excluded_symbol",
            "BTC and ETH are handled by the BTC/ETH contract monitor",
        ));
    }
    if !symbol.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(bad_request("invalid_symbol", "symbol must be alphanumeric"));
    }
    Ok(Some(symbol.trim_end_matches("USDT").to_string()))
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

fn parse_optional_bool(
    value: Option<&str>,
) -> Result<Option<bool>, (StatusCode, Json<serde_json::Value>)> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("all") | Some("ALL") => Ok(None),
        Some(value) if value.eq_ignore_ascii_case("true") || value == "1" => Ok(Some(true)),
        Some(value) if value.eq_ignore_ascii_case("false") || value == "0" => Ok(Some(false)),
        Some(_) => Err(bad_request(
            "invalid_bool_filter",
            "boolean filters must be true, false, 1, 0, or all",
        )),
    }
}

fn parse_optional_u8(
    value: Option<&str>,
    name: &str,
) -> Result<Option<u8>, (StatusCode, Json<serde_json::Value>)> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("all") | Some("ALL") => Ok(None),
        Some(raw) => raw
            .parse::<u8>()
            .map(Some)
            .map_err(|_| bad_request("invalid_numeric_filter", &format!("{name} must be 0-100"))),
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
