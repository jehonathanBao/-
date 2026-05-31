use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::{fs, path::PathBuf};

use crate::{
    app::AppState,
    toxicity::{sweep_service::last_sweep_summary, toxic_service::latest_toxic_summary},
    types::{
        market::{VenueConnectionStatus, VenueHealth},
        status::{
            AlertStatusSummary, LiqHuntStatusSummary, LiquidationStatusSummary,
            MarketDataQualityStatus, MarketDataQualitySummary, MarkoutStatusSummary,
            StatusResponse, StorageStatusSummary, SweepStatusSummary, ToxicStatusSummary,
            VenueDiagnosticsResponse, VenueDiagnosticsSummary, VenueHealthMap, VpinStatusSummary,
        },
    },
};

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(build_status_response(&state))
}

pub async fn venues_diagnostics(State(state): State<AppState>) -> Json<VenueDiagnosticsResponse> {
    Json(build_venue_diagnostics_response(&state))
}

pub async fn flow_state(State(state): State<AppState>) -> Json<crate::types::flow::FlowState> {
    Json(state.flow_state())
}

pub async fn markout_state(
    State(state): State<AppState>,
) -> Json<crate::types::markout::MarkoutState> {
    Json(state.markout_state())
}

pub async fn sweep_state(State(state): State<AppState>) -> Json<crate::types::sweep::SweepState> {
    Json(state.sweep_state())
}

pub async fn toxic_state(State(state): State<AppState>) -> Json<crate::types::toxic::ToxicState> {
    Json(state.toxic_state())
}

pub async fn liquidation_state(
    State(state): State<AppState>,
) -> Json<crate::types::liquidation::LiquidationState> {
    Json(state.liquidation_state())
}

pub async fn vpin_state(State(state): State<AppState>) -> Json<crate::types::vpin::VpinState> {
    Json(state.vpin_state())
}

pub async fn liq_hunt_state(
    State(state): State<AppState>,
) -> Json<crate::types::liq_hunt::LiqHuntState> {
    Json(state.liq_hunt_state())
}

#[derive(Debug, Deserialize)]
pub struct ToxicEventsQuery {
    limit: Option<usize>,
}

pub async fn toxic_events(
    State(state): State<AppState>,
    Query(query): Query<ToxicEventsQuery>,
) -> Json<serde_json::Value> {
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let events = state.recent_toxic_events(limit).unwrap_or_default();
    Json(serde_json::json!({ "events": events }))
}

pub async fn latest_toxic_event(State(state): State<AppState>) -> Json<serde_json::Value> {
    let event = state.latest_toxic_event().ok().flatten();
    Json(serde_json::json!({ "event": event }))
}

pub async fn vpin_buckets(
    State(state): State<AppState>,
    Query(query): Query<ToxicEventsQuery>,
) -> Json<serde_json::Value> {
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let buckets = state.recent_vpin_buckets(limit).unwrap_or_default();
    Json(serde_json::json!({ "buckets": buckets }))
}

pub async fn storage_status(State(state): State<AppState>) -> Json<StorageStatusSummary> {
    let storage = state.storage_state();
    Json(StorageStatusSummary {
        enabled: storage.enabled,
        status: storage.status,
        sqlite_path: storage.sqlite_path,
        last_write_ts: storage.last_write_ts,
        last_error: storage.last_error,
    })
}

pub async fn replay_reports(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "reports": list_replay_reports(state.config().replay_report_dir.clone()).unwrap_or_default()
    }))
}

