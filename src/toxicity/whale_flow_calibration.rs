use std::{collections::BTreeMap, fmt::Write};

use crate::types::{
    toxic_flow::ToxicSide,
    toxic_markout::{ToxicMarkoutOutcome, ToxicMarkoutRecentResponse, ToxicMarkoutSignal},
    toxic_signal_history::ToxicSignalHistoryStatusResponse,
    whale_flow_calibration::{
        WhaleFlowCalibrationBaselineSourceItem, WhaleFlowCalibrationClassificationQualityItem,
        WhaleFlowCalibrationEvidenceSource, WhaleFlowCalibrationManualTuningNote,
        WhaleFlowCalibrationOutcomeLinkage, WhaleFlowCalibrationReportResponse,
        WhaleFlowCalibrationSampleStatus, WhaleFlowCalibrationStatusResponse,
        WhaleFlowCalibrationThresholdPerformanceItem,
        WhaleFlowCalibrationThresholdPerformanceSummary, WhaleFlowCalibrationVenueConfluenceItem,
    },
    whale_flow_signal::{WhaleFlowCandidate, WhaleFlowCandidateType, WhaleFlowRecentResponse},
};

const MIN_CALIBRATION_SAMPLES: usize = 20;
const MIN_RESOLVED_EVIDENCE_SAMPLES: usize = 10;
const MAX_NOT_ENOUGH_DATA_RATE_FOR_TUNING: f64 = 0.50;
const FALLBACK_TIME_TOLERANCE_MS: u64 = 15_000;

#[derive(Debug, Default, Clone, Copy)]
struct OutcomeCounts {
    aligned: usize,
    adverse: usize,
    neutral: usize,
    not_enough_data: usize,
}

impl OutcomeCounts {
    fn record(&mut self, outcome: &str) {
        match outcome {
            "aligned" => self.aligned += 1,
            "adverse" => self.adverse += 1,
            "neutral" => self.neutral += 1,
            _ => self.not_enough_data += 1,
        }
    }

    fn sample_count(self) -> usize {
        self.aligned + self.adverse + self.neutral + self.not_enough_data
    }

    fn linked_markout_samples(self) -> usize {
        self.aligned + self.adverse + self.neutral
    }

    fn aligned_rate(self) -> f64 {
        ratio(self.aligned, self.sample_count())
    }

    fn adverse_rate(self) -> f64 {
        ratio(self.adverse, self.sample_count())
    }

    fn neutral_rate(self) -> f64 {
        ratio(self.neutral, self.sample_count())
    }

