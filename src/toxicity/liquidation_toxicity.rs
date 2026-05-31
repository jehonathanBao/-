use crate::types::{
    liquidation::{
        EstimatedLiquidationCluster, LiquidationClusterSide, LiquidationState,
        LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicSignalType,
        LiquidationToxicityRecentResponse,
    },
    toxic_flow::{
        ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
        ToxicConfidence,
    },
};

const NEARBY_DISTANCE_BPS: f64 = 50.0;
const CASCADE_GAP_BPS: f64 = 35.0;
const ELEVATED_CLUSTER_NOTIONAL_USD: f64 = 1_000_000.0;

#[derive(Debug, Clone)]
pub struct LiquidationToxicityAssessment {
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub signals: Vec<LiquidationToxicSignal>,
}

pub fn analyze_liquidation_toxicity(
    requested_symbol: &str,
    liquidation_state: &LiquidationState,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
) -> LiquidationToxicityAssessment {
    let mut warnings = Vec::new();
    let mut no_trade_reasons = Vec::new();

    if !liquidation_state
        .symbol
        .eq_ignore_ascii_case(requested_symbol)
    {
        return LiquidationToxicityAssessment {
            warnings: vec!["requested symbol does not match liquidation state".to_string()],
            no_trade_reasons: vec![
                "liquidation state is unavailable for the requested symbol".to_string()
            ],
            signals: Vec::new(),
        };
    }

    if !liquidation_state.metrics.enabled {
        return LiquidationToxicityAssessment {
            warnings: vec!["liquidation cluster engine is disabled".to_string()],
            no_trade_reasons: vec![
                "liquidation toxicity requires an enabled liquidation cluster engine".to_string(),
            ],
            signals: Vec::new(),
        };
    }

    let Some(_) = liquidation_state.metrics.current_mid else {
        return LiquidationToxicityAssessment {
            warnings: vec!["current liquidation reference price is unavailable".to_string()],
            no_trade_reasons: vec![
                "no liquidation toxicity can be classified without a current price".to_string(),
            ],
            signals: Vec::new(),
        };
    };

    let nearest_upside = liquidation_state
        .metrics
        .nearest_short_liq_cluster_above
        .as_ref();
    let nearest_downside = liquidation_state
        .metrics
        .nearest_long_liq_cluster_below
        .as_ref();
    if nearest_upside.is_none() && nearest_downside.is_none() {
        warnings.push(
            "no estimated liquidation clusters were found in the current lookback".to_string(),
        );
        no_trade_reasons
            .push("no liquidation clusters are available for toxicity classification".to_string());
        return LiquidationToxicityAssessment {
            warnings,
            no_trade_reasons,
            signals: Vec::new(),
        };
    }

    if active_trade_recent.signals.is_empty() {
        warnings.push(
            "active trade toxicity signals are unavailable for liquidation confluence".to_string(),
        );
    }

    let mut signals = Vec::new();

    if let Some(cluster) = nearest_cluster(nearest_upside, nearest_downside) {
        let nearby_threshold = liquidation_state
            .metrics
            .proximity_threshold_bps
            .max(NEARBY_DISTANCE_BPS);
        if cluster.distance_bps <= nearby_threshold {
            signals.push(build_signal(
                liquidation_state,
                cluster,
                LiquidationToxicSignalType::LiquidationClusterNearby,
                cluster_direction(cluster.side),
                score_from_cluster(cluster, 62.0, 88.0),
                cluster_density_score(cluster),
                magnet_score(cluster, liquidation_state.metrics.proximity_threshold_bps),
                0,
                Vec::new(),
                vec![format!(
                    "nearby liquidation cluster detected {} current price",
                    if matches!(cluster.side, LiquidationClusterSide::ShortAbove) {
                        "above"
                    } else {
                        "below"
                    }
                )],
            ));
        }
    }

    if let Some((signal_type, cluster, score, reason)) = classify_magnet(
        nearest_upside,
        nearest_downside,
        liquidation_state.metrics.proximity_threshold_bps,
    ) {
        signals.push(build_signal(
            liquidation_state,
            cluster,
            signal_type,
            cluster_direction(cluster.side),
            score,
            cluster_density_score(cluster),
            score,
            0,
            Vec::new(),
            vec![reason],
        ));
    }

    if let Some(cluster) = nearest_upside {
        let buy_links =
            matching_active_trade_signal_ids(&active_trade_recent.signals, is_bullish_signal);
        if !buy_links.is_empty() && cluster_supports_squeeze(cluster) {
            signals.push(build_signal(
                liquidation_state,
                cluster,
                LiquidationToxicSignalType::ShortSqueezeRisk,
                LiquidationToxicDirection::Upside,
                score_from_cluster(cluster, 66.0, 91.0),
                cluster_density_score(cluster),
                magnet_score(cluster, liquidation_state.metrics.proximity_threshold_bps),
                0,
                buy_links,
                vec![
                    "upside short liquidation cluster is dense enough to support squeeze risk"
                        .to_string(),
                    "bullish active-trade toxicity is pushing into overhead liquidation pressure"
                        .to_string(),
                ],
            ));
        }
    }

    if let Some(cluster) = nearest_downside {
        let sell_links =
            matching_active_trade_signal_ids(&active_trade_recent.signals, is_bearish_signal);
        if !sell_links.is_empty() && cluster_supports_squeeze(cluster) {
            signals.push(build_signal(
                liquidation_state,
                cluster,
                LiquidationToxicSignalType::LongSqueezeRisk,
                LiquidationToxicDirection::Downside,
                score_from_cluster(cluster, 66.0, 91.0),
                cluster_density_score(cluster),
                magnet_score(cluster, liquidation_state.metrics.proximity_threshold_bps),
                0,
                sell_links,
                vec![
                    "downside long liquidation cluster is dense enough to support squeeze risk"
                        .to_string(),
                    "bearish active-trade toxicity is pressing into downside liquidation pressure"
                        .to_string(),
                ],
            ));
        }
    }

    if let Some((cluster, direction, cascade_score, reason)) =
        classify_cascade(&liquidation_state.recent_clusters)
    {
        signals.push(build_signal(
            liquidation_state,
            cluster,
            LiquidationToxicSignalType::LiquidationCascadeCandidate,
            direction,
            cascade_score,
            cluster_density_score(cluster),
            magnet_score(cluster, liquidation_state.metrics.proximity_threshold_bps),
            cascade_score,
            Vec::new(),
            vec![reason],
        ));
    }

    if let Some(cluster) = nearest_upside {
        let ids = matching_active_trade_signal_ids(&active_trade_recent.signals, |signal| {
            signal.signal_type == ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
        });
        if !ids.is_empty() {
            signals.push(build_signal(
                liquidation_state,
                cluster,
                LiquidationToxicSignalType::LiquidationDeltaConfluence,
                LiquidationToxicDirection::Upside,
                score_from_cluster(cluster, 68.0, 92.0),
                cluster_density_score(cluster),
                magnet_score(cluster, liquidation_state.metrics.proximity_threshold_bps),
                0,
                ids,
                vec![
                    "1h buy delta candidate aligns with the nearby upside liquidation cluster".to_string(),
                    "watch for short-squeeze style liquidation confluence rather than a direct trade trigger".to_string(),
                ],
            ));
        }
    }

    if let Some(cluster) = nearest_downside {
        let ids = matching_active_trade_signal_ids(&active_trade_recent.signals, |signal| {
            signal.signal_type == ActiveTradeToxicSignalType::OneHourDeltaSellDominant
        });
        if !ids.is_empty() {
            signals.push(build_signal(
                liquidation_state,
                cluster,
                LiquidationToxicSignalType::LiquidationDeltaConfluence,
                LiquidationToxicDirection::Downside,
                score_from_cluster(cluster, 68.0, 92.0),
                cluster_density_score(cluster),
                magnet_score(cluster, liquidation_state.metrics.proximity_threshold_bps),
                0,
                ids,
                vec![
                    "1h sell delta candidate aligns with the nearby downside liquidation cluster".to_string(),
                    "watch for long-squeeze style liquidation confluence rather than a direct trade trigger".to_string(),
                ],
            ));
        }
    }

    if signals.is_empty() {
        no_trade_reasons.push(
            "liquidation structure exists, but it has not yet formed a strong toxicity watch candidate"
                .to_string(),
        );
    }

    LiquidationToxicityAssessment {
        warnings,
        no_trade_reasons,
        signals,
    }
}

