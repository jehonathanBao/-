use std::{
    collections::BTreeMap,
    sync::{OnceLock, RwLock},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{
    log_events,
    normalizer::{
        normalize_okx_funding_rate_json_for_inst, normalize_okx_liquidation_order_json_for_inst,
        normalize_okx_open_interest_json_for_inst, okx_usdt_swap_inst_id,
    },
    types::{ContractFundingSnapshot, ContractLiquidationOrder, ContractOiSnapshot},
    LOG_PREFIX, LOG_TARGET,
};

pub const OKX_BTC_USDT_SWAP_TRADES_CHANNEL: &str = "trades:BTC-USDT-SWAP";
pub const OKX_BTC_USDT_SWAP_LIQUIDATION_ORDERS_CHANNEL: &str = "liquidation-orders:BTC-USDT-SWAP";
pub const OKX_PUBLIC_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";
pub const OKX_BTC_USDT_SWAP_OPEN_INTEREST_URL: &str =
    "https://www.okx.com/api/v5/public/open-interest?instType=SWAP&instId=BTC-USDT-SWAP";
pub const OKX_BTC_USDT_SWAP_FUNDING_RATE_URL: &str =
    "https://www.okx.com/api/v5/public/funding-rate?instId=BTC-USDT-SWAP";

#[derive(Debug, Clone, PartialEq)]
pub struct OkxContractValueResolution {
    pub ct_val_base: Option<f64>,
    pub ct_val_available: bool,
    pub evidence_degraded: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OkxInstrumentMetadata {
    pub inst_id: String,
    pub ct_val: f64,
    pub ct_val_ccy: String,
    pub lot_sz: Option<f64>,
    pub tick_sz: Option<f64>,
    pub state: Option<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct OkxInstrumentMetadataCache {
    pub instruments: BTreeMap<String, OkxInstrumentMetadata>,
    pub updated_at: i64,
}

static OKX_INSTRUMENT_METADATA_CACHE: OnceLock<RwLock<OkxInstrumentMetadataCache>> =
    OnceLock::new();

pub fn resolve_okx_contract_value(
    metadata_ct_val_base: Option<f64>,
    fallback_ct_val_base: Option<f64>,
    _symbol: &str,
) -> OkxContractValueResolution {
    if metadata_ct_val_base.is_some_and(|value| value.is_finite() && value > 0.0) {
        return OkxContractValueResolution {
            ct_val_base: metadata_ct_val_base,
            ct_val_available: true,
            evidence_degraded: false,
            reason: None,
        };
    }

    let fallback = fallback_ct_val_base.filter(|value| value.is_finite() && *value > 0.0);
    OkxContractValueResolution {
        ct_val_base: fallback,
        ct_val_available: false,
        evidence_degraded: fallback.is_some(),
        reason: fallback.map(|_| "okx_metadata_fallback".to_string()),
    }
}

fn okx_instrument_metadata_cache() -> &'static RwLock<OkxInstrumentMetadataCache> {
    OKX_INSTRUMENT_METADATA_CACHE.get_or_init(|| RwLock::new(OkxInstrumentMetadataCache::default()))
}

pub async fn resolve_okx_contract_value_with_cache(
    client: &reqwest::Client,
    symbol: &str,
    metadata_enabled: bool,
    refresh_minutes: i64,
    fallback_ct_val_base: Option<f64>,
) -> anyhow::Result<OkxContractValueResolution> {
    let key = okx_usdt_swap_inst_id(symbol);
    let now = crate::normalizers::trade::now_ms();
    let refresh_ms = refresh_minutes.max(1).saturating_mul(60_000);
    if metadata_enabled {
        if let Some(metadata) = okx_instrument_metadata_cache()
            .read()
            .ok()
            .and_then(|cache| {
                (now.saturating_sub(cache.updated_at) <= refresh_ms)
                    .then(|| cache.instruments.get(&key).cloned())
                    .flatten()
            })
        {
            return Ok(resolve_okx_contract_value(
                Some(metadata.ct_val),
                fallback_ct_val_base,
                symbol,
            ));
        }
    }

    if !metadata_enabled {
        let mut resolution = resolve_okx_contract_value(None, fallback_ct_val_base, symbol);
        if resolution.ct_val_base.is_some() {
            resolution.reason = Some("okx_metadata_disabled_fallback".to_string());
        }
        return Ok(resolution);
    }

    match fetch_okx_instrument_metadata(client, symbol).await {
        Ok(Some(metadata)) => {
            if let Ok(mut cache) = okx_instrument_metadata_cache().write() {
                cache.updated_at = now;
                cache.instruments.insert(key, metadata.clone());
            }
            Ok(resolve_okx_contract_value(
                Some(metadata.ct_val),
                fallback_ct_val_base,
                symbol,
            ))
        }
        Ok(None) => {
            let mut resolution = resolve_okx_contract_value(None, fallback_ct_val_base, symbol);
            if resolution.ct_val_base.is_some() {
                resolution.reason = Some("okx_metadata_missing_fallback".to_string());
            }
            Ok(resolution)
        }
        Err(error) => {
            let mut resolution = resolve_okx_contract_value(None, fallback_ct_val_base, symbol);
            if resolution.ct_val_base.is_some() {
                resolution.reason = Some("okx_metadata_fetch_failed".to_string());
                Ok(resolution)
            } else {
                Err(error)
            }
        }
    }
}

pub fn collector_status() -> &'static str {
    "defined_not_started"
}

pub async fn run_okx_liquidation_collector(
    sender: mpsc::Sender<ContractLiquidationOrder>,
    ct_val_btc: f64,
) {
    let mut reconnect_attempt = 0_u32;
    loop {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::WS_CONNECTED,
            "{} connecting okx liquidation-orders stream",
            LOG_PREFIX
        );
        match connect_async(OKX_PUBLIC_WS_URL).await {
            Ok((ws, _)) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    event = log_events::WS_CONNECTED,
                    "{} okx liquidation-orders stream connected",
                    LOG_PREFIX
                );
                let (mut write, mut read) = ws.split();
                let subscribe = serde_json::json!({
                    "op": "subscribe",
                    "args": [{
                        "channel": "liquidation-orders",
                        "instType": "SWAP",
                        "uly": "BTC-USDT"
                    }]
                });
                if let Err(error) = write.send(Message::Text(subscribe.to_string())).await {
                    tracing::warn!(
                        target: LOG_TARGET,
                        event = log_events::WS_DISCONNECTED,
                        error = %error,
                        "{} okx liquidation subscribe failed",
                        LOG_PREFIX
                    );
                }
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                for order in handle_liquidation_order_message(text, ct_val_btc) {
                                    if sender.send(order).await.is_err() {
                                        tracing::warn!(
                                            target: LOG_TARGET,
                                            event = log_events::WS_DISCONNECTED,
                                            "{} okx liquidation receiver dropped",
                                            LOG_PREFIX
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: LOG_TARGET,
                                event = log_events::WS_DISCONNECTED,
                                error = %error,
                                "{} okx liquidation stream disconnected",
                                LOG_PREFIX
                            );
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    event = log_events::WS_DISCONNECTED,
                    error = %error,
                    "{} okx liquidation connect failed",
                    LOG_PREFIX
                );
            }
        }
        reconnect_attempt = reconnect_attempt.saturating_add(1);
        let next_delay_ms = super::collector_binance::reconnect_delay_ms(reconnect_attempt, 29);
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::WS_DISCONNECTED,
            exchange = "okx",
            attempt = reconnect_attempt,
            next_delay_ms,
            "{} okx liquidation reconnect scheduled",
            LOG_PREFIX
        );
        tokio::time::sleep(Duration::from_millis(next_delay_ms)).await;
    }
}