    fn not_enough_data_rate(self) -> f64 {
        ratio(self.not_enough_data, self.sample_count())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutcomeLinkageMode {
    LinkedSignalId,
    TimeSymbolDirectionFallback,
    NoOutcomeLinkage,
}

#[derive(Debug, Clone, Copy)]
struct CandidateOutcome {
    outcome: &'static str,
    linkage_mode: OutcomeLinkageMode,
}

pub fn build_whale_flow_calibration_report(
    selected_symbol: &str,
    whale_flow: &WhaleFlowRecentResponse,
    markout: &ToxicMarkoutRecentResponse,
    history_status: &ToxicSignalHistoryStatusResponse,
) -> WhaleFlowCalibrationReportResponse {
    let markout_map = markout
        .signals
        .iter()
        .map(|signal| (signal.signal_id.as_str(), signal))
        .collect::<BTreeMap<_, _>>();
    let outcome_linkage = build_outcome_linkage(whale_flow, &markout_map);
    let evidence_source = build_evidence_source(whale_flow, history_status, &outcome_linkage);
    let sample_status = build_sample_status(
        whale_flow,
        &markout_map,
        history_status,
        evidence_source.uses_current_snapshot_only,
    );
    let threshold_performance =
        build_threshold_performance(whale_flow, &markout_map, &whale_flow.thresholds);
    let by_classification = build_classification_quality(whale_flow, &markout_map);
    let venue_confluence = build_venue_confluence_quality(whale_flow, &markout_map);
    let baseline_source_quality = build_baseline_source_quality(whale_flow, &markout_map);
    let manual_tuning_notes = build_manual_tuning_notes(
        whale_flow,
        &sample_status,
        &threshold_performance,
        &venue_confluence,
        &baseline_source_quality,
    );

    let mut warnings = whale_flow.warnings.clone();
    if whale_flow.candidates.is_empty() {
        warnings.push("No whale flow candidates available".to_string());
    }
    if !sample_status.enough_data {
        warnings.push("Calibration evidence too thin".to_string());
        warnings.push("Resolved markout evidence is insufficient".to_string());
    }
    if evidence_source.uses_current_snapshot_only {
        warnings.push("Current snapshot only: tuning disabled".to_string());
    }
    if baseline_source_quality
        .iter()
        .all(|item| item.baseline_source != "one_hour_normalized" || item.sample_count == 0)
    {
        warnings.push("Baseline insufficient".to_string());
    }
    if venue_confluence.is_empty() {
        warnings.push("No venue confluence samples".to_string());
    }
    if sample_status.linked_markout_samples == 0 {
        warnings.push("Markout not_enough_data".to_string());
    }
    dedup_strings(&mut warnings);

    let status = if history_status.current_signals == 0 {
        "insufficient_history"
    } else if whale_flow.candidates.is_empty() {
        "no_whale_flow_candidates"
    } else if !sample_status.enough_data {
        sample_status
            .blocked_reason
            .as_deref()
            .unwrap_or("not_enough_samples")
    } else if sample_status.linked_markout_samples == 0 {
        "markout_not_enough_data"
    } else {
        "calibration_ready"
    };

    let markdown = build_markdown_report(
        selected_symbol,
        status,
        &sample_status,
        &whale_flow.thresholds,
        &threshold_performance,
        &by_classification,
        &venue_confluence,
        &baseline_source_quality,
        &manual_tuning_notes,
        &warnings,
    );

    WhaleFlowCalibrationReportResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        runtime_modified: false,
        manual_review_required: true,
        threshold_modified: false,
        config_modified: false,
        runtime_threshold_modified: false,
        auto_apply_enabled: false,
        selected_symbol: selected_symbol.to_string(),
        status: status.to_string(),
        evidence_source,
        outcome_linkage,
        sample_status,
        current_thresholds: whale_flow.thresholds.clone(),
        threshold_performance,
        by_classification,
        venue_confluence,
        baseline_source_quality,
        manual_tuning_notes,
        warnings,
        no_candidate_reasons: whale_flow.no_candidate_reasons.clone(),
        markdown,
    }
}

pub fn build_whale_flow_calibration_status(
    report: &WhaleFlowCalibrationReportResponse,
) -> WhaleFlowCalibrationStatusResponse {
    WhaleFlowCalibrationStatusResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        runtime_modified: false,
        manual_review_required: true,
        threshold_modified: false,
        config_modified: false,
        runtime_threshold_modified: false,
        auto_apply_enabled: false,
        enabled: true,
        selected_symbol: report.selected_symbol.clone(),
        status: report.status.clone(),
        total_candidates: report.sample_status.total_candidates,
        linked_markout_samples: report.sample_status.linked_markout_samples,
        resolved_markout_evidence_count: report.sample_status.resolved_markout_evidence_count,
        min_samples_required: report.sample_status.min_samples_required,
        min_resolved_evidence_required: report.sample_status.min_resolved_evidence_required,
        enough_data: report.sample_status.enough_data,
        current_thresholds: report.current_thresholds.clone(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "runtimeModified=false".to_string(),
            "manualReviewRequired=true".to_string(),
            "thresholdModified=false".to_string(),
            "configModified=false".to_string(),
            "runtimeThresholdModified=false".to_string(),
            "autoApplyEnabled=false".to_string(),
            "No threshold modification".to_string(),
            "No config write".to_string(),
            "No apply / reload".to_string(),
            "No order placement".to_string(),
            "No wallet / signing".to_string(),
            "No live trading".to_string(),
            "No DB / JSONL / SQLite / archive write".to_string(),
        ],
    }
}

pub fn calibration_min_candidates_required() -> usize {
    MIN_CALIBRATION_SAMPLES
}

pub fn calibration_min_resolved_evidence_required() -> usize {
    MIN_RESOLVED_EVIDENCE_SAMPLES
}

pub fn calibration_max_not_enough_data_rate_for_tuning() -> f64 {
    MAX_NOT_ENOUGH_DATA_RATE_FOR_TUNING
}

pub fn resolve_whale_candidate_markout(
    candidate: &WhaleFlowCandidate,
    markout: &ToxicMarkoutRecentResponse,
) -> (&'static str, &'static str) {
    let markout_map = markout
        .signals
        .iter()
        .map(|signal| (signal.signal_id.as_str(), signal))
        .collect::<BTreeMap<_, _>>();
    let outcome = candidate_outcome(candidate, &markout_map);
    let outcome_status = match outcome.linkage_mode {
        OutcomeLinkageMode::NoOutcomeLinkage => "unresolved",
        OutcomeLinkageMode::LinkedSignalId | OutcomeLinkageMode::TimeSymbolDirectionFallback => {
            "resolved"
        }
    };
    (outcome_status, outcome.outcome)
}

