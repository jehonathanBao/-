use serde::Serialize;

use crate::types::toxic_signal::{ToxicSignal, ToxicSignalDirection, ToxicSignalType};

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

/// Safe Discord candidate input. Raw evidence / markout must never be rendered.
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
    let mut fields = vec![
        field("风险评分", format_score(input.score), true),
        field("数据质量", format_score(input.data_quality), true),
        field("异常类型", event_type.to_string(), true),
        field("方向", side.to_string(), true),
        field("核心原因", reason, false),
    ];
    if let Some(impact) = input
        .impact
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .filter(|value| !looks_like_forbidden_alert_content(value))
    {
        fields.push(field("影响解读", truncate_field(impact), false));
    }
    fields.push(field(
        "说明",
        "该信号基于公开盘口 / L2 数据推断，为 Candidate，不是执法或定性结论。详情请在 Dashboard 查看。".to_string(),
        false,
    ));

    DiscordWebhookPayload {
        content: None,
        embeds: vec![DiscordEmbed {
            title: format!("疑似有毒订单候选信号：{}", input.symbol),
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
        let venue = signal
            .evidence
            .as_ref()
            .map(|value| value.venue.clone());
        Self {
            signal_id: Some(signal.signal_id.clone()),
            venue,
            symbol: signal.symbol.clone(),
            event_type: format!("{:?}", signal.signal_type),
            side: side_from_signal(signal),
            score: Some(signal.toxicity_score as f64),
            data_quality: signal.data_quality,
            primary_reason: Some(signal.primary_reason.clone()),
            impact: signal
                .reason
                .first()
                .cloned()
                .filter(|value| !looks_like_forbidden_alert_content(value)),
            timestamp: None,
        }
    }
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
    let event_type = input.event_type.as_str();
    if event_type.contains("SpoofingCandidate") {
        let side_label = match input.side.as_deref() {
            Some("Bid/Buy") => "疑似大额买墙托盘",
            Some("Ask/Sell") => "疑似大额卖墙压盘",
            _ => "疑似诱导挂单",
        };
        return format!(
            "{side_label}：公开盘口出现短时挂单墙后快速撤除的候选行为。详情请在 Dashboard 查看。"
        );
    }
    if event_type.contains("LayeringCandidate") {
        return "疑似分层挂单：同侧多个价位层在短窗口内同步出现/撤除的候选行为。详情请在 Dashboard 查看。"
            .to_string();
    }
    if event_type.contains("IcebergCandidate") {
        return "疑似冰山单：可见挂单与累计成交不匹配的隐藏流动性候选。详情请在 Dashboard 查看。"
            .to_string();
    }
    if event_type.contains("LiquidityPull")
        || event_type.contains("LiquidityThinness")
        || event_type.contains("NoTradeChop")
    {
        return format!(
            "疑似流动性抽离：{} 侧盘口深度短时下降的候选行为。详情请在 Dashboard 查看。",
            input.side.as_deref().unwrap_or("N/A")
        );
    }
    if event_type.contains("Sweep")
        || event_type.contains("ToxicFlow")
        || event_type.contains("WhaleFlow")
    {
        return format!(
            "疑似主动扫单：短时出现大额主动 {} 成交的候选行为。详情请在 Dashboard 查看。",
            input.side.as_deref().unwrap_or("N/A")
        );
    }
    input.primary_reason.as_deref().map_or_else(
        || {
            "检测到疑似盘口异常候选信号。详情请在 Dashboard 查看。".to_string()
        },
        |reason| {
            if reason.trim().is_empty() || looks_like_forbidden_alert_content(reason) {
                "检测到疑似盘口异常候选信号。详情请在 Dashboard 查看。".to_string()
            } else {
                reason.to_string()
            }
        },
    )
}

fn looks_like_forbidden_alert_content(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("markout")
        || lower.contains("rawpayload")
        || lower.contains("raw_payload")
        || lower.contains("webhook")
        || lower.contains("authorization")
        || value.contains("盘口证据")
        || value.contains("撤后 Markout")
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

fn side_from_signal(signal: &ToxicSignal) -> Option<String> {
    match signal.direction {
        ToxicSignalDirection::ShortBias => Some("Ask/Sell".to_string()),
        ToxicSignalDirection::LongBias => Some("Bid/Buy".to_string()),
        ToxicSignalDirection::TrapRisk | ToxicSignalDirection::Neutral => None,
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
