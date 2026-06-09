use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    contract_whale_monitor::{
        aggregator::{
            compute_percentile_threshold, dynamic_multiple_for_volume,
            historical_window_average_btc_with_min_samples, liquidation_context_for_window,
            market_context_from_snapshots, percentile_level_for_volume,
        },
        config::contract_whale_runtime_config,
        detector::detect_contract_whale_signal,
        log_events,
        merge::merge_contract_whale_signals,
        types::{
            ContractWhaleDirection, ContractWhaleExchangeStatus, ContractWhaleLatestResponse,
            ContractWhaleLiquidationContext, ContractWhaleMarketCapability,
            ContractWhaleMarketContext, ContractWhaleMarketType, ContractWhalePercentileThreshold,
            ContractWhalePlatformCapability, ContractWhaleResponseMeta, ContractWhaleSeverity,
            ContractWhaleSignal, ContractWhaleSignalType, ContractWhaleSummary,
            ContractWhaleTrend60s, ContractWhaleWindowStats, ExchangeFlowContribution,
        },
        LOG_PREFIX as CWM_LOG_PREFIX, LOG_TARGET as CWM_LOG_TARGET,
    },
    normalizers::trade::now_ms,
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
    types::{
        flow::{FlowState, FlowWindow, VenueFlowBreakdown},
        market::{VenueConnectionStatus, VenueHealth},
        status::VenueHealthMap,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct ContractWhaleQuery {
    pub limit: Option<String>,
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub direction: Option<String>,
    pub discord_sent: Option<String>,
    pub window_sec: Option<String>,
    pub exchange: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub offset: Option<String>,
}

type ApiJsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Clone, Default)]
pub struct ContractWhaleQualityBaseline {
    pub dynamic_multiple: Option<f64>,
    pub percentile_level: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ContractWhaleWarmupState {
    active: bool,
    until_ms: Option<i64>,
    remaining_ms: Option<i64>,
}

pub struct ContractWhaleResponseRuntime<'a> {
    pub venue_health: Option<&'a VenueHealthMap>,
    pub baselines: &'a BTreeMap<u64, ContractWhaleQualityBaseline>,
    pub liquidations: &'a BTreeMap<u64, ContractWhaleLiquidationContext>,
    pub market_context: &'a ContractWhaleMarketContext,
    pub booted_at_ms: Option<i64>,
}

pub async fn contract_whale_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let exchange_filter = parse_exchange_filter(query.exchange.as_deref())?;
    let config = state.config().contract_whale_monitor;
    let runtime_config = contract_whale_runtime_config();
    let symbol_enabled = config.enabled && contract_whale_runtime_config().symbol_enabled(&symbol);
    let flow_state = state.flow_state_for_symbol(&symbol);
    let venue_health = state.venue_health();
    let baselines = state
        .contract_whale_store()
        .map(|store| load_quality_baselines(&store, &flow_state, &symbol))
        .unwrap_or_default();
    let liquidations = state
        .contract_whale_store()
        .map(|store| load_liquidation_contexts(&store, &flow_state, &symbol))
        .unwrap_or_default();
    let market_context = state
        .contract_whale_store()
        .map(|store| load_market_context(&store, &flow_state, &symbol))
        .unwrap_or_default();
    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        &symbol,
        50,
        None,
        symbol_enabled,
        config.dry_run,
        ContractWhaleResponseRuntime {
            venue_health: Some(&venue_health),
            baselines: &baselines,
            liquidations: &liquidations,
            market_context: &market_context,
            booted_at_ms: Some(state.booted_at_ms()),
        },
    );
    let mut summary = response.summary;
    summary.meta = contract_market_mismatch_meta(&runtime_config, exchange_filter.as_deref());
    Ok(Json(serde_json::json!(summary)))
}

pub async fn contract_whale_latest_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref(), 50, 200)?;
    let exchange_filter = parse_exchange_filter(query.exchange.as_deref())?;
    let flow_state = state.flow_state_for_symbol(&symbol);
    let venue_health = state.venue_health();
    let config = state.config().contract_whale_monitor;
    let store = state.contract_whale_store();
    let cwm_runtime_config = contract_whale_runtime_config();
    if let Some(meta) =
        contract_market_mismatch_meta(&cwm_runtime_config, exchange_filter.as_deref())
    {
        return Ok(Json(serde_json::json!(
            build_contract_whale_history_response(
                Vec::new(),
                &symbol,
                limit,
                None,
                config.enabled,
                config.dry_run,
                Some(meta),
            )
        )));
    }
    if !config.enabled || !cwm_runtime_config.symbol_enabled(&symbol) {
        return Ok(Json(serde_json::json!(
            build_contract_whale_response_with_runtime(
                &flow_state,
                &symbol,
                limit,
                None,
                false,
                config.dry_run,
                Some(&venue_health),
            )
        )));
    }
    if let Some(store) = store.as_ref() {
        match store.query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            exchange: exchange_filter.clone(),
            limit,
            ..ContractWhaleSignalQuery::default()
        }) {
            Ok(items) if !items.is_empty() => {
                let now = if flow_state.updated_at > 0 {
                    flow_state.updated_at
                } else {
                    now_ms()
                };
                return Ok(Json(serde_json::json!(filter_latest_response_by_exchange(
                    build_contract_whale_items_response(
                        items,
                        &symbol,
                        limit,
                        config.enabled,
                        config.dry_run,
                        contract_exchange_statuses(
                            &flow_state,
                            Some(&venue_health),
                            config.enabled,
                            now
                        ),
                        trend_60s_from_flow_state(&flow_state, &symbol, now),
                    ),
                    exchange_filter.as_deref(),
                ))));
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    error = %error,
                    "{} latest query failed",
                    CWM_LOG_PREFIX
                );
            }
        }
    }
    let baselines = store
        .as_ref()
        .map(|store| load_quality_baselines(store, &flow_state, &symbol))
        .unwrap_or_default();
    let liquidations = store
        .as_ref()
        .map(|store| load_liquidation_contexts(store, &flow_state, &symbol))
        .unwrap_or_default();
    let market_context = store
        .as_ref()
        .map(|store| load_market_context(store, &flow_state, &symbol))
        .unwrap_or_default();
    Ok(Json(serde_json::json!(filter_latest_response_by_exchange(
        build_contract_whale_response_with_runtime_and_baselines(
            &flow_state,
            &symbol,
            limit,
            None,
            config.enabled,
            config.dry_run,
            ContractWhaleResponseRuntime {
                venue_health: Some(&venue_health),
                baselines: &baselines,
                liquidations: &liquidations,
                market_context: &market_context,
                booted_at_ms: Some(state.booted_at_ms()),
            },
        ),
        exchange_filter.as_deref(),
    ))))
}

pub fn build_contract_whale_response(
    flow_state: &FlowState,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
) -> ContractWhaleLatestResponse {
    build_contract_whale_response_with_runtime(
        flow_state, symbol, limit, severity, enabled, dry_run, None,
    )
}