fn build_sample_status(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
    history_status: &ToxicSignalHistoryStatusResponse,
    uses_current_snapshot_only: bool,
) -> WhaleFlowCalibrationSampleStatus {
    let mut counts = OutcomeCounts::default();
    for candidate in &whale_flow.candidates {
        counts.record(candidate_markout_outcome(candidate, markout_map));
    }
    let total_candidates = whale_flow.candidates.len();
    let resolved_markout_evidence_count = counts.linked_markout_samples();
    let unresolved_markout_count = counts.not_enough_data;
    let not_enough_data_rate = counts.not_enough_data_rate();
    let mut blocked_reasons = Vec::new();
    if uses_current_snapshot_only {
        blocked_reasons.push("current_snapshot_only".to_string());
    }
    if total_candidates < MIN_CALIBRATION_SAMPLES {
        blocked_reasons.push("candidate_count_too_low".to_string());
    }
    if resolved_markout_evidence_count < MIN_RESOLVED_EVIDENCE_SAMPLES {
        blocked_reasons.push("resolved_markout_evidence_too_thin".to_string());
    }
    if not_enough_data_rate > MAX_NOT_ENOUGH_DATA_RATE_FOR_TUNING {
        blocked_reasons.push("not_enough_data_rate_too_high".to_string());
    }
    let enough_data = blocked_reasons.is_empty();
    let blocked_reason = blocked_reasons.first().cloned();
    WhaleFlowCalibrationSampleStatus {
        total_candidates,
        linked_markout_samples: resolved_markout_evidence_count,
        resolved_markout_evidence_count,
        unresolved_markout_count,
        not_enough_data_rate,
        min_samples_required: MIN_CALIBRATION_SAMPLES,
        min_resolved_evidence_required: MIN_RESOLVED_EVIDENCE_SAMPLES,
        max_not_enough_data_rate_for_tuning: MAX_NOT_ENOUGH_DATA_RATE_FOR_TUNING,
        enough_data,
        blocked_reason,
        blocked_reasons,
        retention_mode: history_status.retention_mode.clone(),
    }
}

fn build_evidence_source(
    whale_flow: &WhaleFlowRecentResponse,
    history_status: &ToxicSignalHistoryStatusResponse,
    outcome_linkage: &WhaleFlowCalibrationOutcomeLinkage,
) -> WhaleFlowCalibrationEvidenceSource {
    let uses_current_snapshot_only = whale_flow.history_baseline_mode != "whale_candidate_history"
        || whale_flow.candidates.is_empty();
    WhaleFlowCalibrationEvidenceSource {
        mode: if !uses_current_snapshot_only {
            "in_memory_whale_candidate_history"
        } else if history_status.current_signals == 0 {
            "insufficient_history"
        } else {
            "current_snapshot_fallback"
        }
        .to_string(),
        uses_current_snapshot_only,
        current_snapshot_fallback_used: uses_current_snapshot_only,
        history_signals_available: if uses_current_snapshot_only {
            history_status.current_signals
        } else {
            whale_flow.candidates.len()
        },
        whale_candidates_evaluated: whale_flow.candidates.len(),
        resolved_markout_evidence_count: outcome_linkage.linked_signal_id_matches
            + outcome_linkage.fallback_matches,
        unresolved_markout_count: outcome_linkage.no_outcome_linkage_count,
    }
}

fn build_outcome_linkage(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
) -> WhaleFlowCalibrationOutcomeLinkage {
    let mut linked_signal_id_matches = 0;
    let mut fallback_matches = 0;
    let mut no_outcome_linkage_count = 0;

    for candidate in &whale_flow.candidates {
        match candidate_outcome(candidate, markout_map).linkage_mode {
            OutcomeLinkageMode::LinkedSignalId => linked_signal_id_matches += 1,
            OutcomeLinkageMode::TimeSymbolDirectionFallback => fallback_matches += 1,
            OutcomeLinkageMode::NoOutcomeLinkage => no_outcome_linkage_count += 1,
        }
    }

    let mut operator_warnings = Vec::new();
    if fallback_matches > 0 {
        operator_warnings.push(
            "Whale flow candidates without linked signal ids matched markout by symbol/time/direction fallback."
                .to_string(),
        );
    }
    if no_outcome_linkage_count > 0 {
        operator_warnings.push(
            "Some whale flow candidates have no markout outcome linkage and remain not_enough_data."
                .to_string(),
        );
    }

    WhaleFlowCalibrationOutcomeLinkage {
        linked_signal_id_matches,
        fallback_matches,
        no_outcome_linkage_count,
        fallback_used: fallback_matches > 0,
        operator_warnings,
    }
}

fn build_threshold_performance(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
    thresholds: &crate::types::whale_flow_signal::WhaleFlowThresholds,
) -> WhaleFlowCalibrationThresholdPerformanceSummary {
    WhaleFlowCalibrationThresholdPerformanceSummary {
        one_second_btc: build_window_threshold_item(
            whale_flow,
            markout_map,
            1_000,
            thresholds.one_second_btc,
        ),
        five_second_btc: build_window_threshold_item(
            whale_flow,
            markout_map,
            5_000,
            thresholds.five_second_btc,
        ),
        fifteen_second_btc: build_window_threshold_item(
            whale_flow,
            markout_map,
            15_000,
            thresholds.fifteen_second_btc,
        ),
        sixty_second_btc: build_window_threshold_item(
            whale_flow,
            markout_map,
            60_000,
            thresholds.sixty_second_btc,
        ),
    }
}

fn build_window_threshold_item(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
    window_ms: u64,
    threshold: f64,
) -> WhaleFlowCalibrationThresholdPerformanceItem {
    let mut counts = OutcomeCounts::default();
    for candidate in whale_flow
        .candidates
        .iter()
        .filter(|candidate| candidate.window_ms == window_ms)
    {
        counts.record(candidate_markout_outcome(candidate, markout_map));
    }

    let adverse_rate = counts.adverse_rate();
    let aligned_rate = counts.aligned_rate();
    let verdict = if counts.linked_markout_samples() < 3 {
        "needs_more_data"
    } else if adverse_rate >= 0.30 && adverse_rate >= aligned_rate {
        "slight_raise_candidate"
    } else {
        "keep"
    };

    WhaleFlowCalibrationThresholdPerformanceItem {
        threshold,
        candidate_count: counts.sample_count(),
        aligned_rate,
        adverse_rate,
        neutral_rate: counts.neutral_rate(),
        not_enough_data_rate: counts.not_enough_data_rate(),
        verdict: verdict.to_string(),
    }
}