pub async fn replay_report_content(
    State(state): State<AppState>,
    Path(file_name): Path<String>,
) -> impl IntoResponse {
    if !valid_report_name(&file_name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid report file name" })),
        )
            .into_response();
    }

    let path = PathBuf::from(&state.config().replay_report_dir).join(&file_name);
    match fs::read_to_string(&path) {
        Ok(content) => Json(serde_json::json!({
            "fileName": file_name,
            "content": content,
        }))
        .into_response(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "report not found" })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub fn build_status_response(state: &AppState) -> StatusResponse {
    let config = state.config();
    let markout_state = state.markout_state();
    let sweep_state = state.sweep_state();
    let vpin_state = state.vpin_state();
    let liquidation_state = state.liquidation_state();
    let liq_hunt_state = state.liq_hunt_state();
    let toxic_state = state.toxic_state();
    let alert_state = state.alert_state();
    let storage_state = state.storage_state();
    let flow_state = state.flow_state();
    let venues = normalize_venue_map(state.venue_health());
    let market_data_quality = build_market_data_quality_summary(
        state.market_data_quality().snapshot(),
        &flow_state,
        &venues,
    );
    let (last_direction, last_sweep_detected) = last_sweep_summary(&sweep_state);
    let (latest_direction, latest_toxic_volume_btc, latest_alert_triggered) =
        latest_toxic_summary(&toxic_state);
    StatusResponse {
        app: "btc-toxic-flow-monitor-rs",
        read_only: true,
        config_source: config.config_source_label(),
        symbol: config.symbol.clone(),
        threshold_btc: config.toxic_volume_alert_btc,
        windows_ms: config.windows_ms.clone(),
        venues,
        market_data_quality,
        markout: MarkoutStatusSummary {
            enabled: true,
            horizons_ms: markout_state.horizons_ms,
            pending_samples: markout_state.quality.pending_samples,
            resolved_samples: markout_state.quality.resolved_samples,
            expired_samples: markout_state.quality.expired_samples,
        },
        sweep: SweepStatusSummary {
            enabled: true,
            windows_ms: sweep_state.windows_ms,
            last_direction,
            last_sweep_detected,
        },
        vpin: VpinStatusSummary {
            enabled: vpin_state.metrics.enabled,
            bucket_size_btc: vpin_state.metrics.bucket_size_btc,
            completed_bucket_count: vpin_state.metrics.completed_bucket_count,
            vpin: vpin_state.metrics.vpin,
            vpin_spike: vpin_state.metrics.vpin_spike,
            vpin_high: vpin_state.metrics.vpin_high,
            vpin_extreme: vpin_state.metrics.vpin_extreme,
            dominant_direction: vpin_state.metrics.dominant_direction,
        },
        liquidation: LiquidationStatusSummary {
            enabled: liquidation_state.metrics.enabled,
            nearest_cluster_side: liquidation_state.metrics.nearest_cluster_side,
            distance_bps: liquidation_state.metrics.distance_bps,
            liq_hunt_pressure: liquidation_state.metrics.liq_hunt_pressure,
            liq_cluster_nearby: liquidation_state.metrics.liq_cluster_nearby,
            possible_liq_hunt_setup: liquidation_state.metrics.possible_liq_hunt_setup,
        },
        liq_hunt: LiqHuntStatusSummary {
            enabled: true,
            level: liq_hunt_state.result.level,
            direction: liq_hunt_state.result.direction,
            score: liq_hunt_state.result.score,
        },
        toxic: ToxicStatusSummary {
            enabled: true,
            threshold_btc: toxic_state.threshold_btc,
            latest_direction,
            latest_toxic_volume_btc,
            latest_alert_triggered,
            recent_event_count: toxic_state.recent_events.len(),
        },
        runtime_control: state.runtime_control_summary(),
        alerts: AlertStatusSummary {
            telegram_enabled: alert_state.telegram_enabled,
            last_sent_ts: alert_state.last_sent_ts,
            sent_count: alert_state.sent_count,
            suppressed_count: alert_state.suppressed_count,
            last_error: alert_state.last_error,
        },
        storage: StorageStatusSummary {
            enabled: storage_state.enabled,
            status: storage_state.status,
            sqlite_path: storage_state.sqlite_path,
            last_write_ts: storage_state.last_write_ts,
            last_error: storage_state.last_error,
        },
    }
}

