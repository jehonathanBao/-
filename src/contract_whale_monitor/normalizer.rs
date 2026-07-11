use serde_json::Value;

use super::types::{
    ContractExchange, ContractFundingSnapshot, ContractLiquidationOrder, ContractLiquidationSide,
    ContractOiSnapshot, ContractTrade, ContractTradeSide,
};

pub fn normalize_binance_agg_trade(
    ts: i64,
    price: f64,
    qty_btc: f64,
    buyer_is_market_maker: bool,
) -> Option<ContractTrade> {
    normalize_binance_agg_trade_for_symbol(ts, "BTC", price, qty_btc, buyer_is_market_maker)
}

pub fn normalize_binance_agg_trade_for_symbol(
    ts: i64,
    symbol: &str,
    price: f64,
    qty_base: f64,
    buyer_is_market_maker: bool,
) -> Option<ContractTrade> {
    build_trade(
        symbol,
        ts,
        ContractExchange::Binance,
        price,
        qty_base,
        if buyer_is_market_maker {
            ContractTradeSide::Sell
        } else {
            ContractTradeSide::Buy
        },
        Some(1),
    )
}

pub fn normalize_okx_swap_trade(
    ts: i64,
    price: f64,
    size_contracts: f64,
    ct_val_btc: f64,
    taker_side: &str,
) -> Option<ContractTrade> {
    normalize_okx_swap_trade_for_symbol(ts, "BTC", price, size_contracts, ct_val_btc, taker_side)
}

pub fn normalize_okx_swap_trade_for_symbol(
    ts: i64,
    symbol: &str,
    price: f64,
    size_contracts: f64,
    ct_val_base: f64,
    taker_side: &str,
) -> Option<ContractTrade> {
    let side = match taker_side {
        "buy" | "Buy" => ContractTradeSide::Buy,
        "sell" | "Sell" => ContractTradeSide::Sell,
        _ => return None,
    };
    build_trade(
        symbol,
        ts,
        ContractExchange::Okx,
        price,
        size_contracts * ct_val_base,
        side,
        Some(1),
    )
}

pub fn normalize_bitfinex_trade(ts: i64, price: f64, amount_btc: f64) -> Option<ContractTrade> {
    normalize_bitfinex_trade_for_symbol(ts, "BTC", price, amount_btc)
}

pub fn normalize_bitfinex_trade_for_symbol(
    ts: i64,
    symbol: &str,
    price: f64,
    amount_base: f64,
) -> Option<ContractTrade> {
    let side = if amount_base >= 0.0 {
        ContractTradeSide::Buy
    } else {
        ContractTradeSide::Sell
    };
    build_trade(
        symbol,
        ts,
        ContractExchange::Bitfinex,
        price,
        amount_base.abs(),
        side,
        Some(1),
    )
}

pub fn normalize_binance_force_order(
    ts: i64,
    price: f64,
    qty_btc: f64,
    order_side: &str,
) -> Option<ContractLiquidationOrder> {
    normalize_binance_force_order_for_symbol(ts, "BTC", price, qty_btc, order_side)
}

pub fn normalize_binance_force_order_for_symbol(
    ts: i64,
    symbol: &str,
    price: f64,
    qty_base: f64,
    order_side: &str,
) -> Option<ContractLiquidationOrder> {
    let side = match order_side {
        "SELL" | "sell" => ContractLiquidationSide::Long,
        "BUY" | "buy" => ContractLiquidationSide::Short,
        _ => return None,
    };
    build_liquidation(symbol, ts, ContractExchange::Binance, price, qty_base, side)
}

pub fn normalize_binance_force_order_json(payload: &Value) -> Option<ContractLiquidationOrder> {
    normalize_binance_force_order_json_for_symbol("BTC", payload)
}

pub fn normalize_binance_force_order_json_for_symbol(
    expected_symbol: &str,
    payload: &Value,
) -> Option<ContractLiquidationOrder> {
    let order = payload.get("o")?;
    let raw_symbol = order.get("s")?.as_str()?;
    if !raw_symbol.eq_ignore_ascii_case(&binance_usdt_perp_symbol(expected_symbol)) {
        return None;
    }
    let ts = order
        .get("T")
        .or_else(|| payload.get("E"))
        .and_then(Value::as_i64)?;
    let price = parse_json_f64(order.get("ap").or_else(|| order.get("p"))?)?;
    let qty_btc = parse_json_f64(order.get("z").or_else(|| order.get("q"))?)?;
    let side = order.get("S")?.as_str()?;
    normalize_binance_force_order_for_symbol(ts, expected_symbol, price, qty_btc, side)
}