fn build_classification_quality(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
) -> Vec<WhaleFlowCalibrationClassificationQualityItem> {
    let classifications = [
        WhaleFlowCandidateType::AggressiveBuy,
        WhaleFlowCandidateType::AggressiveSell,
        WhaleFlowCandidateType::Absorption,
        WhaleFlowCandidateType::LiquidationSweep,
        WhaleFlowCandidateType::Trap,
    ];

    classifications
        .into_iter()
        .map(|classification| {
            let mut counts = OutcomeCounts::default();
            for candidate in whale_flow
                .candidates
                .iter()
                .filter(|candidate| candidate.candidate_type == classification)
            {
                counts.record(candidate_markout_outcome(candidate, markout_map));
            }
            let quality_bucket = quality_bucket_for(counts);
            WhaleFlowCalibrationClassificationQualityItem {
                classification: classification_key(classification).to_string(),
                sample_count: counts.sample_count(),
                aligned_rate: counts.aligned_rate(),
                adverse_rate: counts.adverse_rate(),
                neutral_rate: counts.neutral_rate(),
                not_enough_data_rate: counts.not_enough_data_rate(),
                quality_bucket: quality_bucket.to_string(),
                manual_tuning_note: classification_note(classification_key(classification), counts)
                    .to_string(),
            }
        })
        .collect()
}

fn build_venue_confluence_quality(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
) -> Vec<WhaleFlowCalibrationVenueConfluenceItem> {
    let mut buckets = BTreeMap::<usize, OutcomeCounts>::new();
    for candidate in &whale_flow.candidates {
        buckets
            .entry(candidate.same_direction_venues)
            .or_default()
            .record(candidate_markout_outcome(candidate, markout_map));
    }

    buckets
        .into_iter()
        .map(
            |(venue_count, counts)| WhaleFlowCalibrationVenueConfluenceItem {
                venue_count,
                sample_count: counts.sample_count(),
                aligned_rate: counts.aligned_rate(),
                adverse_rate: counts.adverse_rate(),
                neutral_rate: counts.neutral_rate(),
                not_enough_data_rate: counts.not_enough_data_rate(),
                verdict: venue_confluence_verdict(venue_count, counts).to_string(),
            },
        )
        .collect()
}

fn build_baseline_source_quality(
    whale_flow: &WhaleFlowRecentResponse,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
) -> Vec<WhaleFlowCalibrationBaselineSourceItem> {
    let order = [
        "one_hour_normalized",
        "sixty_second_fallback",
        "longer_window_fallback",
        "insufficient_history",
    ];
    let mut buckets = BTreeMap::<String, OutcomeCounts>::new();
    for candidate in &whale_flow.candidates {
        let key = candidate_baseline_source(candidate).to_string();
        buckets
            .entry(key)
            .or_default()
            .record(candidate_markout_outcome(candidate, markout_map));
    }

    order
        .into_iter()
        .map(|baseline_source| {
            let counts = buckets.get(baseline_source).copied().unwrap_or_default();
            WhaleFlowCalibrationBaselineSourceItem {
                baseline_source: baseline_source.to_string(),
                sample_count: counts.sample_count(),
                aligned_rate: counts.aligned_rate(),
                adverse_rate: counts.adverse_rate(),
                neutral_rate: counts.neutral_rate(),
                not_enough_data_rate: counts.not_enough_data_rate(),
                quality_bucket: quality_bucket_for(counts).to_string(),
                manual_tuning_note: baseline_source_note(baseline_source, counts).to_string(),
            }
        })
        .collect()
}