pub fn build_liquidation_toxicity_recent_response(
    requested_symbol: &str,
    assessment: LiquidationToxicityAssessment,
) -> LiquidationToxicityRecentResponse {
    LiquidationToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        warnings: assessment.warnings,
        no_trade_reasons: assessment.no_trade_reasons,
        signals: assessment.signals,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    liquidation_state: &LiquidationState,
    cluster: &EstimatedLiquidationCluster,
    signal_type: LiquidationToxicSignalType,
    direction: LiquidationToxicDirection,
    toxicity_score: u8,
    cluster_density_score: u8,
    magnet_score: u8,
    cascade_score: u8,
    linked_active_trade_signal_ids: Vec<String>,
    reason: Vec<String>,
) -> LiquidationToxicSignal {
    let current_price = liquidation_state.metrics.current_mid.unwrap_or_default();
    let distance_usd = (cluster.price - current_price).abs();
    LiquidationToxicSignal {
        signal_id: format!(
            "liq-toxic-{}-{}-{}",
            signal_type_key(signal_type),
            liquidation_state.symbol.to_ascii_lowercase(),
            liquidation_state.updated_at.max(0)
        ),
        symbol: liquidation_state.symbol.clone(),
        ts_ms: liquidation_state.updated_at.max(0) as u64,
        signal_type,
        direction,
        current_price: round_price(current_price),
        cluster_price: round_price(cluster.price),
        distance_usd: round_price(distance_usd),
        distance_bps: round_bps(cluster.distance_bps),
        estimated_liquidation_notional: round_price(cluster.cluster_notional_usd),
        cluster_density_score,
        magnet_score,
        cascade_score,
        linked_active_trade_signal_ids,
        toxicity_score,
        confidence: confidence_for_score(toxicity_score),
        reason,
        read_only: true,
    }
}

