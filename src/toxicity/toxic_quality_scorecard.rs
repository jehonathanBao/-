use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::types::toxic_markout::{ToxicMarkoutOutcome, ToxicMarkoutRecentResponse};
use crate::types::toxic_quality_scorecard::{
    ToxicQualityScorecardBucket, ToxicQualityScorecardCandidate,
    ToxicQualityScorecardStatusResponse, ToxicQualityScorecardSummaryResponse,
    ToxicQualityScorecardSymbolSummary,
};

pub fn build_toxic_quality_scorecard_summary(
    recent: &ToxicMarkoutRecentResponse,
) -> ToxicQualityScorecardSummaryResponse {
    let mut overall = OutcomeCounter::default();
    let mut signal_type_buckets: BTreeMap<String, BucketAccumulator> = BTreeMap::new();
    let mut window_buckets: BTreeMap<String, BucketAccumulator> = BTreeMap::new();
    let mut symbol_buckets: BTreeMap<String, OutcomeCounter> = BTreeMap::new();

    for signal in &recent.signals {
        overall.record(signal.overall_outcome);
        symbol_buckets
            .entry(signal.symbol.clone())
            .or_default()
            .record(signal.overall_outcome);

        let signal_bucket = signal_type_buckets
            .entry(signal.signal_kind.clone())
            .or_insert_with(|| BucketAccumulator::new(signal.signal_kind.clone()));
        signal_bucket.record(
            signal.overall_outcome,
            &signal.symbol,
            &signal.no_trade_reasons,
        );

        for window in &signal.windows {
            let bucket = window_buckets
                .entry(window.label.clone())
                .or_insert_with(|| BucketAccumulator::new(window.label.clone()));
            bucket.record(window.outcome, &signal.symbol, &signal.no_trade_reasons);
        }
    }

    let by_signal_type = signal_type_buckets
        .into_values()
        .map(|bucket| bucket.into_bucket())
        .collect::<Vec<_>>();
    let by_window = window_buckets
        .into_values()
        .map(|bucket| bucket.into_bucket())
        .collect::<Vec<_>>();
    let by_symbol = symbol_buckets
        .into_iter()
        .map(|(symbol, counts)| ToxicQualityScorecardSymbolSummary {
            symbol,
            total_evaluations: counts.total(),
            aligned_ratio: counts.aligned_ratio(),
            adverse_ratio: counts.adverse_ratio(),
            neutral_ratio: counts.neutral_ratio(),
            not_enough_data_ratio: counts.not_enough_data_ratio(),
        })
        .collect::<Vec<_>>();

    let downgrade_candidates = by_signal_type
        .iter()
        .filter(|bucket| bucket.downgrade_candidate)
        .map(|bucket| ToxicQualityScorecardCandidate {
            key: bucket.key.clone(),
            label: bucket.label.clone(),
            reason: "adverse markout pressure is stronger than aligned follow-through".to_string(),
            total_evaluations: bucket.total_evaluations,
            aligned_ratio: bucket.aligned_ratio,
            adverse_ratio: bucket.adverse_ratio,
            neutral_ratio: bucket.neutral_ratio,
            not_enough_data_ratio: bucket.not_enough_data_ratio,
            top_no_trade_reasons: bucket.top_no_trade_reasons.clone(),
        })
        .collect::<Vec<_>>();
    let no_trade_candidates = by_signal_type
        .iter()
        .filter(|bucket| bucket.no_trade_candidate)
        .map(|bucket| ToxicQualityScorecardCandidate {
            key: bucket.key.clone(),
            label: bucket.label.clone(),
            reason: "neutral or not-enough-data outcomes dominate this signal type".to_string(),
            total_evaluations: bucket.total_evaluations,
            aligned_ratio: bucket.aligned_ratio,
            adverse_ratio: bucket.adverse_ratio,
            neutral_ratio: bucket.neutral_ratio,
            not_enough_data_ratio: bucket.not_enough_data_ratio,
            top_no_trade_reasons: bucket.top_no_trade_reasons.clone(),
        })
        .collect::<Vec<_>>();

    let mut warnings = recent.warnings.clone();
    if recent.signals.is_empty() {
        warnings.push(
            "No toxic markout signals are currently available, so the scorecard is observational only."
                .to_string(),
        );
    }
    if !downgrade_candidates.is_empty() {
        warnings.push(
            "Downgrade candidates were surfaced from adverse-heavy markout follow-through."
                .to_string(),
        );
    }
    if !no_trade_candidates.is_empty() {
        warnings.push(
            "No-trade candidates were surfaced where neutral or insufficient-data behavior dominates."
                .to_string(),
        );
    }

    ToxicQualityScorecardSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: recent.selected_symbol.clone(),
        status: if recent.signals.is_empty() {
            "no_quality_data".to_string()
        } else {
            "quality_scorecard_ready".to_string()
        },
        warnings,
        total_evaluations: overall.total(),
        aligned_ratio: overall.aligned_ratio(),
        adverse_ratio: overall.adverse_ratio(),
        neutral_ratio: overall.neutral_ratio(),
        not_enough_data_ratio: overall.not_enough_data_ratio(),
        by_signal_type,
        by_window,
        by_symbol,
        downgrade_candidates,
        no_trade_candidates,
    }
}