pub async fn contract_whale_history_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let history_query = parse_history_query(&query)?;
    let symbol_for_filter = history_query.symbol.as_deref().unwrap_or("all").to_string();
    let config = state.config().contract_whale_monitor;
    let runtime_config = contract_whale_runtime_config();
    if let Some(meta) =
        contract_market_mismatch_meta(&runtime_config, history_query.exchange.as_deref())
    {
        return Ok(Json(serde_json::json!(
            build_contract_whale_history_response(
                Vec::new(),
                &symbol_for_filter,
                history_query.limit,
                None,
                config.enabled,
                config.dry_run,
                Some(meta),
            )
        )));
    }
    if !config.enabled {
        return Ok(Json(serde_json::json!(
            build_contract_whale_history_response(
                Vec::new(),
                &symbol_for_filter,
                history_query.limit,
                None,
                false,
                config.dry_run,
                None,
            )
        )));
    }
    if let Some(store) = state.contract_whale_store() {
        match store.query_contract_whale_signals(&history_query) {
            Ok(items) => {
                return Ok(Json(serde_json::json!(
                    build_contract_whale_history_response(
                        items,
                        &symbol_for_filter,
                        history_query.limit,
                        None,
                        config.enabled,
                        config.dry_run,
                        None,
                    )
                )));
            }
            Err(error) => {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    error = %error,
                    "{} history query failed",
                    CWM_LOG_PREFIX
                );
            }
        }
    }
    Ok(Json(serde_json::json!(
        build_contract_whale_history_response(
            Vec::new(),
            &symbol_for_filter,
            history_query.limit,
            None,
            config.enabled,
            config.dry_run,
            None,
        )
    )))
}

pub async fn contract_whale_metrics_route(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config().contract_whale_monitor;
    let flow_state = state.flow_state();
    let venue_health = state.venue_health();
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let exchanges =
        contract_exchange_statuses(&flow_state, Some(&venue_health), config.enabled, now);
    let trend_60s = trend_60s_from_flow_state(&flow_state, "BTC", now);
    let items = state
        .contract_whale_store()
        .and_then(|store| {
            store
                .query_contract_whale_signals(&ContractWhaleSignalQuery {
                    limit: 200,
                    ..ContractWhaleSignalQuery::default()
                })
                .ok()
        })
        .unwrap_or_else(|| {
            build_contract_whale_response_with_runtime(
                &flow_state,
                "BTC",
                200,
                None,
                config.enabled,
                config.dry_run,
                Some(&venue_health),
            )
            .items
        });
    let body = build_contract_whale_metrics_text(config.enabled, &exchanges, &trend_60s, &items);
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

pub fn build_contract_whale_metrics_text(
    enabled: bool,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s: &ContractWhaleTrend60s,
    signals: &[ContractWhaleSignal],
) -> String {
    let mut output = String::new();
    output.push_str("# HELP cwm_enabled Contract whale monitor enabled flag.\n");
    output.push_str("# TYPE cwm_enabled gauge\n");
    output.push_str(&format!("cwm_enabled {}\n", i32::from(enabled)));
    output.push_str("# HELP cwm_ws_connected WebSocket connection status by exchange.\n");
    output.push_str("# TYPE cwm_ws_connected gauge\n");
    for (exchange, status) in exchanges {
        output.push_str(&format!(
            "cwm_ws_connected{{exchange=\"{}\"}} {}\n",
            metric_label(exchange),
            i32::from(enabled && status.connected)
        ));
        output.push_str(&format!(
            "cwm_ws_reconnect_total{{exchange=\"{}\"}} {}\n",
            metric_label(exchange),
            status.reconnect_count
        ));
        output.push_str(&format!(
            "cwm_trades_received_total{{exchange=\"{}\"}} {}\n",
            metric_label(exchange),
            status.last_trade_at.map(|_| 1).unwrap_or(0)
        ));
        output.push_str(&format!(
            "cwm_trade_normalize_errors_total{{exchange=\"{}\"}} 0\n",
            metric_label(exchange)
        ));
    }
    output.push_str(&format!(
        "cwm_bucket_flush_total {}\n",
        i32::from(trend_60s.total_volume_btc > 0.0)
    ));
    output.push_str("cwm_bucket_flush_errors_total 0\n");

    let mut generated: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut discord_sent_total = 0_usize;
    let mut data_quality_total = 0_u64;
    for signal in signals {
        *generated
            .entry((
                metric_label(&format!("{:?}", signal.severity).to_ascii_lowercase()),
                metric_label(&format!("{:?}", signal.signal_type).to_ascii_lowercase()),
            ))
            .or_default() += 1;
        if signal.discord_sent {
            discord_sent_total += 1;
        } else {
            *skipped
                .entry(metric_label(&signal.discord_reason))
                .or_default() += 1;
        }
        data_quality_total += signal.data_quality as u64;
    }
    for ((severity, signal_type), count) in generated {
        output.push_str(&format!(
            "cwm_signals_generated_total{{severity=\"{severity}\",type=\"{signal_type}\"}} {count}\n"
        ));
    }
    output.push_str(&format!("cwm_discord_sent_total {discord_sent_total}\n"));
    for (reason, count) in skipped {
        output.push_str(&format!(
            "cwm_discord_skipped_total{{reason=\"{reason}\"}} {count}\n"
        ));
    }
    let data_quality = if signals.is_empty() {
        0.0
    } else {
        data_quality_total as f64 / signals.len() as f64
    };
    output.push_str(&format!("cwm_data_quality {data_quality:.1}\n"));
    output.push_str(&format!(
        "cwm_trend_60s_buy_volume_btc {:.6}\n",
        trend_60s.buy_volume_btc
    ));
    output.push_str(&format!(
        "cwm_trend_60s_sell_volume_btc {:.6}\n",
        trend_60s.sell_volume_btc
    ));
    output
}

pub fn build_contract_whale_response_with_runtime(
    flow_state: &FlowState,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
    venue_health: Option<&VenueHealthMap>,
) -> ContractWhaleLatestResponse {
    build_contract_whale_response_with_runtime_and_baselines(
        flow_state,
        symbol,
        limit,
        severity,
        enabled,
        dry_run,
        ContractWhaleResponseRuntime {
            venue_health,
            baselines: &BTreeMap::new(),
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: None,
        },
    )
}

pub fn build_contract_whale_response_with_runtime_and_baselines(
    flow_state: &FlowState,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
    runtime: ContractWhaleResponseRuntime<'_>,
) -> ContractWhaleLatestResponse {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let filter = response_filter(symbol, enabled, dry_run);
    let exchanges = contract_exchange_statuses(flow_state, runtime.venue_health, enabled, now);
    let warmup = warmup_state(now, enabled, runtime.booted_at_ms);
    let trend_60s = trend_60s_from_flow_state(flow_state, symbol, now);
    if !enabled {
        return ContractWhaleLatestResponse {
            summary: disabled_summary(now, dry_run, exchanges, trend_60s),
            items: Vec::new(),
            filter,
            meta: None,
        };
    }

    let mut items: Vec<ContractWhaleSignal> = [5_u64, 15, 60]
        .into_iter()
        .filter_map(|window_sec| flow_window_for_seconds(flow_state, window_sec))
        .filter_map(|window| {
            stats_from_flow_window(
                window,
                symbol,
                now,
                runtime.baselines,
                runtime.liquidations,
                runtime.market_context,
                runtime.booted_at_ms,
            )
        })
        .filter_map(|stats| detect_contract_whale_signal(&stats))
        .filter(|signal| severity_matches(signal.severity, severity))
        .collect();
    items = merge_contract_whale_signals(items);
    items.sort_by(|left, right| {
        right
            .severity
            .rank()
            .cmp(&left.severity.rank())
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| right.ts.cmp(&left.ts))
    });
    items.truncate(limit);
    let summary = build_summary(&items, now, enabled, dry_run, exchanges, warmup, trend_60s);
    ContractWhaleLatestResponse {
        summary,
        items,
        filter,
        meta: None,
    }
}

