use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltcoinManipulationInput {
    pub symbol: String,
    pub price_change_pct: f64,
    pub oi_change_pct: f64,
    pub volume_spike_multiple: f64,
    pub funding_rate: f64,
    pub liquidation_ratio: f64,
    pub price_impact_efficiency: f64,
    pub flow_bias_score: f64,
    pub data_quality: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltcoinManipulationState {
    pub symbol: String,
    pub regime: String,
    pub bias: String,
    pub confidence: f64,
    pub manipulation_score: f64,
    pub oi_signal_score: f64,
    pub volume_signal_score: f64,
    pub funding_signal_score: f64,
    pub price_signal_score: f64,
    pub pump_dump_score: f64,
    pub signals: Vec<String>,
    pub risk_tags: Vec<String>,
    pub metrics: BTreeMap<String, f64>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

pub fn analyze_altcoin_manipulation(input: &AltcoinManipulationInput) -> AltcoinManipulationState {
    let oi = oi_structure_engine(input);
    let volume = volume_structure_engine(input);
    let funding = funding_stress_engine(input);
    let price = price_manipulation_engine(input);
    let pump_dump_score = pump_dump_score(input);

    let mut signals = Vec::new();
    signals.extend(oi.signals.iter().cloned());
    signals.extend(volume.signals.iter().cloned());
    signals.extend(funding.signals.iter().cloned());
    signals.extend(price.signals.iter().cloned());
    if pump_dump_score >= 0.60 {
        signals.push("PUMP_DUMP_RISK".to_string());
    }
    signals.sort();
    signals.dedup();

    let mut risk_tags = Vec::new();
    risk_tags.extend(oi.risk_tags.iter().cloned());
    risk_tags.extend(volume.risk_tags.iter().cloned());
    risk_tags.extend(funding.risk_tags.iter().cloned());
    risk_tags.extend(price.risk_tags.iter().cloned());
    if pump_dump_score >= 0.70 {
        risk_tags.push("PUMP_DUMP_RISK".to_string());
    }
    risk_tags.sort();
    risk_tags.dedup();

    let raw_manipulation_score =
        oi.score * 0.30 + volume.score * 0.30 + funding.score * 0.20 + price.score * 0.20;
    let manipulation_score =
        (raw_manipulation_score * input.data_quality.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let regime = if manipulation_score >= 0.75 {
        "MANIPULATION_HIGH"
    } else if manipulation_score >= 0.45 || pump_dump_score >= 0.70 {
        "MANIPULATION_MEDIUM"
    } else {
        "CLEAN_MARKET"
    };
    let bias = if manipulation_score >= 0.45 || pump_dump_score >= 0.70 {
        if input.price_change_pct > 0.0
            && (input.oi_change_pct <= 0.0
                || input.funding_rate > 0.0
                || risk_tags.iter().any(|tag| tag == "PUMP_RISK"))
        {
            "SHORT"
        } else if input.price_change_pct < 0.0
            && (input.oi_change_pct >= 0.0
                || input.funding_rate < 0.0
                || risk_tags.iter().any(|tag| tag == "DUMP_RISK"))
        {
            "LONG"
        } else {
            "NEUTRAL"
        }
    } else if input.flow_bias_score >= 0.20 {
        "LONG"
    } else if input.flow_bias_score <= -0.20 {
        "SHORT"
    } else {
        "NEUTRAL"
    };
    let confidence = (manipulation_score * 0.50
        + pump_dump_score * 0.25
        + input.data_quality.clamp(0.0, 1.0) * 0.25)
        .clamp(0.0, 1.0);

    AltcoinManipulationState {
        symbol: input.symbol.trim().to_ascii_uppercase(),
        regime: regime.to_string(),
        bias: bias.to_string(),
        confidence: round4(confidence),
        manipulation_score: round4(manipulation_score),
        oi_signal_score: round4(oi.score),
        volume_signal_score: round4(volume.score),
        funding_signal_score: round4(funding.score),
        price_signal_score: round4(price.score),
        pump_dump_score: round4(pump_dump_score),
        signals,
        risk_tags,
        metrics: BTreeMap::from([
            (
                "price_change_pct".to_string(),
                round4(input.price_change_pct),
            ),
            ("oi_change_pct".to_string(), round4(input.oi_change_pct)),
            (
                "volume_spike_multiple".to_string(),
                round4(input.volume_spike_multiple),
            ),
            ("funding_rate".to_string(), round6(input.funding_rate)),
            (
                "liquidation_ratio".to_string(),
                round4(input.liquidation_ratio),
            ),
            (
                "price_impact_efficiency".to_string(),
                round4(input.price_impact_efficiency),
            ),
            ("flow_bias_score".to_string(), round4(input.flow_bias_score)),
            ("data_quality".to_string(), round4(input.data_quality)),
            ("oi_signal_score".to_string(), round4(oi.score)),
            ("volume_signal_score".to_string(), round4(volume.score)),
            ("funding_signal_score".to_string(), round4(funding.score)),
            ("price_signal_score".to_string(), round4(price.score)),
            ("pump_dump_score".to_string(), round4(pump_dump_score)),
        ]),
        read_only: true,
        runtime_modified: false,
    }
}

#[derive(Debug, Clone, Default)]
struct ComponentAssessment {
    score: f64,
    signals: Vec<String>,
    risk_tags: Vec<String>,
}

impl ComponentAssessment {
    fn push_signal(&mut self, signal: &str) {
        self.signals.push(signal.to_string());
    }

    fn push_risk(&mut self, tag: &str) {
        self.risk_tags.push(tag.to_string());
    }
}

fn oi_structure_engine(input: &AltcoinManipulationInput) -> ComponentAssessment {
    let price_up = input.price_change_pct >= 0.05;
    let price_down = input.price_change_pct <= -0.05;
    let oi_up = input.oi_change_pct >= 0.20;
    let oi_down = input.oi_change_pct <= -0.20;
    let oi_flat = input.oi_change_pct.abs() < 0.15;
    let mut component = ComponentAssessment::default();

    if (price_up && oi_down) || (price_down && oi_up) {
        component.push_signal("OI_DIVERGENCE");
        component.push_risk("OI_INDUCEMENT_RISK");
        component.score += 0.55;
    }
    if oi_up && input.price_change_pct.abs() < 0.12 {
        component.push_signal("OI_BUILDUP_FLAT_PRICE");
        component.push_risk("SQUEEZE_PRECURSOR");
        component.score += 0.35;
    }
    if price_up && oi_up {
        component.push_signal("OI_EXPANSION_WITH_PRICE");
        component.push_risk("LEVERAGE_CHASE");
        component.score += 0.20;
    }
    if input.oi_change_pct.abs() >= 0.75 {
        component.push_signal(if input.oi_change_pct > 0.0 {
            "OI_ABNORMAL_EXPANSION"
        } else {
            "OI_UNWIND"
        });
        component.score += 0.25;
    }
    if price_up && oi_flat && input.volume_spike_multiple >= 3.0 {
        component.push_signal("NO_OI_SUPPORT_PUMP");
        component.push_risk("PUMP_RISK");
        component.score += 0.20;
    }

    component.score = component.score.clamp(0.0, 1.0);
    component
}

fn volume_structure_engine(input: &AltcoinManipulationInput) -> ComponentAssessment {
    let volume_spike = input.volume_spike_multiple >= 3.0;
    let low_price_impact =
        input.price_change_pct.abs() < 0.12 || input.price_impact_efficiency <= 0.12;
    let same_direction_flow = input.flow_bias_score.abs() >= 0.20
        && input.flow_bias_score.signum() == input.price_change_pct.signum();
    let mut component = ComponentAssessment::default();

    if volume_spike {
        component.push_signal("VOLUME_SPIKE");
        component.score += 0.25;
    }
    if volume_spike && low_price_impact {
        component.push_signal("FAKE_BREAKOUT");
        component.push_risk("FAKE_BREAKOUT_RISK");
        component.score += 0.45;
    }
    if input.volume_spike_multiple >= 2.0
        && same_direction_flow
        && input.price_change_pct.abs() >= 0.35
    {
        component.push_signal("TREND_SUPPORT_VOLUME");
        component.score -= 0.15;
    }
    if volume_spike
        && input.price_change_pct.abs() >= 0.50
        && input.flow_bias_score.signum() != input.price_change_pct.signum()
    {
        component.push_signal("DELTA_DIVERGENCE");
        component.push_risk("REVERSAL_RISK");
        component.score += 0.30;
    }

    component.score = component.score.clamp(0.0, 1.0);
    component
}

fn funding_stress_engine(input: &AltcoinManipulationInput) -> ComponentAssessment {
    let mut component = ComponentAssessment::default();
    let abs_funding = input.funding_rate.abs();

    if input.funding_rate > 0.0005 {
        component.push_signal("FUNDING_EXTREME_LONG");
        component.push_risk("LONG_CROWDING");
    } else if input.funding_rate < -0.0005 {
        component.push_signal("FUNDING_EXTREME_SHORT");
        component.push_risk("SHORT_CROWDING");
    }
    if abs_funding > 0.0004 {
        component.push_signal("FUNDING_SQUEEZE_RISK");
        component.push_risk("SQUEEZE_PRECURSOR");
    }
    component.score += (abs_funding / 0.001).clamp(0.0, 1.0) * 0.75;
    if abs_funding > 0.0004 && input.price_change_pct.abs() < 0.12 {
        component.push_signal("FUNDING_STRESS_IN_RANGE");
        component.score += 0.25;
    }

    component.score = component.score.clamp(0.0, 1.0);
    component
}

fn price_manipulation_engine(input: &AltcoinManipulationInput) -> ComponentAssessment {
    let volume_high = input.volume_spike_multiple >= 2.0;
    let volume_spike = input.volume_spike_multiple >= 3.0;
    let low_price_impact =
        input.price_change_pct.abs() < 0.12 || input.price_impact_efficiency <= 0.12;
    let mut component = ComponentAssessment::default();

    if volume_high && low_price_impact {
        component.push_signal("ABSORPTION");
        component.score += 0.35;
    }
    if volume_spike && input.oi_change_pct.abs() < 0.15 && input.price_change_pct.abs() < 0.18 {
        component.push_signal("PRICE_FAKE_BREAKOUT");
        component.push_risk("FAKE_BREAKOUT_RISK");
        component.score += 0.30;
    }
    if input.price_change_pct >= 0.50 && volume_high {
        component.push_signal("RAPID_PUMP");
        component.push_risk("PUMP_RISK");
        component.score += 0.25;
    } else if input.price_change_pct <= -0.50 && volume_high {
        component.push_signal("RAPID_DUMP");
        component.push_risk("DUMP_RISK");
        component.score += 0.25;
    }
    component.score += pump_dump_score(input) * 0.35;

    component.score = component.score.clamp(0.0, 1.0);
    component
}

fn pump_dump_score(input: &AltcoinManipulationInput) -> f64 {
    let volume = (input.volume_spike_multiple / 5.0).clamp(0.0, 1.0);
    let reversal = if input.price_change_pct.abs() >= 0.50
        && input.flow_bias_score.signum() != input.price_change_pct.signum()
    {
        1.0
    } else {
        0.0
    };
    let low_liquidity_move = input.price_impact_efficiency.clamp(0.0, 1.0);
    (volume * 0.45 + reversal * 0.35 + low_liquidity_move * 0.20).clamp(0.0, 1.0)
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
    fn fake_breakout_is_altcoin_manipulation_signal() {
        let state = analyze_altcoin_manipulation(&AltcoinManipulationInput {
            symbol: "PEPEUSDT".to_string(),
            price_change_pct: 0.04,
            oi_change_pct: -0.35,
            volume_spike_multiple: 4.0,
            price_impact_efficiency: 0.05,
            data_quality: 0.9,
            ..AltcoinManipulationInput::default()
        });

        assert!(state.manipulation_score > 0.20);
        assert!(state.signals.iter().any(|signal| signal == "FAKE_BREAKOUT"));
    }

    #[test]
    fn high_control_altcoin_outputs_component_scores_and_risk_tags() {
        let state = analyze_altcoin_manipulation(&AltcoinManipulationInput {
            symbol: "ASTERUSDT".to_string(),
            price_change_pct: 0.68,
            oi_change_pct: -0.82,
            volume_spike_multiple: 4.6,
            funding_rate: 0.0008,
            liquidation_ratio: 0.12,
            price_impact_efficiency: 0.08,
            flow_bias_score: -0.55,
            data_quality: 0.95,
        });

        assert_eq!(state.regime, "MANIPULATION_HIGH");
        assert_eq!(state.bias, "SHORT");
        assert!(state.manipulation_score >= 0.75);
        assert!(state.oi_signal_score > 0.50);
        assert!(state.volume_signal_score > 0.30);
        assert!(state.funding_signal_score > 0.50);
        assert!(state.price_signal_score > 0.30);
        assert!(state.signals.iter().any(|signal| signal == "OI_DIVERGENCE"));
        assert!(state
            .signals
            .iter()
            .any(|signal| signal == "FUNDING_EXTREME_LONG"));
        assert!(state.risk_tags.iter().any(|tag| tag == "PUMP_RISK"));
    }

    #[test]
    fn clean_altcoin_market_stays_low_risk() {
        let state = analyze_altcoin_manipulation(&AltcoinManipulationInput {
            symbol: "JTOUSDT".to_string(),
            price_change_pct: 0.08,
            oi_change_pct: 0.04,
            volume_spike_multiple: 1.1,
            funding_rate: 0.00008,
            price_impact_efficiency: 0.24,
            flow_bias_score: 0.12,
            data_quality: 0.90,
            ..AltcoinManipulationInput::default()
        });

        assert_eq!(state.regime, "CLEAN_MARKET");
        assert!(state.manipulation_score < 0.25);
        assert!(state.risk_tags.is_empty());
    }
}
