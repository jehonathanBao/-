use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueReliability {
    pub venue: String,
    pub score: f64,
    pub latency_ms: Option<i64>,
    pub snapshot_gap_count: u32,
    pub sequence_gap_count: u32,
    pub last_update_age_ms: Option<i64>,
}

pub fn reliability_score(venue: &VenueReliability) -> f64 {
    let mut score = venue.score.clamp(0.0, 100.0);
    if venue.latency_ms.is_some_and(|latency| latency > 1_000) {
        score -= 15.0;
    }
    score -= (venue.snapshot_gap_count.min(10) as f64) * 4.0;
    score -= (venue.sequence_gap_count.min(10) as f64) * 4.0;
    if venue
        .last_update_age_ms
        .is_some_and(|age_ms| age_ms > 30_000)
    {
        score -= 20.0;
    }
    score.clamp(0.0, 100.0)
}
