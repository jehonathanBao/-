use super::{
    config::{BinanceAltContractRuntimeConfig, BinanceAltUniverseMode},
    types::{AltContractSymbolMeta, AltContractSymbolTier},
};

#[derive(Debug, Clone)]
pub struct BinanceAltSymbolCandidate {
    pub symbol: String,
    pub quote_asset: String,
    pub contract_type: String,
    pub status: String,
    pub quote_volume_24h_usd: f64,
}

pub fn build_symbol_universe(
    candidates: &[BinanceAltSymbolCandidate],
    config: &BinanceAltContractRuntimeConfig,
) -> Vec<AltContractSymbolMeta> {
    let whitelist = config
        .symbol_universe
        .whitelist
        .iter()
        .map(|symbol| symbol.to_ascii_uppercase())
        .collect::<std::collections::BTreeSet<_>>();
    let blacklist = config
        .symbol_universe
        .blacklist
        .iter()
        .map(|symbol| symbol.to_ascii_uppercase())
        .collect::<std::collections::BTreeSet<_>>();
    let exclude = config
        .symbol_universe
        .exclude_symbols
        .iter()
        .map(|symbol| symbol.to_ascii_uppercase())
        .collect::<std::collections::BTreeSet<_>>();

    let mut items = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .quote_asset
                .eq_ignore_ascii_case(&config.symbol_universe.quote_asset)
                && candidate
                    .contract_type
                    .eq_ignore_ascii_case(&config.symbol_universe.contract_type)
                && candidate
                    .status
                    .eq_ignore_ascii_case(&config.symbol_universe.status)
        })
        .filter(|candidate| !blacklist.contains(&candidate.symbol.to_ascii_uppercase()))
        .filter(|candidate| !exclude.contains(&candidate.symbol.to_ascii_uppercase()))
        .filter(|candidate| {
            let symbol = candidate.symbol.to_ascii_uppercase();
            match config.effective_universe_mode() {
                BinanceAltUniverseMode::AllBinanceUsdtPerp => true,
                BinanceAltUniverseMode::TopN => {
                    candidate.quote_volume_24h_usd
                        >= config.symbol_universe.min_24h_quote_volume_usd
                }
                BinanceAltUniverseMode::WhitelistOnly => whitelist.contains(&symbol),
            }
        })
        .map(|candidate| AltContractSymbolMeta {
            symbol: base_symbol(&candidate.symbol),
            product_id: candidate.symbol.to_ascii_uppercase(),
            tier: tier_for_quote_volume(candidate.quote_volume_24h_usd),
            quote_volume_24h_usd: candidate.quote_volume_24h_usd,
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .quote_volume_24h_usd
            .total_cmp(&left.quote_volume_24h_usd)
            .then_with(|| left.product_id.cmp(&right.product_id))
    });
    if matches!(
        config.effective_universe_mode(),
        BinanceAltUniverseMode::TopN
    ) && config.symbol_universe.symbol_limit > 0
    {
        items.truncate(config.symbol_universe.symbol_limit);
    }
    items
}

pub fn tier_for_quote_volume(quote_volume_24h_usd: f64) -> AltContractSymbolTier {
    if quote_volume_24h_usd >= 500_000_000.0 {
        AltContractSymbolTier::A
    } else if quote_volume_24h_usd >= 100_000_000.0 {
        AltContractSymbolTier::B
    } else if quote_volume_24h_usd >= 20_000_000.0 {
        AltContractSymbolTier::C
    } else if quote_volume_24h_usd >= 5_000_000.0 {
        AltContractSymbolTier::D
    } else {
        AltContractSymbolTier::E
    }
}

pub fn meta_from_product_id(product_id: &str) -> AltContractSymbolMeta {
    AltContractSymbolMeta {
        symbol: base_symbol(product_id),
        product_id: product_id.to_ascii_uppercase(),
        tier: AltContractSymbolTier::B,
        quote_volume_24h_usd: 250_000_000.0,
    }
}

fn base_symbol(product_id: &str) -> String {
    product_id
        .trim()
        .to_ascii_uppercase()
        .trim_end_matches("USDT")
        .to_string()
}
