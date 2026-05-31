pub const MIGRATIONS: &[&str] = &[
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
];