fn build_manual_tuning_notes(
    whale_flow: &WhaleFlowRecentResponse,
    sample_status: &WhaleFlowCalibrationSampleStatus,
    threshold_performance: &WhaleFlowCalibrationThresholdPerformanceSummary,
    venue_confluence: &[WhaleFlowCalibrationVenueConfluenceItem],
    baseline_source_quality: &[WhaleFlowCalibrationBaselineSourceItem],
) -> Vec<WhaleFlowCalibrationManualTuningNote> {
    let thresholds = &whale_flow.thresholds;
    if !sample_status.enough_data {
        return vec![
            manual_note(
                "oneSecondBtc",
                thresholds.one_second_btc,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
            manual_note(
                "fiveSecondBtc",
                thresholds.five_second_btc,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
            manual_note(
                "fifteenSecondBtc",
                thresholds.fifteen_second_btc,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
            manual_note(
                "sixtySecondBtc",
                thresholds.sixty_second_btc,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
            manual_note(
                "directionRatioMin",
                thresholds.direction_ratio_min,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
            manual_note(
                "relativeVolumeMultipleMin",
                thresholds.relative_volume_multiple_min,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
            manual_note(
                "minVenueConfirmations",
                thresholds.min_venue_confirmations as f64,
                "needs_more_data",
                "Not enough samples for calibration.",
            ),
        ];
    }

    let one_hour_samples = baseline_source_quality
        .iter()
        .find(|item| item.baseline_source == "one_hour_normalized")
        .map(|item| item.sample_count)
        .unwrap_or_default();
    let best_two_venue = venue_confluence
        .iter()
        .find(|item| item.venue_count == thresholds.min_venue_confirmations);
    let stronger_multi_venue = venue_confluence.iter().find(|item| {
        item.venue_count > thresholds.min_venue_confirmations && item.sample_count > 0
    });
    let average_direction_ratio = ratio_sum(
        whale_flow
            .candidates
            .iter()
            .map(|candidate| candidate.direction_bias),
        whale_flow.candidates.len(),
    );

    vec![
        threshold_note(
            "oneSecondBtc",
            thresholds.one_second_btc,
            &threshold_performance.one_second_btc,
        ),
        threshold_note(
            "fiveSecondBtc",
            thresholds.five_second_btc,
            &threshold_performance.five_second_btc,
        ),
        threshold_note(
            "fifteenSecondBtc",
            thresholds.fifteen_second_btc,
            &threshold_performance.fifteen_second_btc,
        ),
        threshold_note(
            "sixtySecondBtc",
            thresholds.sixty_second_btc,
            &threshold_performance.sixty_second_btc,
        ),
        if sample_status.linked_markout_samples < 5 {
            manual_note(
                "directionRatioMin",
                thresholds.direction_ratio_min,
                "needs_more_data",
                "Too few linked markout samples are available for direction-ratio review.",
            )
        } else if average_direction_ratio >= 0.85
            && overall_adverse_rate(threshold_performance) >= 0.25
        {
            manual_note(
                "directionRatioMin",
                thresholds.direction_ratio_min,
                "slight_raise_candidate",
                "Direction concentration is already high while adverse follow-through remains elevated.",
            )
        } else {
            manual_note(
                "directionRatioMin",
                thresholds.direction_ratio_min,
                "keep",
                "Current direction-ratio gate is bounded and does not justify runtime tuning.",
            )
        },
        if one_hour_samples < 5 {
            manual_note(
                "relativeVolumeMultipleMin",
                thresholds.relative_volume_multiple_min,
                "needs_more_data",
                "Too few valid 1h baseline samples are available.",
            )
        } else {
            manual_note(
                "relativeVolumeMultipleMin",
                thresholds.relative_volume_multiple_min,
                "keep",
                "One-hour baseline samples are present and do not justify changing the relative-volume gate.",
            )
        },
        if let Some(two_venue) = best_two_venue {
            if two_venue.sample_count < 3 {
                manual_note(
                    "minVenueConfirmations",
                    thresholds.min_venue_confirmations as f64,
                    "needs_more_data",
                    "Too few two-venue samples are available for venue-confluence review.",
                )
            } else if two_venue.adverse_rate >= 0.30
                && stronger_multi_venue
                    .is_some_and(|item| item.aligned_rate > two_venue.aligned_rate)
            {
                manual_note(
                    "minVenueConfirmations",
                    thresholds.min_venue_confirmations as f64,
                    "slight_raise_candidate",
                    "Higher venue confluence looks cleaner than the current minimum gate.",
                )
            } else {
                manual_note(
                    "minVenueConfirmations",
                    thresholds.min_venue_confirmations as f64,
                    "keep",
                    "Current minimum venue confluence looks reasonable in the available sample.",
                )
            }
        } else {
            manual_note(
                "minVenueConfirmations",
                thresholds.min_venue_confirmations as f64,
                "needs_more_data",
                "No venue confluence samples are available for review.",
            )
        },
    ]
}

fn threshold_note(
    target: &str,
    current_value: f64,
    performance: &WhaleFlowCalibrationThresholdPerformanceItem,
) -> WhaleFlowCalibrationManualTuningNote {
    if resolved_threshold_samples(performance) < 3 {
        return manual_note(
            target,
            current_value,
            "needs_more_data",
            "Too few resolved markout samples are available for this threshold window.",
        );
    }
    if performance.adverse_rate >= 0.30 && performance.adverse_rate >= performance.aligned_rate {
        manual_note(
            target,
            current_value,
            "slight_raise_candidate",
            "Adverse follow-through is elevated relative to aligned follow-through.",
        )
    } else {
        manual_note(
            target,
            current_value,
            "keep",
            "Aligned rate is acceptable and adverse rate remains bounded.",
        )
    }
}

fn manual_note(
    target: &str,
    current_value: f64,
    suggested_action: &str,
    reason: &str,
) -> WhaleFlowCalibrationManualTuningNote {
    WhaleFlowCalibrationManualTuningNote {
        target: target.to_string(),
        current_value,
        suggested_action: suggested_action.to_string(),
        reason: reason.to_string(),
        auto_applied: false,
        config_modified: false,
        manual_review_required: true,
    }
}

fn quality_bucket_for(counts: OutcomeCounts) -> &'static str {
    if counts.linked_markout_samples() < 3 {
        "insufficient_data"
    } else if counts.adverse_rate() <= 0.20 && counts.aligned_rate() >= 0.55 {
        "good"
    } else if counts.adverse_rate() <= 0.30 && counts.aligned_rate() >= 0.35 {
        "mixed"
    } else {
        "weak"
    }
}

fn classification_note(_classification: &str, counts: OutcomeCounts) -> &'static str {
    if counts.linked_markout_samples() < 3 {
        return "Do not tune this classification until more samples are collected.";
    }
    if counts.adverse_rate() >= 0.30 {
        return "Manual review should check whether this classification is too noisy.";
    }
    if counts.aligned_rate() >= 0.55 {
        return "Current classification behavior looks acceptable in the available sample.";
    }
    "Classification quality looks mixed; keep bounded and continue collecting evidence."
}

fn venue_confluence_verdict(venue_count: usize, counts: OutcomeCounts) -> &'static str {
    if counts.linked_markout_samples() < 3 {
        return "insufficient_data";
    }
    if venue_count < 2 {
        return "too_noisy";
    }
    if counts.aligned_rate() >= 0.55 && counts.adverse_rate() <= 0.20 {
        "current_minimum_reasonable"
    } else {
        "mixed"
    }
}

fn baseline_source_note(baseline_source: &str, counts: OutcomeCounts) -> &'static str {
    if counts.linked_markout_samples() < 3 {
        return "Do not tune this baseline path until more samples are collected.";
    }
    match baseline_source {
        "one_hour_normalized" => "One-hour normalized baseline is the preferred evidence path.",
        "sixty_second_fallback" | "longer_window_fallback" => {
            "Fallback baseline should be treated with lower confidence."
        }
        _ => "Insufficient baseline history should not drive tuning decisions.",
    }
}

fn candidate_markout_outcome(
    candidate: &WhaleFlowCandidate,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
) -> &'static str {
    candidate_outcome(candidate, markout_map).outcome
}

fn candidate_outcome(
    candidate: &WhaleFlowCandidate,
    markout_map: &BTreeMap<&str, &ToxicMarkoutSignal>,
) -> CandidateOutcome {
    if let Some(signal) = linked_markout_signal(candidate, markout_map) {
        return CandidateOutcome {
            outcome: markout_outcome_key(signal.overall_outcome),
            linkage_mode: OutcomeLinkageMode::LinkedSignalId,
        };
    }

    if let Some(signal) = fallback_markout_signal(candidate, markout_map) {
        return CandidateOutcome {
            outcome: markout_outcome_key(signal.overall_outcome),
            linkage_mode: OutcomeLinkageMode::TimeSymbolDirectionFallback,
        };
    }

    CandidateOutcome {
        outcome: "not_enough_data",
        linkage_mode: OutcomeLinkageMode::NoOutcomeLinkage,
    }
}

fn markout_outcome_key(outcome: ToxicMarkoutOutcome) -> &'static str {
    match outcome {
        ToxicMarkoutOutcome::Aligned => "aligned",
        ToxicMarkoutOutcome::Adverse => "adverse",
        ToxicMarkoutOutcome::Neutral => "neutral",
        ToxicMarkoutOutcome::NotEnoughData => "not_enough_data",
    }
}

