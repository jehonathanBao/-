use std::collections::{BTreeMap, BTreeSet};

use crate::{
    config::{
        thresholds::{LiqHuntParams, LiquidationClusterParams, ToxicVolumeParams, VpinParams},
        AppConfig,
    },
    market_data::{
        book_state::BookState, price_index::PriceIndex, rolling_windows::RollingWindows,
        trade_ring_buffer::TradeRingBuffer,
    },
    normalizers::book::{normalize_book, RawBookInput},
    normalizers::symbol::canonical_perp_symbol,
    toxicity::{
        liq_hunt_detector::{LiqHuntDetector, LiqHuntDetectorInput},
        liquidation_cluster_engine::LiquidationClusterEngine,
        liquidity_thinness::LiquidityThinness,
        markout_engine::{
            MarkoutEngine, DEFAULT_MARKOUT_EXPIRE_GRACE_MS, DEFAULT_MARKOUT_MAX_AGE_MS,
        },
        sweep_detector::{SweepDetector, SweepInput, SweepParams},
        toxic_volume_engine::ToxicVolumeEngine,
        vpin_bucket_engine::VpinBucketEngine,
    },
    types::{
        liq_hunt::LiqHuntState,
        liquidation::empty_liquidation_state,
        market::{venue_symbol_mapping, NormalizedTrade},
        sweep::{SweepQuality, SweepState},
        toxic::{ToxicEvent, ToxicState},
    },
};

use super::{
    liq_hunt_replay_report::LiqHuntReplayAccumulator,
    liquidation_replay_report::LiquidationReplayAccumulator,
    replay_loader::load_jsonl,
    replay_report::{ReplayMarkerOutcome, ReplayReport},
    replay_types::{ReplayEvent, ReplayExpectToxicRecord},
    vpin_replay_report::VpinReplayAccumulator,
};

pub struct ReplayRunner {
    config: AppConfig,
}

impl ReplayRunner {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn run_file(&mut self, path: &str) -> anyhow::Result<ReplayReport> {
        let mut events = load_jsonl(path)?;
        let event_count = events.len();
        events.sort_by_key(event_ts);

        let mut config = self.config.clone();
        if let Some(symbol) = canonical_perp_symbol(&config.symbol) {
            config.symbol = symbol;
        }

        let mut trade_buffer = TradeRingBuffer::new(config.max_buffer_age_ms.max(120_000));
        let mut book_state = BookState::default();
        let mut price_index =
            PriceIndex::new(config.max_buffer_age_ms.max(120_000), config.book_stale_ms);
        let mut markout_engine = MarkoutEngine::new(
            config.markout_horizons_ms.clone(),
            config.max_buffer_age_ms.max(DEFAULT_MARKOUT_MAX_AGE_MS),
            DEFAULT_MARKOUT_EXPIRE_GRACE_MS,
        );
        let sweep_detector = SweepDetector::new(SweepParams::default());
        let liquidity = LiquidityThinness::default();
        let toxic_engine = ToxicVolumeEngine::new(ToxicVolumeParams {
            threshold_btc: config.toxic_volume_alert_btc,
            ..ToxicVolumeParams::default()
        });
        let liq_hunt_detector = LiqHuntDetector::new(LiqHuntParams {
            cluster_large_notional_usd: config.liq_hunt_cluster_large_notional_usd,
            near_distance_bps: config.liq_hunt_near_distance_bps,
            active_score: config.liq_hunt_active_score,
            likely_score: config.liq_hunt_likely_score,
            watch_score: config.liq_hunt_watch_score,
            ..LiqHuntParams::default()
        });
        let liquidation_engine = LiquidationClusterEngine::new(LiquidationClusterParams {
            enabled: config.liquidation_enabled,
            lookback_ms: config.liquidation_lookback_ms,
            cluster_band_bps: config.liquidation_cluster_band_bps,
            min_cluster_distance_bps: config.liquidation_min_cluster_distance_bps,
            max_cluster_distance_bps: config.liquidation_max_cluster_distance_bps,
            proximity_threshold_bps: config.liquidation_proximity_threshold_bps,
            min_touches: config.liquidation_min_cluster_touches,
            pressure_threshold: config.liquidation_pressure_threshold,
        });
        let mut vpin_engine = VpinBucketEngine::new_for_symbol(
            VpinParams {
                enabled: config.vpin_enabled,
                bucket_size_btc: config.vpin_bucket_size_btc,
                lookback_buckets: config.vpin_lookback_buckets,
                min_buckets: config.vpin_min_buckets,
                spike_zscore: config.vpin_spike_zscore,
                high_threshold: config.vpin_high_threshold,
                extreme_threshold: config.vpin_extreme_threshold,
                persist_buckets: config.vpin_persist_buckets,
                ..VpinParams::default()
            },
            config.symbol.clone(),
        );

