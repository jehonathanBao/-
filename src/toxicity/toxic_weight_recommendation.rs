use std::collections::{BTreeMap, HashMap};

use crate::types::{
    toxic_markout::{ToxicMarkoutOutcome, ToxicMarkoutRecentResponse},
    toxic_weight_recommendation::{
        ToxicWeightRecommendationItem, ToxicWeightRecommendationKind,
        ToxicWeightRecommendationReviewFlagSummary, ToxicWeightRecommendationSignalTypeSummary,
        ToxicWeightRecommendationStatusResponse, ToxicWeightRecommendationSummaryResponse,
        ToxicWeightRecommendationSymbolSummary,
    },
};

const MIN_SAMPLE_COUNT: usize = 20;
const STRONG_KEEP_ALIGNED_RATIO: f64 = 0.65;
const STRONG_KEEP_MAX_ADVERSE_RATIO: f64 = 0.25;
const SLIGHT_UPGRADE_ALIGNED_RATIO: f64 = 0.75;
const SLIGHT_UPGRADE_MAX_ADVERSE_RATIO: f64 = 0.20;
const BASE_KEEP_ALIGNED_RATIO: f64 = 0.50;
const BASE_KEEP_MAX_ADVERSE_RATIO: f64 = 0.35;
const DOWNGRADE_MAX_ALIGNED_RATIO: f64 = 0.45;
const DOWNGRADE_MIN_ADVERSE_RATIO: f64 = 0.45;
const NO_TRADE_MIN_ADVERSE_RATIO: f64 = 0.60;
const DISABLE_MIN_ADVERSE_RATIO: f64 = 0.70;
const DISABLE_MIN_SAMPLE_COUNT: usize = 50;

pub fn build_toxic_weight_recommendation_summary(
    recent: &ToxicMarkoutRecentResponse,
) -> ToxicWeightRecommendationSummaryResponse {
    let mut buckets: BTreeMap<String, RecommendationAccumulator> = BTreeMap::new();

    for signal in &recent.signals {
        let bucket = buckets
            .entry(signal.signal_kind.clone())
            .or_insert_with(|| RecommendationAccumulator::new(&signal.symbol, &signal.signal_kind));
        bucket.record(signal);
    }

    let recommendations = buckets
        .into_values()
        .map(|bucket| bucket.into_item())
        .collect::<Vec<_>>();
    let by_signal_type = recommendations
        .iter()
        .map(|item| ToxicWeightRecommendationSignalTypeSummary {
            signal_type: item.signal_type.clone(),
            sample_count: item.sample_count,
            aligned_ratio: item.aligned_ratio,
            adverse_ratio: item.adverse_ratio,
            neutral_ratio: item.neutral_ratio,
            best_window: item.best_window.clone(),
            worst_window: item.worst_window.clone(),
            recommendation: item.recommendation,
            confidence: item.confidence.clone(),
            reason_codes: item.reason_codes.clone(),
            manual_review_required: item.manual_review_required,
        })
        .collect::<Vec<_>>();
    let by_symbol = build_symbol_summaries(&recommendations);
    let review_flags = build_review_flags(&recommendations);

    let keep_count = count_recommendations(&recommendations, ToxicWeightRecommendationKind::Keep);
    let slight_upgrade_candidate_count = count_recommendations(
        &recommendations,
        ToxicWeightRecommendationKind::SlightUpgradeCandidate,
    );
    let slight_downgrade_candidate_count = count_recommendations(
        &recommendations,
        ToxicWeightRecommendationKind::SlightDowngradeCandidate,
    );
    let downgrade_candidate_count = count_recommendations(
        &recommendations,
        ToxicWeightRecommendationKind::DowngradeCandidate,
    );
    let no_trade_only_candidate_count = count_recommendations(
        &recommendations,
        ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
    );
    let disable_candidate_count = count_recommendations(
        &recommendations,
        ToxicWeightRecommendationKind::DisableCandidate,
    );
    let insufficient_data_count = count_recommendations(
        &recommendations,
        ToxicWeightRecommendationKind::InsufficientData,
    );

    let mut warnings = recent.warnings.clone();
    if recommendations.is_empty() {
        warnings.push(
            "No toxic markout evaluations are currently available for weight recommendations."
                .to_string(),
        );
    }
    if disable_candidate_count > 0 {
        warnings.push(
            "Disable candidates were surfaced from signal types with strongly adverse markout behavior."
                .to_string(),
        );
    }
    if no_trade_only_candidate_count > 0 {
        warnings.push(
            "No-trade-only candidates were surfaced where adverse markout pressure dominates."
                .to_string(),
        );
    }
    if insufficient_data_count > 0 {
        warnings.push(
            "Some signal types still have insufficient evaluated samples for a weight change suggestion."
                .to_string(),
        );
    }

    ToxicWeightRecommendationSummaryResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: recent.selected_symbol.clone(),
        status: if recommendations.is_empty() {
            "no_weight_recommendation_data".to_string()
        } else {
            "weight_recommendations_ready".to_string()
        },
        warnings,
        total_recommendations: recommendations.len(),
        keep_count,
        slight_upgrade_candidate_count,
        slight_downgrade_candidate_count,
        downgrade_candidate_count,
        no_trade_only_candidate_count,
        disable_candidate_count,
        insufficient_data_count,
        recommendations,
        by_signal_type,
        by_symbol,
        review_flags,
    }
}