pub fn build_toxic_quality_scorecard_status(
    summary: &ToxicQualityScorecardSummaryResponse,
) -> ToxicQualityScorecardStatusResponse {
    ToxicQualityScorecardStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_evaluations: summary.total_evaluations,
        signal_type_count: summary.by_signal_type.len(),
        window_count: summary.by_window.len(),
        downgrade_candidate_count: summary.downgrade_candidates.len(),
        no_trade_candidate_count: summary.no_trade_candidates.len(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No cancel/amend".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
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

    fn total(&self) -> usize {
        self.aligned + self.adverse + self.neutral + self.not_enough_data
    }

    fn aligned_ratio(&self) -> f64 {
        ratio(self.aligned, self.total())
    }

    fn adverse_ratio(&self) -> f64 {
        ratio(self.adverse, self.total())
    }

    fn neutral_ratio(&self) -> f64 {
        ratio(self.neutral, self.total())
    }

    fn not_enough_data_ratio(&self) -> f64 {
        ratio(self.not_enough_data, self.total())
    }
}

struct BucketAccumulator {
    key: String,
    counts: OutcomeCounter,
    symbols: BTreeSet<String>,
    no_trade_reasons: HashMap<String, usize>,
}

impl BucketAccumulator {
    fn new(key: String) -> Self {
        Self {
            key,
            counts: OutcomeCounter::default(),
            symbols: BTreeSet::new(),
            no_trade_reasons: HashMap::new(),
        }
    }

    fn record(&mut self, outcome: ToxicMarkoutOutcome, symbol: &str, reasons: &[String]) {
        self.counts.record(outcome);
        self.symbols.insert(symbol.to_string());
        for reason in reasons {
            *self.no_trade_reasons.entry(reason.clone()).or_insert(0) += 1;
        }
    }

    fn into_bucket(self) -> ToxicQualityScorecardBucket {
        let total = self.counts.total();
        let aligned_ratio = self.counts.aligned_ratio();
        let adverse_ratio = self.counts.adverse_ratio();
        let neutral_ratio = self.counts.neutral_ratio();
        let not_enough_data_ratio = self.counts.not_enough_data_ratio();
        ToxicQualityScorecardBucket {
            key: self.key.clone(),
            label: self.key,
            total_evaluations: total,
            aligned_count: self.counts.aligned,
            adverse_count: self.counts.adverse,
            neutral_count: self.counts.neutral,
            not_enough_data_count: self.counts.not_enough_data,
            aligned_ratio,
            adverse_ratio,
            neutral_ratio,
            not_enough_data_ratio,
            downgrade_candidate: total >= 2
                && adverse_ratio >= 0.4
                && adverse_ratio > aligned_ratio,
            no_trade_candidate: total >= 1
                && (neutral_ratio >= 0.6
                    || not_enough_data_ratio >= 0.5
                    || (aligned_ratio == 0.0 && adverse_ratio == 0.0)),
            top_no_trade_reasons: top_reasons(&self.no_trade_reasons),
            symbols: self.symbols.into_iter().collect(),
        }
    }
}

fn top_reasons(reasons: &HashMap<String, usize>) -> Vec<String> {
    let mut items = reasons.iter().collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    items
        .into_iter()
        .take(3)
        .map(|(reason, _)| reason.clone())
        .collect()
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    ((numerator as f64 / denominator as f64) * 10_000.0).round() / 10_000.0
}
