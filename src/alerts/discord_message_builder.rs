use serde::Serialize;

use crate::types::toxic_signal::{
    SignalEvidence, ToxicSignal, ToxicSignalDirection, ToxicSignalType,
};

const FIELD_LIMIT: usize = 900;

#[derive(Debug, Clone, Serialize)]
pub struct DiscordWebhookPayload {
    pub content: Option<String>,
    pub embeds: Vec<DiscordEmbed>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub color: u32,
    pub fields: Vec<DiscordEmbedField>,
    pub footer: Option<DiscordEmbedFooter>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscordEmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscordEmbedFooter {
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct DiscordCandidateMessageInput {
    pub signal_id: Option<String>,
    pub venue: Option<String>,
    pub symbol: String,
    pub event_type: String,
    pub side: Option<String>,
    pub score: Option<f64>,
    pub data_quality: Option<f64>,
    pub primary_reason: Option<String>,
    pub impact: Option<String>,
    pub add_qty: Option<f64>,
    pub cancel_qty: Option<f64>,
    pub fill_qty: Option<f64>,
    pub cancel_to_trade_ratio: Option<f64>,
    pub depth_before: Option<f64>,
    pub depth_after: Option<f64>,
    pub depth_impact: Option<f64>,
    pub price_impact_bps: Option<f64>,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
    pub price_range: Option<String>,
    pub timestamp: Option<String>,
}

pub fn build_discord_candidate_message(signal: &ToxicSignal) -> DiscordWebhookPayload {
    build_discord_candidate_message_from_input(&DiscordCandidateMessageInput::from(signal))
}

pub fn build_discord_candidate_message_from_input(
    input: &DiscordCandidateMessageInput,
) -> DiscordWebhookPayload {
    let side = input.side.as_deref().unwrap_or("N/A");
    let event_type = if input.event_type.trim().is_empty() {
        "Unknown"
    } else {
        input.event_type.as_str()
    };
    let reason = truncate_field(&event_reason(input));
    let evidence = truncate_field(&market_evidence(input));
    let markout = truncate_field(&format!(
        "1s: {}\n5s: {}\n30s: {}",
        format_bps(input.markout_1s_bps),
        format_bps(input.markout_5s_bps),
        format_bps(input.markout_30s_bps)
    ));
    let mut fields = vec![
        field("风险评分", format_score(input.score), true),
        field("数据质量", format_score(input.data_quality), true),
        field("异常类型", event_type.to_string(), true),
        field("方向", side.to_string(), true),
        field("核心原因", reason, false),
        field("盘口证据", evidence, false),
        field("撤后 Markout", markout, false),
    ];
    if let Some(impact) = input
        .impact
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        fields.push(field("影响解读", truncate_field(impact), false));
    }
    fields.push(field(
        "说明",
        "该信号基于公开盘口 / L2 数据推断，为 Candidate，不是执法或定性结论。".to_string(),
        false,
    ));

    DiscordWebhookPayload {
        content: None,
        embeds: vec![DiscordEmbed {
            title: format!("🚨 疑似有毒订单候选信号：{}", input.symbol),
            description: format!(
                "{} / {} · {} · {}",
                input.venue.as_deref().unwrap_or("N/A"),
                input.symbol,
                event_type,
                side
            ),
            color: embed_color(input.score),
            fields,
            footer: Some(DiscordEmbedFooter {
                text: format!(
                    "Candidate only. Not an enforcement conclusion. Signal: {}",
                    input.signal_id.as_deref().unwrap_or("N/A")
                ),
            }),
            timestamp: input.timestamp.clone(),
        }],
    }
}

impl From<&ToxicSignal> for DiscordCandidateMessageInput {
    fn from(signal: &ToxicSignal) -> Self {
        let evidence = signal.evidence.as_ref();
        Self {
            signal_id: Some(signal.signal_id.clone()),
            venue: evidence.map(|value| value.venue.clone()),
            symbol: signal.symbol.clone(),
            event_type: format!("{:?}", signal.signal_type),
            side: side_from_signal(signal),
            score: Some(signal.toxicity_score as f64),
            data_quality: signal.data_quality,
            primary_reason: Some(signal.primary_reason.clone()),
            impact: signal.reason.first().cloned(),
            add_qty: evidence.map(|value| value.add_qty),
            cancel_qty: evidence.map(|value| value.cancel_qty),
            fill_qty: evidence.map(|value| value.fill_qty),
            cancel_to_trade_ratio: evidence.and_then(|value| value.cancel_to_trade_ratio),
            depth_before: evidence.and_then(|value| value.depth_before),
            depth_after: evidence.and_then(|value| value.depth_after),
            depth_impact: evidence.and_then(|value| value.depth_impact),
            price_impact_bps: evidence.and_then(|value| value.price_impact_bps),
            markout_1s_bps: evidence.and_then(|value| value.markout_1s_bps),
            markout_5s_bps: evidence.and_then(|value| value.markout_5s_bps),
            markout_30s_bps: evidence.and_then(|value| value.markout_30s_bps),
            price_range: price_range(evidence),
            timestamp: None,
        }
    }
}

pub fn format_qty(value: Option<f64>, unit: &str) -> String {
    value.map_or_else(
        || "N/A".to_string(),
        |value| format!("{} {unit}", format_number(value, 2)),
    )
}

pub fn format_bps(value: Option<f64>) -> String {
    value.map_or_else(|| "N/A".to_string(), |value| format!("{value:.2} bps"))
}

pub fn format_ratio(value: Option<f64>) -> String {
    value.map_or_else(|| "N/A".to_string(), |value| format!("{value:.2}"))
}

pub fn format_score(value: Option<f64>) -> String {
    value.map_or_else(
        || "N/A".to_string(),
        |value| format!("{}/100", format_number(value, 0)),
    )
}

pub fn embed_color(score: Option<f64>) -> u32 {
    match score.unwrap_or(0.0) {
        value if value >= 90.0 => 0x8B0000,
        value if value >= 80.0 => 0xFF0000,
        value if value >= 60.0 => 0xFFA500,
        value if value >= 40.0 => 0xFFD700,
        _ => 0x607D8B,
    }
}

fn event_reason(input: &DiscordCandidateMessageInput) -> String {
    let unit = base_asset(&input.symbol);
    let event_type = input.event_type.as_str();
    if event_type.contains("SpoofingCandidate") {
        let side_label = match input.side.as_deref() {
            Some("Bid/Buy") => "疑似大额买墙托盘",
            Some("Ask/Sell") => "疑似大额卖墙压盘",
            _ => "疑似诱导挂单",
        };
        return format!(
            "{side_label}：盘口附近出现约 {} 可见挂单墙，随后快速撤除约 {}，成交参与量约 {}。撤单/成交比为 {}，可能存在短时制造卖压/买压后撤单的候选行为。",
            format_qty(input.add_qty, unit),
            format_qty(input.cancel_qty, unit),
            format_qty(input.fill_qty, unit),
            format_ratio(input.cancel_to_trade_ratio)
        );
    }
    if event_type.contains("LayeringCandidate") {
        return format!(
            "疑似分层挂单：同侧多个价位层在短窗口内同步出现/撤除，累计挂单约 {}，撤除约 {}，成交约 {}。该行为可能形成短时盘口压力，诱导市场方向判断。",
            format_qty(input.add_qty, unit),
            format_qty(input.cancel_qty, unit),
            format_qty(input.fill_qty, unit)
        );
    }
    if event_type.contains("IcebergCandidate") {
        return "疑似冰山单：同一价位附近出现反复补量，累计成交量明显高于最大可见挂单量，显示量与实际成交不匹配，可能存在隐藏流动性。"
            .to_string();
    }
    if event_type.contains("LiquidityPull")
        || event_type.contains("LiquidityThinness")
        || event_type.contains("NoTradeChop")
    {
        return format!(
            "疑似流动性抽离：{} 侧盘口深度从 {} 降至 {}，下降约 {}，价差/冲击可能同步扩大，可能导致短时滑点增加。",
            input.side.as_deref().unwrap_or("N/A"),
            format_qty(input.depth_before, unit),
            format_qty(input.depth_after, unit),
            format_percent(input.depth_impact)
        );
    }
    if event_type.contains("Sweep")
        || event_type.contains("ToxicFlow")
        || event_type.contains("WhaleFlow")
    {
        return format!(
            "疑似主动扫单：短时间内出现大额主动 {} 成交，成交规模约 {}，价格冲击约 {}，可能导致短线方向性波动。",
            input.side.as_deref().unwrap_or("N/A"),
            format_qty(input.fill_qty.or(input.add_qty), unit),
            format_bps(input.price_impact_bps)
        );
    }
    input.primary_reason.as_deref().map_or_else(
        || {
            "检测到疑似盘口异常候选信号，但当前信号缺少完整细分证据。请查看 Dashboard 详情面板。"
                .to_string()
        },
        |reason| {
            if reason.trim().is_empty() {
                "检测到疑似盘口异常候选信号，但当前信号缺少完整细分证据。请查看 Dashboard 详情面板。"
                    .to_string()
            } else {
                reason.to_string()
            }
        },
    )
}

fn market_evidence(input: &DiscordCandidateMessageInput) -> String {
    let unit = base_asset(&input.symbol);
    format!(
        "交易所：{}\n交易对：{}\n价格区间：{}\n挂单量：{}\n撤除量：{}\n成交量：{}\n撤单/成交比：{}",
        input.venue.as_deref().unwrap_or("N/A"),
        input.symbol,
        input.price_range.as_deref().unwrap_or("N/A"),
        format_qty(input.add_qty, unit),
        format_qty(input.cancel_qty, unit),
        format_qty(input.fill_qty, unit),
        format_ratio(input.cancel_to_trade_ratio)
    )
}

fn field(name: &str, value: String, inline: bool) -> DiscordEmbedField {
    DiscordEmbedField {
        name: name.to_string(),
        value: truncate_field(&value),
        inline,
    }
}

fn truncate_field(value: &str) -> String {
    if value.chars().count() <= FIELD_LIMIT {
        return value.to_string();
    }
    value
        .chars()
        .take(FIELD_LIMIT.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

fn format_percent(value: Option<f64>) -> String {
    value.map_or_else(
        || "N/A".to_string(),
        |value| format!("{}%", format_number(value * 100.0, 2)),
    )
}

fn format_number(value: f64, decimals: usize) -> String {
    let raw = format!("{value:.decimals$}");
    let (sign, digits) = raw
        .strip_prefix('-')
        .map_or(("", raw.as_str()), |rest| ("-", rest));
    let mut parts = digits.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default().trim_end_matches('0');
    let mut grouped = String::new();
    for (idx, ch) in integer.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let integer_grouped = grouped.chars().rev().collect::<String>();
    if fraction.is_empty() {
        format!("{sign}{integer_grouped}")
    } else {
        format!("{sign}{integer_grouped}.{fraction}")
    }
}

fn base_asset(symbol: &str) -> &str {
    let upper = symbol.to_ascii_uppercase();
    if upper.starts_with("BTC") {
        "BTC"
    } else if upper.starts_with("ETH") {
        "ETH"
    } else if upper.starts_with("SOL") {
        "SOL"
    } else if upper.starts_with("BNB") {
        "BNB"
    } else {
        "units"
    }
}

fn side_from_signal(signal: &ToxicSignal) -> Option<String> {
    match signal.direction {
        ToxicSignalDirection::ShortBias => Some("Ask/Sell".to_string()),
        ToxicSignalDirection::LongBias => Some("Bid/Buy".to_string()),
        ToxicSignalDirection::TrapRisk | ToxicSignalDirection::Neutral => None,
    }
}

fn price_range(evidence: Option<&SignalEvidence>) -> Option<String> {
    let evidence = evidence?;
    match (evidence.depth_before, evidence.depth_after) {
        (Some(before), Some(after)) => Some(format!(
            "depth {} -> {}",
            format_number(before, 2),
            format_number(after, 2)
        )),
        _ => None,
    }
}

#[allow(dead_code)]
fn signal_type_key(signal_type: ToxicSignalType) -> &'static str {
    match signal_type {
        ToxicSignalType::ShortBiasToxicFlow => "ShortBiasToxicFlow",
        ToxicSignalType::LongBiasToxicFlow => "LongBiasToxicFlow",
        ToxicSignalType::TrapRisk => "TrapRisk",
        ToxicSignalType::BullTrapRisk => "BullTrapRisk",
        ToxicSignalType::BearTrapRisk => "BearTrapRisk",
        ToxicSignalType::SqueezeRiskUpside => "SqueezeRiskUpside",
        ToxicSignalType::SqueezeRiskDownside => "SqueezeRiskDownside",
        ToxicSignalType::AbsorptionReversalCandidate => "AbsorptionReversalCandidate",
        ToxicSignalType::LiquiditySweepReversalCandidate => "LiquiditySweepReversalCandidate",
        ToxicSignalType::NoTradeChopRisk => "NoTradeChopRisk",
        ToxicSignalType::SpoofingCandidate => "SpoofingCandidate",
        ToxicSignalType::LayeringCandidate => "LayeringCandidate",
        ToxicSignalType::IcebergCandidate => "IcebergCandidate",
    }
}