fn response_filter(symbol: &str, enabled: bool, dry_run: bool) -> BTreeMap<String, String> {
    let mut filter = BTreeMap::new();
    filter.insert("symbol".to_string(), symbol.to_string());
    filter.insert("market".to_string(), "perp".to_string());
    filter.insert("marketType".to_string(), "perp".to_string());
    filter.insert("readOnly".to_string(), "true".to_string());
    filter.insert("enabled".to_string(), enabled.to_string());
    filter.insert("dryRun".to_string(), dry_run.to_string());
    filter
}

fn metric_label(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' => Some(ch),
            ' ' => Some('_'),
            _ => None,
        })
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn build_contract_whale_history_response(
    mut items: Vec<ContractWhaleSignal>,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
    meta: Option<ContractWhaleResponseMeta>,
) -> ContractWhaleLatestResponse {
    let now = now_ms();
    let filter = response_filter(symbol, enabled, dry_run);
    if !enabled {
        return ContractWhaleLatestResponse {
            summary: disabled_summary(
                now,
                dry_run,
                default_exchange_statuses(),
                ContractWhaleTrend60s::default(),
            ),
            items: Vec::new(),
            filter,
            meta,
        };
    }
    items.retain(|signal| is_perp_signal(signal) && severity_matches(signal.severity, severity));
    items.sort_by(|left, right| {
        right
            .ts
            .cmp(&left.ts)
            .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
            .then_with(|| right.score.cmp(&left.score))
    });
    items.truncate(limit);
    let summary = build_summary(
        &items,
        now,
        enabled,
        dry_run,
        default_exchange_statuses(),
        warmup_state(now, enabled, None),
        ContractWhaleTrend60s::default(),
    );
    ContractWhaleLatestResponse {
        summary,
        items,
        filter,
        meta,
    }
}

pub fn build_contract_whale_items_response(
    mut items: Vec<ContractWhaleSignal>,
    symbol: &str,
    limit: usize,
    enabled: bool,
    dry_run: bool,
    exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s: ContractWhaleTrend60s,
) -> ContractWhaleLatestResponse {
    let now = now_ms();
    let filter = response_filter(symbol, enabled, dry_run);
    if !enabled {
        return ContractWhaleLatestResponse {
            summary: disabled_summary(now, dry_run, exchanges, ContractWhaleTrend60s::default()),
            items: Vec::new(),
            filter,
            meta: None,
        };
    }
    items.retain(is_perp_signal);
    items = merge_contract_whale_signals(items);
    items.sort_by(|left, right| {
        right
            .ts
            .cmp(&left.ts)
            .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
            .then_with(|| right.score.cmp(&left.score))
    });
    items.truncate(limit);
    let summary = build_summary(
        &items,
        now,
        enabled,
        dry_run,
        exchanges,
        warmup_state(now, enabled, None),
        trend_60s,
    );
    ContractWhaleLatestResponse {
        summary,
        items,
        filter,
        meta: None,
    }
}

fn filter_latest_response_by_exchange(
    mut response: ContractWhaleLatestResponse,
    exchange: Option<&str>,
) -> ContractWhaleLatestResponse {
    let Some(exchange) = exchange else {
        return response;
    };
    response.items.retain(|signal| {
        signal
            .exchanges
            .iter()
            .any(|item| item.exchange.eq_ignore_ascii_case(exchange))
    });
    response
        .filter
        .insert("exchange".to_string(), exchange.to_ascii_lowercase());
    response.summary = build_summary(
        &response.items,
        response.summary.updated_at_ms,
        response.summary.enabled,
        response.summary.dry_run,
        response.summary.exchanges.clone(),
        ContractWhaleWarmupState {
            active: response.summary.warmup,
            until_ms: response.summary.warmup_until_ms,
            remaining_ms: response.summary.warmup_remaining_ms,
        },
        response.summary.trend_60s.clone(),
    );
    response
}

fn flow_window_for_seconds(flow_state: &FlowState, window_sec: u64) -> Option<&FlowWindow> {
    let key_ms = (window_sec * 1000).to_string();
    let key_sec = window_sec.to_string();
    flow_state
        .windows
        .get(&key_ms)
        .or_else(|| flow_state.windows.get(&key_sec))
}

fn stats_from_flow_window(
    window: &FlowWindow,
    symbol: &str,
    now: i64,
    baselines: &BTreeMap<u64, ContractWhaleQualityBaseline>,
    liquidations: &BTreeMap<u64, ContractWhaleLiquidationContext>,
    market_context: &ContractWhaleMarketContext,
    booted_at_ms: Option<i64>,
) -> Option<ContractWhaleWindowStats> {
    if !symbol_matches_window(&window.symbol, symbol) {
        return None;
    }
    let exchanges = exchange_contributions(&window.venue_breakdown);
    if exchanges.is_empty() {
        return None;
    }
    let buy_volume_btc = exchanges
        .iter()
        .map(|item| item.buy_volume_btc)
        .sum::<f64>();
    let sell_volume_btc = exchanges
        .iter()
        .map(|item| item.sell_volume_btc)
        .sum::<f64>();
    let buy_notional_usd = exchanges
        .iter()
        .map(|item| item.buy_notional_usd)
        .sum::<f64>();
    let sell_notional_usd = exchanges
        .iter()
        .map(|item| item.sell_notional_usd)
        .sum::<f64>();
    let exchange_count = exchanges
        .iter()
        .filter(|item| item.total_volume_btc > 0.0)
        .count();
    let main_exchange = exchanges.first().map(|item| item.exchange.clone());
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    if total_volume_btc <= 0.0 {
        return None;
    }
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    let data_quality = market_context_quality_score(data_quality_score(window), market_context);
    let window_sec = window.window_ms / 1000;
    let baseline = baselines.get(&window_sec);
    let liquidation_context = liquidations.get(&window_sec).cloned().unwrap_or_default();
    let liquidation_driven = liquidation_context
        .liq_to_volume_ratio
        .is_some_and(|ratio| ratio >= 0.25 && liquidation_context.total_liq_btc >= 50.0);
    Some(ContractWhaleWindowStats {
        symbol: symbol.to_string(),
        window_sec,
        ts: now,
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        net_volume_btc,
        dominance: net_volume_btc.abs() / total_volume_btc,
        buy_notional_usd,
        sell_notional_usd,
        total_notional_usd: buy_notional_usd + sell_notional_usd,
        price_move_pct: window.price_move_bps.map(|bps| bps / 100.0),
        exchange_count,
        main_exchange,
        dominant_venue_net_contribution_share: dominant_venue_net_contribution_share(&exchanges),
        exchanges,
        dynamic_multiple: baseline.and_then(|item| item.dynamic_multiple),
        percentile_level: baseline.and_then(|item| item.percentile_level),
        multi_exchange_confirmed: false,
        liquidation_context,
        market_context: market_context.clone(),
        price_reversal_ratio: None,
        data_quality,
        ws_latency_ms: None,
        startup_age_ms: booted_at_ms.map(|booted_at_ms| now.saturating_sub(booted_at_ms)),
        liquidation_driven,
        price_jump_anomaly: false,
    })
}