pub fn normalize_okx_liquidation_order(
    ts: i64,
    price: f64,
    size_contracts: f64,
    ct_val_btc: f64,
    side_hint: &str,
) -> Option<ContractLiquidationOrder> {
    normalize_okx_liquidation_order_for_symbol(
        ts,
        "BTC",
        price,
        size_contracts,
        ct_val_btc,
        side_hint,
    )
}

pub fn normalize_okx_liquidation_order_for_symbol(
    ts: i64,
    symbol: &str,
    price: f64,
    size_contracts: f64,
    ct_val_base: f64,
    side_hint: &str,
) -> Option<ContractLiquidationOrder> {
    let side = okx_liquidation_side(side_hint)?;
    build_liquidation(
        symbol,
        ts,
        ContractExchange::Okx,
        price,
        size_contracts * ct_val_base,
        side,
    )
}

pub fn normalize_okx_liquidation_order_json(
    payload: &Value,
    ct_val_btc: f64,
) -> Vec<ContractLiquidationOrder> {
    normalize_okx_liquidation_order_json_for_inst("BTC-USDT-SWAP", payload, ct_val_btc)
}

pub fn normalize_okx_liquidation_order_json_for_inst(
    expected_inst_id: &str,
    payload: &Value,
    ct_val_base: f64,
) -> Vec<ContractLiquidationOrder> {
    payload
        .get("data")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .flat_map(|item| {
                    normalize_okx_liquidation_data_item(item, ct_val_base, expected_inst_id)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn normalize_binance_open_interest_json(
    payload: &Value,
    mark_price: Option<f64>,
    fallback_ts: i64,
) -> Option<ContractOiSnapshot> {
    normalize_binance_open_interest_json_for_symbol("BTC", payload, mark_price, fallback_ts)
}

pub fn normalize_binance_open_interest_json_for_symbol(
    expected_symbol: &str,
    payload: &Value,
    mark_price: Option<f64>,
    fallback_ts: i64,
) -> Option<ContractOiSnapshot> {
    let symbol = payload.get("symbol").and_then(Value::as_str)?;
    if !symbol.eq_ignore_ascii_case(&binance_usdt_perp_symbol(expected_symbol)) {
        return None;
    }
    let oi_base = parse_json_f64(payload.get("openInterest")?)?;
    let ts = payload
        .get("time")
        .or_else(|| payload.get("E"))
        .and_then(Value::as_i64)
        .unwrap_or(fallback_ts);
    build_oi_snapshot(
        expected_symbol,
        ts,
        ContractExchange::Binance,
        oi_base,
        mark_price,
    )
}

pub fn normalize_okx_open_interest_json(
    payload: &Value,
    ct_val_btc: f64,
) -> Option<ContractOiSnapshot> {
    normalize_okx_open_interest_json_for_inst("BTC-USDT-SWAP", payload, ct_val_btc)
}

pub fn normalize_okx_open_interest_json_for_inst(
    expected_inst_id: &str,
    payload: &Value,
    ct_val_base: f64,
) -> Option<ContractOiSnapshot> {
    let item = payload
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .or(Some(payload))?;
    let inst_id = item
        .get("instId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !inst_id.eq_ignore_ascii_case(expected_inst_id) {
        return None;
    }
    let ts = item.get("ts").and_then(parse_json_i64)?;
    let oi_base = item.get("oiCcy").and_then(parse_json_f64).or_else(|| {
        item.get("oi")
            .and_then(parse_json_f64)
            .map(|oi| oi * ct_val_base)
    })?;
    let oi_notional_usd = item
        .get("oiUsd")
        .and_then(parse_json_f64)
        .filter(|value| *value > 0.0);
    build_oi_snapshot_with_notional(
        okx_inst_id_to_symbol(expected_inst_id),
        ts,
        ContractExchange::Okx,
        oi_base,
        oi_notional_usd,
    )
}

pub fn normalize_binance_funding_rate_json(
    payload: &Value,
    fallback_ts: i64,
) -> Option<ContractFundingSnapshot> {
    normalize_binance_funding_rate_json_for_symbol("BTC", payload, fallback_ts)
}

pub fn normalize_binance_funding_rate_json_for_symbol(
    expected_symbol: &str,
    payload: &Value,
    fallback_ts: i64,
) -> Option<ContractFundingSnapshot> {
    let symbol = payload.get("symbol").and_then(Value::as_str)?;
    if !symbol.eq_ignore_ascii_case(&binance_usdt_perp_symbol(expected_symbol)) {
        return None;
    }
    let funding_rate = payload
        .get("lastFundingRate")
        .or_else(|| payload.get("fundingRate"))
        .and_then(parse_json_f64)?;
    let ts = payload
        .get("time")
        .or_else(|| payload.get("E"))
        .and_then(Value::as_i64)
        .unwrap_or(fallback_ts);
    build_funding_snapshot(expected_symbol, ts, ContractExchange::Binance, funding_rate)
}

pub fn normalize_okx_funding_rate_json(payload: &Value) -> Option<ContractFundingSnapshot> {
    normalize_okx_funding_rate_json_for_inst("BTC-USDT-SWAP", payload)
}

pub fn normalize_okx_funding_rate_json_for_inst(
    expected_inst_id: &str,
    payload: &Value,
) -> Option<ContractFundingSnapshot> {
    let item = payload
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .or(Some(payload))?;
    let inst_id = item
        .get("instId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !inst_id.eq_ignore_ascii_case(expected_inst_id) {
        return None;
    }
    let funding_rate = item.get("fundingRate").and_then(parse_json_f64)?;
    let ts = item.get("ts").and_then(parse_json_i64)?;
    build_funding_snapshot(
        okx_inst_id_to_symbol(expected_inst_id),
        ts,
        ContractExchange::Okx,
        funding_rate,
    )
}

fn build_trade(
    symbol: &str,
    ts: i64,
    exchange: ContractExchange,
    price: f64,
    qty_base: f64,
    side: ContractTradeSide,
    raw_trade_count: Option<u64>,
) -> Option<ContractTrade> {
    if ts <= 0 || !price.is_finite() || !qty_base.is_finite() || price <= 0.0 || qty_base <= 0.0 {
        return None;
    }
    Some(ContractTrade {
        ts,
        exchange,
        symbol: normalize_contract_symbol(symbol),
        market: "perp".to_string(),
        price,
        qty_btc: qty_base,
        notional_usd: price * qty_base,
        side,
        raw_trade_count,
    })
}

fn normalize_okx_liquidation_data_item(
    item: &Value,
    ct_val_base: f64,
    expected_inst_id: &str,
) -> Vec<ContractLiquidationOrder> {
    if !okx_item_matches_inst(item, expected_inst_id) {
        return Vec::new();
    }
    let details = item
        .get("details")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty());
    match details {
        Some(details) => details
            .iter()
            .filter_map(|detail| {
                okx_liquidation_from_value(detail, Some(item), ct_val_base, expected_inst_id)
            })
            .collect(),
        None => okx_liquidation_from_value(item, None, ct_val_base, expected_inst_id)
            .into_iter()
            .collect(),
    }
}

fn okx_liquidation_from_value(
    value: &Value,
    parent: Option<&Value>,
    ct_val_base: f64,
    expected_inst_id: &str,
) -> Option<ContractLiquidationOrder> {
    let ts = value
        .get("ts")
        .or_else(|| parent.and_then(|item| item.get("ts")))
        .and_then(parse_json_i64)?;
    let price = value
        .get("bkPx")
        .or_else(|| value.get("price"))
        .or_else(|| value.get("px"))
        .and_then(parse_json_f64)?;
    let size_contracts = value
        .get("sz")
        .or_else(|| value.get("size"))
        .and_then(parse_json_f64)?;
    let side_hint = value
        .get("posSide")
        .or_else(|| value.get("side"))
        .or_else(|| parent.and_then(|item| item.get("posSide")))
        .or_else(|| parent.and_then(|item| item.get("side")))
        .and_then(Value::as_str)?;
    let symbol = okx_inst_id_to_symbol(expected_inst_id);
    normalize_okx_liquidation_order_for_symbol(
        ts,
        &symbol,
        price,
        size_contracts,
        ct_val_base,
        side_hint,
    )
}

fn okx_item_matches_inst(item: &Value, expected_inst_id: &str) -> bool {
    let expected_uly = okx_inst_id_to_uly(expected_inst_id);
    item.get("instId")
        .and_then(Value::as_str)
        .is_some_and(|inst_id| inst_id.eq_ignore_ascii_case(expected_inst_id))
        || item
            .get("uly")
            .and_then(Value::as_str)
            .is_some_and(|uly| uly.eq_ignore_ascii_case(&expected_uly))
}

fn okx_liquidation_side(side_hint: &str) -> Option<ContractLiquidationSide> {
    match side_hint.to_ascii_lowercase().as_str() {
        "long" | "sell" => Some(ContractLiquidationSide::Long),
        "short" | "buy" => Some(ContractLiquidationSide::Short),
        _ => None,
    }
}

fn build_liquidation(
    symbol: &str,
    ts: i64,
    exchange: ContractExchange,
    price: f64,
    qty_base: f64,
    side: ContractLiquidationSide,
) -> Option<ContractLiquidationOrder> {
    if ts <= 0 || !price.is_finite() || !qty_base.is_finite() || price <= 0.0 || qty_base <= 0.0 {
        return None;
    }
    Some(ContractLiquidationOrder {
        ts,
        exchange,
        symbol: normalize_contract_symbol(symbol),
        price,
        qty_btc: qty_base,
        notional_usd: price * qty_base,
        side,
    })
}

fn build_oi_snapshot(
    symbol: &str,
    ts: i64,
    exchange: ContractExchange,
    oi_base: f64,
    mark_price: Option<f64>,
) -> Option<ContractOiSnapshot> {
    let oi_notional_usd = mark_price
        .filter(|price| price.is_finite() && *price > 0.0)
        .map(|price| price * oi_base);
    build_oi_snapshot_with_notional(symbol, ts, exchange, oi_base, oi_notional_usd)
}

fn build_oi_snapshot_with_notional(
    symbol: impl AsRef<str>,
    ts: i64,
    exchange: ContractExchange,
    oi_base: f64,
    oi_notional_usd: Option<f64>,
) -> Option<ContractOiSnapshot> {
    if ts <= 0 || !oi_base.is_finite() || oi_base <= 0.0 {
        return None;
    }
    Some(ContractOiSnapshot {
        ts,
        exchange,
        symbol: normalize_contract_symbol(symbol.as_ref()),
        oi_btc: oi_base,
        oi_notional_usd,
        ct_val_available: true,
        evidence_degraded_reason: None,
    })
}

fn build_funding_snapshot(
    symbol: impl AsRef<str>,
    ts: i64,
    exchange: ContractExchange,
    funding_rate: f64,
) -> Option<ContractFundingSnapshot> {
    if ts <= 0 || !funding_rate.is_finite() {
        return None;
    }
    Some(ContractFundingSnapshot {
        ts,
        exchange,
        symbol: normalize_contract_symbol(symbol.as_ref()),
        funding_rate,
    })
}

pub fn binance_usdt_perp_symbol(symbol: &str) -> String {
    format!("{}USDT", normalize_contract_symbol(symbol))
}

pub fn okx_usdt_swap_inst_id(symbol: &str) -> String {
    format!("{}-USDT-SWAP", normalize_contract_symbol(symbol))
}

fn okx_inst_id_to_symbol(inst_id: &str) -> String {
    normalize_contract_symbol(inst_id)
}

fn okx_inst_id_to_uly(inst_id: &str) -> String {
    let symbol = normalize_contract_symbol(inst_id);
    format!("{symbol}-USDT")
}

fn normalize_contract_symbol(symbol: &str) -> String {
    let upper = symbol.trim().to_ascii_uppercase();
    let first = upper
        .split([':', '/', '_'])
        .next()
        .unwrap_or(upper.as_str());
    let base = first.split('-').next().unwrap_or(first);
    base.trim_end_matches("PERP")
        .trim_end_matches("USDT")
        .trim_end_matches("USD")
        .trim_end_matches("F0")
        .to_string()
}

fn parse_json_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        .filter(|number| number.is_finite())
}

fn parse_json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<i64>().ok()))
}