        let mut detected_events = Vec::new();
        let mut seen_ids = BTreeSet::new();
        let mut reason_code_frequency = BTreeMap::new();
        let mut threshold_buckets = BTreeMap::from([
            (">=300 BTC".to_string(), 0_usize),
            (">=600 BTC".to_string(), 0_usize),
            (">=1000 BTC".to_string(), 0_usize),
            (">=2000 BTC".to_string(), 0_usize),
        ]);
        let mut markers = Vec::new();
        let mut trade_count = 0;
        let mut book_count = 0;
        let mut vpin_accumulator = VpinReplayAccumulator::default();
        let mut liquidation_accumulator = LiquidationReplayAccumulator::default();
        let mut liq_hunt_accumulator = LiqHuntReplayAccumulator::default();

        for event in events {
            match event {
                ReplayEvent::Trade(trade_record) => {
                    trade_count += 1;
                    let trade = NormalizedTrade {
                        venue: trade_record.venue,
                        symbol: config.symbol.clone(),
                        ts: trade_record.ts,
                        price: trade_record.price,
                        size_btc: trade_record.size_btc,
                        size_usd: trade_record.price * trade_record.size_btc,
                        aggressor_side: trade_record.aggressor_side,
                        trade_id: trade_record.trade_id,
                    };
                    trade_buffer.add_trade(trade.clone());
                    vpin_engine.on_trade(&trade);
                    markout_engine.on_trade(&trade);
                    markout_engine.resolve_due_samples_for_symbol(&config.symbol, trade.ts, |ts| {
                        price_index.mid_at_or_before_for_symbol(ts, &config.symbol)
                    });
                    vpin_accumulator.observe(&vpin_engine.get_state(trade.ts).metrics);
                    collect_events(
                        trade.ts,
                        &config,
                        &trade_buffer,
                        &book_state,
                        &price_index,
                        &markout_engine,
                        &sweep_detector,
                        &liquidity,
                        &toxic_engine,
                        &vpin_engine,
                        &liquidation_engine,
                        &liq_hunt_detector,
                        &mut liquidation_accumulator,
                        &mut liq_hunt_accumulator,
                        &mut detected_events,
                        &mut seen_ids,
                        &mut reason_code_frequency,
                        &mut threshold_buckets,
                    );
                }
                ReplayEvent::Book(book_record) => {
                    book_count += 1;
                    let mut bids = book_record.bids;
                    let mut asks = book_record.asks;
                    if bids.is_empty() {
                        bids.push((book_record.best_bid, 1.0));
                    }
                    if asks.is_empty() {
                        asks.push((book_record.best_ask, 1.0));
                    }
                    if let Some(book) = normalize_book(RawBookInput {
                        venue: book_record.venue,
                        symbol: venue_symbol_mapping(book_record.venue, &config.symbol)
                            .venue_symbol
                            .unwrap_or_else(|| config.symbol.clone()),
                        ts: book_record.ts,
                        bids,
                        asks,
                    }) {
                        book_state.update_book(book.clone());
                        price_index.update_book(book);
                    }
                    markout_engine.resolve_due_samples_for_symbol(
                        &config.symbol,
                        book_record.ts,
                        |ts| price_index.mid_at_or_before_for_symbol(ts, &config.symbol),
                    );
                    collect_events(
                        book_record.ts,
                        &config,
                        &trade_buffer,
                        &book_state,
                        &price_index,
                        &markout_engine,
                        &sweep_detector,
                        &liquidity,
                        &toxic_engine,
                        &vpin_engine,
                        &liquidation_engine,
                        &liq_hunt_detector,
                        &mut liquidation_accumulator,
                        &mut liq_hunt_accumulator,
                        &mut detected_events,
                        &mut seen_ids,
                        &mut reason_code_frequency,
                        &mut threshold_buckets,
                    );
                }
                ReplayEvent::ExpectToxic(marker) => markers.push(marker),
            }
        }

        let markers = evaluate_markers(&markers, &detected_events);
        let vpin_summary = vpin_accumulator.finalize(vpin_engine.recent_buckets(usize::MAX));
        let liquidation_summary = liquidation_accumulator.finalize();
        let liq_hunt_summary = liq_hunt_accumulator.finalize();

