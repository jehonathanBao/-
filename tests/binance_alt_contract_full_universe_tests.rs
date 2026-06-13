use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    collector::shard_symbols,
    config::{BinanceAltContractRuntimeConfig, BinanceAltUniverseMode},
    symbol_universe::{build_symbol_universe, BinanceAltSymbolCandidate},
    types::{AltContractMarketTier, AltContractSymbolTier},
};

fn candidate(symbol: &str, quote_volume_24h_usd: f64) -> BinanceAltSymbolCandidate {
    BinanceAltSymbolCandidate {
        symbol: symbol.to_string(),
        quote_asset: "USDT".to_string(),
        contract_type: "PERPETUAL".to_string(),
        status: "TRADING".to_string(),
        quote_volume_24h_usd,
    }
}

fn all_mode_config() -> BinanceAltContractRuntimeConfig {
    let mut config = BinanceAltContractRuntimeConfig::default();
    config.symbol_universe.universe_mode = BinanceAltUniverseMode::AllBinanceUsdtPerp;
    config.symbol_universe.symbol_limit = 0;
    config.symbol_universe.min_24h_quote_volume_usd = 0.0;
    config.symbol_universe.whitelist = Vec::new();
    config.symbol_universe.blacklist = Vec::new();
    config.symbol_universe.exclude_symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    config
}

#[test]
fn all_binance_usdt_perp_mode_keeps_all_matching_symbols_without_top_n_limit() {
    let config = all_mode_config();
    let candidates = [
        candidate("BTCUSDT", 2_000_000_000.0),
        candidate("ETHUSDT", 1_500_000_000.0),
        candidate("SOLUSDT", 600_000_000.0),
        candidate("DOGEUSDT", 120_000_000.0),
        candidate("SUIUSDT", 25_000_000.0),
        candidate("TINYUSDT", 1_000_000.0),
        BinanceAltSymbolCandidate {
            symbol: "SOLUSDC".to_string(),
            quote_asset: "USDC".to_string(),
            contract_type: "PERPETUAL".to_string(),
            status: "TRADING".to_string(),
            quote_volume_24h_usd: 900_000_000.0,
        },
        BinanceAltSymbolCandidate {
            symbol: "OLDUSDT".to_string(),
            quote_asset: "USDT".to_string(),
            contract_type: "PERPETUAL".to_string(),
            status: "BREAK".to_string(),
            quote_volume_24h_usd: 900_000_000.0,
        },
    ];

    let universe = build_symbol_universe(&candidates, &config);
    let product_ids = universe
        .iter()
        .map(|meta| meta.product_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        product_ids,
        vec!["SOLUSDT", "DOGEUSDT", "SUIUSDT", "TINYUSDT"]
    );
    assert_eq!(
        universe.last().expect("tiny").tier,
        AltContractSymbolTier::E
    );
    assert_eq!(universe[0].market_tier, AltContractMarketTier::UltraCore);
    assert_eq!(universe[1].market_tier, AltContractMarketTier::Mainstream);
    assert_eq!(universe[2].market_tier, AltContractMarketTier::Mainstream);
    assert_eq!(universe[3].market_tier, AltContractMarketTier::Alt);
}

#[test]
fn market_classification_splits_ultra_core_mainstream_and_alt() {
    let config = all_mode_config();

    for symbol in ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT"] {
        assert_eq!(
            config.classify_market_tier(symbol),
            AltContractMarketTier::UltraCore,
            "{symbol} should be ultra core"
        );
        assert_eq!(config.display_threshold_for_product(symbol), 750_000.0);
    }

    for symbol in ["XRPUSDT", "ADAUSDT", "AVAXUSDT"] {
        assert_eq!(
            config.classify_market_tier(symbol),
            AltContractMarketTier::Mainstream,
            "{symbol} should be mainstream"
        );
        assert_eq!(config.display_threshold_for_product(symbol), 500_000.0);
    }

    for symbol in ["PEPEUSDT", "WIFUSDT", "FLOKIUSDT"] {
        assert_eq!(
            config.classify_market_tier(symbol),
            AltContractMarketTier::Alt,
            "{symbol} should be alt"
        );
        assert_eq!(config.display_threshold_for_product(symbol), 150_000.0);
    }
}

#[test]
fn top_n_mode_still_supports_debug_limits_and_volume_filter() {
    let mut config = all_mode_config();
    config.symbol_universe.universe_mode = BinanceAltUniverseMode::TopN;
    config.symbol_universe.symbol_limit = 2;
    config.symbol_universe.min_24h_quote_volume_usd = 20_000_000.0;
    let candidates = [
        candidate("SOLUSDT", 600_000_000.0),
        candidate("DOGEUSDT", 120_000_000.0),
        candidate("SUIUSDT", 25_000_000.0),
        candidate("TINYUSDT", 1_000_000.0),
    ];

    let universe = build_symbol_universe(&candidates, &config);

    assert_eq!(universe.len(), 2);
    assert_eq!(universe[0].product_id, "SOLUSDT");
    assert_eq!(universe[1].product_id, "DOGEUSDT");
}

#[test]
fn whitelist_only_mode_ignores_non_whitelisted_symbols() {
    let mut config = all_mode_config();
    config.symbol_universe.universe_mode = BinanceAltUniverseMode::WhitelistOnly;
    config.symbol_universe.whitelist = vec!["SUIUSDT".to_string()];
    let candidates = [
        candidate("SOLUSDT", 600_000_000.0),
        candidate("SUIUSDT", 25_000_000.0),
        candidate("TINYUSDT", 1_000_000.0),
    ];

    let universe = build_symbol_universe(&candidates, &config);

    assert_eq!(universe.len(), 1);
    assert_eq!(universe[0].product_id, "SUIUSDT");
}

#[test]
fn shard_symbols_never_exceeds_configured_stream_count() {
    let symbols = (0..450)
        .map(|index| format!("ALT{index}USDT"))
        .collect::<Vec<_>>();

    let shards = shard_symbols(&symbols, 200);

    assert_eq!(shards.len(), 3);
    assert!(shards.iter().all(|shard| shard.len() <= 200));
    assert_eq!(shards[0].len(), 200);
    assert_eq!(shards[1].len(), 200);
    assert_eq!(shards[2].len(), 50);
}