fn linked_markout_signal<'a>(
    candidate: &WhaleFlowCandidate,
    markout_map: &'a BTreeMap<&str, &'a ToxicMarkoutSignal>,
) -> Option<&'a ToxicMarkoutSignal> {
    linked_signal_ids(candidate)
        .into_iter()
        .find_map(|signal_id| markout_map.get(signal_id).copied())
}

fn fallback_markout_signal<'a>(
    candidate: &WhaleFlowCandidate,
    markout_map: &'a BTreeMap<&str, &'a ToxicMarkoutSignal>,
) -> Option<&'a ToxicMarkoutSignal> {
    if !linked_signal_ids(candidate).is_empty() {
        return None;
    }

    markout_map
        .values()
        .copied()
        .filter(|signal| symbol_matches(&candidate.symbol, &signal.symbol))
        .filter(|signal| timestamps_overlap(candidate, signal))
        .filter(|signal| direction_matches(candidate.direction, &signal.direction))
        .filter(|signal| candidate_type_matches_markout(candidate.candidate_type, signal))
        .min_by_key(|signal| candidate.ts_ms.abs_diff(signal.created_at_ms))
}

fn symbol_matches(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn timestamps_overlap(candidate: &WhaleFlowCandidate, signal: &ToxicMarkoutSignal) -> bool {
    let tolerance = candidate.window_ms.max(FALLBACK_TIME_TOLERANCE_MS);
    candidate.ts_ms.abs_diff(signal.created_at_ms) <= tolerance
}

fn direction_matches(direction: ToxicSide, markout_direction: &str) -> bool {
    let normalized = markout_direction.to_ascii_lowercase();
    match direction {
        ToxicSide::Buy => {
            normalized.contains("buy")
                || normalized.contains("long")
                || normalized.contains("bid")
                || normalized.contains("up")
        }
        ToxicSide::Sell => {
            normalized.contains("sell")
                || normalized.contains("short")
                || normalized.contains("ask")
                || normalized.contains("down")
        }
        ToxicSide::Neutral => normalized.contains("neutral") || normalized.contains("flat"),
    }
}

fn candidate_type_matches_markout(
    candidate_type: WhaleFlowCandidateType,
    signal: &ToxicMarkoutSignal,
) -> bool {
    let signal_kind = signal.signal_kind.to_ascii_lowercase();
    match candidate_type {
        WhaleFlowCandidateType::AggressiveBuy => {
            signal_kind.contains("buy") || direction_matches(ToxicSide::Buy, &signal.direction)
        }
        WhaleFlowCandidateType::AggressiveSell => {
            signal_kind.contains("sell") || direction_matches(ToxicSide::Sell, &signal.direction)
        }
        WhaleFlowCandidateType::Absorption => signal_kind.contains("absorption"),
        WhaleFlowCandidateType::LiquidationSweep => {
            signal_kind.contains("liq") || signal_kind.contains("sweep")
        }
        WhaleFlowCandidateType::Trap => signal_kind.contains("trap"),
    }
}

fn linked_signal_ids(candidate: &WhaleFlowCandidate) -> Vec<&str> {
    candidate
        .linked_fusion_signal_ids
        .iter()
        .chain(candidate.linked_structural_signal_ids.iter())
        .chain(candidate.linked_active_trade_signal_ids.iter())
        .chain(candidate.linked_liquidation_signal_ids.iter())
        .chain(candidate.linked_wall_interpretation_signal_ids.iter())
        .map(String::as_str)
        .collect()
}

fn candidate_baseline_source(candidate: &WhaleFlowCandidate) -> &'static str {
    match candidate.historical_baseline_window_ms {
        Some(3_600_000) => "one_hour_normalized",
        Some(60_000) => "sixty_second_fallback",
        Some(_) => "longer_window_fallback",
        None => "insufficient_history",
    }
}

