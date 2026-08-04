use std::sync::Arc;

use btc_toxic_flow_monitor_rs::contract_whale_monitor::impact_grade::{
    assess_contract_impact_episode, ContractEventImpactAssessment, ContractImpactEpisode,
};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::ContractWhaleRuntimeConfig;
use btc_toxic_flow_monitor_rs::storage::{ContractEventGradeRepo, SqliteStore};

fn assessment() -> ContractEventImpactAssessment {
    let mut episode = ContractImpactEpisode {
        episode_id: "episode-persist".to_string(),
        symbol: "BTC".to_string(),
        start_time_ms: 1_700_000_000_000,
        end_time_ms: 1_700_000_060_000,
        source_event_ids: vec!["event-persist".to_string()],
        total_volume_btc: 3_000.0,
        total_notional_usd: 190_000_000.0,
        net_volume_btc: 1_000.0,
        unique_turnover_btc: None,
        unique_turnover_notional_usd: None,
        live_liquidation_btc: None,
        live_liquidation_notional_usd: None,
        peak_abs_price_move_pct: Some(0.6),
        peak_abs_oi_change_pct: Some(0.4),
        confirmed_sources: vec!["binance".to_string(), "bitfinex".to_string()],
        data_quality: 90,
        robust_percentile: Some(99.5),
        robust_z: Some(4.2),
        baseline_sample_count: 20_000,
    };
    episode.episode_id = "episode-persist".to_string();
    assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    )
}

#[test]
fn assessment_upsert_is_restart_safe_and_send_marker_is_idempotent() {
    let path = std::env::temp_dir().join(format!("impact-grade-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
    store.migrate().unwrap();
    let repo = ContractEventGradeRepo::new(store.clone());
    let value = assessment();
    repo.upsert_assessment(&value, 1_700_000_100_000).unwrap();
    repo.upsert_assessment(&value, 1_700_000_100_001).unwrap();
    let loaded = repo
        .get_assessment(&value.event_id, &value.grade_version)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.grade, value.grade);
    assert!(repo
        .mark_alert_sent(&value.event_id, &value.grade_version, 1_700_000_100_002)
        .unwrap());
    assert!(!repo
        .mark_alert_sent(&value.event_id, &value.grade_version, 1_700_000_100_003)
        .unwrap());

    let reopened = SqliteStore::open(path.to_str().unwrap()).unwrap();
    reopened.migrate().unwrap();
    let reopened_repo = ContractEventGradeRepo::new(reopened);
    let loaded_after_restart = reopened_repo
        .get_assessment(&value.event_id, &value.grade_version)
        .unwrap()
        .unwrap();
    assert_eq!(loaded_after_restart.episode_id, value.episode_id);
    let _ = std::fs::remove_file(path);
}

#[test]
fn concurrent_upserts_keep_one_materialized_row() {
    let path = std::env::temp_dir().join(format!(
        "impact-grade-concurrent-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
    store.migrate().unwrap();
    let repo = Arc::new(ContractEventGradeRepo::new(store.clone()));
    let value = Arc::new(assessment());
    let handles: Vec<_> = (0..20)
        .map(|index| {
            let repo = Arc::clone(&repo);
            let value = Arc::clone(&value);
            std::thread::spawn(move || repo.upsert_assessment(&value, 1_700_000_100_000 + index))
        })
        .collect();
    for handle in handles {
        handle.join().unwrap().unwrap();
    }
    let count: i64 = store
        .with_connection(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM contract_event_impact_grades WHERE event_id = ?1 AND grade_version = ?2",
                [&value.event_id, &value.grade_version],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(count, 1);
    let _ = std::fs::remove_file(path);
}