pub fn handle_liquidation_order_message(
    text: &str,
    ct_val_btc: f64,
) -> Vec<ContractLiquidationOrder> {
    handle_liquidation_order_message_for_inst("BTC-USDT-SWAP", text, ct_val_btc)
}

pub fn handle_liquidation_order_message_for_inst(
    inst_id: &str,
    text: &str,
    ct_val_base: f64,
) -> Vec<ContractLiquidationOrder> {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    normalize_okx_liquidation_order_json_for_inst(inst_id, &payload, ct_val_base)
}

pub fn okx_open_interest_url(symbol: &str) -> String {
    format!(
        "https://www.okx.com/api/v5/public/open-interest?instType=SWAP&instId={}",
        okx_usdt_swap_inst_id(symbol)
    )
}

pub fn okx_funding_rate_url(symbol: &str) -> String {
    format!(
        "https://www.okx.com/api/v5/public/funding-rate?instId={}",
        okx_usdt_swap_inst_id(symbol)
    )
}

pub fn okx_instruments_url(symbol: &str) -> String {
    format!(
        "https://www.okx.com/api/v5/public/instruments?instType=SWAP&instId={}",
        okx_usdt_swap_inst_id(symbol)
    )
}

pub async fn fetch_okx_contract_value_base(
    client: &reqwest::Client,
    symbol: &str,
) -> anyhow::Result<Option<f64>> {
    let payload = client
        .get(okx_instruments_url(symbol))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    Ok(parse_okx_contract_value_base(&payload, symbol))
}

pub async fn fetch_okx_instrument_metadata(
    client: &reqwest::Client,
    symbol: &str,
) -> anyhow::Result<Option<OkxInstrumentMetadata>> {
    let payload = client
        .get(okx_instruments_url(symbol))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    Ok(parse_okx_instrument_metadata(&payload, symbol))
}

pub fn parse_okx_instrument_metadata(
    payload: &serde_json::Value,
    symbol: &str,
) -> Option<OkxInstrumentMetadata> {
    let expected_inst = okx_usdt_swap_inst_id(symbol);
    let expected_base = symbol.trim().to_ascii_uppercase();
    payload
        .get("data")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                let inst_id = item.get("instId")?.as_str()?.to_string();
                let ct_val_ccy = item.get("ctValCcy")?.as_str()?.to_string();
                let ct_val = item.get("ctVal").and_then(parse_f64)?;
                if !inst_id.eq_ignore_ascii_case(&expected_inst)
                    || !ct_val_ccy.eq_ignore_ascii_case(&expected_base)
                    || !ct_val.is_finite()
                    || ct_val <= 0.0
                {
                    return None;
                }
                Some(OkxInstrumentMetadata {
                    inst_id,
                    ct_val,
                    ct_val_ccy,
                    lot_sz: item.get("lotSz").and_then(parse_f64),
                    tick_sz: item.get("tickSz").and_then(parse_f64),
                    state: item
                        .get("state")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    updated_at: item.get("uTime").and_then(parse_i64),
                })
            })
        })
}

fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
}

fn parse_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<i64>().ok()))
}

pub fn parse_okx_contract_value_base(payload: &serde_json::Value, symbol: &str) -> Option<f64> {
    let expected_inst = okx_usdt_swap_inst_id(symbol);
    let expected_base = symbol.trim().to_ascii_uppercase();
    payload
        .get("data")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                let inst_matches = item
                    .get("instId")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(&expected_inst));
                let ccy_matches = item
                    .get("ctValCcy")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(&expected_base));
                item.get("ctVal")
                    .and_then(|value| {
                        value
                            .as_f64()
                            .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
                    })
                    .filter(|value| {
                        inst_matches && ccy_matches && value.is_finite() && *value > 0.0
                    })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::{
        parse_okx_contract_value_base, parse_okx_instrument_metadata, resolve_okx_contract_value,
    };

    #[test]
    fn configured_ctval_fallback_is_explicitly_degraded() {
        let resolved = resolve_okx_contract_value(None, Some(0.01), "BTC");

        assert_eq!(resolved.ct_val_base, Some(0.01));
        assert!(!resolved.ct_val_available);
        assert!(resolved.evidence_degraded);
        assert_eq!(resolved.reason.as_deref(), Some("okx_metadata_fallback"));
    }

    #[test]
    fn live_ctval_metadata_is_not_marked_degraded() {
        let resolved = resolve_okx_contract_value(Some(0.01), Some(0.02), "BTC");

        assert_eq!(resolved.ct_val_base, Some(0.01));
        assert!(resolved.ct_val_available);
        assert!(!resolved.evidence_degraded);
        assert_eq!(resolved.reason, None);
    }

    #[test]
    fn instrument_metadata_parser_keeps_quality_fields() {
        let payload = serde_json::json!({
            "data": [{
                "instId": "BTC-USDT-SWAP",
                "ctValCcy": "BTC",
                "ctVal": "0.01",
                "lotSz": "1",
                "tickSz": "0.1",
                "state": "live",
                "uTime": "1700000000000"
            }]
        });
        let metadata = parse_okx_instrument_metadata(&payload, "BTC").expect("metadata");

        assert_eq!(metadata.ct_val, 0.01);
        assert_eq!(metadata.lot_sz, Some(1.0));
        assert_eq!(metadata.tick_sz, Some(0.1));
        assert_eq!(metadata.state.as_deref(), Some("live"));
        assert_eq!(metadata.updated_at, Some(1_700_000_000_000));
    }

    #[test]
    fn instrument_metadata_accepts_only_matching_positive_ctval() {
        let payload = serde_json::json!({
            "data": [{"instId": "ETH-USDT-SWAP", "ctValCcy": "ETH", "ctVal": "0.1"}]
        });
        assert_eq!(parse_okx_contract_value_base(&payload, "ETH"), Some(0.1));

        let wrong_ccy = serde_json::json!({
            "data": [{"instId": "ETH-USDT-SWAP", "ctValCcy": "BTC", "ctVal": "0.1"}]
        });
        assert_eq!(parse_okx_contract_value_base(&wrong_ccy, "ETH"), None);

        let invalid = serde_json::json!({
            "data": [{"instId": "ETH-USDT-SWAP", "ctValCcy": "ETH", "ctVal": "0"}]
        });
        assert_eq!(parse_okx_contract_value_base(&invalid, "ETH"), None);
    }
}

pub async fn fetch_okx_open_interest_snapshot(
    client: &reqwest::Client,
    ct_val_btc: f64,
) -> anyhow::Result<Option<ContractOiSnapshot>> {
    fetch_okx_open_interest_snapshot_for_symbol(client, "BTC", ct_val_btc).await
}

pub async fn fetch_okx_open_interest_snapshot_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    ct_val_base: f64,
) -> anyhow::Result<Option<ContractOiSnapshot>> {
    let payload = client
        .get(okx_open_interest_url(symbol))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_okx_open_interest_json_for_inst(
        &okx_usdt_swap_inst_id(symbol),
        &payload,
        ct_val_base,
    ))
}

pub async fn fetch_okx_funding_snapshot(
    client: &reqwest::Client,
) -> anyhow::Result<Option<ContractFundingSnapshot>> {
    fetch_okx_funding_snapshot_for_symbol(client, "BTC").await
}

pub async fn fetch_okx_funding_snapshot_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
) -> anyhow::Result<Option<ContractFundingSnapshot>> {
    let payload = client
        .get(okx_funding_rate_url(symbol))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_okx_funding_rate_json_for_inst(
        &okx_usdt_swap_inst_id(symbol),
        &payload,
    ))
}