fn exchange_contributions(
    breakdown: &BTreeMap<String, VenueFlowBreakdown>,
) -> Vec<ExchangeFlowContribution> {
    let runtime_config = contract_whale_runtime_config();
    let total_net_volume_btc = breakdown
        .iter()
        .filter(|(exchange, _)| runtime_config.exchange_enabled(exchange))
        .map(|(_, item)| item.aggressive_buy_btc - item.aggressive_sell_btc)
        .sum::<f64>();
    let mut contributions: Vec<ExchangeFlowContribution> = breakdown
        .iter()
        .filter(|(exchange, _)| runtime_config.exchange_enabled(exchange))
        .map(|(exchange, item)| {
            let total_volume_btc = item.aggressive_buy_btc + item.aggressive_sell_btc;
            let net_volume_btc = item.aggressive_buy_btc - item.aggressive_sell_btc;
            ExchangeFlowContribution {
                exchange: exchange.clone(),
                buy_volume_btc: item.aggressive_buy_btc,
                sell_volume_btc: item.aggressive_sell_btc,
                total_volume_btc,
                buy_share: share(item.aggressive_buy_btc, total_volume_btc),
                sell_share: share(item.aggressive_sell_btc, total_volume_btc),
                buy_notional_usd: item.aggressive_buy_usd,
                sell_notional_usd: item.aggressive_sell_usd,
                total_notional_usd: item.aggressive_buy_usd + item.aggressive_sell_usd,
                net_volume_btc,
                dominance: dominance(net_volume_btc.abs(), total_volume_btc),
                net_contribution_share: 0.0,
                trade_count: item.trade_count,
            }
        })
        .filter(|item| item.total_volume_btc > 0.0)
        .collect();
    apply_net_contribution_shares(&mut contributions, total_net_volume_btc);
    contributions.sort_by(|left, right| {
        right
            .total_volume_btc
            .partial_cmp(&left.total_volume_btc)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    contributions
}

pub(crate) fn load_quality_baselines(
    store: &impl ContractWhaleRepo,
    flow_state: &FlowState,
    symbol: &str,
) -> BTreeMap<u64, ContractWhaleQualityBaseline> {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let min_dynamic_samples = contract_whale_runtime_config()
        .data_quality
        .min_dynamic_samples;
    [5_u64, 15, 60]
        .into_iter()
        .filter_map(|window_sec| {
            let window = flow_window_for_seconds(flow_state, window_sec)?;
            let current_total = window.aggressive_buy_btc + window.aggressive_sell_btc;
            let window_ms = (window_sec as i64).saturating_mul(1000);
            let dynamic_to = now.saturating_sub(window_ms);
            let dynamic_from = dynamic_to.saturating_sub(60 * 60 * 1000);
            let dynamic_multiple =
                match store.list_contract_flow_buckets_between(symbol, dynamic_from, dynamic_to) {
                    Ok(buckets) => dynamic_multiple_for_volume(
                        current_total,
                        historical_window_average_btc_with_min_samples(
                            &buckets,
                            symbol,
                            window_sec,
                            dynamic_from,
                            dynamic_to,
                            min_dynamic_samples,
                        ),
                    ),
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            error = %error,
                            window_sec,
                            "{} dynamic baseline query failed",
                            CWM_LOG_PREFIX
                        );
                        None
                    }
                };
            let percentile_level =
                latest_percentile_level(store, symbol, window_sec, current_total, now);
            Some((
                window_sec,
                ContractWhaleQualityBaseline {
                    dynamic_multiple,
                    percentile_level,
                },
            ))
        })
        .collect()
}

fn latest_percentile_level(
    store: &impl ContractWhaleRepo,
    symbol: &str,
    window_sec: u64,
    current_total: f64,
    now: i64,
) -> Option<f64> {
    let threshold = match store.latest_contract_whale_percentile(symbol, "all", window_sec) {
        Ok(Some(threshold)) if threshold.computed_at >= now.saturating_sub(60 * 60 * 1000) => {
            Some(threshold)
        }
        Ok(_) => refresh_percentile_threshold(store, symbol, window_sec, now),
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                window_sec,
                "{} percentile threshold query failed",
                CWM_LOG_PREFIX
            );
            None
        }
    };
    percentile_level_for_volume(current_total, threshold.as_ref())
}

fn refresh_percentile_threshold(
    store: &impl ContractWhaleRepo,
    symbol: &str,
    window_sec: u64,
    now: i64,
) -> Option<ContractWhalePercentileThreshold> {
    let from = now.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let buckets = match store.list_contract_flow_buckets_between(symbol, from, now) {
        Ok(buckets) => buckets,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                window_sec,
                "{} percentile bucket query failed",
                CWM_LOG_PREFIX
            );
            return None;
        }
    };
    let threshold =
        compute_percentile_threshold(&buckets, symbol, "all", window_sec, from, now, now)?;
    if let Err(error) = store.upsert_contract_whale_percentiles(std::slice::from_ref(&threshold)) {
        tracing::warn!(
            target: CWM_LOG_TARGET,
            error = %error,
            window_sec,
            "{} percentile threshold upsert failed",
            CWM_LOG_PREFIX
        );
    }
    Some(threshold)
}

pub(crate) fn load_liquidation_contexts(
    store: &impl ContractWhaleRepo,
    flow_state: &FlowState,
    symbol: &str,
) -> BTreeMap<u64, ContractWhaleLiquidationContext> {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let from = now.saturating_sub(60_000);
    let buckets = match store.list_contract_liquidation_buckets_between(symbol, from, now) {
        Ok(buckets) => buckets,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                "{} liquidation bucket query failed",
                CWM_LOG_PREFIX
            );
            return BTreeMap::new();
        }
    };
    [5_u64, 15, 60]
        .into_iter()
        .filter_map(|window_sec| {
            let window = flow_window_for_seconds(flow_state, window_sec)?;
            let total_volume_btc = window.aggressive_buy_btc + window.aggressive_sell_btc;
            Some((
                window_sec,
                liquidation_context_for_window(&buckets, symbol, window_sec, now, total_volume_btc),
            ))
        })
        .collect()
}

pub(crate) fn load_market_context(
    store: &impl ContractWhaleRepo,
    flow_state: &FlowState,
    symbol: &str,
) -> ContractWhaleMarketContext {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let from = now.saturating_sub(6 * 60_000);
    let oi_snapshots = match store.list_contract_oi_snapshots_between(symbol, from, now) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                "{} oi snapshot query failed",
                CWM_LOG_PREFIX
            );
            Vec::new()
        }
    };
    let funding_snapshots = match store.list_contract_funding_snapshots_between(symbol, from, now) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                "{} funding snapshot query failed",
                CWM_LOG_PREFIX
            );
            Vec::new()
        }
    };
    market_context_from_snapshots(&oi_snapshots, &funding_snapshots, symbol, now)
}

