use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketRegime {
    Accumulation,
    Manipulation,
    Distribution,
    Liquidation,
}

impl MarketRegime {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Accumulation => "ACCUMULATION",
            Self::Manipulation => "MANIPULATION",
            Self::Distribution => "DISTRIBUTION",
            Self::Liquidation => "LIQUIDATION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DirectionBias {
    Long,
    Short,
    Neutral,
}

impl DirectionBias {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
            Self::Neutral => "NEUTRAL",
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketFeatureSet {
    pub symbol: String,
    pub price_change_5m_pct: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub volume_spike_multiple: Option<f64>,
    pub funding_rate: Option<f64>,
    pub spot_futures_divergence_pct: Option<f64>,
    pub liquidation_ratio: Option<f64>,
    pub price_impact_efficiency: Option<f64>,
    pub flow_direction: Option<DirectionBias>,
    pub data_quality: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManipulationAssessment {
    pub symbol: String,
    pub score: f64,
    pub signals: Vec<String>,
    pub metrics: BTreeMap<String, f64>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRegimeAssessment {
    pub symbol: String,
    pub regime: String,
    pub confidence: f64,
    pub direction_bias: String,
    pub signals: Vec<String>,
    pub metrics: BTreeMap<String, f64>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSignalAssessment {
    pub symbol: String,
    pub regime: String,
    pub confidence: f64,
    pub manipulation_score: f64,
    pub direction_bias: String,
    pub signals: Vec<String>,
    pub adjusted_signal_strength: f64,
    pub allowed_signal_family: String,
    pub risk_note: String,
    pub metrics: BTreeMap<String, f64>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRegimeEngineOutput {
    pub regime: MarketRegimeAssessment,
    pub manipulation: ManipulationAssessment,
    pub signal: MarketSignalAssessment,
}

pub fn analyze_market_regime(features: &MarketFeatureSet) -> MarketRegimeEngineOutput {
    let normalized = normalized_features(features);
    let manipulation = assess_manipulation(&normalized);
    let regime = classify_regime(&normalized, &manipulation);
    let signal = compress_signal(&normalized, &regime, &manipulation);

    MarketRegimeEngineOutput {
        regime,
        manipulation,
        signal,
    }
}

pub fn normalize_market_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(symbol)
        .trim_end_matches("USDT")
        .trim_end_matches("USD")
        .trim()
        .to_ascii_uppercase()
}

struct NormalizedFeatures {
    symbol: String,
    price_change_5m_pct: f64,
    oi_change_pct: f64,
    volume_spike_multiple: f64,
    funding_rate: f64,
    spot_futures_divergence_pct: f64,
    liquidation_ratio: f64,
    price_impact_efficiency: f64,
    flow_direction: DirectionBias,
    data_quality: f64,
}

fn normalized_features(features: &MarketFeatureSet) -> NormalizedFeatures {
    NormalizedFeatures {
        symbol: normalize_market_symbol(&features.symbol),
        price_change_5m_pct: finite_or(features.price_change_5m_pct, 0.0),
        oi_change_pct: finite_or(features.oi_change_pct, 0.0),
        volume_spike_multiple: finite_or(features.volume_spike_multiple, 1.0).max(0.0),
        funding_rate: finite_or(features.funding_rate, 0.0),
        spot_futures_divergence_pct: finite_or(features.spot_futures_divergence_pct, 0.0),
        liquidation_ratio: finite_or(features.liquidation_ratio, 0.0).clamp(0.0, 1.0),
        price_impact_efficiency: finite_or(features.price_impact_efficiency, 0.0).max(0.0),
        flow_direction: features.flow_direction.unwrap_or(DirectionBias::Neutral),
        data_quality: finite_or(features.data_quality, 1.0).clamp(0.0, 1.0),
    }
}

fn assess_manipulation(features: &NormalizedFeatures) -> ManipulationAssessment {
    let mut signals = Vec::new();
    let mut score = 0.0;

    let price_up = features.price_change_5m_pct >= 0.05;
    let price_down = features.price_change_5m_pct <= -0.05;
    let oi_up = features.oi_change_pct >= 0.20;
    let oi_down = features.oi_change_pct <= -0.20;
    let volume_spike = features.volume_spike_multiple >= 3.0;
    let funding_extreme = features.funding_rate.abs() > 0.0005;
    let low_price_impact =
        features.price_change_5m_pct.abs() < 0.12 || features.price_impact_efficiency <= 0.12;
    let liquidation_heavy = features.liquidation_ratio >= 0.30;

    if (price_up && oi_down) || (price_down && oi_up) {
        signals.push("OI_DIVERGENCE".to_string());
        score += 0.30;
    }
    if volume_spike && low_price_impact {
        signals.push("FAKE_BREAKOUT".to_string());
        score += 0.25;
    }
    if funding_extreme {
        signals.push("FUNDING_EXTREME".to_string());
        score += 0.20;
    }
    if features.volume_spike_multiple >= 2.0 && low_price_impact {
        signals.push("ABSORPTION".to_string());
        score += 0.15;
    }
    if liquidation_heavy {
        signals.push("LIQUIDATION_CLUSTER".to_string());
        score += 0.25;
    }
    if features.spot_futures_divergence_pct.abs() >= 0.30 {
        signals.push("SPOT_FUTURES_DIVERGENCE".to_string());
        score += 0.10;
    }

    signals.sort();
    signals.dedup();

    ManipulationAssessment {
        symbol: features.symbol.clone(),
        score: round4((score * features.data_quality).clamp(0.0, 1.0)),
        signals,
        metrics: metrics(features),
        read_only: true,
        runtime_modified: false,
    }
}

fn classify_regime(
    features: &NormalizedFeatures,
    manipulation: &ManipulationAssessment,
) -> MarketRegimeAssessment {
    let price_up = features.price_change_5m_pct >= 0.05;
    let price_flat = features.price_change_5m_pct.abs() <= 0.12;
    let oi_up = features.oi_change_pct >= 0.20;
    let oi_down = features.oi_change_pct <= -0.20;
    let oi_crash = features.oi_change_pct <= -1.0;
    let volume_spike = features.volume_spike_multiple >= 2.0;
    let funding_extreme = features.funding_rate.abs() > 0.0005;
    let liquidation_heavy = features.liquidation_ratio >= 0.30;

    let (regime, confidence) = if liquidation_heavy || (funding_extreme && oi_crash && volume_spike)
    {
        (
            MarketRegime::Liquidation,
            0.62 + features.liquidation_ratio * 0.25
                + features.volume_spike_multiple.min(5.0) * 0.025,
        )
    } else if manipulation.score >= 0.50 {
        (MarketRegime::Manipulation, 0.55 + manipulation.score * 0.40)
    } else if price_up && oi_down && volume_spike {
        (
            MarketRegime::Distribution,
            0.58 + features.volume_spike_multiple.min(5.0) * 0.04
                + features.oi_change_pct.abs().min(3.0) * 0.04,
        )
    } else if oi_up && price_flat && features.volume_spike_multiple <= 2.0 {
        (
            MarketRegime::Accumulation,
            0.55 + features.oi_change_pct.min(3.0) * 0.06
                + (0.12 - features.price_change_5m_pct.abs()).max(0.0),
        )
    } else if oi_down {
        (MarketRegime::Distribution, 0.42)
    } else {
        (MarketRegime::Accumulation, 0.35)
    };

    let direction_bias = direction_bias(features, regime);

    MarketRegimeAssessment {
        symbol: features.symbol.clone(),
        regime: regime.as_key().to_string(),
        confidence: round4((confidence * features.data_quality).clamp(0.0, 1.0)),
        direction_bias: direction_bias.as_key().to_string(),
        signals: manipulation.signals.clone(),
        metrics: metrics(features),
        read_only: true,
        runtime_modified: false,
    }
}

fn compress_signal(
    features: &NormalizedFeatures,
    regime: &MarketRegimeAssessment,
    manipulation: &ManipulationAssessment,
) -> MarketSignalAssessment {
    let raw_strength = (features.volume_spike_multiple / 5.0)
        .max(features.oi_change_pct.abs() / 3.0)
        .max(features.liquidation_ratio)
        .clamp(0.0, 1.0);
    let adjusted = match regime.regime.as_str() {
        "MANIPULATION" => raw_strength * 0.5,
        "LIQUIDATION" => raw_strength * 0.7,
        _ => raw_strength,
    };
    let allowed_signal_family = match regime.regime.as_str() {
        "LIQUIDATION" => "MEAN_REVERSION_ONLY",
        "MANIPULATION" => "REDUCED_STRENGTH_ONLY",
        _ => "NORMAL_ANALYSIS_ONLY",
    };
    let risk_note = match regime.regime.as_str() {
        "LIQUIDATION" => "清算状态下仅保留均值回归类观察信号，不作为执行指令。",
        "MANIPULATION" => "控盘/诱导状态下信号强度降权 50%，避免把假突破当趋势。",
        "DISTRIBUTION" => "派发状态下优先观察上涨效率下降和 OI 背离。",
        _ => "吸筹状态下观察 OI 扩张与低波动延续。",
    };

    MarketSignalAssessment {
        symbol: features.symbol.clone(),
        regime: regime.regime.clone(),
        confidence: regime.confidence,
        manipulation_score: manipulation.score,
        direction_bias: regime.direction_bias.clone(),
        signals: manipulation.signals.clone(),
        adjusted_signal_strength: round4(adjusted),
        allowed_signal_family: allowed_signal_family.to_string(),
        risk_note: risk_note.to_string(),
        metrics: metrics(features),
        read_only: true,
        runtime_modified: false,
    }
}

fn direction_bias(features: &NormalizedFeatures, regime: MarketRegime) -> DirectionBias {
    match regime {
        MarketRegime::Accumulation => DirectionBias::Long,
        MarketRegime::Distribution => DirectionBias::Short,
        MarketRegime::Liquidation => {
            if features.liquidation_ratio >= 0.30 && features.price_change_5m_pct < 0.0 {
                DirectionBias::Long
            } else if features.liquidation_ratio >= 0.30 && features.price_change_5m_pct > 0.0 {
                DirectionBias::Short
            } else {
                DirectionBias::Neutral
            }
        }
        MarketRegime::Manipulation => {
            if features.price_change_5m_pct > 0.0 && features.oi_change_pct < 0.0 {
                DirectionBias::Short
            } else if features.price_change_5m_pct < 0.0 && features.oi_change_pct > 0.0 {
                DirectionBias::Long
            } else {
                features.flow_direction
            }
        }
    }
}

fn metrics(features: &NormalizedFeatures) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (
            "price_change_5m_pct".to_string(),
            round4(features.price_change_5m_pct),
        ),
        ("oi_change_pct".to_string(), round4(features.oi_change_pct)),
        (
            "volume_spike_multiple".to_string(),
            round4(features.volume_spike_multiple),
        ),
        ("funding_rate".to_string(), round6(features.funding_rate)),
        (
            "spot_futures_divergence_pct".to_string(),
            round4(features.spot_futures_divergence_pct),
        ),
        (
            "liquidation_ratio".to_string(),
            round4(features.liquidation_ratio),
        ),
        (
            "price_impact_efficiency".to_string(),
            round4(features.price_impact_efficiency),
        ),
        ("data_quality".to_string(), round4(features.data_quality)),
    ])
}

fn finite_or(value: Option<f64>, fallback: f64) -> f64 {
    value.filter(|value| value.is_finite()).unwrap_or(fallback)
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}
