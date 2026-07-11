use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    scheduler::{AltCandidatePriority, FairSchedulerConfig, FairScoringScheduler},
    types::AltContractSymbolTier,
};

fn candidate(
    product_id: &str,
    tier: AltContractSymbolTier,
    created_at: i64,
) -> AltCandidatePriority {
    AltCandidatePriority {
        product_id: product_id.to_string(),
        tier,
        window_sec: 60,
        relative_notional: 1.0,
        dynamic_multiple: 4.0,
        dominance: 0.8,
        abs_price_move_pct: 0.5,
        liquidation_present: false,
        candidate_created_at: created_at,
        last_scored_at: None,
    }
}

#[test]
fn tier_de_candidates_keep_a_minimum_share_under_hot_tier_ab_load() {
    let mut scheduler = FairScoringScheduler::default();
    let config = FairSchedulerConfig {
        full_scores_per_second: 10,
        max_scores_per_symbol_per_second: 3,
        tier_a_b_share: 0.4,
        tier_c_share: 0.3,
        tier_d_e_share: 0.3,
        ..FairSchedulerConfig::default()
    };
    for index in 0..20 {
        scheduler.upsert(candidate(
            &format!("HOT{index}USDT"),
            AltContractSymbolTier::A,
            1_000,
        ));
    }
    for index in 0..5 {
        scheduler.upsert(candidate(
            &format!("TAIL{index}USDT"),
            AltContractSymbolTier::E,
            1_000,
        ));
    }

    let selected = scheduler.select(1_500, &config);
    let tail_selected = selected
        .iter()
        .filter(|candidate| candidate.tier == AltContractSymbolTier::E)
        .count();

    assert!(tail_selected >= 3, "selected={selected:?}");
}

#[test]
fn one_symbol_cannot_take_the_whole_scoring_budget() {
    let mut scheduler = FairScoringScheduler::default();
    let config = FairSchedulerConfig {
        full_scores_per_second: 8,
        max_scores_per_symbol_per_second: 1,
        ..FairSchedulerConfig::default()
    };
    scheduler.upsert(candidate("HOTUSDT", AltContractSymbolTier::A, 1_000));
    scheduler.upsert(candidate("TAILUSDT", AltContractSymbolTier::D, 1_000));
    scheduler.upsert(candidate("OTHERUSDT", AltContractSymbolTier::C, 1_000));

    let selected = scheduler.select(1_500, &config);
    assert_eq!(
        selected
            .iter()
            .filter(|candidate| candidate.product_id == "HOTUSDT")
            .count(),
        1
    );
    assert!(selected
        .iter()
        .any(|candidate| candidate.product_id == "TAILUSDT"));
}

#[test]
fn older_candidate_beats_an_equally_strong_new_candidate() {
    let mut scheduler = FairScoringScheduler::default();
    let config = FairSchedulerConfig {
        full_scores_per_second: 1,
        max_scores_per_symbol_per_second: 1,
        ageing_points_per_second: 2.0,
        ..FairSchedulerConfig::default()
    };
    scheduler.upsert(candidate("NEWUSDT", AltContractSymbolTier::C, 9_000));
    scheduler.upsert(candidate("OLDUSDT", AltContractSymbolTier::C, 1_000));

    let selected = scheduler.select(10_000, &config);
    assert_eq!(selected[0].product_id, "OLDUSDT");
}

#[test]
fn liquidation_candidate_receives_priority_bonus() {
    let mut scheduler = FairScoringScheduler::default();
    let config = FairSchedulerConfig {
        full_scores_per_second: 1,
        max_scores_per_symbol_per_second: 1,
        liquidation_priority_bonus: 20.0,
        ..FairSchedulerConfig::default()
    };
    scheduler.upsert(candidate("NORMALUSDT", AltContractSymbolTier::C, 1_000));
    let mut liquidation = candidate("LIQUSDT", AltContractSymbolTier::C, 1_000);
    liquidation.liquidation_present = true;
    scheduler.upsert(liquidation);

    let selected = scheduler.select(1_500, &config);
    assert_eq!(selected[0].product_id, "LIQUSDT");
}

#[test]
fn diagnostics_expose_scored_and_waiting_candidates() {
    let mut scheduler = FairScoringScheduler::default();
    let config = FairSchedulerConfig {
        full_scores_per_second: 1,
        max_scores_per_symbol_per_second: 1,
        ..FairSchedulerConfig::default()
    };
    scheduler.upsert(candidate("AUSDT", AltContractSymbolTier::A, 1_000));
    scheduler.upsert(candidate("EUSDT", AltContractSymbolTier::E, 1_000));

    let selected = scheduler.select(1_500, &config);
    let diagnostics = scheduler.diagnostics(1_500);

    assert_eq!(selected.len(), 1);
    assert_eq!(diagnostics.scored_by_tier.values().sum::<u64>(), 1);
    assert_eq!(diagnostics.skipped_by_tier.values().sum::<u64>(), 1);
    assert_eq!(diagnostics.per_symbol_score_count.values().sum::<u64>(), 1);
    assert_eq!(diagnostics.oldest_candidate_age_ms, Some(500));
}