fn nearest_cluster<'a>(
    upside: Option<&'a EstimatedLiquidationCluster>,
    downside: Option<&'a EstimatedLiquidationCluster>,
) -> Option<&'a EstimatedLiquidationCluster> {
    match (upside, downside) {
        (Some(upside), Some(downside)) => {
            if upside.distance_bps <= downside.distance_bps {
                Some(upside)
            } else {
                Some(downside)
            }
        }
        (Some(cluster), None) | (None, Some(cluster)) => Some(cluster),
        (None, None) => None,
    }
}

fn classify_magnet<'a>(
    upside: Option<&'a EstimatedLiquidationCluster>,
    downside: Option<&'a EstimatedLiquidationCluster>,
    proximity_threshold_bps: f64,
) -> Option<(
    LiquidationToxicSignalType,
    &'a EstimatedLiquidationCluster,
    u8,
    String,
)> {
    let upside = upside?;
    let downside = downside?;
    let upside_score = magnet_weight(upside, proximity_threshold_bps);
    let downside_score = magnet_weight(downside, proximity_threshold_bps);
    if (upside_score - downside_score).abs() < 6.0 {
        return None;
    }
    if upside_score > downside_score {
        Some((
            LiquidationToxicSignalType::UpsideLiquidationMagnet,
            upside,
            clamp_score(upside_score),
            "upside liquidation cluster is closer and denser than the downside cluster".to_string(),
        ))
    } else {
        Some((
            LiquidationToxicSignalType::DownsideLiquidationMagnet,
            downside,
            clamp_score(downside_score),
            "downside liquidation cluster is closer and denser than the upside cluster".to_string(),
        ))
    }
}

fn classify_cascade(
    clusters: &[EstimatedLiquidationCluster],
) -> Option<(
    &EstimatedLiquidationCluster,
    LiquidationToxicDirection,
    u8,
    String,
)> {
    for side in [
        LiquidationClusterSide::ShortAbove,
        LiquidationClusterSide::LongBelow,
    ] {
        let mut same_side = clusters
            .iter()
            .filter(|cluster| cluster.side == side)
            .collect::<Vec<_>>();
        same_side.sort_by(|left, right| left.distance_bps.total_cmp(&right.distance_bps));
        if same_side.len() < 2 {
            continue;
        }
        let gap_bps = same_side[1].distance_bps - same_side[0].distance_bps;
        if gap_bps <= CASCADE_GAP_BPS {
            let score = clamp_score(
                (cluster_density_score(same_side[0]) as f64 * 0.45
                    + cluster_density_score(same_side[1]) as f64 * 0.35
                    + ((CASCADE_GAP_BPS - gap_bps).max(0.0) / CASCADE_GAP_BPS) * 20.0)
                    .clamp(35.0, 95.0),
            );
            return Some((
                same_side[0],
                cluster_direction(side),
                score,
                "same-direction liquidation clusters are stair-stepped closely enough to form a cascade watch candidate"
                    .to_string(),
            ));
        }
    }
    None
}

fn cluster_supports_squeeze(cluster: &EstimatedLiquidationCluster) -> bool {
    cluster.cluster_density >= 0.45 || cluster.cluster_notional_usd >= ELEVATED_CLUSTER_NOTIONAL_USD
}

