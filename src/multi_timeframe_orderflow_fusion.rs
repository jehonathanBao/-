use std::collections::BTreeMap;

use crate::market_domain::MarketDomain;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfofeInput {
    pub symbol: String,
    pub market_domain: MarketDomain,
    pub micro_5s: MtfofeLayerInput,
    pub flow_60s: MtfofeLayerInput,
    pub structure_5m: MtfofeLayerInput,
    pub regime_1h: MtfofeLayerInput,
    pub liquidation_cascade_probability: f64,
    pub liquidation_cascade_direction: String,
}

impl Default for MtfofeInput {
    fn default() -> Self {
        Self {
            symbol: "BTC".to_string(),
            market_domain: MarketDomain::BtcStructure,
            micro_5s: MtfofeLayerInput::new("5s"),
            flow_60s: MtfofeLayerInput::new("60s"),
            structure_5m: MtfofeLayerInput::new("5m"),
            regime_1h: MtfofeLayerInput::new("1h"),
            liquidation_cascade_probability: 0.0,
            liquidation_cascade_direction: "NEUTRAL".to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfofeLayerInput {
    pub timeframe: String,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub price_change_pct: f64,
    pub oi_change_pct: f64,
    pub funding_rate: f64,
    pub liquidation_ratio: f64,
    pub altcoin_control_score: f64,
    pub volume_spike_multiple: f64,
    pub data_quality: f64,
}

impl MtfofeLayerInput {
    pub fn new(timeframe: impl Into<String>) -> Self {
        Self {
            timeframe: timeframe.into(),
            buy_volume: 0.0,
            sell_volume: 0.0,
            price_change_pct: 0.0,
            oi_change_pct: 0.0,
            funding_rate: 0.0,
            liquidation_ratio: 0.0,
            altcoin_control_score: 0.0,
            volume_spike_multiple: 0.0,
            data_quality: 0.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfofeTimeframeState {
    pub timeframe: String,
    pub alignment: String,
    pub bias: String,
    pub score: f64,
    pub confidence: f64,
    pub regime: String,
    pub signals: Vec<String>,
    pub metrics: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfofeStateResponse {
    pub symbol: String,
    pub regime: String,
    pub bias: String,
    pub confidence: f64,
    pub fusion_score: f64,
    pub conflict_score: f64,
    pub decision: String,
    pub timeframe_alignment: BTreeMap<String, String>,
    pub signals: Vec<String>,
    pub layers: Vec<MtfofeTimeframeState>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfofeDecisionResponse {
    pub symbol: String,
    pub regime: String,
    pub bias: String,
    pub confidence: f64,
    pub fusion_score: f64,
    pub conflict_score: f64,
    pub decision: String,
    pub signals: Vec<String>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfofeBreakdownResponse {
    pub symbol: String,
    pub timeframe_alignment: BTreeMap<String, String>,
    pub layers: Vec<MtfofeTimeframeState>,
    pub signals: Vec<String>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

pub fn analyze_mtf_orderflow_fusion(input: &MtfofeInput) -> MtfofeStateResponse {
    let layers = vec![
        analyze_layer(&input.micro_5s, input.market_domain),
        analyze_layer(&input.flow_60s, input.market_domain),
        analyze_layer(&input.structure_5m, input.market_domain),
        analyze_layer(&input.regime_1h, input.market_domain),
    ];
    let weights = BTreeMap::from([
        ("5s".to_string(), 0.15),
        ("60s".to_string(), 0.30),
        ("5m".to_string(), 0.30),
        ("1h".to_string(), 0.25),
    ]);
    let weighted_score = layers
        .iter()
        .map(|layer| weights.get(&layer.timeframe).copied().unwrap_or(0.0) * layer.score)
        .sum::<f64>()
        .clamp(-1.0, 1.0);
    let conflict_score = timeframe_conflict_score(&layers);
    let alignment_strength = weighted_score.abs();
    let avg_layer_confidence =
        layers.iter().map(|layer| layer.confidence).sum::<f64>() / layers.len().max(1) as f64;
    let confidence = round4(
        (alignment_strength * 0.55 + avg_layer_confidence * 0.45) * (1.0 - conflict_score * 0.55),
    )
    .clamp(0.0, 1.0);
    let bias = final_bias(weighted_score, confidence, conflict_score);
    let mut signals = collect_signals(&layers);

    if conflict_score >= 0.70 {
        signals.push("REGIME_CONFLICT_HIGH".to_string());
    }
    if input.liquidation_cascade_probability >= 0.75 {
        signals.push("LIQUIDATION_IMMINENT".to_string());
    } else if input.liquidation_cascade_probability >= 0.60 {
        signals.push("CASCADE_WARNING".to_string());
    }
    if input.liquidation_cascade_probability >= 0.60
        && input.liquidation_cascade_direction != "NEUTRAL"
    {
        signals.push(format!(
            "LIQUIDATION_{}",
            input.liquidation_cascade_direction.to_ascii_uppercase()
        ));
    }
    signals.sort();
    signals.dedup();

    let regime = final_regime(
        &layers,
        input.liquidation_cascade_probability,
        conflict_score,
    );
    let decision = final_decision(confidence, conflict_score, &bias);
    let timeframe_alignment = layers
        .iter()
        .map(|layer| (layer.timeframe.clone(), layer.alignment.clone()))
        .collect::<BTreeMap<_, _>>();

    MtfofeStateResponse {
        symbol: input.symbol.clone(),
        regime,
        bias,
        confidence,
        fusion_score: round4(((weighted_score + 1.0) / 2.0).clamp(0.0, 1.0)),
        conflict_score: round4(conflict_score),
        decision,
        timeframe_alignment,
        signals,
        layers,
        read_only: true,
        runtime_modified: false,
    }
}

impl From<MtfofeStateResponse> for MtfofeDecisionResponse {
    fn from(state: MtfofeStateResponse) -> Self {
        Self {
            symbol: state.symbol,
            regime: state.regime,
            bias: state.bias,
            confidence: state.confidence,
            fusion_score: state.fusion_score,
            conflict_score: state.conflict_score,
            decision: state.decision,
            signals: state.signals,
            read_only: true,
            runtime_modified: false,
        }
    }
}

impl From<MtfofeStateResponse> for MtfofeBreakdownResponse {
    fn from(state: MtfofeStateResponse) -> Self {
        Self {
            symbol: state.symbol,
            timeframe_alignment: state.timeframe_alignment,
            layers: state.layers,
            signals: state.signals,
            read_only: true,
            runtime_modified: false,
        }
    }
}

fn analyze_layer(input: &MtfofeLayerInput, _domain: MarketDomain) -> MtfofeTimeframeState {
    let total_volume = (input.buy_volume + input.sell_volume).max(0.0);
    let flow_delta = if total_volume > f64::EPSILON {
        (input.buy_volume - input.sell_volume) / total_volume
    } else {
        0.0
    };
    let price_score = (input.price_change_pct / 0.8).clamp(-1.0, 1.0);
    let oi_score = (input.oi_change_pct / 2.0).clamp(-1.0, 1.0);
    let funding_penalty = (input.funding_rate.abs() / 0.001).clamp(0.0, 1.0);
    let liquidation_penalty = input.liquidation_ratio.clamp(0.0, 1.0);
    let score = (flow_delta * 0.45 + price_score * 0.20 + oi_score * 0.20
        - liquidation_penalty * 0.05
        - funding_penalty * 0.05)
        .clamp(-1.0, 1.0);

    let mut signals = Vec::new();
    if input.volume_spike_multiple >= 3.0 {
        signals.push("BURST_VOLUME".to_string());
    }
    if input.funding_rate.abs() > 0.0005 {
        signals.push("FUNDING_EXTREME".to_string());
    }
    if input.oi_change_pct.abs() >= 0.35
        && input.price_change_pct.signum() != input.oi_change_pct.signum()
    {
        signals.push("OI_STRUCTURE_SHIFT".to_string());
    }
    if input.liquidation_ratio >= 0.30 {
        signals.push("LIQUIDATION_CLUSTER".to_string());
    }

    let bias = if score >= 0.20 {
        "LONG"
    } else if score <= -0.20 {
        "SHORT"
    } else {
        "NEUTRAL"
    };
    let alignment = match bias {
        "LONG" => "bullish",
        "SHORT" => "bearish",
        _ => "neutral",
    };
    let regime = if input.liquidation_ratio >= 0.30 {
        "LIQUIDATION"
    } else if score <= -0.20 {
        "DISTRIBUTION"
    } else {
        "ACCUMULATION"
    };
    let confidence = (score.abs() * 0.45
        + input.volume_spike_multiple.min(5.0) / 5.0 * 0.20
        + input.data_quality.clamp(0.0, 1.0) * 0.35)
        .clamp(0.0, 1.0);
    let metrics = BTreeMap::from([
        ("buy_volume".to_string(), round4(input.buy_volume)),
        ("sell_volume".to_string(), round4(input.sell_volume)),
        ("flow_delta".to_string(), round4(flow_delta)),
        (
            "price_change_pct".to_string(),
            round4(input.price_change_pct),
        ),
        ("oi_change_pct".to_string(), round4(input.oi_change_pct)),
        ("funding_rate".to_string(), round6(input.funding_rate)),
        (
            "liquidation_ratio".to_string(),
            round4(input.liquidation_ratio),
        ),
        (
            "volume_spike_multiple".to_string(),
            round4(input.volume_spike_multiple),
        ),
        ("data_quality".to_string(), round4(input.data_quality)),
    ]);
    MtfofeTimeframeState {
        timeframe: input.timeframe.clone(),
        alignment: alignment.to_string(),
        bias: bias.to_string(),
        score: round4(score),
        confidence: round4(confidence),
        regime: regime.to_string(),
        signals,
        metrics,
    }
}

fn timeframe_conflict_score(layers: &[MtfofeTimeframeState]) -> f64 {
    let long = layers.iter().filter(|layer| layer.bias == "LONG").count() as f64;
    let short = layers.iter().filter(|layer| layer.bias == "SHORT").count() as f64;
    let non_neutral = long + short;
    if non_neutral <= f64::EPSILON {
        return 0.0;
    }
    let base_conflict = long.min(short) / non_neutral;
    let regime_layer = layers.iter().find(|layer| layer.timeframe == "1h");
    let mid_layers_opposing_regime = regime_layer
        .map(|regime| {
            if regime.bias == "NEUTRAL" {
                0.0
            } else {
                let opposing = layers
                    .iter()
                    .filter(|layer| layer.timeframe == "60s" || layer.timeframe == "5m")
                    .filter(|layer| layer.bias != "NEUTRAL" && layer.bias != regime.bias)
                    .count() as f64;
                opposing / 2.0
            }
        })
        .unwrap_or(0.0);
    (base_conflict * 0.55 + mid_layers_opposing_regime * 0.45).clamp(0.0, 1.0)
}

fn final_bias(weighted_score: f64, confidence: f64, conflict_score: f64) -> String {
    if conflict_score >= 0.70 || confidence < 0.25 {
        "NEUTRAL"
    } else if weighted_score >= 0.22 {
        "LONG"
    } else if weighted_score <= -0.22 {
        "SHORT"
    } else {
        "NEUTRAL"
    }
    .to_string()
}

fn final_regime(
    layers: &[MtfofeTimeframeState],
    liquidation_cascade_probability: f64,
    _conflict_score: f64,
) -> String {
    if liquidation_cascade_probability >= 0.70
        || layers
            .iter()
            .any(|layer| layer.regime == "LIQUIDATION" && layer.confidence >= 0.45)
    {
        return "LIQUIDATION".to_string();
    }
    let weighted = layers.iter().map(|layer| layer.score).sum::<f64>() / layers.len().max(1) as f64;
    if weighted <= -0.20 {
        "DISTRIBUTION".to_string()
    } else {
        "ACCUMULATION".to_string()
    }
}

fn final_decision(confidence: f64, conflict_score: f64, bias: &str) -> String {
    if conflict_score >= 0.70 || bias == "NEUTRAL" {
        "NO_TRADE".to_string()
    } else if confidence >= 0.75 {
        "HIGH_CONVICTION".to_string()
    } else {
        "LOW_CONVICTION".to_string()
    }
}

fn collect_signals(layers: &[MtfofeTimeframeState]) -> Vec<String> {
    let mut signals = layers
        .iter()
        .flat_map(|layer| layer.signals.iter().cloned())
        .collect::<Vec<_>>();
    signals.sort();
    signals.dedup();
    signals
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_bullish_layers_produce_long_bias() {
        let mut input = MtfofeInput::default();
        input.micro_5s = bullish_layer("5s");
        input.flow_60s = bullish_layer("60s");
        input.structure_5m = bullish_layer("5m");
        input.regime_1h = bullish_layer("1h");

        let output = analyze_mtf_orderflow_fusion(&input);

        assert_eq!(output.bias, "LONG");
        assert_ne!(output.decision, "NO_TRADE");
        assert!(output.confidence > 0.40);
        assert_eq!(output.timeframe_alignment.get("5m").unwrap(), "bullish");
    }

    #[test]
    fn high_timeframe_conflict_forces_neutral_no_trade() {
        let mut input = MtfofeInput::default();
        input.micro_5s = bullish_layer("5s");
        input.flow_60s = bearish_layer("60s");
        input.structure_5m = bearish_layer("5m");
        input.regime_1h = bullish_layer("1h");

        let output = analyze_mtf_orderflow_fusion(&input);

        assert_eq!(output.bias, "NEUTRAL");
        assert_eq!(output.decision, "NO_TRADE");
        assert!(output
            .signals
            .iter()
            .any(|signal| signal == "REGIME_CONFLICT_HIGH"));
    }

    #[test]
    fn liquidation_probability_promotes_liquidation_regime() {
        let mut input = MtfofeInput::default();
        input.micro_5s = bullish_layer("5s");
        input.flow_60s = bullish_layer("60s");
        input.structure_5m = bullish_layer("5m");
        input.regime_1h = bullish_layer("1h");
        input.liquidation_cascade_probability = 0.82;
        input.liquidation_cascade_direction = "DOWN".to_string();

        let output = analyze_mtf_orderflow_fusion(&input);

        assert_eq!(output.regime, "LIQUIDATION");
        assert!(output
            .signals
            .iter()
            .any(|signal| signal == "LIQUIDATION_IMMINENT"));
        assert!(output
            .signals
            .iter()
            .any(|signal| signal == "LIQUIDATION_DOWN"));
    }

    fn bullish_layer(timeframe: &str) -> MtfofeLayerInput {
        MtfofeLayerInput {
            timeframe: timeframe.to_string(),
            buy_volume: 800.0,
            sell_volume: 300.0,
            price_change_pct: 0.35,
            oi_change_pct: 0.45,
            funding_rate: 0.0001,
            liquidation_ratio: 0.08,
            altcoin_control_score: 0.10,
            volume_spike_multiple: 2.2,
            data_quality: 0.9,
        }
    }

    fn bearish_layer(timeframe: &str) -> MtfofeLayerInput {
        MtfofeLayerInput {
            timeframe: timeframe.to_string(),
            buy_volume: 250.0,
            sell_volume: 900.0,
            price_change_pct: -0.45,
            oi_change_pct: -0.35,
            funding_rate: -0.0001,
            liquidation_ratio: 0.10,
            altcoin_control_score: 0.12,
            volume_spike_multiple: 2.5,
            data_quality: 0.9,
        }
    }
}
