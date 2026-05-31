use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::Context;

use super::{
    calibration_report_store::{CalibrationReportEntry, CalibrationReportStore},
    calibration_types::{CalibrationRecommendation, CalibrationReport},
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    #[serde(alias = "Pending")]
    Pending,
    #[serde(alias = "ApprovedForManualApply")]
    ApprovedForManualApply,
    #[serde(alias = "Rejected")]
    Rejected,
    #[serde(alias = "Watch")]
    Watch,
    #[serde(alias = "NeedsMoreData")]
    NeedsMoreData,
    #[serde(alias = "Archived")]
    Archived,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationSourceMetrics {
    pub hit_rate: Option<f64>,
    pub false_positive_rate: Option<f64>,
    pub event_count: Option<usize>,
    pub baseline_value: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationCard {
    pub recommendation_id: String,
    pub report_id: String,
    pub generated_at: Option<i64>,
    pub parameter_key: String,
    pub current_value: Option<f64>,
    pub recommended_value: Option<f64>,
    pub current_config_summary: Option<String>,
    pub recommended_config_summary: Option<String>,
    pub direction: String,
    pub confidence: String,
    pub reason: String,
    pub expected_effect: Option<String>,
    pub risk_note: Option<String>,
    pub source_metrics: RecommendationSourceMetrics,
    pub current_review: Option<ReviewDecision>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDecision {
    pub recommendation_id: String,
    pub report_id: String,
    pub status: ReviewStatus,
    pub reviewer_note: Option<String>,
    pub reviewer: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReviewDecisionInput {
    pub recommendation_id: String,
    pub report_id: String,
    pub status: ReviewStatus,
    pub reviewer_note: Option<String>,
    pub reviewer: Option<String>,
}

pub struct ParameterRecommendationReviewStore {
    report_store: CalibrationReportStore,
    ledger_path: PathBuf,
}

impl ParameterRecommendationReviewStore {
    pub fn new(report_dir: impl Into<PathBuf>) -> Self {
        let report_dir = report_dir.into();
        let runtime_dir = report_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".runtime"));
        let ledger_path = runtime_dir
            .join("reviews")
            .join("parameter-recommendation-reviews.jsonl");
        Self {
            report_store: CalibrationReportStore::new(report_dir),
            ledger_path,
        }
    }

    pub fn list_recommendations(&self) -> anyhow::Result<Vec<RecommendationCard>> {
        let reviews = self.latest_reviews_map()?;
        let mut cards = Vec::new();
        for summary in self.report_store.list_reports()? {
            if let Some(entry) = self.report_store.get_report(&summary.id)? {
                cards.extend(cards_from_report(&entry, &reviews));
            }
        }
        Ok(cards)
    }

    pub fn latest_recommendations(&self) -> anyhow::Result<Vec<RecommendationCard>> {
        let reviews = self.latest_reviews_map()?;
        Ok(self
            .report_store
            .latest_report()?
            .map(|entry| cards_from_report(&entry, &reviews))
            .unwrap_or_default())
    }

    pub fn get_recommendation(
        &self,
        recommendation_id: &str,
    ) -> anyhow::Result<Option<RecommendationCard>> {
        let reviews = self.latest_reviews_map()?;
        for summary in self.report_store.list_reports()? {
            if let Some(entry) = self.report_store.get_report(&summary.id)? {
                if let Some(card) = cards_from_report(&entry, &reviews)
                    .into_iter()
                    .find(|card| card.recommendation_id == recommendation_id)
                {
                    return Ok(Some(card));
                }
            }
        }
        Ok(None)
    }

    pub fn list_reviews(&self) -> anyhow::Result<Vec<ReviewDecision>> {
        let mut reviews = self.load_reviews()?;
        reviews.sort_by_key(|review| std::cmp::Reverse(review.updated_at));
        Ok(reviews)
    }

    pub fn append_review(
        &self,
        input: ReviewDecisionInput,
        now_ms: i64,
    ) -> anyhow::Result<ReviewDecision> {
        let created_at = self
            .latest_reviews_map()?
            .get(&input.recommendation_id)
            .map(|review| review.created_at)
            .unwrap_or(now_ms);
        let review = ReviewDecision {
            recommendation_id: input.recommendation_id,
            report_id: input.report_id,
            status: input.status,
            reviewer_note: input.reviewer_note,
            reviewer: input.reviewer,
            created_at,
            updated_at: now_ms,
        };
        if let Some(parent) = self.ledger_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .with_context(|| format!("open {}", self.ledger_path.display()))?;
        writeln!(file, "{}", serde_json::to_string(&review)?)
            .with_context(|| format!("write {}", self.ledger_path.display()))?;
        Ok(review)
    }

    pub fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    fn latest_reviews_map(
        &self,
    ) -> anyhow::Result<std::collections::BTreeMap<String, ReviewDecision>> {
        let mut map: std::collections::BTreeMap<String, ReviewDecision> =
            std::collections::BTreeMap::new();
        for review in self.load_reviews()? {
            match map.get(&review.recommendation_id) {
                Some(existing) if existing.updated_at >= review.updated_at => {}
                _ => {
                    map.insert(review.recommendation_id.clone(), review);
                }
            }
        }
        Ok(map)
    }

    fn load_reviews(&self) -> anyhow::Result<Vec<ReviewDecision>> {
        if !self.ledger_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.ledger_path)
            .with_context(|| format!("read {}", self.ledger_path.display()))?;
        let mut reviews = Vec::new();
        for (index, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let review = serde_json::from_str::<ReviewDecision>(line).with_context(|| {
                format!("parse {} line {}", self.ledger_path.display(), index + 1)
            })?;
            reviews.push(review);
        }
        Ok(reviews)
    }
}

fn cards_from_report(
    entry: &CalibrationReportEntry,
    reviews: &std::collections::BTreeMap<String, ReviewDecision>,
) -> Vec<RecommendationCard> {
    let report = &entry.report;
    let report_id = entry.summary.id.clone();
    report
        .recommendations
        .iter()
        .flat_map(|recommendation| {
            normalize_recommendation(
                &report_id,
                entry.summary.created_at_ms,
                report,
                recommendation,
            )
        })
        .map(|mut card| {
            card.current_review = reviews.get(&card.recommendation_id).cloned();
            card
        })
        .collect()
}

fn normalize_recommendation(
    report_id: &str,
    generated_at: Option<i64>,
    report: &CalibrationReport,
    recommendation: &CalibrationRecommendation,
) -> Vec<RecommendationCard> {
    match recommendation.title.as_str() {
        "Threshold Comparison" => {
            let recommended = parse_after_marker(&recommendation.detail, "came from ", " BTC");
            vec![build_scalar_card(ScalarCardSpec {
                report_id,
                generated_at,
                parameter_key: "toxicity.threshold_btc",
                current_value: report.baseline.toxic_threshold_btc,
                recommended_value: recommended.unwrap_or(report.baseline.toxic_threshold_btc),
                reason: recommendation.detail.clone(),
                hit_rate: report.baseline.hit_rate,
                false_positive_rate: report.baseline.false_positive_rate,
                event_count: report.baseline.event_count,
                expected_effect: "reduce false positives by tightening the trigger volume",
                risk_note: "may miss weaker early toxic-flow setups if pushed too high",
            })]
        }
        "Toxic Ratio Comparison" => {
            let recommended =
                parse_after_marker(&recommendation.detail, "Min toxic ratio ", " produced");
            vec![build_scalar_card(ScalarCardSpec {
                report_id,
                generated_at,
                parameter_key: "toxicity.min_toxic_ratio",
                current_value: report.baseline.min_toxic_ratio,
                recommended_value: recommended.unwrap_or(report.baseline.min_toxic_ratio),
                reason: recommendation.detail.clone(),
                hit_rate: report.baseline.hit_rate,
                false_positive_rate: report.baseline.false_positive_rate,
                event_count: report.baseline.event_count,
                expected_effect: "improve precision by filtering weaker composite toxic signals",
                risk_note:
                    "higher ratio can reduce recall on valid but less concentrated flow bursts",
            })]
        }
        "VPIN Parameter Comparison" => {
            let bucket_size = parse_after_marker(&recommendation.detail, "bucket_size ", " BTC");
            let lookback = parse_after_marker(&recommendation.detail, "lookback ", ",");
            let zscore = parse_after_marker(&recommendation.detail, "z-score ", " (");
            vec![
                build_scalar_card(ScalarCardSpec {
                    report_id,
                    generated_at,
                    parameter_key: "vpin.bucket_size_btc",
                    current_value: report.baseline.vpin_bucket_size_btc,
                    recommended_value: bucket_size
                        .unwrap_or(report.baseline.vpin_bucket_size_btc),
                    reason: recommendation.detail.clone(),
                    hit_rate: report.baseline.hit_rate,
                    false_positive_rate: report.baseline.false_positive_rate,
                    event_count: report.baseline.event_count,
                    expected_effect:
                        "rebalance VPIN sensitivity between noisy small buckets and slower large buckets",
                    risk_note: "larger buckets can smooth noise but delay signal recognition",
                }),
                build_scalar_card(ScalarCardSpec {
                    report_id,
                    generated_at,
                    parameter_key: "vpin.lookback_buckets",
                    current_value: report.baseline.vpin_lookback_buckets as f64,
                    recommended_value: lookback
                        .unwrap_or(report.baseline.vpin_lookback_buckets as f64),
                    reason: recommendation.detail.clone(),
                    hit_rate: report.baseline.hit_rate,
                    false_positive_rate: report.baseline.false_positive_rate,
                    event_count: report.baseline.event_count,
                    expected_effect: "tune how quickly VPIN reacts to fresh imbalance regimes",
                    risk_note: "longer lookbacks can underreact during abrupt regime shifts",
                }),
                build_scalar_card(ScalarCardSpec {
                    report_id,
                    generated_at,
                    parameter_key: "vpin.spike_zscore",
                    current_value: report.baseline.vpin_spike_zscore,
                    recommended_value: zscore.unwrap_or(report.baseline.vpin_spike_zscore),
                    reason: recommendation.detail.clone(),
                    hit_rate: report.baseline.hit_rate,
                    false_positive_rate: report.baseline.false_positive_rate,
                    event_count: report.baseline.event_count,
                    expected_effect: "adjust how strict VPIN spike confirmation should be",
                    risk_note:
                        "raising the z-score can hide meaningful spikes in thinner conditions",
                }),
            ]
        }
        "Liq Hunt Score Comparison" => {
            let likely = parse_after_marker(&recommendation.detail, "Likely ", " /");
            let active = parse_after_marker(&recommendation.detail, "Active ", " gave");
            vec![
                build_scalar_card(ScalarCardSpec {
                    report_id,
                    generated_at,
                    parameter_key: "liq_hunt.likely_score",
                    current_value: report.baseline.liq_hunt_likely_score,
                    recommended_value: likely.unwrap_or(report.baseline.liq_hunt_likely_score),
                    reason: recommendation.detail.clone(),
                    hit_rate: report.baseline.hit_rate,
                    false_positive_rate: report.baseline.false_positive_rate,
                    event_count: report.baseline.event_count,
                    expected_effect: "set a cleaner boundary for likely liq-hunt setups",
                    risk_note:
                        "lowering the likely score may raise early warnings that never mature",
                }),
                build_scalar_card(ScalarCardSpec {
                    report_id,
                    generated_at,
                    parameter_key: "liq_hunt.active_score",
                    current_value: report.baseline.liq_hunt_active_score,
                    recommended_value: active.unwrap_or(report.baseline.liq_hunt_active_score),
                    reason: recommendation.detail.clone(),
                    hit_rate: report.baseline.hit_rate,
                    false_positive_rate: report.baseline.false_positive_rate,
                    event_count: report.baseline.event_count,
                    expected_effect:
                        "tighten the active liq-hunt threshold to reduce noisy escalations",
                    risk_note: "higher active thresholds can delay action on fast squeezes",
                }),
            ]
        }
        _ => vec![RecommendationCard {
            recommendation_id: format!("{report_id}::{}", slugify(&recommendation.title)),
            report_id: report_id.to_string(),
            generated_at,
            parameter_key: recommendation.title.clone(),
            current_value: None,
            recommended_value: None,
            current_config_summary: None,
            recommended_config_summary: None,
            direction: "review".to_string(),
            confidence: "low".to_string(),
            reason: recommendation.detail.clone(),
            expected_effect: None,
            risk_note: Some("manual interpretation required".to_string()),
            source_metrics: RecommendationSourceMetrics {
                hit_rate: Some(report.baseline.hit_rate),
                false_positive_rate: Some(report.baseline.false_positive_rate),
                event_count: Some(report.baseline.event_count),
                baseline_value: None,
            },
            current_review: None,
        }],
    }
}

struct ScalarCardSpec<'a> {
    report_id: &'a str,
    generated_at: Option<i64>,
    parameter_key: &'a str,
    current_value: f64,
    recommended_value: f64,
    reason: String,
    hit_rate: f64,
    false_positive_rate: f64,
    event_count: usize,
    expected_effect: &'a str,
    risk_note: &'a str,
}