fn matching_active_trade_signal_ids(
    signals: &[ActiveTradeToxicSignal],
    predicate: impl Fn(&ActiveTradeToxicSignal) -> bool,
) -> Vec<String> {
    signals
        .iter()
        .filter(|signal| predicate(signal))
        .map(|signal| signal.signal_id.clone())
        .collect()
}

fn is_bullish_signal(signal: &ActiveTradeToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        ActiveTradeToxicSignalType::LargeAggressiveBuy
            | ActiveTradeToxicSignalType::BuySweep
            | ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
    ) || matches!(
        signal.signal_type,
        ActiveTradeToxicSignalType::TradeImbalance
    ) && matches!(signal.side, crate::types::toxic_flow::ToxicSide::Buy)
}

fn is_bearish_signal(signal: &ActiveTradeToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        ActiveTradeToxicSignalType::LargeAggressiveSell
            | ActiveTradeToxicSignalType::SellSweep
            | ActiveTradeToxicSignalType::OneHourDeltaSellDominant
    ) || matches!(
        signal.signal_type,
        ActiveTradeToxicSignalType::TradeImbalance
    ) && matches!(signal.side, crate::types::toxic_flow::ToxicSide::Sell)
}

fn cluster_direction(side: LiquidationClusterSide) -> LiquidationToxicDirection {
    match side {
        LiquidationClusterSide::ShortAbove => LiquidationToxicDirection::Upside,
        LiquidationClusterSide::LongBelow => LiquidationToxicDirection::Downside,
    }
}

fn magnet_weight(cluster: &EstimatedLiquidationCluster, proximity_threshold_bps: f64) -> f64 {
    let proximity_base = (proximity_threshold_bps.max(NEARBY_DISTANCE_BPS) * 3.0).max(1.0);
    let proximity_score = (1.0 - (cluster.distance_bps / proximity_base).min(1.0)) * 45.0;
    let density_score = cluster.cluster_density.clamp(0.0, 1.0) * 35.0;
    let notional_score =
        ((cluster.cluster_notional_usd / ELEVATED_CLUSTER_NOTIONAL_USD).min(2.0) / 2.0) * 20.0;
    proximity_score + density_score + notional_score
}

fn score_from_cluster(cluster: &EstimatedLiquidationCluster, floor: f64, cap: f64) -> u8 {
    let distance_component =
        (1.0 - (cluster.distance_bps / (NEARBY_DISTANCE_BPS * 2.0)).min(1.0)) * 35.0;
    let density_component = cluster.cluster_density.clamp(0.0, 1.0) * 35.0;
    let notional_component =
        ((cluster.cluster_notional_usd / ELEVATED_CLUSTER_NOTIONAL_USD).min(2.0) / 2.0) * 25.0;
    clamp_score(
        (floor + distance_component + density_component + notional_component).clamp(floor, cap),
    )
}

fn cluster_density_score(cluster: &EstimatedLiquidationCluster) -> u8 {
    clamp_score((cluster.cluster_density.clamp(0.0, 1.0) * 100.0).round())
}

fn magnet_score(cluster: &EstimatedLiquidationCluster, proximity_threshold_bps: f64) -> u8 {
    clamp_score(magnet_weight(cluster, proximity_threshold_bps))
}

fn signal_type_key(signal_type: LiquidationToxicSignalType) -> &'static str {
    match signal_type {
        LiquidationToxicSignalType::LiquidationClusterNearby => "liquidation_cluster_nearby",
        LiquidationToxicSignalType::UpsideLiquidationMagnet => "upside_liquidation_magnet",
        LiquidationToxicSignalType::DownsideLiquidationMagnet => "downside_liquidation_magnet",
        LiquidationToxicSignalType::LongSqueezeRisk => "long_squeeze_risk",
        LiquidationToxicSignalType::ShortSqueezeRisk => "short_squeeze_risk",
        LiquidationToxicSignalType::LiquidationCascadeCandidate => "liquidation_cascade_candidate",
        LiquidationToxicSignalType::LiquidationDeltaConfluence => "liquidation_delta_confluence",
    }
}

fn confidence_for_score(score: u8) -> ToxicConfidence {
    if score >= 80 {
        ToxicConfidence::High
    } else if score >= 55 {
        ToxicConfidence::Medium
    } else {
        ToxicConfidence::Low
    }
}

fn clamp_score(score: f64) -> u8 {
    score.round().clamp(0.0, 100.0) as u8
}

fn round_price(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_bps(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
