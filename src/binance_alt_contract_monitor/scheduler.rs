use std::collections::{BTreeMap, BTreeSet};

use super::types::AltContractSymbolTier;

#[derive(Debug, Clone)]
pub struct AltCandidatePriority {
    pub product_id: String,
    pub tier: AltContractSymbolTier,
    pub window_sec: u64,
    pub relative_notional: f64,
    pub dynamic_multiple: f64,
    pub dominance: f64,
    pub abs_price_move_pct: f64,
    pub liquidation_present: bool,
    pub candidate_created_at: i64,
    pub last_scored_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FairSchedulerConfig {
    pub enabled: bool,
    pub full_scores_per_second: u64,
    pub max_scores_per_symbol_per_second: u64,
    pub candidate_max_age_seconds: u64,
    pub tier_a_b_share: f64,
    pub tier_c_share: f64,
    pub tier_d_e_share: f64,
    pub liquidation_priority_bonus: f64,
    pub ageing_points_per_second: f64,
}

impl Default for FairSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            full_scores_per_second: 60,
            max_scores_per_symbol_per_second: 3,
            candidate_max_age_seconds: 10,
            tier_a_b_share: 0.40,
            tier_c_share: 0.30,
            tier_d_e_share: 0.30,
            liquidation_priority_bonus: 20.0,
            ageing_points_per_second: 2.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FairScoringScheduler {
    pending: BTreeMap<String, AltCandidatePriority>,
    window_start_ms: i64,
    scored_by_symbol: BTreeMap<String, u64>,
    scored_by_tier: BTreeMap<String, u64>,
    skipped_by_tier: BTreeMap<String, u64>,
}

impl FairScoringScheduler {
    pub fn upsert(&mut self, mut candidate: AltCandidatePriority) {
        if let Some(existing) = self.pending.get(&candidate.product_id) {
            candidate.candidate_created_at = candidate
                .candidate_created_at
                .min(existing.candidate_created_at);
            candidate.last_scored_at = existing.last_scored_at;
        }
        self.pending.insert(candidate.product_id.clone(), candidate);
    }