fn classification_key(candidate_type: WhaleFlowCandidateType) -> &'static str {
    match candidate_type {
        WhaleFlowCandidateType::AggressiveBuy => "aggressive_buy",
        WhaleFlowCandidateType::AggressiveSell => "aggressive_sell",
        WhaleFlowCandidateType::Absorption => "absorption",
        WhaleFlowCandidateType::LiquidationSweep => "liquidation_sweep",
        WhaleFlowCandidateType::Trap => "trap",
    }
}

fn overall_adverse_rate(
    threshold_performance: &WhaleFlowCalibrationThresholdPerformanceSummary,
) -> f64 {
    let items = [
        &threshold_performance.one_second_btc,
        &threshold_performance.five_second_btc,
        &threshold_performance.fifteen_second_btc,
        &threshold_performance.sixty_second_btc,
    ];
    let total_candidates = items.iter().map(|item| item.candidate_count).sum::<usize>();
    if total_candidates == 0 {
        return 0.0;
    }
    items
        .iter()
        .map(|item| item.adverse_rate * item.candidate_count as f64)
        .sum::<f64>()
        / total_candidates as f64
}

fn resolved_threshold_samples(item: &WhaleFlowCalibrationThresholdPerformanceItem) -> usize {
    ((item.candidate_count as f64) * (1.0 - item.not_enough_data_rate)).round() as usize
}