fn build_scalar_card(spec: ScalarCardSpec<'_>) -> RecommendationCard {
    RecommendationCard {
        recommendation_id: format!("{}::{}", spec.report_id, spec.parameter_key),
        report_id: spec.report_id.to_string(),
        generated_at: spec.generated_at,
        parameter_key: spec.parameter_key.to_string(),
        current_value: Some(spec.current_value),
        recommended_value: Some(spec.recommended_value),
        current_config_summary: Some(format_number(spec.current_value)),
        recommended_config_summary: Some(format_number(spec.recommended_value)),
        direction: value_direction(spec.current_value, spec.recommended_value).to_string(),
        confidence: confidence_label(spec.hit_rate, spec.false_positive_rate, spec.event_count)
            .to_string(),
        reason: spec.reason,
        expected_effect: Some(spec.expected_effect.to_string()),
        risk_note: Some(spec.risk_note.to_string()),
        source_metrics: RecommendationSourceMetrics {
            hit_rate: Some(spec.hit_rate),
            false_positive_rate: Some(spec.false_positive_rate),
            event_count: Some(spec.event_count),
            baseline_value: Some(spec.current_value),
        },
        current_review: None,
    }
}

fn parse_after_marker(detail: &str, start: &str, end: &str) -> Option<f64> {
    let tail = detail.split_once(start)?.1;
    let value = tail.split_once(end).map(|(value, _)| value).unwrap_or(tail);
    value.trim().parse::<f64>().ok()
}

fn value_direction(current: f64, recommended: f64) -> &'static str {
    if (recommended - current).abs() < f64::EPSILON {
        "keep"
    } else if recommended > current {
        "raise"
    } else {
        "lower"
    }
}

fn confidence_label(hit_rate: f64, false_positive_rate: f64, event_count: usize) -> &'static str {
    if event_count >= 20 && hit_rate >= 0.6 && false_positive_rate <= 0.1 {
        "high"
    } else if event_count >= 10 && hit_rate >= 0.4 {
        "medium"
    } else {
        "low"
    }
}

fn format_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn review_ledger_timestamp(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
}