fn build_market_data_quality_summary(
    snapshot: crate::market_data::quality::MarketDataQualitySnapshot,
    flow_state: &crate::types::flow::FlowState,
    venues: &VenueHealthMap,
) -> MarketDataQualitySummary {
    let flow_windows_populated = flow_state
        .windows
        .values()
        .any(|window| window.trade_count > 0 || window.data_quality.has_trades);
    let any_active_venue = flow_state
        .windows
        .values()
        .any(|window| !window.data_quality.active_venues.is_empty());
    let any_stale_venue = flow_state
        .windows
        .values()
        .any(|window| !window.data_quality.stale_venues.is_empty());
    let lagged_or_dropped = snapshot.event_bus_dropped_events > 0
        || snapshot.event_bus_send_errors > 0
        || snapshot.flow_window_lagged_events > 0
        || snapshot.markout_lagged_events > 0
        || snapshot.vpin_lagged_events > 0;
    let status = if lagged_or_dropped {
        MarketDataQualityStatus::Degraded
    } else if !flow_windows_populated {
        MarketDataQualityStatus::NoData
    } else if any_stale_venue && !any_active_venue {
        MarketDataQualityStatus::Stale
    } else {
        MarketDataQualityStatus::Healthy
    };
    let operator_warning = match status {
        MarketDataQualityStatus::Degraded => Some(
            "Market data consumers lagged or dropped events; signal output may be incomplete.",
        ),
        MarketDataQualityStatus::Stale => {
            Some("Market data is stale; current empty signal lists may be incomplete.")
        }
        MarketDataQualityStatus::NoData => {
            Some("No flow window is populated yet; empty signal lists do not prove there are no toxic orders.")
        }
        MarketDataQualityStatus::Healthy => None,
    };

    MarketDataQualitySummary {
        status,
        event_bus_dropped_events: snapshot.event_bus_dropped_events,
        event_bus_send_errors: snapshot.event_bus_send_errors,
        flow_window_lagged_events: snapshot.flow_window_lagged_events,
        markout_lagged_events: snapshot.markout_lagged_events,
        vpin_lagged_events: snapshot.vpin_lagged_events,
        last_lagged_at_ms: snapshot.last_lagged_at_ms,
        last_message_ts: venues
            .values()
            .filter_map(|venue| venue.last_message_ts)
            .max(),
        latest_trade_ts: venues
            .values()
            .filter_map(|venue| venue.last_trade_ts)
            .max(),
        flow_windows_populated,
        operator_warning,
    }
}

pub fn build_venue_diagnostics_response(state: &AppState) -> VenueDiagnosticsResponse {
    let runtime_control = state.runtime_control_summary();
    let flow_state = state.flow_state();
    let mut venues = normalize_venue_map(state.venue_health())
        .into_values()
        .collect::<Vec<_>>();
    refresh_venue_activity(&mut venues);
    let enabled_venues = venues.iter().filter(|venue| venue.enabled).count();
    let connector_constructed_venues = venues
        .iter()
        .filter(|venue| venue.connector_constructed)
        .count();
    let start_attempted_venues = venues.iter().filter(|venue| venue.start_attempted).count();
    let connected_venues = venues
        .iter()
        .filter(|venue| matches!(venue.status, VenueConnectionStatus::Connected))
        .count();
    let ws_connect_attempted_venues = venues
        .iter()
        .filter(|venue| venue.ws_connect_attempted)
        .count();
    let ws_connected_venues = venues.iter().filter(|venue| venue.ws_connected).count();
    let symbol_mapped_venues = venues
        .iter()
        .filter(|venue| venue.symbol_mapping_status == "ok")
        .count();
    let venues_with_network_errors = venues
        .iter()
        .filter(|venue| {
            !matches!(
                venue.ws_error_class.as_str(),
                "none" | "message_parse_error" | "schema_error"
            )
        })
        .count();
    let active_venues = flow_state
        .windows
        .values()
        .flat_map(|window| window.data_quality.active_venues.iter())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let active_trade_venues = venues
        .iter()
        .filter(|venue| venue.last_trade_ts.is_some())
        .count();
    let active_book_venues = venues
        .iter()
        .filter(|venue| venue.last_book_ts.is_some())
        .count();
    let trade_active_venues = venues.iter().filter(|venue| venue.trade_active).count();
    let book_active_venues = venues.iter().filter(|venue| venue.book_active).count();
    let latest_venue_trade_available = venues.iter().any(|venue| venue.last_trade_ts.is_some());
    let latest_venue_book_available = venues.iter().any(|venue| venue.last_book_ts.is_some());
    let flow_windows_populated = flow_state
        .windows
        .values()
        .any(|window| window.trade_count > 0);
    let diagnostic_status = venue_diagnostic_status(VenueDiagnosticStatusInput {
        monitoring_started: runtime_control.monitoring_started,
        enabled_venues,
        connected_venues,
        ws_connect_attempted_venues,
        ws_connected_venues,
        venues_with_network_errors,
        trade_active_venues,
        book_active_venues,
        flow_windows_populated,
    });
    let mut operator_notes = vec![
        "Only public market data streams are diagnosed.".to_string(),
        "No private API key, wallet, signing, order placement, or live trading path is enabled."
            .to_string(),
        "monitoringStarted=true only means runtime start was requested.".to_string(),
        "No public trade/orderbook stream is active until at least one venue is enabled and connected.".to_string(),
    ];
    if enabled_venues == 0 {
        operator_notes.push(
            "No public stream active: all venue enable flags are false or missing.".to_string(),
        );
    } else if runtime_control.monitoring_started && active_trade_venues == 0 {
        operator_notes
            .push("Runtime is started, but no venue has delivered a trade event yet.".to_string());
    }

    VenueDiagnosticsResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        runtime_modified: false,
        monitoring_started: runtime_control.monitoring_started,
        diagnostic_status,
        summary: VenueDiagnosticsSummary {
            configured_venues: venues.len(),
            enabled_venues,
            connector_constructed_venues,
            start_attempted_venues,
            connected_venues,
            ws_connect_attempted_venues,
            ws_connected_venues,
            symbol_mapped_venues,
            venues_with_network_errors,
            active_trade_venues,
            active_book_venues,
            trade_active_venues,
            book_active_venues,
            active_venues,
            diagnostic_status,
            latest_venue_trade_available,
            latest_venue_book_available,
            flow_windows_populated,
        },
        venues,
        operator_notes,
    }
}