pub fn build_toxic_weight_recommendation_status(
    summary: &ToxicWeightRecommendationSummaryResponse,
) -> ToxicWeightRecommendationStatusResponse {
    ToxicWeightRecommendationStatusResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_recommendations: summary.total_recommendations,
        manual_review_required_count: summary
            .recommendations
            .iter()
            .filter(|item| item.manual_review_required)
            .count(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysisOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "runtimeWeightModified=false".to_string(),
            "configModified=false".to_string(),
            "No automatic weight update".to_string(),
            "No runtime config mutation".to_string(),
            "No strategy reload".to_string(),
            "No order execution".to_string(),
        ],
    }
}

fn count_recommendations(
    recommendations: &[ToxicWeightRecommendationItem],
    kind: ToxicWeightRecommendationKind,
) -> usize {
    recommendations
        .iter()
        .filter(|item| item.recommendation == kind)
        .count()
}

fn build_symbol_summaries(
    recommendations: &[ToxicWeightRecommendationItem],
) -> Vec<ToxicWeightRecommendationSymbolSummary> {
    let mut buckets = BTreeMap::<String, ToxicWeightRecommendationSymbolSummary>::new();
    for item in recommendations {
        let bucket = buckets.entry(item.symbol.clone()).or_insert_with(|| {
            ToxicWeightRecommendationSymbolSummary {
                symbol: item.symbol.clone(),
                total_recommendations: 0,
                keep_count: 0,
                slight_upgrade_candidate_count: 0,
                slight_downgrade_candidate_count: 0,
                downgrade_candidate_count: 0,
                no_trade_only_candidate_count: 0,
                disable_candidate_count: 0,
                insufficient_data_count: 0,
                manual_review_required_count: 0,
            }
        });
        bucket.total_recommendations += 1;
        if item.manual_review_required {
            bucket.manual_review_required_count += 1;
        }
        match item.recommendation {
            ToxicWeightRecommendationKind::Keep => bucket.keep_count += 1,
            ToxicWeightRecommendationKind::SlightUpgradeCandidate => {
                bucket.slight_upgrade_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::SlightDowngradeCandidate => {
                bucket.slight_downgrade_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::DowngradeCandidate => {
                bucket.downgrade_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::NoTradeOnlyCandidate => {
                bucket.no_trade_only_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::DisableCandidate => {
                bucket.disable_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::InsufficientData => {
                bucket.insufficient_data_count += 1;
            }
        }
    }
    buckets.into_values().collect()
}

fn build_review_flags(
    recommendations: &[ToxicWeightRecommendationItem],
) -> Vec<ToxicWeightRecommendationReviewFlagSummary> {
    let mut counts = BTreeMap::<String, usize>::new();
    for item in recommendations {
        for code in &item.reason_codes {
            if let Some(review_flag) = review_flag_for_reason(code) {
                *counts.entry(review_flag.to_string()).or_insert(0) += 1;
            }
        }
    }

    counts
        .into_iter()
        .map(
            |(review_flag, count)| ToxicWeightRecommendationReviewFlagSummary {
                severity: severity_for_review_flag(&review_flag).to_string(),
                review_flag,
                count,
                manual_review_required: true,
            },
        )
        .collect()
}

fn review_flag_for_reason(reason_code: &str) -> Option<&'static str> {
    match reason_code {
        "insufficient_data" => Some("insufficient_data_manual_review"),
        "slight_upgrade_candidate" => Some("slight_upgrade_manual_review"),
        "slight_downgrade_candidate" => Some("slight_downgrade_manual_review"),
        "downgrade_candidate" => Some("downgrade_manual_review"),
        "no_trade_only_candidate" => Some("no_trade_only_manual_review"),
        "disable_candidate" => Some("disable_candidate_manual_review"),
        _ => None,
    }
}

fn severity_for_review_flag(review_flag: &str) -> &'static str {
    match review_flag {
        "disable_candidate_manual_review" => "high",
        "no_trade_only_manual_review" => "high",
        "downgrade_manual_review" => "medium",
        "slight_downgrade_manual_review" => "medium",
        "slight_upgrade_manual_review" => "low",
        "insufficient_data_manual_review" => "low",
        _ => "info",
    }
}

#[derive(Default, Clone, Copy)]
struct OutcomeCounter {
    aligned: usize,
    adverse: usize,
    neutral: usize,
    not_enough_data: usize,
}

impl OutcomeCounter {
    fn record(&mut self, outcome: ToxicMarkoutOutcome) {
        match outcome {
            ToxicMarkoutOutcome::Aligned => self.aligned += 1,
            ToxicMarkoutOutcome::Adverse => self.adverse += 1,
            ToxicMarkoutOutcome::Neutral => self.neutral += 1,
            ToxicMarkoutOutcome::NotEnoughData => self.not_enough_data += 1,
        }
    }

    fn total(self) -> usize {
        self.aligned + self.adverse + self.neutral + self.not_enough_data
    }

    fn aligned_ratio(self) -> f64 {
        ratio(self.aligned, self.total())
    }

    fn adverse_ratio(self) -> f64 {
        ratio(self.adverse, self.total())
    }

    fn neutral_ratio(self) -> f64 {
        ratio(self.neutral, self.total())
    }

    fn not_enough_data_ratio(self) -> f64 {
        ratio(self.not_enough_data, self.total())
    }
}

struct RecommendationAccumulator {
    symbol: String,
    signal_type: String,
    overall: OutcomeCounter,
    windows: BTreeMap<String, OutcomeCounter>,
    evidence_reasons: HashMap<String, usize>,
}

impl RecommendationAccumulator {
    fn new(symbol: &str, signal_type: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            signal_type: signal_type.to_string(),
            overall: OutcomeCounter::default(),
            windows: BTreeMap::new(),
            evidence_reasons: HashMap::new(),
        }
    }

    fn record(&mut self, signal: &crate::types::toxic_markout::ToxicMarkoutSignal) {
        self.overall.record(signal.overall_outcome);
        for window in &signal.windows {
            self.windows
                .entry(window.label.clone())
                .or_default()
                .record(window.outcome);
        }
        for reason in &signal.no_trade_reasons {
            *self.evidence_reasons.entry(reason.clone()).or_insert(0) += 1;
        }
    }

    fn into_item(self) -> ToxicWeightRecommendationItem {
        let sample_count = self.overall.total();
        let aligned_ratio = self.overall.aligned_ratio();
        let adverse_ratio = self.overall.adverse_ratio();
        let neutral_ratio = self.overall.neutral_ratio();
        let best_window = pick_best_window(&self.windows);
        let worst_window = pick_worst_window(&self.windows);
        let recommendation = classify_recommendation(sample_count, aligned_ratio, adverse_ratio);
        let confidence = classify_confidence(sample_count, recommendation);
        let reason_codes =
            build_reason_codes(sample_count, aligned_ratio, adverse_ratio, recommendation);
        let evidence = build_evidence(
            self.overall,
            best_window.as_deref(),
            worst_window.as_deref(),
            &self.evidence_reasons,
        );
        ToxicWeightRecommendationItem {
            symbol: self.symbol,
            signal_type: self.signal_type,
            sample_count,
            aligned_ratio,
            adverse_ratio,
            neutral_ratio,
            best_window,
            worst_window,
            recommendation,
            current_weight_hint: "read_only_not_loaded".to_string(),
            suggested_weight_hint: suggested_weight_hint(recommendation).to_string(),
            confidence: confidence.to_string(),
            reason_codes,
            evidence,
            manual_review_required: recommendation != ToxicWeightRecommendationKind::Keep,
            runtime_weight_modified: false,
            config_modified: false,
        }
    }
}

fn pick_best_window(windows: &BTreeMap<String, OutcomeCounter>) -> Option<String> {
    windows
        .iter()
        .max_by(|left, right| {
            left.1
                .aligned_ratio()
                .total_cmp(&right.1.aligned_ratio())
                .then_with(|| right.1.adverse_ratio().total_cmp(&left.1.adverse_ratio()))
        })
        .map(|(label, _)| label.clone())
}

fn pick_worst_window(windows: &BTreeMap<String, OutcomeCounter>) -> Option<String> {
    windows
        .iter()
        .max_by(|left, right| {
            left.1
                .adverse_ratio()
                .total_cmp(&right.1.adverse_ratio())
                .then_with(|| right.1.aligned_ratio().total_cmp(&left.1.aligned_ratio()))
        })
        .map(|(label, _)| label.clone())
}

fn classify_recommendation(
    sample_count: usize,
    aligned_ratio: f64,
    adverse_ratio: f64,
) -> ToxicWeightRecommendationKind {
    if sample_count < MIN_SAMPLE_COUNT {
        return ToxicWeightRecommendationKind::InsufficientData;
    }
    if adverse_ratio >= DISABLE_MIN_ADVERSE_RATIO && sample_count >= DISABLE_MIN_SAMPLE_COUNT {
        return ToxicWeightRecommendationKind::DisableCandidate;
    }
    if adverse_ratio >= NO_TRADE_MIN_ADVERSE_RATIO {
        return ToxicWeightRecommendationKind::NoTradeOnlyCandidate;
    }
    if aligned_ratio >= SLIGHT_UPGRADE_ALIGNED_RATIO
        && adverse_ratio <= SLIGHT_UPGRADE_MAX_ADVERSE_RATIO
    {
        return ToxicWeightRecommendationKind::SlightUpgradeCandidate;
    }
    if aligned_ratio >= STRONG_KEEP_ALIGNED_RATIO && adverse_ratio <= STRONG_KEEP_MAX_ADVERSE_RATIO
    {
        return ToxicWeightRecommendationKind::Keep;
    }
    if aligned_ratio < DOWNGRADE_MAX_ALIGNED_RATIO && adverse_ratio >= DOWNGRADE_MIN_ADVERSE_RATIO {
        return ToxicWeightRecommendationKind::DowngradeCandidate;
    }
    if adverse_ratio >= BASE_KEEP_MAX_ADVERSE_RATIO
        || (aligned_ratio < BASE_KEEP_ALIGNED_RATIO && adverse_ratio > 0.0)
    {
        return ToxicWeightRecommendationKind::SlightDowngradeCandidate;
    }
    if aligned_ratio >= BASE_KEEP_ALIGNED_RATIO && adverse_ratio <= BASE_KEEP_MAX_ADVERSE_RATIO {
        return ToxicWeightRecommendationKind::Keep;
    }
    ToxicWeightRecommendationKind::Keep
}

fn classify_confidence(
    sample_count: usize,
    recommendation: ToxicWeightRecommendationKind,
) -> &'static str {
    if sample_count >= 50
        && matches!(
            recommendation,
            ToxicWeightRecommendationKind::DisableCandidate
                | ToxicWeightRecommendationKind::NoTradeOnlyCandidate
                | ToxicWeightRecommendationKind::SlightUpgradeCandidate
        )
    {
        return "high";
    }
    if sample_count >= MIN_SAMPLE_COUNT {
        return "medium";
    }
    "low"
}

fn build_reason_codes(
    sample_count: usize,
    aligned_ratio: f64,
    adverse_ratio: f64,
    recommendation: ToxicWeightRecommendationKind,
) -> Vec<String> {
    let mut reason_codes = Vec::new();
    if sample_count < MIN_SAMPLE_COUNT {
        reason_codes.push("insufficient_data".to_string());
    }
    match recommendation {
        ToxicWeightRecommendationKind::Keep => {
            if aligned_ratio >= STRONG_KEEP_ALIGNED_RATIO
                && adverse_ratio <= STRONG_KEEP_MAX_ADVERSE_RATIO
            {
                reason_codes.push("strong_keep_zone".to_string());
            } else {
                reason_codes.push("balanced_keep_zone".to_string());
            }
        }
        ToxicWeightRecommendationKind::SlightUpgradeCandidate => {
            reason_codes.push("slight_upgrade_candidate".to_string());
            reason_codes.push("strong_aligned_low_adverse".to_string());
        }
        ToxicWeightRecommendationKind::SlightDowngradeCandidate => {
            reason_codes.push("slight_downgrade_candidate".to_string());
            reason_codes.push("mixed_quality_needs_caution".to_string());
        }
        ToxicWeightRecommendationKind::DowngradeCandidate => {
            reason_codes.push("downgrade_candidate".to_string());
            reason_codes.push("adverse_ratio_high".to_string());
        }
        ToxicWeightRecommendationKind::NoTradeOnlyCandidate => {
            reason_codes.push("no_trade_only_candidate".to_string());
            reason_codes.push("adverse_ratio_extreme".to_string());
        }
        ToxicWeightRecommendationKind::DisableCandidate => {
            reason_codes.push("disable_candidate".to_string());
            reason_codes.push("adverse_ratio_disable_threshold".to_string());
        }
        ToxicWeightRecommendationKind::InsufficientData => {
            if !reason_codes.iter().any(|code| code == "insufficient_data") {
                reason_codes.push("insufficient_data".to_string());
            }
        }
    }
    reason_codes
}

fn build_evidence(
    overall: OutcomeCounter,
    best_window: Option<&str>,
    worst_window: Option<&str>,
    reasons: &HashMap<String, usize>,
) -> Vec<String> {
    let mut evidence = vec![
        format!("sample_count={}", overall.total()),
        format!("aligned_ratio={:.4}", overall.aligned_ratio()),
        format!("adverse_ratio={:.4}", overall.adverse_ratio()),
        format!("neutral_ratio={:.4}", overall.neutral_ratio()),
        format!(
            "not_enough_data_ratio={:.4}",
            overall.not_enough_data_ratio()
        ),
    ];
    if let Some(best_window) = best_window {
        evidence.push(format!("best_window={best_window}"));
    }
    if let Some(worst_window) = worst_window {
        evidence.push(format!("worst_window={worst_window}"));
    }
    if let Some((reason, count)) = reasons.iter().max_by_key(|(_, count)| *count) {
        evidence.push(format!("top_no_trade_reason={reason} ({count})"));
    }
    evidence
}

fn suggested_weight_hint(recommendation: ToxicWeightRecommendationKind) -> &'static str {
    match recommendation {
        ToxicWeightRecommendationKind::Keep => "keep_current_weight",
        ToxicWeightRecommendationKind::SlightUpgradeCandidate => "candidate_plus_10_percent_weight",
        ToxicWeightRecommendationKind::SlightDowngradeCandidate => {
            "candidate_minus_10_percent_weight"
        }
        ToxicWeightRecommendationKind::DowngradeCandidate => "candidate_minus_25_percent_weight",
        ToxicWeightRecommendationKind::NoTradeOnlyCandidate => "candidate_no_trade_only_gate",
        ToxicWeightRecommendationKind::DisableCandidate => "candidate_disable_signal",
        ToxicWeightRecommendationKind::InsufficientData => "defer_until_more_samples",
    }
}

fn ratio(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}