fn data_quality_score(window: &FlowWindow) -> u8 {
    let runtime_config = contract_whale_runtime_config();
    let active_exchange_count = window
        .venue_breakdown
        .iter()
        .filter(|(exchange, breakdown)| {
            runtime_config.exchange_enabled(exchange)
                && breakdown.aggressive_buy_btc + breakdown.aggressive_sell_btc > 0.0
        })
        .count();
    let has_active_trades = active_exchange_count > 0;
    if has_active_trades && active_exchange_count >= 2 {
        85
    } else if has_active_trades {
        70
    } else {
        40
    }
}

fn market_context_quality_score(base: u8, context: &ContractWhaleMarketContext) -> u8 {
    if !context.context_expected {
        return base;
    }
    let mut score = base;
    if !context.oi_available {
        score = score.saturating_sub(3);
    }
    if !context.funding_available {
        score = score.saturating_sub(2);
    }
    score
}

fn dominance(abs_net_volume: f64, total_volume: f64) -> f64 {
    if total_volume <= f64::EPSILON {
        0.0
    } else {
        abs_net_volume / total_volume
    }
}

fn share(part: f64, total: f64) -> f64 {
    if total <= f64::EPSILON {
        0.0
    } else {
        part.max(0.0) / total
    }
}

fn apply_net_contribution_shares(
    contributions: &mut [ExchangeFlowContribution],
    total_net_volume_btc: f64,
) {
    let net_positive = total_net_volume_btc > 0.0;
    let same_direction_net_sum = contributions
        .iter()
        .filter(|item| item.net_volume_btc.abs() > f64::EPSILON)
        .filter(|item| (item.net_volume_btc > 0.0) == net_positive)
        .map(|item| item.net_volume_btc.abs())
        .sum::<f64>();
    for item in contributions {
        item.net_contribution_share = if same_direction_net_sum > f64::EPSILON
            && item.net_volume_btc.abs() > f64::EPSILON
            && (item.net_volume_btc > 0.0) == net_positive
        {
            item.net_volume_btc.abs() / same_direction_net_sum
        } else {
            0.0
        };
    }
}

fn dominant_venue_net_contribution_share(
    contributions: &[ExchangeFlowContribution],
) -> Option<f64> {
    contributions
        .iter()
        .map(|item| item.net_contribution_share)
        .filter(|value| value.is_finite() && *value > 0.0)
        .max_by(|left, right| left.total_cmp(right))
}

fn trend_60s_from_flow_state(
    flow_state: &FlowState,
    symbol: &str,
    now: i64,
) -> ContractWhaleTrend60s {
    let Some(window) = flow_window_for_seconds(flow_state, 60) else {
        return ContractWhaleTrend60s::default();
    };
    if !symbol_matches_window(&window.symbol, symbol) {
        return ContractWhaleTrend60s::default();
    }
    let contributions = exchange_contributions(&window.venue_breakdown);
    if contributions.is_empty() {
        return ContractWhaleTrend60s::default();
    }
    let buy_volume_btc = contributions
        .iter()
        .map(|item| item.buy_volume_btc)
        .sum::<f64>()
        .max(0.0);
    let sell_volume_btc = contributions
        .iter()
        .map(|item| item.sell_volume_btc)
        .sum::<f64>()
        .max(0.0);
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    let dominance = dominance(net_volume_btc.abs(), total_volume_btc);
    let buy_ratio = if total_volume_btc > f64::EPSILON {
        buy_volume_btc / total_volume_btc
    } else {
        0.0
    };
    ContractWhaleTrend60s {
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        net_volume_btc,
        dominance,
        buy_ratio,
        sell_ratio: 1.0_f64.min(1.0 - buy_ratio).max(0.0),
        updated_at_ms: Some(window.now_ts).filter(|ts| *ts > 0).or(Some(now)),
    }
}

fn symbol_matches_window(window_symbol: &str, requested_symbol: &str) -> bool {
    let normalize = |value: &str| {
        value
            .trim()
            .split(['-', '_', '/', ':'])
            .next()
            .unwrap_or(value)
            .to_ascii_uppercase()
    };
    normalize(window_symbol) == normalize(requested_symbol)
}

fn contract_whale_health_status(
    enabled: bool,
    warmup: ContractWhaleWarmupState,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
) -> (String, String) {
    if !enabled {
        return (
            "disabled".to_string(),
            "contract_whale_monitor_disabled".to_string(),
        );
    }
    if warmup.active {
        return ("warming_up".to_string(), "warmup_collect_only".to_string());
    }

    let runtime_config = contract_whale_runtime_config();
    let enabled_exchanges = runtime_config.enabled_exchanges();
    if enabled_exchanges.is_empty() {
        return (
            "unhealthy".to_string(),
            "no_enabled_contract_exchanges".to_string(),
        );
    }
    let recent_exchange_count = enabled_exchanges
        .iter()
        .filter(|exchange| {
            let silence_ms = if exchange.as_str() == "bitfinex" {
                60_000
            } else {
                30_000
            };
            exchange_recent(exchanges.get(exchange.as_str()), now, silence_ms)
        })
        .count();
    let primary_recent = runtime_config
        .primary_contract_exchanges()
        .iter()
        .any(|exchange| exchange_recent(exchanges.get(exchange.as_str()), now, 30_000));
    let all_sources_stale = enabled_exchanges.iter().all(|exchange| {
        exchanges
            .get(exchange.as_str())
            .and_then(|status| status.last_trade_at)
            .is_none_or(|last_trade_at| now.saturating_sub(last_trade_at) > 60_000)
    });

    if recent_exchange_count == enabled_exchanges.len() && primary_recent {
        ("healthy".to_string(), "enabled_sources_recent".to_string())
    } else if recent_exchange_count > 0 {
        (
            "degraded".to_string(),
            if primary_recent {
                "partial_sources_recent".to_string()
            } else {
                "primary_source_missing".to_string()
            },
        )
    } else if all_sources_stale {
        (
            "unhealthy".to_string(),
            "all_sources_no_recent_trades".to_string(),
        )
    } else {
        (
            "unhealthy".to_string(),
            "primary_sources_disconnected".to_string(),
        )
    }
}

fn exchange_recent(
    status: Option<&ContractWhaleExchangeStatus>,
    now: i64,
    max_silence_ms: i64,
) -> bool {
    status
        .and_then(|status| status.last_trade_at)
        .is_some_and(|last_trade_at| now.saturating_sub(last_trade_at) <= max_silence_ms)
}