    pub fn select(
        &mut self,
        now_ms: i64,
        config: &FairSchedulerConfig,
    ) -> Vec<AltCandidatePriority> {
        self.reset_window_if_due(now_ms);
        self.pending.retain(|_, candidate| {
            now_ms.saturating_sub(candidate.candidate_created_at)
                <= i64::try_from(config.candidate_max_age_seconds)
                    .unwrap_or(i64::MAX)
                    .saturating_mul(1_000)
        });
        let capacity = usize::try_from(config.full_scores_per_second)
            .unwrap_or(usize::MAX)
            .saturating_sub(self.scored_by_symbol.values().sum::<u64>() as usize);
        if capacity == 0 || self.pending.is_empty() {
            self.refresh_skipped_by_tier();
            return Vec::new();
        }

        let quotas = tier_quotas(capacity, config);
        let mut selected = Vec::new();
        let mut selected_products = BTreeSet::new();
        for (group, quota) in quotas {
            self.select_from_group(
                group,
                quota,
                now_ms,
                config,
                &mut selected_products,
                &mut selected,
            );
        }
        let remaining = capacity.saturating_sub(selected.len());
        self.select_from_group(
            TierGroup::Any,
            remaining,
            now_ms,
            config,
            &mut selected_products,
            &mut selected,
        );
        for candidate in &selected {
            self.pending.remove(&candidate.product_id);
            *self
                .scored_by_symbol
                .entry(candidate.product_id.clone())
                .or_default() += 1;
            *self
                .scored_by_tier
                .entry(tier_name(candidate.tier).to_string())
                .or_default() += 1;
        }
        self.refresh_skipped_by_tier();
        selected
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn diagnostics(&self, now_ms: i64) -> FairSchedulerDiagnostics {
        let oldest_candidate_age_ms = self
            .pending
            .values()
            .map(|candidate| now_ms.saturating_sub(candidate.candidate_created_at).max(0))
            .max();
        FairSchedulerDiagnostics {
            scored_by_tier: self.scored_by_tier.clone(),
            skipped_by_tier: self.skipped_by_tier.clone(),
            oldest_candidate_age_ms,
            per_symbol_score_count: self.scored_by_symbol.clone(),
            starved_candidate_count: self
                .pending
                .values()
                .filter(|candidate| now_ms.saturating_sub(candidate.candidate_created_at) >= 5_000)
                .count(),
        }
    }

    fn reset_window_if_due(&mut self, now_ms: i64) {
        if now_ms.saturating_sub(self.window_start_ms) >= 1_000 {
            self.window_start_ms = now_ms;
            self.scored_by_symbol.clear();
            self.scored_by_tier.clear();
            self.skipped_by_tier.clear();
        }
    }

    fn refresh_skipped_by_tier(&mut self) {
        self.skipped_by_tier.clear();
        for candidate in self.pending.values() {
            *self
                .skipped_by_tier
                .entry(tier_name(candidate.tier).to_string())
                .or_default() += 1;
        }
    }

    fn select_from_group(
        &self,
        group: TierGroup,
        limit: usize,
        now_ms: i64,
        config: &FairSchedulerConfig,
        selected_products: &mut BTreeSet<String>,
        selected: &mut Vec<AltCandidatePriority>,
    ) {
        if limit == 0 {
            return;
        }
        let mut candidates = self
            .pending
            .values()
            .filter(|candidate| group.contains(candidate.tier))
            .filter(|candidate| !selected_products.contains(&candidate.product_id))
            .filter(|candidate| {
                self.scored_by_symbol
                    .get(&candidate.product_id)
                    .copied()
                    .unwrap_or_default()
                    < config.max_scores_per_symbol_per_second.max(1)
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            candidate_score(right, now_ms, config)
                .total_cmp(&candidate_score(left, now_ms, config))
                .then_with(|| left.candidate_created_at.cmp(&right.candidate_created_at))
                .then_with(|| left.product_id.cmp(&right.product_id))
        });
        for candidate in candidates.into_iter().take(limit) {
            selected_products.insert(candidate.product_id.clone());
            selected.push(candidate);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FairSchedulerDiagnostics {
    pub scored_by_tier: BTreeMap<String, u64>,
    pub skipped_by_tier: BTreeMap<String, u64>,
    pub oldest_candidate_age_ms: Option<i64>,
    pub per_symbol_score_count: BTreeMap<String, u64>,
    pub starved_candidate_count: usize,
}

#[derive(Debug, Clone, Copy)]
enum TierGroup {
    AB,
    C,
    DE,
    Any,
}

impl TierGroup {
    fn contains(self, tier: AltContractSymbolTier) -> bool {
        match self {
            Self::AB => matches!(tier, AltContractSymbolTier::A | AltContractSymbolTier::B),
            Self::C => tier == AltContractSymbolTier::C,
            Self::DE => matches!(tier, AltContractSymbolTier::D | AltContractSymbolTier::E),
            Self::Any => true,
        }
    }
}

fn tier_quotas(capacity: usize, config: &FairSchedulerConfig) -> [(TierGroup, usize); 3] {
    let shares = [
        (TierGroup::AB, config.tier_a_b_share),
        (TierGroup::C, config.tier_c_share),
        (TierGroup::DE, config.tier_d_e_share),
    ];
    let mut quotas = shares.map(|(group, share)| {
        (
            group,
            ((capacity as f64 * share.clamp(0.0, 1.0)).floor() as usize).min(capacity),
        )
    });
    let assigned = quotas.iter().map(|(_, quota)| *quota).sum::<usize>();
    let mut remaining = capacity.saturating_sub(assigned);
    for (_, quota) in &mut quotas {
        if remaining == 0 {
            break;
        }
        *quota = quota.saturating_add(1);
        remaining -= 1;
    }
    quotas
}

fn candidate_score(
    candidate: &AltCandidatePriority,
    now_ms: i64,
    config: &FairSchedulerConfig,
) -> f64 {
    let age_seconds = now_ms.saturating_sub(candidate.candidate_created_at).max(0) as f64 / 1_000.0;
    candidate.relative_notional.max(0.0) * 20.0
        + candidate.dynamic_multiple.max(0.0) * 5.0
        + candidate.dominance.clamp(0.0, 1.0) * 10.0
        + candidate.abs_price_move_pct.max(0.0) * 2.0
        + if candidate.liquidation_present {
            config.liquidation_priority_bonus.max(0.0)
        } else {
            0.0
        }
        + age_seconds * config.ageing_points_per_second.max(0.0)
}

fn tier_name(tier: AltContractSymbolTier) -> &'static str {
    match tier {
        AltContractSymbolTier::A => "a",
        AltContractSymbolTier::B => "b",
        AltContractSymbolTier::C => "c",
        AltContractSymbolTier::D => "d",
        AltContractSymbolTier::E => "e",
    }
}