fn refresh_venue_activity(venues: &mut [VenueHealth]) {
    let now = crate::normalizers::trade::now_ms();
    for venue in venues {
        venue.trade_active = venue
            .last_parsed_trade_at_ms
            .is_some_and(|ts| now.saturating_sub(ts) <= venue.active_window_ms);
        venue.book_active = venue
            .last_parsed_book_at_ms
            .is_some_and(|ts| now.saturating_sub(ts) <= venue.active_window_ms);
        venue.activity_status = if !venue.enabled {
            "disabled".to_string()
        } else if !venue.start_attempted {
            "not_started".to_string()
        } else if !matches!(
            venue.ws_error_class.as_str(),
            "none" | "message_parse_error" | "schema_error"
        ) {
            "error".to_string()
        } else if venue.trade_active || venue.book_active {
            "active".to_string()
        } else if venue.last_parsed_trade_at_ms.is_some() || venue.last_parsed_book_at_ms.is_some()
        {
            "stale".to_string()
        } else {
            "no_data".to_string()
        };
    }
}

struct VenueDiagnosticStatusInput {
    monitoring_started: bool,
    enabled_venues: usize,
    connected_venues: usize,
    ws_connect_attempted_venues: usize,
    ws_connected_venues: usize,
    venues_with_network_errors: usize,
    trade_active_venues: usize,
    book_active_venues: usize,
    flow_windows_populated: bool,
}

fn venue_diagnostic_status(input: VenueDiagnosticStatusInput) -> &'static str {
    if input.enabled_venues == 0 {
        return "no_public_stream_enabled";
    }
    if !input.monitoring_started {
        return "monitoring_not_started";
    }
    if input.ws_connect_attempted_venues == 0 {
        return "ws_not_attempted";
    }
    if input.venues_with_network_errors > 0 && input.ws_connected_venues == 0 {
        return "network_error";
    }
    if input.connected_venues == 0 || input.ws_connected_venues == 0 {
        return "enabled_but_not_connected";
    }
    if input.trade_active_venues == 0 && input.book_active_venues == 0 {
        return "connected_but_no_events";
    }
    if !input.flow_windows_populated {
        return "events_seen_but_flow_empty";
    }
    "public_stream_active"
}

fn normalize_venue_map(map: VenueHealthMap) -> VenueHealthMap {
    map
}

fn list_replay_reports(dir: String) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut reports = Vec::new();
    let path = PathBuf::from(dir);
    if !path.exists() {
        return Ok(reports);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".md") {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64);
        reports.push(serde_json::json!({
            "fileName": file_name,
            "path": entry.path().display().to_string(),
            "modifiedAt": modified_at,
        }));
    }

    reports.sort_by(|left, right| {
        let left_ts = left["modifiedAt"].as_i64().unwrap_or_default();
        let right_ts = right["modifiedAt"].as_i64().unwrap_or_default();
        right_ts.cmp(&left_ts)
    });

    Ok(reports)
}

fn valid_report_name(file_name: &str) -> bool {
    file_name.ends_with(".md")
        && !file_name.contains("..")
        && !file_name.contains('\\')
        && !file_name.contains('/')
}