fn disabled_summary(
    now: i64,
    dry_run: bool,
    exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s: ContractWhaleTrend60s,
) -> ContractWhaleSummary {
    let runtime_config = contract_whale_runtime_config();
    ContractWhaleSummary {
        status: "disabled".to_string(),
        health_status: "disabled".to_string(),
        health_reason: "contract_whale_monitor_disabled".to_string(),
        market_type: "perp".to_string(),
        meta: None,
        threshold_profile: runtime_config.threshold_profile_key().to_string(),
        active_exchange_count: runtime_config.active_exchange_count(),
        enabled_exchanges: runtime_config.enabled_exchanges(),
        disabled_exchanges: runtime_config.disabled_exchanges(),
        active_contract_exchanges: runtime_config.enabled_exchanges(),
        direction: "disabled".to_string(),
        latest_direction: "disabled".to_string(),
        latest_severity: ContractWhaleSeverity::Calm,
        latest_signal_at: None,
        latest_pushed_at_ms: None,
        last_discord_sent_at: None,
        updated_at_ms: now,
        signal_count: 0,
        read_only: true,
        enabled: false,
        dry_run,
        contract_data_quality: 0,
        spot_data_quality: 0,
        overall_data_quality: 0,
        warmup: false,
        warmup_until_ms: None,
        warmup_remaining_ms: None,
        trend_60s,
        exchanges,
        platforms: build_platform_capabilities(&runtime_config),
    }
}

fn build_summary(
    items: &[ContractWhaleSignal],
    now: i64,
    enabled: bool,
    dry_run: bool,
    exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    warmup: ContractWhaleWarmupState,
    trend_60s: ContractWhaleTrend60s,
) -> ContractWhaleSummary {
    let latest = items.first();
    let latest_direction = latest
        .map(|signal| direction_key(signal.direction).to_string())
        .unwrap_or_else(|| "neutral".to_string());
    let last_discord_sent_at = items
        .iter()
        .filter(|signal| signal.discord_sent)
        .filter_map(|signal| signal.discord_sent_at.or(Some(signal.ts)))
        .max();
    let (health_status, health_reason) =
        contract_whale_health_status(enabled, warmup, &exchanges, now);
    tracing::debug!(
        target: CWM_LOG_TARGET,
        event = log_events::HEALTH_EVALUATED,
        health_status = health_status.as_str(),
        health_reason = health_reason.as_str(),
        "{} health evaluated",
        CWM_LOG_PREFIX
    );
    let runtime_config = contract_whale_runtime_config();
    let (contract_data_quality, spot_data_quality, overall_data_quality) =
        summary_data_quality_scores(&runtime_config, &exchanges, now, warmup.active);
    ContractWhaleSummary {
        status: if warmup.active {
            "warmup".to_string()
        } else {
            latest
                .map(|signal| status_code(signal.severity))
                .unwrap_or("calm")
                .to_string()
        },
        health_status,
        health_reason,
        market_type: "perp".to_string(),
        meta: None,
        threshold_profile: runtime_config.threshold_profile_key().to_string(),
        active_exchange_count: runtime_config.active_exchange_count(),
        enabled_exchanges: runtime_config.enabled_exchanges(),
        disabled_exchanges: runtime_config.disabled_exchanges(),
        active_contract_exchanges: runtime_config.enabled_exchanges(),
        direction: latest_direction.clone(),
        latest_direction,
        latest_severity: latest
            .map(|signal| signal.severity)
            .unwrap_or(ContractWhaleSeverity::Calm),
        latest_signal_at: latest.map(|signal| signal.ts),
        latest_pushed_at_ms: last_discord_sent_at,
        last_discord_sent_at,
        updated_at_ms: now,
        signal_count: items.len(),
        read_only: true,
        enabled,
        dry_run,
        contract_data_quality,
        spot_data_quality,
        overall_data_quality,
        warmup: warmup.active,
        warmup_until_ms: warmup.until_ms,
        warmup_remaining_ms: warmup.remaining_ms,
        trend_60s,
        exchanges,
        platforms: build_platform_capabilities(&runtime_config),
    }
}

fn build_platform_capabilities(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
) -> BTreeMap<String, ContractWhalePlatformCapability> {
    runtime_config
        .platform_keys()
        .into_iter()
        .map(|exchange| {
            let capability = runtime_config
                .exchange_platform(&exchange)
                .map(platform_capability_from_config)
                .unwrap_or_default();
            (exchange, capability)
        })
        .collect()
}

fn platform_capability_from_config(
    platform: &crate::contract_whale_monitor::config::ContractWhalePlatformConfig,
) -> ContractWhalePlatformCapability {
    let status = if !platform.enabled {
        "disabled"
    } else if platform.contract_markets_enabled() {
        "active"
    } else if platform.any_market_enabled() {
        "spot_only"
    } else {
        "disabled"
    };
    ContractWhalePlatformCapability {
        platform_enabled: platform.enabled,
        status: status.to_string(),
        markets: [
            market_capability_entry("spot", platform.spot.enabled, platform.spot.role.as_key()),
            market_capability_entry("perp", platform.perp.enabled, platform.perp.role.as_key()),
            market_capability_entry(
                "level2",
                platform.level2.enabled,
                platform.level2.role.as_key(),
            ),
            market_capability_entry(
                "funding",
                platform.funding.enabled,
                platform.funding.role.as_key(),
            ),
            market_capability_entry("oi", platform.oi.enabled, platform.oi.role.as_key()),
            market_capability_entry(
                "liquidation",
                platform.liquidation.enabled,
                platform.liquidation.role.as_key(),
            ),
        ]
        .into_iter()
        .collect(),
    }
}

fn market_capability_entry(
    market: &str,
    enabled: bool,
    role: &str,
) -> (String, ContractWhaleMarketCapability) {
    (
        market.to_string(),
        ContractWhaleMarketCapability {
            enabled,
            status: if enabled {
                if market == "perp" {
                    "active".to_string()
                } else {
                    "enabled".to_string()
                }
            } else {
                "disabled".to_string()
            },
            role: role.to_string(),
        },
    )
}

fn summary_data_quality_scores(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
    warmup_active: bool,
) -> (u8, u8, u8) {
    let contract_data_quality =
        contract_data_quality_score(runtime_config, exchanges, now, warmup_active);
    let spot_data_quality = spot_data_quality_score(runtime_config, exchanges, now, warmup_active);
    let overall_data_quality =
        ((contract_data_quality as f64 * 0.60) + (spot_data_quality as f64 * 0.40)).round() as u8;
    (
        contract_data_quality,
        spot_data_quality,
        overall_data_quality.min(100),
    )
}

fn contract_data_quality_score(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
    warmup_active: bool,
) -> u8 {
    let enabled_exchanges = runtime_config.enabled_exchanges();
    if enabled_exchanges.is_empty() {
        return 0;
    }
    let recent_count = enabled_exchanges
        .iter()
        .filter(|exchange| exchange_recent(exchanges.get(*exchange), now, 30_000))
        .count();
    let primary_recent = runtime_config
        .primary_contract_exchanges()
        .iter()
        .all(|exchange| exchange_recent(exchanges.get(exchange), now, 30_000));
    let mut score: u8 = if recent_count == 0 {
        20
    } else if recent_count == enabled_exchanges.len() && primary_recent {
        95
    } else if primary_recent && recent_count + 1 >= enabled_exchanges.len() {
        78
    } else if primary_recent {
        72
    } else if recent_count > 0 {
        58
    } else {
        20
    };
    if warmup_active {
        score = score.saturating_sub(20);
    }
    score
}

