pub const MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS alt_contract_signals (
      signal_id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL,
      ts INTEGER NOT NULL,
      signal_type TEXT NOT NULL,
      severity TEXT NOT NULL,
      direction TEXT,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_alt_contract_signals_product_ts
      ON alt_contract_signals(product_id, ts DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alt_contract_signal_outcomes (
      signal_id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL,
      tier TEXT NOT NULL,
      signal_ts INTEGER NOT NULL,
      window_sec INTEGER NOT NULL,
      signal_type TEXT NOT NULL,
      anomaly_severity TEXT NOT NULL,
      structure_confidence TEXT NOT NULL,
      exposure_tier TEXT NOT NULL,
      ais_score REAL NOT NULL,
      abnormal_score REAL NOT NULL,
      build_score REAL NOT NULL,
      regime TEXT NOT NULL,
      oi_context TEXT NOT NULL,
      liquidation_context TEXT NOT NULL,
      entry_price REAL,
      markout_5m_bps REAL,
      markout_15m_bps REAL,
      markout_1h_bps REAL,
      mfe_1h_bps REAL,
      mae_1h_bps REAL,
      follow_through_5m INTEGER,
      follow_through_15m INTEGER,
      follow_through_1h INTEGER,
      evaluated_5m_at INTEGER,
      evaluated_15m_at INTEGER,
      evaluated_1h_at INTEGER,
      outcome_version TEXT NOT NULL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_alt_contract_signal_outcomes_product_ts
      ON alt_contract_signal_outcomes(product_id, signal_ts DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alt_contract_events (
      event_id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL,
      signal_type TEXT NOT NULL,
      direction TEXT,
      start_ts INTEGER NOT NULL,
      last_update_ts INTEGER NOT NULL,
      status TEXT NOT NULL,
      latest_signal_id TEXT,
      peak_signal_id TEXT,
      signal_count INTEGER NOT NULL,
      peak_abnormal_score REAL,
      peak_build_score REAL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_alt_contract_events_product_ts
      ON alt_contract_events(product_id, last_update_ts DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS toxic_events (
      id TEXT PRIMARY KEY,
      ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      direction TEXT NOT NULL,
      severity TEXT NOT NULL,
      toxic_volume_btc REAL NOT NULL,
      toxic_ratio REAL NOT NULL,
      threshold_btc REAL NOT NULL,
      window_ms INTEGER NOT NULL,
      leader_venue TEXT,
      aggressive_buy_btc REAL NOT NULL,
      aggressive_sell_btc REAL NOT NULL,
      net_aggressive_btc REAL NOT NULL,
      abs_aggressive_btc REAL NOT NULL,
      markout_1s_bps REAL,
      markout_5s_bps REAL,
      sweep_detected INTEGER NOT NULL,
      liquidity_thin INTEGER NOT NULL,
      cross_venue_confirmed INTEGER NOT NULL,
      reason_codes_json TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      created_at INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_toxic_events_ts ON toxic_events(ts);
    CREATE INDEX IF NOT EXISTS idx_toxic_events_direction_ts ON toxic_events(direction, ts);
    CREATE INDEX IF NOT EXISTS idx_toxic_events_window_ts ON toxic_events(window_ms, ts);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS toxic_snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      max_toxic_volume_btc REAL NOT NULL,
      max_toxic_ratio REAL NOT NULL,
      max_window_ms INTEGER NOT NULL,
      direction TEXT NOT NULL,
      severity TEXT NOT NULL,
      threshold_btc REAL NOT NULL,
      alert_triggered INTEGER NOT NULL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_toxic_snapshots_ts ON toxic_snapshots(ts);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS flow_snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      window_ms INTEGER NOT NULL,
      aggressive_buy_btc REAL NOT NULL,
      aggressive_sell_btc REAL NOT NULL,
      net_aggressive_btc REAL NOT NULL,
      abs_aggressive_btc REAL NOT NULL,
      price_move_bps REAL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_flow_snapshots_ts ON flow_snapshots(ts);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS venue_health_snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      ts INTEGER NOT NULL,
      venue TEXT NOT NULL,
      enabled INTEGER NOT NULL,
      status TEXT NOT NULL,
      last_trade_ts INTEGER,
      last_book_ts INTEGER,
      last_message_ts INTEGER,
      reconnect_count INTEGER NOT NULL,
      last_error TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_venue_health_snapshots_ts ON venue_health_snapshots(ts);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS replay_runs (
      id TEXT PRIMARY KEY,
      started_at INTEGER NOT NULL,
      finished_at INTEGER,
      input_path TEXT NOT NULL,
      event_count INTEGER NOT NULL,
      toxic_event_count INTEGER NOT NULL,
      report_path TEXT,
      status TEXT NOT NULL,
      error TEXT
    );
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS vpin_buckets (
      id INTEGER PRIMARY KEY,
      start_ts INTEGER NOT NULL,
      end_ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      bucket_size_btc REAL NOT NULL,
      total_btc REAL NOT NULL,
      buy_btc REAL NOT NULL,
      sell_btc REAL NOT NULL,
      net_btc REAL NOT NULL,
      imbalance_btc REAL NOT NULL,
      imbalance_ratio REAL NOT NULL,
      direction TEXT NOT NULL,
      venue_breakdown_json TEXT NOT NULL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_vpin_buckets_end_ts ON vpin_buckets(end_ts);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS contract_flow_1s (
      ts_bucket INTEGER NOT NULL,
      exchange TEXT NOT NULL,
      symbol TEXT NOT NULL,
      buy_volume_btc REAL NOT NULL,
      sell_volume_btc REAL NOT NULL,
      buy_notional_usd REAL NOT NULL,
      sell_notional_usd REAL NOT NULL,
      trade_count INTEGER NOT NULL,
      max_single_trade_btc REAL,
      vwap REAL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
      PRIMARY KEY (ts_bucket, exchange, symbol)
    );
    CREATE INDEX IF NOT EXISTS idx_contract_flow_1s_symbol_ts
      ON contract_flow_1s(symbol, ts_bucket DESC);

    CREATE TABLE IF NOT EXISTS contract_liquidation_1s (
      ts_bucket INTEGER NOT NULL,
      exchange TEXT NOT NULL,
      symbol TEXT NOT NULL,
      long_liq_btc REAL NOT NULL,
      short_liq_btc REAL NOT NULL,
      liq_notional_usd REAL NOT NULL,
      order_count INTEGER NOT NULL,
      max_single_liq_btc REAL,
      vwap REAL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
      PRIMARY KEY (ts_bucket, exchange, symbol)
    );
    CREATE INDEX IF NOT EXISTS idx_contract_liquidation_1s_symbol_ts
      ON contract_liquidation_1s(symbol, ts_bucket DESC);

    CREATE TABLE IF NOT EXISTS contract_oi_snapshots (
      ts INTEGER NOT NULL,
      exchange TEXT NOT NULL,
      symbol TEXT NOT NULL,
      oi_btc REAL NOT NULL,
      oi_notional_usd REAL,
      ct_val_available INTEGER NOT NULL DEFAULT 1,
      evidence_degraded_reason TEXT,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
      PRIMARY KEY (ts, exchange, symbol)
    );
    CREATE INDEX IF NOT EXISTS idx_contract_oi_snapshots_symbol_ts
      ON contract_oi_snapshots(symbol, ts DESC);

    CREATE TABLE IF NOT EXISTS contract_funding_snapshots (
      ts INTEGER NOT NULL,
      exchange TEXT NOT NULL,
      symbol TEXT NOT NULL,
      funding_rate REAL NOT NULL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
      PRIMARY KEY (ts, exchange, symbol)
    );
    CREATE INDEX IF NOT EXISTS idx_contract_funding_snapshots_symbol_ts
      ON contract_funding_snapshots(symbol, ts DESC);

    CREATE TABLE IF NOT EXISTS contract_whale_signals (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      signal_id TEXT NOT NULL UNIQUE,
      ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      window_sec INTEGER NOT NULL,
      signal_type TEXT NOT NULL,
      direction TEXT NOT NULL,
      severity TEXT NOT NULL,
      score INTEGER NOT NULL,
      total_volume_btc REAL NOT NULL,
      net_volume_btc REAL NOT NULL,
      total_notional_usd REAL NOT NULL,
      dominance REAL NOT NULL,
      price_start REAL,
      price_end REAL,
      price_move_pct REAL,
      main_exchange TEXT,
      exchanges_json TEXT NOT NULL,
      dynamic_multiple REAL,
      data_quality INTEGER,
      discord_eligible INTEGER NOT NULL DEFAULT 0,
      discord_sent INTEGER NOT NULL DEFAULT 0,
      discord_sent_at INTEGER,
      payload_json TEXT NOT NULL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
    );
    CREATE INDEX IF NOT EXISTS idx_contract_whale_signals_ts
      ON contract_whale_signals(ts DESC);
    CREATE INDEX IF NOT EXISTS idx_contract_whale_signals_symbol_severity_ts
      ON contract_whale_signals(symbol, severity, ts DESC);

    CREATE TABLE IF NOT EXISTS contract_whale_percentile_thresholds (
      computed_at INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      exchange TEXT NOT NULL,
      window_sec INTEGER NOT NULL,
      p99_0_btc REAL NOT NULL,
      p99_5_btc REAL NOT NULL,
      p99_9_btc REAL NOT NULL,
      sample_count INTEGER NOT NULL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
      PRIMARY KEY (symbol, exchange, window_sec, computed_at)
    );
    CREATE INDEX IF NOT EXISTS idx_contract_whale_percentile_latest
      ON contract_whale_percentile_thresholds(symbol, exchange, window_sec, computed_at DESC);

    CREATE TABLE IF NOT EXISTS main_force_events (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      symbol TEXT NOT NULL,
      started_at INTEGER NOT NULL,
      ended_at INTEGER,
      peak_at INTEGER NOT NULL,
      last_observed_at INTEGER NOT NULL,
      inactive_since INTEGER,
      regime_type TEXT NOT NULL,
      severity TEXT NOT NULL,
      peak_main_force_score REAL NOT NULL,
      peak_extreme_impact_score REAL NOT NULL,
      peak_structure_bias REAL NOT NULL,
      confidence REAL NOT NULL,
      spot_score REAL,
      contract_score REAL,
      cross_confirm_score REAL,
      cwm_score REAL,
      oi_score REAL,
      liquidation_score REAL,
      funding_crowding_score REAL,
      main_force_confirmed INTEGER NOT NULL DEFAULT 0,
      extreme_impact_confirmed INTEGER NOT NULL DEFAULT 0,
      liquidation_driven INTEGER NOT NULL DEFAULT 0,
      reasons_json TEXT NOT NULL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
    );
    CREATE INDEX IF NOT EXISTS idx_main_force_events_symbol_started
      ON main_force_events(symbol, started_at DESC);
    CREATE INDEX IF NOT EXISTS idx_main_force_events_symbol_active
      ON main_force_events(symbol, ended_at, started_at DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS spot_whale_signals (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      signal_id TEXT NOT NULL UNIQUE,
      ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      window_sec INTEGER NOT NULL,
      signal_type TEXT NOT NULL,
      direction TEXT NOT NULL,
      severity TEXT NOT NULL,
      score INTEGER NOT NULL,
      total_volume_base REAL NOT NULL,
      net_volume_base REAL NOT NULL,
      total_notional_usd REAL NOT NULL,
      dominance REAL NOT NULL,
      price_move_pct REAL,
      coinbase_premium_pct REAL,
      main_exchange TEXT,
      exchanges_json TEXT NOT NULL,
      dynamic_multiple REAL,
      multi_exchange_confirmed INTEGER NOT NULL DEFAULT 0,
      data_quality INTEGER NOT NULL,
      discord_eligible INTEGER NOT NULL DEFAULT 0,
      discord_sent INTEGER NOT NULL DEFAULT 0,
      discord_sent_at INTEGER,
      discord_reason TEXT NOT NULL,
      is_permanent INTEGER NOT NULL DEFAULT 0,
      payload_json TEXT NOT NULL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
    );
    CREATE INDEX IF NOT EXISTS idx_spot_whale_signals_ts
      ON spot_whale_signals(ts DESC);
    CREATE INDEX IF NOT EXISTS idx_spot_whale_signals_symbol_severity_ts
      ON spot_whale_signals(symbol, severity, ts DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS contract_whale_discord_outbox (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      signal_id TEXT NOT NULL UNIQUE,
      symbol TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      status TEXT NOT NULL,
      attempts INTEGER NOT NULL DEFAULT 0,
      next_attempt_at INTEGER,
      created_at INTEGER NOT NULL,
      sent_at INTEGER,
      last_error TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_contract_whale_discord_outbox_due
      ON contract_whale_discord_outbox(status, next_attempt_at, created_at);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS contract_whale_emission_watermarks (
      emission_key TEXT PRIMARY KEY,
      payload_json TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS contract_whale_signal_outcomes (
      signal_id TEXT PRIMARY KEY,
      symbol TEXT NOT NULL,
      signal_ts INTEGER NOT NULL,
      signal_type TEXT NOT NULL,
      classification_v2 TEXT,
      severity TEXT NOT NULL,
      impact_level TEXT,
      window_sec INTEGER NOT NULL,
      oi_context TEXT,
      regime TEXT,
      entry_price REAL,
      markout_30s_bps REAL,
      markout_2m_bps REAL,
      markout_5m_bps REAL,
      mfe_5m_bps REAL,
      mae_5m_bps REAL,
      absolute_return_30s_bps REAL,
      absolute_return_2m_bps REAL,
      absolute_return_5m_bps REAL,
      realized_volatility_5m_bps REAL,
      max_absolute_excursion_5m_bps REAL,
      price_sample_count_5m INTEGER,
      liquidity_recovered_5m INTEGER,
      liquidity_recovery_ms INTEGER,
      liquidity_recovery_reason TEXT,
      setup_outcome TEXT,
      follow_through_30s INTEGER,
      follow_through_2m INTEGER,
      follow_through_5m INTEGER,
      evaluated_at INTEGER,
      outcome_version TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_contract_whale_signal_outcomes_summary
      ON contract_whale_signal_outcomes(symbol, severity, signal_ts DESC);
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_contract_flow_1s_symbol_exchange_ts
      ON contract_flow_1s(symbol, exchange, ts_bucket DESC);
    CREATE INDEX IF NOT EXISTS idx_contract_oi_snapshots_symbol_exchange_ts
      ON contract_oi_snapshots(symbol, exchange, ts DESC);
    CREATE INDEX IF NOT EXISTS idx_contract_funding_snapshots_symbol_exchange_ts
      ON contract_funding_snapshots(symbol, exchange, ts DESC);
    CREATE INDEX IF NOT EXISTS idx_contract_whale_signals_symbol_ts
      ON contract_whale_signals(symbol, ts DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS new_token_l2_metrics (
      ts INTEGER NOT NULL,
      symbol TEXT NOT NULL,
      readiness TEXT NOT NULL,
      evidence_mode TEXT NOT NULL,
      spread_bps REAL NOT NULL,
      imbalance REAL NOT NULL,
      visible_cancel_to_add_ratio REAL NOT NULL,
      intent_state TEXT NOT NULL,
      intent_confidence REAL NOT NULL,
      intent_available INTEGER NOT NULL,
      wall_count INTEGER NOT NULL,
      payload_json TEXT NOT NULL,
      PRIMARY KEY (symbol, ts)
    );
    CREATE INDEX IF NOT EXISTS idx_new_token_l2_metrics_symbol_ts
      ON new_token_l2_metrics(symbol, ts DESC);
    CREATE TABLE IF NOT EXISTS new_token_l2_outcomes (
      event_id TEXT NOT NULL,
      symbol TEXT NOT NULL,
      observed_at INTEGER NOT NULL,
      horizon_sec INTEGER NOT NULL,
      intent_state TEXT NOT NULL,
      entry_price REAL NOT NULL,
      observed_price REAL NOT NULL,
      price_move_bps REAL,
      outcome_label TEXT NOT NULL,
      shadow_only INTEGER NOT NULL,
      discord_eligible INTEGER NOT NULL,
      execution_enabled INTEGER NOT NULL,
      outcome_reason TEXT NOT NULL,
      PRIMARY KEY (event_id, horizon_sec)
    );
    CREATE INDEX IF NOT EXISTS idx_new_token_l2_outcomes_symbol_observed
      ON new_token_l2_outcomes(symbol, observed_at DESC);
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_contract_whale_signals_event_feed
      ON contract_whale_signals(symbol, ts DESC, signal_id DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS hourly_delta_alert_records (
      record_key TEXT PRIMARY KEY,
      exchange TEXT NOT NULL,
      symbol TEXT NOT NULL,
      interval TEXT NOT NULL,
      kline_open_time_ms INTEGER NOT NULL,
      kline_close_time_ms INTEGER NOT NULL,
      taker_buy_btc REAL NOT NULL,
      taker_sell_btc REAL NOT NULL,
      delta_btc REAL NOT NULL,
      volume_btc REAL NOT NULL,
      direction TEXT NOT NULL,
      above_threshold INTEGER NOT NULL,
      data_status TEXT NOT NULL,
      discord_status TEXT NOT NULL,
      discord_sent_at_ms INTEGER,
      attempts INTEGER NOT NULL DEFAULT 0,
      last_error TEXT,
      payload_json TEXT NOT NULL,
      created_at_ms INTEGER NOT NULL,
      updated_at_ms INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_hourly_delta_alert_records_symbol_open
      ON hourly_delta_alert_records(symbol, kline_open_time_ms DESC);
    CREATE TABLE IF NOT EXISTS hourly_delta_discord_outbox (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      record_key TEXT NOT NULL UNIQUE,
      symbol TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      status TEXT NOT NULL,
      attempts INTEGER NOT NULL DEFAULT 0,
      next_attempt_at INTEGER,
      created_at INTEGER NOT NULL,
      sent_at INTEGER,
      last_error TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_hourly_delta_discord_outbox_due
      ON hourly_delta_discord_outbox(status, next_attempt_at, created_at);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS contract_event_impact_baselines (
      symbol TEXT NOT NULL,
      window_sec INTEGER NOT NULL,
      threshold_profile TEXT NOT NULL,
      computed_at_ms INTEGER NOT NULL,
      lookback_from_ms INTEGER NOT NULL,
      lookback_to_ms INTEGER NOT NULL,
      sample_count INTEGER NOT NULL,
      median_log_volume REAL NOT NULL,
      mad_log_volume REAL NOT NULL,
      sorted_samples_json TEXT NOT NULL,
      PRIMARY KEY(symbol, window_sec, threshold_profile)
    );
    CREATE TABLE IF NOT EXISTS contract_event_impact_grades (
      event_id TEXT NOT NULL,
      grade_version TEXT NOT NULL,
      episode_id TEXT NOT NULL,
      symbol TEXT NOT NULL,
      grade TEXT NOT NULL CHECK(grade IN ('C','B','A','S')),
      state TEXT NOT NULL CHECK(state IN ('evidence_insufficient','provisional','confirmed')),
      reason_codes_json TEXT NOT NULL,
      evidence_json TEXT NOT NULL,
      assessed_at_ms INTEGER NOT NULL,
      discord_sent_at_ms INTEGER,
      created_at_ms INTEGER NOT NULL,
      updated_at_ms INTEGER NOT NULL,
      PRIMARY KEY(event_id, grade_version)
    );
    CREATE UNIQUE INDEX IF NOT EXISTS idx_contract_event_impact_episode_version
      ON contract_event_impact_grades(episode_id, grade_version);
    CREATE INDEX IF NOT EXISTS idx_contract_event_impact_grades_symbol_assessed
      ON contract_event_impact_grades(symbol, assessed_at_ms DESC);
    "#,
];
