use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    storage::main_force_events_repo::MainForceEventsRepo,
    types::main_force_event::{MainForceEventQuery, MainForceEventsResponse},
};

#[derive(Debug, Deserialize, Default)]
pub struct MainForceEventsQuery {
    pub symbol: Option<String>,
    pub regime_type: Option<String>,
    pub severity: Option<String>,
    pub active: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<String>,
    pub offset: Option<String>,
}

pub async fn main_force_events_route(
    State(state): State<AppState>,
    Query(query): Query<MainForceEventsQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol(query.symbol.as_deref(), &state.config().symbol);
    let parsed = parse_main_force_query(&query, &requested_symbol);
    let items = match (state.contract_whale_store(), parsed) {
        (Some(store), Ok(query)) => store.list_main_force_events(&query).unwrap_or_default(),
        _ => Vec::new(),
    };
    Json(serde_json::json!(MainForceEventsResponse {
        read_only: true,
        execution_enabled: false,
        items,
        filter: filter_map(&requested_symbol, &query),
    }))
}

fn parse_main_force_query(
    query: &MainForceEventsQuery,
    requested_symbol: &str,
) -> Result<MainForceEventQuery, ()> {
    Ok(MainForceEventQuery {
        symbol: Some(requested_symbol.to_string()),
        regime_type: query
            .regime_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase()),
        severity: query
            .severity
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase()),
        active_only: parse_optional_bool(query.active.as_deref()).ok().flatten(),
        from_ts: parse_optional_i64(query.from.as_deref()).ok().flatten(),
        to_ts: parse_optional_i64(query.to.as_deref()).ok().flatten(),
        limit: parse_limit(query.limit.as_deref(), 20, 100).unwrap_or(20),
        offset: parse_optional_usize(query.offset.as_deref()).unwrap_or(0),
    })
}

fn filter_map(symbol: &str, query: &MainForceEventsQuery) -> BTreeMap<String, String> {
    let mut filter = BTreeMap::new();
    filter.insert("symbol".to_string(), symbol.to_string());
    filter.insert("readOnly".to_string(), "true".to_string());
    if let Some(active) = query.active.as_deref().filter(|value| !value.is_empty()) {
        filter.insert("active".to_string(), active.to_string());
    }
    if let Some(regime_type) = query
        .regime_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        filter.insert("regimeType".to_string(), regime_type.to_string());
    }
    if let Some(severity) = query
        .severity
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        filter.insert("severity".to_string(), severity.to_string());
    }
    filter
}

fn normalize_symbol(symbol: Option<&str>, fallback: &str) -> String {
    symbol
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_ascii_uppercase()
}

fn parse_limit(value: Option<&str>, default: usize, max: usize) -> Result<usize, ()> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(default);
    };
    let limit = value.parse::<usize>().map_err(|_| ())?;
    Ok(limit.clamp(1, max))
}

fn parse_optional_usize(value: Option<&str>) -> Result<usize, ()> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };
    value.parse::<usize>().map_err(|_| ())
}

fn parse_optional_i64(value: Option<&str>) -> Result<Option<i64>, ()> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    value.parse::<i64>().map(Some).map_err(|_| ())
}

fn parse_optional_bool(value: Option<&str>) -> Result<Option<bool>, ()> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(Some(true)),
        "false" | "0" | "no" => Ok(Some(false)),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{filter_map, normalize_symbol, parse_main_force_query, MainForceEventsQuery};

    #[test]
    fn parse_main_force_query_normalizes_symbol_and_clamps_limit() {
        let query = MainForceEventsQuery {
            symbol: Some(" eth ".to_string()),
            regime_type: Some(" Main_Force_Long_Build ".to_string()),
            severity: Some(" Major ".to_string()),
            active: Some("yes".to_string()),
            from: Some("100".to_string()),
            to: Some("200".to_string()),
            limit: Some("999".to_string()),
            offset: Some("3".to_string()),
        };

        let requested_symbol = normalize_symbol(query.symbol.as_deref(), "BTC");
        let parsed = parse_main_force_query(&query, &requested_symbol).expect("parsed query");

        assert_eq!(parsed.symbol.as_deref(), Some("ETH"));
        assert_eq!(parsed.regime_type.as_deref(), Some("main_force_long_build"));
        assert_eq!(parsed.severity.as_deref(), Some("major"));
        assert_eq!(parsed.active_only, Some(true));
        assert_eq!(parsed.from_ts, Some(100));
        assert_eq!(parsed.to_ts, Some(200));
        assert_eq!(parsed.limit, 100);
        assert_eq!(parsed.offset, 3);
    }

    #[test]
    fn filter_map_keeps_read_only_symbol_and_trimmed_filters() {
        let query = MainForceEventsQuery {
            active: Some("false".to_string()),
            regime_type: Some(" downside_absorption ".to_string()),
            severity: Some(" Extreme ".to_string()),
            ..MainForceEventsQuery::default()
        };

        let filter = filter_map("BTC", &query);

        assert_eq!(filter.get("symbol").map(String::as_str), Some("BTC"));
        assert_eq!(filter.get("readOnly").map(String::as_str), Some("true"));
        assert_eq!(filter.get("active").map(String::as_str), Some("false"));
        assert_eq!(
            filter.get("regimeType").map(String::as_str),
            Some("downside_absorption")
        );
        assert_eq!(filter.get("severity").map(String::as_str), Some("Extreme"));
    }

    #[test]
    fn normalize_symbol_falls_back_to_default() {
        assert_eq!(normalize_symbol(None, "btc"), "BTC");
        assert_eq!(normalize_symbol(Some("   "), "eth"), "ETH");
    }
}