fn spot_data_quality_score(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
    warmup_active: bool,
) -> u8 {
    let enabled_spot_sources: Vec<String> = runtime_config
        .platform_keys()
        .into_iter()
        .filter(|exchange| {
            runtime_config.market_enabled(
                exchange,
                crate::contract_whale_monitor::types::ContractWhaleMarketType::Spot,
            )
        })
        .collect();
    if enabled_spot_sources.is_empty() {
        return 0;
    }
    let recent_count = enabled_spot_sources
        .iter()
        .filter(|exchange| exchange_recent(exchanges.get(*exchange), now, 30_000))
        .count();
    let mut score: u8 = if recent_count == 0 {
        25
    } else if recent_count == enabled_spot_sources.len() {
        92
    } else if recent_count + 1 >= enabled_spot_sources.len() {
        78
    } else {
        60
    };
    if warmup_active {
        score = score.saturating_sub(10);
    }
    score
}

fn contract_market_mismatch_meta(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    exchange: Option<&str>,
) -> Option<ContractWhaleResponseMeta> {
    let exchange = exchange?;
    let platform = runtime_config.exchange_platform(exchange)?;
    if platform.market_enabled(ContractWhaleMarketType::Perp) {
        return None;
    }
    let spot_enabled = platform.market_enabled(ContractWhaleMarketType::Spot);
    let reason = if exchange.eq_ignore_ascii_case("coinbase") && platform.enabled && spot_enabled {
        "coinbase_perp_disabled"
    } else if platform.enabled && spot_enabled {
        "perp_market_disabled"
    } else if platform.enabled {
        "contract_market_disabled"
    } else {
        "exchange_disabled"
    };
    Some(ContractWhaleResponseMeta {
        exchange: Some(exchange.to_string()),
        market_type: Some("perp".to_string()),
        exchange_status: Some(
            if platform.enabled && spot_enabled && !platform.contract_markets_enabled() {
                "spot_only".to_string()
            } else {
                "disabled".to_string()
            },
        ),
        reason: Some(reason.to_string()),
    })
}

fn is_perp_signal(signal: &ContractWhaleSignal) -> bool {
    signal.market_type == ContractWhaleMarketType::Perp
}

fn warmup_state(now: i64, enabled: bool, booted_at_ms: Option<i64>) -> ContractWhaleWarmupState {
    if !enabled {
        return ContractWhaleWarmupState::default();
    }
    let warmup_ms = contract_whale_runtime_config().data_quality.warmup_ms;
    if warmup_ms <= 0 {
        return ContractWhaleWarmupState::default();
    }
    let Some(booted_at_ms) = booted_at_ms else {
        return ContractWhaleWarmupState::default();
    };
    let until_ms = booted_at_ms.saturating_add(warmup_ms);
    let remaining_ms = until_ms.saturating_sub(now);
    ContractWhaleWarmupState {
        active: remaining_ms > 0,
        until_ms: Some(until_ms),
        remaining_ms: (remaining_ms > 0).then_some(remaining_ms),
    }
}

fn contract_exchange_statuses(
    flow_state: &FlowState,
    venue_health: Option<&VenueHealthMap>,
    enabled: bool,
    now: i64,
) -> BTreeMap<String, ContractWhaleExchangeStatus> {
    let mut statuses = default_exchange_statuses();
    let flow_last_trades = flow_last_trades(flow_state);
    let runtime_config = contract_whale_runtime_config();
    for exchange in runtime_config.platform_keys() {
        let platform = runtime_config
            .exchange_platform(&exchange)
            .expect("known contract whale platform");
        let platform_enabled = enabled && platform.any_market_enabled();
        let contract_enabled = enabled && runtime_config.exchange_enabled(&exchange);
        let last_trade_at = if contract_enabled {
            max_option(
                flow_last_trades.get(&exchange).copied().flatten(),
                health_for_exchange(venue_health, &exchange).and_then(health_last_trade_at),
            )
        } else if platform_enabled
            && platform
                .market_enabled(crate::contract_whale_monitor::types::ContractWhaleMarketType::Spot)
        {
            health_for_exchange(venue_health, &exchange).and_then(health_last_trade_at)
        } else {
            None
        };
        let reconnect_count = health_for_exchange(venue_health, &exchange)
            .map(|health| health.ws_reconnect_count.max(health.reconnect_count))
            .unwrap_or(0);
        let health_connected = health_for_exchange(venue_health, &exchange)
            .map(health_connected)
            .unwrap_or(false);
        let flow_connected = last_trade_at.is_some_and(|ts| now.saturating_sub(ts) <= 30_000);
        let connected = contract_enabled && (health_connected || flow_connected);
        let status = exchange_status_label(
            platform_enabled,
            contract_enabled,
            connected,
            health_for_exchange(venue_health, &exchange),
        );
        let latency_ms = last_trade_at
            .map(|ts| now.saturating_sub(ts).max(0))
            .filter(|latency| *latency <= 24 * 60 * 60 * 1000);
        statuses.insert(
            exchange.clone(),
            ContractWhaleExchangeStatus {
                connected,
                status: status.to_string(),
                last_trade_at,
                latency_ms,
                reconnect_count,
                platform_enabled,
                contract_enabled,
                enabled_markets: platform.enabled_markets(),
                market_roles: platform.enabled_market_roles(),
            },
        );
    }
    statuses
}

fn default_exchange_statuses() -> BTreeMap<String, ContractWhaleExchangeStatus> {
    contract_whale_runtime_config()
        .platform_keys()
        .into_iter()
        .map(|exchange| {
            (
                exchange,
                ContractWhaleExchangeStatus {
                    connected: false,
                    status: "disabled".to_string(),
                    last_trade_at: None,
                    latency_ms: None,
                    reconnect_count: 0,
                    platform_enabled: false,
                    contract_enabled: false,
                    enabled_markets: Vec::new(),
                    market_roles: BTreeMap::new(),
                },
            )
        })
        .collect()
}

fn flow_last_trades(flow_state: &FlowState) -> BTreeMap<String, Option<i64>> {
    let mut last_trades: BTreeMap<String, Option<i64>> = BTreeMap::new();
    let runtime_config = contract_whale_runtime_config();
    for window in flow_state.windows.values() {
        for (exchange, breakdown) in &window.venue_breakdown {
            if runtime_config.exchange_platform(exchange).is_none() {
                continue;
            }
            let candidate = breakdown.last_trade_ts.or_else(|| {
                let total_volume = breakdown.aggressive_buy_btc + breakdown.aggressive_sell_btc;
                (total_volume > 0.0).then_some(window.now_ts)
            });
            let current = last_trades.entry(exchange.clone()).or_insert(None);
            *current = max_option(*current, candidate);
        }
    }
    last_trades
}

fn health_for_exchange<'a>(
    venue_health: Option<&'a VenueHealthMap>,
    exchange: &str,
) -> Option<&'a VenueHealth> {
    venue_health?.get(exchange)
}

fn health_last_trade_at(health: &VenueHealth) -> Option<i64> {
    [
        health.last_trade_ts,
        health.last_trade_message_at_ms,
        health.last_parsed_trade_at_ms,
    ]
    .into_iter()
    .flatten()
    .max()
}

fn health_connected(health: &VenueHealth) -> bool {
    health.ws_connected
        || health.trade_active
        || matches!(health.status, VenueConnectionStatus::Connected)
}

