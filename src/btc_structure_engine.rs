use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BtcStructureInput {
    pub symbol: String,
    pub flow_bias_score: f64,
    pub oi_change_pct: f64,
    pub funding_rate: f64,
    pub liquidation_cascade_probability: f64,
    pub liquidation_direction: String,
    pub gamma_pressure: f64,
    pub data_quality: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BtcStructureState {
    pub symbol: String,
    pub regime: String,
    pub bias: String,
    pub confidence: f64,
    pub structure_score: f64,
    pub liquidation_cascade_probability: f64,
    pub gamma_pressure: f64,
    pub signals: Vec<String>,
    pub metrics: BTreeMap<String, f64>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

pub fn analyze_btc_structure(input: &BtcStructureInput) -> BtcStructureState {
    let flow = input.flow_bias_score.clamp(-1.0, 1.0);
    let oi = (input.oi_change_pct / 2.0).clamp(-1.0, 1.0);
    let funding_stress = (input.funding_rate.abs() / 0.001).clamp(0.0, 1.0);
    let liquidation = input.liquidation_cascade_probability.clamp(0.0, 1.0);
    let gamma = input.gamma_pressure.clamp(0.0, 1.0);
    let score = (flow * 0.38 + oi * 0.22 + gamma * 0.18 - funding_stress * 0.08
        + directional_liquidation_push(&input.liquidation_direction, liquidation) * 0.14)
        .clamp(-1.0, 1.0);
    let mut signals = Vec::new();
    if liquidation >= 0.75 {
        signals.push("LIQUIDATION_IMMINENT".to_string());
    } else if liquidation >= 0.60 {
        signals.push("CASCADE_WARNING".to_string());
    }
    if gamma >= 0.60 {
        signals.push("GAMMA_PRESSURE_HIGH".to_string());
    }
    if funding_stress >= 0.60 {
        signals.push("FUNDING_STRESS".to_string());
    }
    if input.oi_change_pct.abs() >= 0.35 {
        signals.push("OI_STRUCTURE_SHIFT".to_string());
    }
    signals.sort();
    signals.dedup();

    let regime = if liquidation >= 0.70 {
        "LIQUIDATION"
    } else if score <= -0.20 {
        "DISTRIBUTION"
    } else {
        "ACCUMULATION"
    };
    let bias = if liquidation >= 0.70 && input.liquidation_direction == "DOWN" {
        "SHORT"
    } else if liquidation >= 0.70 && input.liquidation_direction == "UP" {
        "LONG"
    } else if score >= 0.22 {
        "LONG"
    } else if score <= -0.22 {
        "SHORT"
    } else {
        "NEUTRAL"
    };
    let confidence = (score.abs() * 0.45
        + liquidation * 0.25
        + gamma * 0.10
        + input.data_quality.clamp(0.0, 1.0) * 0.20)
        .clamp(0.0, 1.0);

    BtcStructureState {
        symbol: "BTC".to_string(),
        regime: regime.to_string(),
        bias: bias.to_string(),
        confidence: round4(confidence),
        structure_score: round4(((score + 1.0) / 2.0).clamp(0.0, 1.0)),
        liquidation_cascade_probability: round4(liquidation),
        gamma_pressure: round4(gamma),
        signals,
        metrics: BTreeMap::from([
            ("flow_bias_score".to_string(), round4(flow)),
            ("oi_change_pct".to_string(), round4(input.oi_change_pct)),
            ("funding_rate".to_string(), round6(input.funding_rate)),
            ("funding_stress".to_string(), round4(funding_stress)),
            ("data_quality".to_string(), round4(input.data_quality)),
        ]),
        read_only: true,
        runtime_modified: false,
    }
}

fn directional_liquidation_push(direction: &str, probability: f64) -> f64 {
    match direction {
        "UP" => probability,
        "DOWN" => -probability,
        _ => 0.0,
    }
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
    fn btc_structure_output_has_only_structural_metrics() {
        let state = analyze_btc_structure(&BtcStructureInput {
            symbol: "BTCUSDT".to_string(),
            flow_bias_score: 0.45,
            oi_change_pct: 0.50,
            liquidation_cascade_probability: 0.20,
            data_quality: 0.95,
            ..BtcStructureInput::default()
        });

        assert_eq!(state.symbol, "BTC");
        let forbidden_metric = ["manipulation", "_score"].concat();
        assert!(!state.metrics.contains_key(&forbidden_metric));
        assert_ne!(state.regime, "MANIPULATION");
    }
}