#[allow(clippy::too_many_arguments)]
fn build_markdown_report(
    selected_symbol: &str,
    status: &str,
    sample_status: &WhaleFlowCalibrationSampleStatus,
    current_thresholds: &crate::types::whale_flow_signal::WhaleFlowThresholds,
    threshold_performance: &WhaleFlowCalibrationThresholdPerformanceSummary,
    by_classification: &[WhaleFlowCalibrationClassificationQualityItem],
    venue_confluence: &[WhaleFlowCalibrationVenueConfluenceItem],
    baseline_source_quality: &[WhaleFlowCalibrationBaselineSourceItem],
    manual_tuning_notes: &[WhaleFlowCalibrationManualTuningNote],
    warnings: &[String],
) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "# Whale Flow Threshold Calibration Report");
    let _ = writeln!(output);
    let _ = writeln!(output, "## Safety Boundary");
    let _ = writeln!(output, "- Read-only");
    let _ = writeln!(output, "- Analysis only");
    let _ = writeln!(output, "- Execution disabled");
    let _ = writeln!(output, "- No threshold modification");
    let _ = writeln!(output, "- No config write");
    let _ = writeln!(output, "- No apply/reload");
    let _ = writeln!(output, "- No order placement");
    let _ = writeln!(output, "- No wallet/signing");
    let _ = writeln!(output, "- No live trading");
    let _ = writeln!(output);
    let _ = writeln!(output, "## Sample Status");
    let _ = writeln!(output, "- Selected symbol: {selected_symbol}");
    let _ = writeln!(output, "- Status: {status}");
    let _ = writeln!(
        output,
        "- Total candidates: {}",
        sample_status.total_candidates
    );
    let _ = writeln!(
        output,
        "- Linked markout samples: {}",
        sample_status.linked_markout_samples
    );
    let _ = writeln!(
        output,
        "- Resolved markout evidence: {}",
        sample_status.resolved_markout_evidence_count
    );
    let _ = writeln!(
        output,
        "- Unresolved markout samples: {}",
        sample_status.unresolved_markout_count
    );
    let _ = writeln!(
        output,
        "- Min samples required: {}",
        sample_status.min_samples_required
    );
    let _ = writeln!(
        output,
        "- Min resolved evidence required: {}",
        sample_status.min_resolved_evidence_required
    );
    let _ = writeln!(
        output,
        "- Not enough data rate: {:.2}%",
        sample_status.not_enough_data_rate * 100.0
    );
    let _ = writeln!(output, "- Enough data: {}", sample_status.enough_data);
    if let Some(blocked_reason) = &sample_status.blocked_reason {
        let _ = writeln!(output, "- Blocked reason: {blocked_reason}");
    }
    let _ = writeln!(output, "- Retention mode: {}", sample_status.retention_mode);
    if !warnings.is_empty() {
        let _ = writeln!(output, "- Warnings:");
        for warning in warnings {
            let _ = writeln!(output, "  - {warning}");
        }
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "## Current Thresholds");
    let _ = writeln!(output, "- 1s BTC: {}", current_thresholds.one_second_btc);
    let _ = writeln!(output, "- 5s BTC: {}", current_thresholds.five_second_btc);
    let _ = writeln!(
        output,
        "- 15s BTC: {}",
        current_thresholds.fifteen_second_btc
    );
    let _ = writeln!(output, "- 60s BTC: {}", current_thresholds.sixty_second_btc);
    let _ = writeln!(
        output,
        "- Direction ratio: {:.2}",
        current_thresholds.direction_ratio_min
    );
    let _ = writeln!(
        output,
        "- Relative volume multiple: {:.2}",
        current_thresholds.relative_volume_multiple_min
    );
    let _ = writeln!(
        output,
        "- Venue confluence: {}",
        current_thresholds.min_venue_confirmations
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "## Threshold Performance");
    append_threshold_line(&mut output, "1s", &threshold_performance.one_second_btc);
    append_threshold_line(&mut output, "5s", &threshold_performance.five_second_btc);
    append_threshold_line(
        &mut output,
        "15s",
        &threshold_performance.fifteen_second_btc,
    );
    append_threshold_line(&mut output, "60s", &threshold_performance.sixty_second_btc);
    let _ = writeln!(output);
    let _ = writeln!(output, "## Classification Quality");
    for item in by_classification {
        let _ = writeln!(
            output,
            "- {}: samples {}, aligned {:.2}%, adverse {:.2}%, neutral {:.2}%, notEnoughData {:.2}%, bucket {}, note {}",
            item.classification,
            item.sample_count,
            item.aligned_rate * 100.0,
            item.adverse_rate * 100.0,
            item.neutral_rate * 100.0,
            item.not_enough_data_rate * 100.0,
            item.quality_bucket,
            item.manual_tuning_note
        );
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "## Venue Confluence Effect");
    for item in venue_confluence {
        let _ = writeln!(
            output,
            "- {} venues: samples {}, aligned {:.2}%, adverse {:.2}%, neutral {:.2}%, notEnoughData {:.2}%, verdict {}",
            item.venue_count,
            item.sample_count,
            item.aligned_rate * 100.0,
            item.adverse_rate * 100.0,
            item.neutral_rate * 100.0,
            item.not_enough_data_rate * 100.0,
            item.verdict
        );
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "## Baseline Source Effect");
    for item in baseline_source_quality {
        let _ = writeln!(
            output,
            "- {}: samples {}, aligned {:.2}%, adverse {:.2}%, neutral {:.2}%, notEnoughData {:.2}%, bucket {}, note {}",
            item.baseline_source,
            item.sample_count,
            item.aligned_rate * 100.0,
            item.adverse_rate * 100.0,
            item.neutral_rate * 100.0,
            item.not_enough_data_rate * 100.0,
            item.quality_bucket,
            item.manual_tuning_note
        );
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "## Manual Tuning Notes");
    for note in manual_tuning_notes {
        let _ = writeln!(
            output,
            "- {}: current {}, action {}, reason {}, autoApplied={}, configModified={}, manualReviewRequired={}",
            note.target,
            note.current_value,
            note.suggested_action,
            note.reason,
            note.auto_applied,
            note.config_modified,
            note.manual_review_required
        );
    }
    output
}

fn append_threshold_line(
    output: &mut String,
    label: &str,
    item: &WhaleFlowCalibrationThresholdPerformanceItem,
) {
    let _ = writeln!(
        output,
        "- {}: threshold {}, candidates {}, aligned {:.2}%, adverse {:.2}%, neutral {:.2}%, notEnoughData {:.2}%, verdict {}",
        label,
        item.threshold,
        item.candidate_count,
        item.aligned_rate * 100.0,
        item.adverse_rate * 100.0,
        item.neutral_rate * 100.0,
        item.not_enough_data_rate * 100.0,
        item.verdict
    );
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn ratio_sum(values: impl Iterator<Item = f64>, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        values.sum::<f64>() / denominator as f64
    }
}

fn dedup_strings(items: &mut Vec<String>) {
    items.sort();
    items.dedup();
}