fn exchange_status_label(
    platform_enabled: bool,
    contract_enabled: bool,
    connected: bool,
    health: Option<&VenueHealth>,
) -> &'static str {
    if !platform_enabled {
        return "disabled";
    }
    if !contract_enabled {
        return "spot_only";
    }
    if connected {
        return "connected";
    }
    match health.map(|item| item.status) {
        Some(VenueConnectionStatus::Reconnecting) | Some(VenueConnectionStatus::Connecting) => {
            "reconnecting"
        }
        Some(VenueConnectionStatus::Disabled) => "disabled",
        _ => "disconnected",
    }
}

fn max_option(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn status_code(severity: ContractWhaleSeverity) -> &'static str {
    match severity {
        ContractWhaleSeverity::S | ContractWhaleSeverity::Critical => "strong",
        ContractWhaleSeverity::High | ContractWhaleSeverity::Medium => "active",
        ContractWhaleSeverity::Calm => "calm",
    }
}

fn direction_key(
    direction: crate::contract_whale_monitor::types::ContractWhaleDirection,
) -> &'static str {
    match direction {
        crate::contract_whale_monitor::types::ContractWhaleDirection::Buy => "buy",
        crate::contract_whale_monitor::types::ContractWhaleDirection::Sell => "sell",
        crate::contract_whale_monitor::types::ContractWhaleDirection::Absorption => "absorption",
        crate::contract_whale_monitor::types::ContractWhaleDirection::Suppression => "suppression",
    }
}

fn severity_matches(severity: ContractWhaleSeverity, filter: Option<&str>) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    match filter.to_ascii_lowercase().as_str() {
        "s" => severity == ContractWhaleSeverity::S,
        "critical" => severity == ContractWhaleSeverity::Critical,
        "high" => severity == ContractWhaleSeverity::High,
        "medium" => severity == ContractWhaleSeverity::Medium,
        _ => true,
    }
}

pub fn parse_history_query(
    query: &ContractWhaleQuery,
) -> Result<ContractWhaleSignalQuery, (StatusCode, Json<serde_json::Value>)> {
    let from_ts = parse_optional_i64(query.from.as_deref(), "from")?;
    let to_ts = parse_optional_i64(query.to.as_deref(), "to")?;
    if let (Some(from_ts), Some(to_ts)) = (from_ts, to_ts) {
        if from_ts > to_ts {
            return Err(bad_request("from_must_be_before_to"));
        }
    }
    Ok(ContractWhaleSignalQuery {
        symbol: parse_symbol_filter(query.symbol.as_deref())?,
        severity: parse_severity_filter(query.severity.as_deref())?,
        signal_type: parse_signal_type_filter(query.signal_type.as_deref())?,
        direction: parse_direction_filter(query.direction.as_deref())?,
        discord_sent: parse_optional_bool(query.discord_sent.as_deref(), "discord_sent")?,
        window_sec: parse_window_sec_filter(query.window_sec.as_deref())?,
        exchange: parse_exchange_filter(query.exchange.as_deref())?,
        from_ts,
        to_ts,
        limit: parse_limit(query.limit.as_deref(), 50, 200)?,
        offset: parse_offset(query.offset.as_deref())?,
    })
}

fn parse_symbol_for_latest(
    symbol: Option<&str>,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    Ok(parse_symbol_filter(symbol)?.unwrap_or_else(|| "BTC".to_string()))
}

fn parse_symbol_filter(
    symbol: Option<&str>,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(symbol) = symbol.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !symbol
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/'))
    {
        return Err(bad_request("symbol_invalid"));
    }
    Ok(Some(symbol.to_ascii_uppercase()))
}

fn parse_severity_filter(
    filter: Option<&str>,
) -> Result<Option<ContractWhaleSeverity>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match filter.to_ascii_lowercase().as_str() {
        "s" => Ok(Some(ContractWhaleSeverity::S)),
        "critical" => Ok(Some(ContractWhaleSeverity::Critical)),
        "high" => Ok(Some(ContractWhaleSeverity::High)),
        "medium" => Ok(Some(ContractWhaleSeverity::Medium)),
        "calm" => Ok(Some(ContractWhaleSeverity::Calm)),
        _ => Err(bad_request("severity_invalid")),
    }
}

fn parse_signal_type_filter(
    filter: Option<&str>,
) -> Result<Option<ContractWhaleSignalType>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match normalize_token(filter).as_str() {
        "aggressivebuy" | "aggressive_buy" => Ok(Some(ContractWhaleSignalType::AggressiveBuy)),
        "aggressivesell" | "aggressive_sell" => Ok(Some(ContractWhaleSignalType::AggressiveSell)),
        "downsideabsorption" | "downside_absorption" => {
            Ok(Some(ContractWhaleSignalType::DownsideAbsorption))
        }
        "upsidesuppression" | "upside_suppression" => {
            Ok(Some(ContractWhaleSignalType::UpsideSuppression))
        }
        _ => Err(bad_request("signal_type_invalid")),
    }
}

fn parse_direction_filter(
    filter: Option<&str>,
) -> Result<Option<ContractWhaleDirection>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match normalize_token(filter).as_str() {
        "buy" | "long" | "activebuy" | "active_buy" | "aggressivebuy" | "aggressive_buy"
        | "主动买入" => Ok(Some(ContractWhaleDirection::Buy)),
        "sell" | "short" | "activesell" | "active_sell" | "aggressivesell" | "aggressive_sell"
        | "主动卖出" => Ok(Some(ContractWhaleDirection::Sell)),
        "absorption" | "downsideabsorption" | "downside_absorption" => {
            Ok(Some(ContractWhaleDirection::Absorption))
        }
        "suppression" | "upsidesuppression" | "upside_suppression" => {
            Ok(Some(ContractWhaleDirection::Suppression))
        }
        _ => Err(bad_request("direction_invalid")),
    }
}

fn parse_exchange_filter(
    filter: Option<&str>,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match filter.to_ascii_lowercase().as_str() {
        "binance" | "okx" | "bitfinex" | "coinbase" => Ok(Some(filter.to_ascii_lowercase())),
        _ => Err(bad_request("exchange_invalid")),
    }
}

fn parse_window_sec_filter(
    filter: Option<&str>,
) -> Result<Option<u64>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match filter.parse::<u64>() {
        Ok(window_sec @ (5 | 15 | 60)) => Ok(Some(window_sec)),
        _ => Err(bad_request("window_sec_invalid")),
    }
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' ', '/'], "_")
}

fn parse_limit(
    value: Option<&str>,
    default: usize,
    max: usize,
) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(default);
    };
    let limit = value
        .parse::<usize>()
        .map_err(|_| bad_request("limit_invalid"))?;
    Ok(limit.clamp(1, max))
}

fn parse_offset(value: Option<&str>) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };
    value
        .parse::<usize>()
        .map_err(|_| bad_request("offset_invalid"))
}

fn parse_optional_bool(
    value: Option<&str>,
    field: &str,
) -> Result<Option<bool>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(Some(true)),
        "false" | "0" | "no" => Ok(Some(false)),
        _ => Err(bad_request(&format!("{field}_invalid"))),
    }
}

fn parse_optional_i64(
    value: Option<&str>,
    field: &str,
) -> Result<Option<i64>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| bad_request(&format!("{field}_invalid")))
}

fn bad_request(reason: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "bad_request",
            "reason": reason,
            "readOnly": true,
            "executionEnabled": false
        })),
    )
}
