use std::sync::{OnceLock, RwLock};

static GLOBAL_CONFIG: OnceLock<RwLock<ScoreRuntimeConfig>> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub struct ScoreRuntimeConfig {
    pub toxic_short: ToxicShortRuntimeConfig,
    pub market_structure: MarketStructureRuntimeConfig,
}

#[derive(Debug, Clone)]
pub struct ToxicShortRuntimeConfig {
    pub enabled: bool,
    pub windows_sec: Vec<u64>,
    pub half_life_sec: u64,
    pub max_ttl_sec: u64,
    pub weights: ToxicShortWeights,
    pub discord: ToxicShortDiscordConfig,
}

impl Default for ToxicShortRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            windows_sec: vec![1, 5, 15, 60],
            half_life_sec: 45,
            max_ttl_sec: 300,
            weights: ToxicShortWeights::default(),
            discord: ToxicShortDiscordConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToxicShortWeights {
    pub toxic_order_cluster: f64,
    pub aggressive_sweep: f64,
    pub orderbook_deformation: f64,
    pub spoof_cancel: f64,
    pub adverse_move: f64,
    pub liquidity_gap: f64,
    pub micro_volatility_shock: f64,
}

impl Default for ToxicShortWeights {
    fn default() -> Self {
        Self {
            toxic_order_cluster: 0.25,
            aggressive_sweep: 0.20,
            orderbook_deformation: 0.15,
            spoof_cancel: 0.15,
            adverse_move: 0.10,
            liquidity_gap: 0.10,
            micro_volatility_shock: 0.05,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToxicShortDiscordConfig {
    pub enabled: bool,
    pub min_score: u8,
    pub min_confidence: f64,
    pub min_data_quality: f64,
    pub cooldown_sec: u64,
}

impl Default for ToxicShortDiscordConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_score: 85,
            min_confidence: 70.0,
            min_data_quality: 70.0,
            cooldown_sec: 60,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketStructureRuntimeConfig {
    pub enabled: bool,
    pub windows_min: Vec<u64>,
    pub event_start_main_force_score: u8,
    pub event_start_extreme_impact_score: u8,
    pub event_end_main_force_score: u8,
    pub event_end_extreme_impact_score: u8,
    pub event_end_hold_minutes: u64,
    pub structure_weights: MarketStructureCompositeWeights,
    pub main_force_weights: MainForceWeights,
    pub spot_weights: SpotWeights,
    pub contract_weights: ContractWeights,
    pub cross_confirm_weights: CrossConfirmWeights,
    pub confirmation: MarketStructureConfirmationConfig,
    pub discord: MarketStructureDiscordConfig,
}

impl Default for MarketStructureRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            windows_min: vec![5, 15, 60, 240],
            event_start_main_force_score: 75,
            event_start_extreme_impact_score: 85,
            event_end_main_force_score: 55,
            event_end_extreme_impact_score: 60,
            event_end_hold_minutes: 15,
            structure_weights: MarketStructureCompositeWeights::default(),
            main_force_weights: MainForceWeights::default(),
            spot_weights: SpotWeights::default(),
            contract_weights: ContractWeights::default(),
            cross_confirm_weights: CrossConfirmWeights::default(),
            confirmation: MarketStructureConfirmationConfig::default(),
            discord: MarketStructureDiscordConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketStructureCompositeWeights {
    pub spot_score: f64,
    pub contract_score: f64,
    pub cross_confirm_score: f64,
}

impl Default for MarketStructureCompositeWeights {
    fn default() -> Self {
        Self {
            spot_score: 0.40,
            contract_score: 0.40,
            cross_confirm_score: 0.20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MainForceWeights {
    pub structure_raw: f64,
    pub spot_contract_min: f64,
    pub duration_score: f64,
}

impl Default for MainForceWeights {
    fn default() -> Self {
        Self {
            structure_raw: 0.65,
            spot_contract_min: 0.25,
            duration_score: 0.10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotWeights {
    pub spot_cvd: f64,
    pub spot_volume_anomaly: f64,
    pub spot_absorption: f64,
    pub spot_liquidity_shift: f64,
    pub spot_price_response: f64,
}

impl Default for SpotWeights {
    fn default() -> Self {
        Self {
            spot_cvd: 0.30,
            spot_volume_anomaly: 0.25,
            spot_absorption: 0.20,
            spot_liquidity_shift: 0.15,
            spot_price_response: 0.10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWeights {
    pub cwm_aggressive_flow: f64,
    pub oi_impulse: f64,
    pub liquidation_context: f64,
    pub funding_crowding: f64,
    pub basis_premium: f64,
    pub active_exchange_confirmation: f64,
}

impl Default for ContractWeights {
    fn default() -> Self {
        Self {
            cwm_aggressive_flow: 0.30,
            oi_impulse: 0.20,
            liquidation_context: 0.15,
            funding_crowding: 0.15,
            basis_premium: 0.10,
            active_exchange_confirmation: 0.10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrossConfirmWeights {
    pub spot_contract_direction_consistency: f64,
    pub multi_window_consistency: f64,
    pub price_response_consistency: f64,
    pub source_coverage: f64,
}

impl Default for CrossConfirmWeights {
    fn default() -> Self {
        Self {
            spot_contract_direction_consistency: 0.40,
            multi_window_consistency: 0.25,
            price_response_consistency: 0.20,
            source_coverage: 0.15,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketStructureConfirmationConfig {
    pub min_main_force_score: u8,
    pub min_confidence: f64,
    pub min_data_quality: f64,
    pub min_confirm_conditions: u8,
}

impl Default for MarketStructureConfirmationConfig {
    fn default() -> Self {
        Self {
            min_main_force_score: 75,
            min_confidence: 70.0,
            min_data_quality: 70.0,
            min_confirm_conditions: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketStructureDiscordConfig {
    pub enabled: bool,
    pub min_main_force_score: u8,
    pub min_extreme_impact_score: u8,
    pub min_confidence: f64,
    pub min_data_quality: f64,
    pub cooldown_sec: u64,
}

impl Default for MarketStructureDiscordConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_main_force_score: 80,
            min_extreme_impact_score: 85,
            min_confidence: 70.0,
            min_data_quality: 70.0,
            cooldown_sec: 1_200,
        }
    }
}

pub fn score_runtime_config() -> ScoreRuntimeConfig {
    global_config()
        .read()
        .expect("score config lock poisoned")
        .clone()
}

pub fn set_score_runtime_config(config: ScoreRuntimeConfig) {
    *global_config().write().expect("score config lock poisoned") = config;
}

pub fn reset_score_runtime_config() {
    set_score_runtime_config(ScoreRuntimeConfig::default());
}

pub fn load_score_runtime_config_from_settings(settings: &::config::Config) -> ScoreRuntimeConfig {
    let defaults = ScoreRuntimeConfig::default();
    ScoreRuntimeConfig {
        toxic_short: load_toxic_short_runtime_config(settings, &defaults.toxic_short),
        market_structure: load_market_structure_runtime_config(
            settings,
            &defaults.market_structure,
        ),
    }
}

fn load_toxic_short_runtime_config(
    settings: &::config::Config,
    defaults: &ToxicShortRuntimeConfig,
) -> ToxicShortRuntimeConfig {
    ToxicShortRuntimeConfig {
        enabled: bool_setting(settings, "scoring.toxic_short.enabled", defaults.enabled),
        windows_sec: positive_u64_list_setting(
            settings,
            "scoring.toxic_short.windows_sec",
            &defaults.windows_sec,
        ),
        half_life_sec: positive_u64_setting(
            settings,
            "scoring.toxic_short.half_life_sec",
            defaults.half_life_sec,
        ),
        max_ttl_sec: positive_u64_setting(
            settings,
            "scoring.toxic_short.max_ttl_sec",
            defaults.max_ttl_sec,
        ),
        weights: ToxicShortWeights {
            toxic_order_cluster: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.toxic_order_cluster",
                defaults.weights.toxic_order_cluster,
            ),
            aggressive_sweep: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.aggressive_sweep",
                defaults.weights.aggressive_sweep,
            ),
            orderbook_deformation: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.orderbook_deformation",
                defaults.weights.orderbook_deformation,
            ),
            spoof_cancel: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.spoof_cancel",
                defaults.weights.spoof_cancel,
            ),
            adverse_move: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.adverse_move",
                defaults.weights.adverse_move,
            ),
            liquidity_gap: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.liquidity_gap",
                defaults.weights.liquidity_gap,
            ),
            micro_volatility_shock: probability_weight_setting(
                settings,
                "scoring.toxic_short.weights.micro_volatility_shock",
                defaults.weights.micro_volatility_shock,
            ),
        },
        discord: ToxicShortDiscordConfig {
            enabled: bool_setting(
                settings,
                "scoring.toxic_short.discord.enabled",
                defaults.discord.enabled,
            ),
            min_score: bounded_u8_setting(
                settings,
                "scoring.toxic_short.discord.min_score",
                defaults.discord.min_score,
            ),
            min_confidence: bounded_f64_setting(
                settings,
                "scoring.toxic_short.discord.min_confidence",
                defaults.discord.min_confidence,
            ),
            min_data_quality: bounded_f64_setting(
                settings,
                "scoring.toxic_short.discord.min_data_quality",
                defaults.discord.min_data_quality,
            ),
            cooldown_sec: positive_u64_setting(
                settings,
                "scoring.toxic_short.discord.cooldown_sec",
                defaults.discord.cooldown_sec,
            ),
        },
    }
}

fn load_market_structure_runtime_config(
    settings: &::config::Config,
    defaults: &MarketStructureRuntimeConfig,
) -> MarketStructureRuntimeConfig {
    MarketStructureRuntimeConfig {
        enabled: bool_setting(
            settings,
            "scoring.market_structure.enabled",
            defaults.enabled,
        ),
        windows_min: positive_u64_list_setting(
            settings,
            "scoring.market_structure.windows_min",
            &defaults.windows_min,
        ),
        event_start_main_force_score: bounded_u8_setting(
            settings,
            "scoring.market_structure.event_start_main_force_score",
            defaults.event_start_main_force_score,
        ),
        event_start_extreme_impact_score: bounded_u8_setting(
            settings,
            "scoring.market_structure.event_start_extreme_impact_score",
            defaults.event_start_extreme_impact_score,
        ),
        event_end_main_force_score: bounded_u8_setting(
            settings,
            "scoring.market_structure.event_end_main_force_score",
            defaults.event_end_main_force_score,
        ),
        event_end_extreme_impact_score: bounded_u8_setting(
            settings,
            "scoring.market_structure.event_end_extreme_impact_score",
            defaults.event_end_extreme_impact_score,
        ),
        event_end_hold_minutes: positive_u64_setting(
            settings,
            "scoring.market_structure.event_end_hold_minutes",
            defaults.event_end_hold_minutes,
        ),
        structure_weights: MarketStructureCompositeWeights {
            spot_score: probability_weight_setting(
                settings,
                "scoring.market_structure.structure_weights.spot_score",
                defaults.structure_weights.spot_score,
            ),
            contract_score: probability_weight_setting(
                settings,
                "scoring.market_structure.structure_weights.contract_score",
                defaults.structure_weights.contract_score,
            ),
            cross_confirm_score: probability_weight_setting(
                settings,
                "scoring.market_structure.structure_weights.cross_confirm_score",
                defaults.structure_weights.cross_confirm_score,
            ),
        },
        main_force_weights: MainForceWeights {
            structure_raw: probability_weight_setting(
                settings,
                "scoring.market_structure.main_force_weights.structure_raw",
                defaults.main_force_weights.structure_raw,
            ),
            spot_contract_min: probability_weight_setting(
                settings,
                "scoring.market_structure.main_force_weights.spot_contract_min",
                defaults.main_force_weights.spot_contract_min,
            ),
            duration_score: probability_weight_setting(
                settings,
                "scoring.market_structure.main_force_weights.duration_score",
                defaults.main_force_weights.duration_score,
            ),
        },
        spot_weights: SpotWeights {
            spot_cvd: probability_weight_setting(
                settings,
                "scoring.market_structure.spot_weights.spot_cvd",
                defaults.spot_weights.spot_cvd,
            ),
            spot_volume_anomaly: probability_weight_setting(
                settings,
                "scoring.market_structure.spot_weights.spot_volume_anomaly",
                defaults.spot_weights.spot_volume_anomaly,
            ),
            spot_absorption: probability_weight_setting(
                settings,
                "scoring.market_structure.spot_weights.spot_absorption",
                defaults.spot_weights.spot_absorption,
            ),
            spot_liquidity_shift: probability_weight_setting(
                settings,
                "scoring.market_structure.spot_weights.spot_liquidity_shift",
                defaults.spot_weights.spot_liquidity_shift,
            ),
            spot_price_response: probability_weight_setting(
                settings,
                "scoring.market_structure.spot_weights.spot_price_response",
                defaults.spot_weights.spot_price_response,
            ),
        },
        contract_weights: ContractWeights {
            cwm_aggressive_flow: probability_weight_setting(
                settings,
                "scoring.market_structure.contract_weights.cwm_aggressive_flow",
                defaults.contract_weights.cwm_aggressive_flow,
            ),
            oi_impulse: probability_weight_setting(
                settings,
                "scoring.market_structure.contract_weights.oi_impulse",
                defaults.contract_weights.oi_impulse,
            ),
            liquidation_context: probability_weight_setting(
                settings,
                "scoring.market_structure.contract_weights.liquidation_context",
                defaults.contract_weights.liquidation_context,
            ),
            funding_crowding: probability_weight_setting(
                settings,
                "scoring.market_structure.contract_weights.funding_crowding",
                defaults.contract_weights.funding_crowding,
            ),
            basis_premium: probability_weight_setting(
                settings,
                "scoring.market_structure.contract_weights.basis_premium",
                defaults.contract_weights.basis_premium,
            ),
            active_exchange_confirmation: probability_weight_setting(
                settings,
                "scoring.market_structure.contract_weights.active_exchange_confirmation",
                defaults.contract_weights.active_exchange_confirmation,
            ),
        },
        cross_confirm_weights: CrossConfirmWeights {
            spot_contract_direction_consistency: probability_weight_setting(
                settings,
                "scoring.market_structure.cross_confirm_weights.spot_contract_direction_consistency",
                defaults.cross_confirm_weights.spot_contract_direction_consistency,
            ),
            multi_window_consistency: probability_weight_setting(
                settings,
                "scoring.market_structure.cross_confirm_weights.multi_window_consistency",
                defaults.cross_confirm_weights.multi_window_consistency,
            ),
            price_response_consistency: probability_weight_setting(
                settings,
                "scoring.market_structure.cross_confirm_weights.price_response_consistency",
                defaults.cross_confirm_weights.price_response_consistency,
            ),
            source_coverage: probability_weight_setting(
                settings,
                "scoring.market_structure.cross_confirm_weights.source_coverage",
                defaults.cross_confirm_weights.source_coverage,
            ),
        },
        confirmation: MarketStructureConfirmationConfig {
            min_main_force_score: bounded_u8_setting(
                settings,
                "scoring.market_structure.confirmation.min_main_force_score",
                defaults.confirmation.min_main_force_score,
            ),
            min_confidence: bounded_f64_setting(
                settings,
                "scoring.market_structure.confirmation.min_confidence",
                defaults.confirmation.min_confidence,
            ),
            min_data_quality: bounded_f64_setting(
                settings,
                "scoring.market_structure.confirmation.min_data_quality",
                defaults.confirmation.min_data_quality,
            ),
            min_confirm_conditions: positive_u8_setting(
                settings,
                "scoring.market_structure.confirmation.min_confirm_conditions",
                defaults.confirmation.min_confirm_conditions,
            ),
        },
        discord: MarketStructureDiscordConfig {
            enabled: bool_setting(
                settings,
                "scoring.market_structure.discord.enabled",
                defaults.discord.enabled,
            ),
            min_main_force_score: bounded_u8_setting(
                settings,
                "scoring.market_structure.discord.min_main_force_score",
                defaults.discord.min_main_force_score,
            ),
            min_extreme_impact_score: bounded_u8_setting(
                settings,
                "scoring.market_structure.discord.min_extreme_impact_score",
                defaults.discord.min_extreme_impact_score,
            ),
            min_confidence: bounded_f64_setting(
                settings,
                "scoring.market_structure.discord.min_confidence",
                defaults.discord.min_confidence,
            ),
            min_data_quality: bounded_f64_setting(
                settings,
                "scoring.market_structure.discord.min_data_quality",
                defaults.discord.min_data_quality,
            ),
            cooldown_sec: positive_u64_setting(
                settings,
                "scoring.market_structure.discord.cooldown_sec",
                defaults.discord.cooldown_sec,
            ),
        },
    }
}

fn global_config() -> &'static RwLock<ScoreRuntimeConfig> {
    GLOBAL_CONFIG.get_or_init(|| RwLock::new(ScoreRuntimeConfig::default()))
}

fn bool_setting(settings: &::config::Config, path: &str, default: bool) -> bool {
    settings.get_bool(path).unwrap_or(default)
}

fn positive_u64_list_setting(settings: &::config::Config, path: &str, default: &[u64]) -> Vec<u64> {
    match settings.get::<Vec<u64>>(path) {
        Ok(values) if !values.is_empty() && values.iter().all(|value| *value > 0) => values,
        Ok(values) => {
            warn_invalid(path, format!("{values:?}"), format!("{default:?}"));
            default.to_vec()
        }
        Err(_) => default.to_vec(),
    }
}

fn positive_u64_setting(settings: &::config::Config, path: &str, default: u64) -> u64 {
    match settings.get_int(path) {
        Ok(value) if value > 0 => value as u64,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn positive_u8_setting(settings: &::config::Config, path: &str, default: u8) -> u8 {
    match settings.get_int(path) {
        Ok(value) if value > 0 && value <= i64::from(u8::MAX) => value as u8,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn bounded_u8_setting(settings: &::config::Config, path: &str, default: u8) -> u8 {
    match settings.get_int(path) {
        Ok(value) if (0..=100).contains(&value) => value as u8,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn bounded_f64_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    match settings.get_float(path) {
        Ok(value) if value.is_finite() && (0.0..=100.0).contains(&value) => value,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn probability_weight_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    match settings.get_float(path) {
        Ok(value) if value.is_finite() && value > 0.0 && value <= 1.0 => value,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn warn_invalid<T: std::fmt::Display, D: std::fmt::Display>(path: &str, value: T, default: D) {
    tracing::warn!(
        path,
        value = %value,
        default = %default,
        "invalid score config value, using default"
    );
}