        Ok(ReplayReport {
            input_path: path.to_string(),
            event_count,
            trade_count,
            book_count,
            detected_events,
            threshold_buckets,
            reason_code_frequency,
            markers,
            vpin_summary,
            liquidation_summary,
            liq_hunt_summary,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_events(
    now_ts: i64,
    config: &AppConfig,
    trade_buffer: &TradeRingBuffer,
    book_state: &BookState,
    price_index: &PriceIndex,
    markout_engine: &MarkoutEngine,
    sweep_detector: &SweepDetector,
    liquidity: &LiquidityThinness,
    toxic_engine: &ToxicVolumeEngine,
    vpin_engine: &VpinBucketEngine,
    liquidation_engine: &LiquidationClusterEngine,
    liq_hunt_detector: &LiqHuntDetector,
    liquidation_accumulator: &mut LiquidationReplayAccumulator,
    liq_hunt_accumulator: &mut LiqHuntReplayAccumulator,
    detected_events: &mut Vec<ToxicEvent>,
    seen_ids: &mut BTreeSet<String>,
    reason_code_frequency: &mut BTreeMap<String, usize>,
    threshold_buckets: &mut BTreeMap<String, usize>,
) {
    let flow_state = RollingWindows::new_for_symbol(
        trade_buffer,
        book_state,
        price_index,
        &config.windows_ms,
        config.book_stale_ms,
        &config.symbol,
    )
    .compute_all(now_ts);
    let markout_state = markout_engine.get_state_for_symbol(
        &config.symbol,
        now_ts,
        price_index
            .current_snapshot_for_symbol(now_ts, &config.symbol)
            .is_some(),
    );
    let vpin_state = vpin_engine.get_state(now_ts);
    let sweep_state = compute_sweep_state(
        now_ts,
        config,
        trade_buffer,
        book_state,
        price_index,
        sweep_detector,
        liquidity,
    );
    let liquidation_snapshots = price_index
        .snapshots_since_for_symbol(now_ts - config.liquidation_lookback_ms, &config.symbol);
    let liquidation_state = if config.liquidation_enabled {
        liquidation_engine.compute(
            now_ts,
            &flow_state,
            &sweep_state,
            &vpin_state,
            &liquidation_snapshots,
        )
    } else {
        empty_liquidation_state(now_ts)
    };
    liquidation_accumulator.observe(
        &liquidation_state,
        liquidation_snapshots.len(),
        &config.symbol,
        0.65,
    );
    let mut toxic_results = BTreeMap::new();

    for window in flow_state.windows.values() {
        let result = toxic_engine.compute_window(
            window,
            &markout_state,
            &sweep_state,
            &vpin_state,
            &liquidation_state,
        );
        toxic_results.insert(window.window_ms.to_string(), result.clone());
        if let Some(event) = toxic_engine.build_event_if_triggered(&result) {
            if seen_ids.insert(event.id.clone()) {
                for reason in &event.reason_codes {
                    *reason_code_frequency.entry(reason.clone()).or_insert(0) += 1;
                }
                increment_buckets(threshold_buckets, event.toxic_volume_btc);
                detected_events.push(event);
            }
        }
    }

    let liq_hunt_state = LiqHuntState {
        symbol: config.symbol.clone(),
        updated_at: now_ts,
        result: liq_hunt_detector.detect(LiqHuntDetectorInput {
            now_ts,
            symbol: config.symbol.clone(),
            toxic_state: ToxicState {
                symbol: config.symbol.clone(),
                updated_at: now_ts,
                threshold_btc: config.toxic_volume_alert_btc,
                windows_ms: config.windows_ms.clone(),
                results: toxic_results,
                latest_event: detected_events.last().cloned(),
                recent_events: detected_events.clone(),
                quality: crate::types::toxic::ToxicQuality {
                    has_flow: flow_state
                        .windows
                        .values()
                        .any(|window| window.trade_count > 0),
                    has_markout: markout_state.quality.pending_samples > 0
                        || markout_state.quality.resolved_samples > 0,
                    has_sweep: sweep_state
                        .results
                        .values()
                        .any(|result| result.sweep_detected),
                    has_liquidation: liquidation_state.metrics.current_mid.is_some(),
                    liquidation: Some(liquidation_state.metrics.clone()),
                    active_venues: sweep_state.quality.active_venues.clone(),
                    stale_venues: sweep_state.quality.stale_venues.clone(),
                },
            },
            vpin_state: Some(vpin_state.clone()),
            sweep_state: sweep_state.clone(),
            liquidation_state: liquidation_state.clone(),
            flow_state: flow_state.clone(),
        }),
        recent_results: Vec::new(),
    };
    liq_hunt_accumulator.observe(&liq_hunt_state.result);
}

fn compute_sweep_state(
    now_ts: i64,
    config: &AppConfig,
    trade_buffer: &TradeRingBuffer,
    book_state: &BookState,
    price_index: &PriceIndex,
    sweep_detector: &SweepDetector,
    liquidity: &LiquidityThinness,
) -> SweepState {
    let mut results = BTreeMap::new();
    for window_ms in &config.sweep_windows_ms {
        let since_ts = now_ts - *window_ms as i64;
        let flow_window = RollingWindows::new_for_symbol(
            trade_buffer,
            book_state,
            price_index,
            &[*window_ms],
            config.book_stale_ms,
            &config.symbol,
        )
        .compute_window(*window_ms, now_ts);
        let liq = liquidity.detect(
            &config.symbol,
            *window_ms,
            price_index.snapshot_at_or_before_for_symbol(since_ts, &config.symbol),
            price_index.current_snapshot_for_symbol(now_ts, &config.symbol),
        );
        let result = sweep_detector.detect(SweepInput {
            symbol: config.symbol.clone(),
            window_ms: *window_ms,
            trades: trade_buffer
                .get_trades_since(since_ts)
                .into_iter()
                .filter(|trade| trade.ts <= now_ts)
                .filter(|trade| trade.symbol.eq_ignore_ascii_case(&config.symbol))
                .collect(),
            flow_window,
            liquidity: Some(liq),
        });
        results.insert(window_ms.to_string(), result);
    }
    SweepState {
        symbol: config.symbol.clone(),
        updated_at: now_ts,
        windows_ms: config.sweep_windows_ms.clone(),
        results,
        quality: SweepQuality {
            has_trades: trade_buffer
                .get_trades_since(now_ts - config.max_buffer_age_ms)
                .iter()
                .any(|trade| {
                    trade.ts <= now_ts && trade.symbol.eq_ignore_ascii_case(&config.symbol)
                }),
            has_books: price_index
                .current_snapshot_for_symbol(now_ts, &config.symbol)
                .is_some(),
            active_venues: crate::types::market::Venue::ALL
                .into_iter()
                .filter(|venue| {
                    book_state
                        .latest_books_for_symbol(&config.symbol)
                        .get(venue)
                        .is_some_and(|book| now_ts - book.ts <= config.book_stale_ms)
                })
                .collect(),
            stale_venues: crate::types::market::Venue::ALL
                .into_iter()
                .filter(|venue| {
                    book_state
                        .latest_books_for_symbol(&config.symbol)
                        .get(venue)
                        .is_some_and(|book| now_ts - book.ts > config.book_stale_ms)
                })
                .collect(),
        },
    }
}

fn increment_buckets(threshold_buckets: &mut BTreeMap<String, usize>, toxic_volume_btc: f64) {
    if toxic_volume_btc >= 300.0 {
        *threshold_buckets
            .entry(">=300 BTC".to_string())
            .or_insert(0) += 1;
    }
    if toxic_volume_btc >= 600.0 {
        *threshold_buckets
            .entry(">=600 BTC".to_string())
            .or_insert(0) += 1;
    }
    if toxic_volume_btc >= 1000.0 {
        *threshold_buckets
            .entry(">=1000 BTC".to_string())
            .or_insert(0) += 1;
    }
    if toxic_volume_btc >= 2000.0 {
        *threshold_buckets
            .entry(">=2000 BTC".to_string())
            .or_insert(0) += 1;
    }
}

fn evaluate_markers(
    markers: &[ReplayExpectToxicRecord],
    detected: &[ToxicEvent],
) -> ReplayMarkerOutcome {
    let mut matched = 0;
    let mut missed = 0;
    let mut matched_event_ids = BTreeSet::new();

    for marker in markers {
        let found = detected.iter().find(|event| {
            event.window_ms == marker.window_ms
                && event.direction == marker.direction
                && event.toxic_volume_btc >= marker.min_toxic_volume_btc
                && event.ts >= marker.ts
        });
        if let Some(event) = found {
            matched += 1;
            matched_event_ids.insert(event.id.clone());
        } else {
            missed += 1;
        }
    }

    let unexpected = detected
        .iter()
        .filter(|event| !matched_event_ids.contains(&event.id))
        .count();

    ReplayMarkerOutcome {
        matched,
        missed,
        unexpected,
    }
}

fn event_ts(event: &ReplayEvent) -> i64 {
    match event {
        ReplayEvent::Trade(record) => record.ts,
        ReplayEvent::Book(record) => record.ts,
        ReplayEvent::ExpectToxic(record) => record.ts,
    }
}
