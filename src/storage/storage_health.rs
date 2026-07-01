use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use parking_lot::RwLock;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorageHealthGuardConfig {
    pub enabled: bool,
    pub db_warn_gb: f64,
    pub db_critical_gb: f64,
    pub wal_warn_gb: f64,
    pub wal_critical_gb: f64,
    pub disk_warn_percent: f64,
    pub disk_critical_percent: f64,
    pub refresh_interval_ms: u64,
    pub degraded_mode_enabled: bool,
}

impl Default for StorageHealthGuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            db_warn_gb: 20.0,
            db_critical_gb: 40.0,
            wal_warn_gb: 1.0,
            wal_critical_gb: 3.0,
            disk_warn_percent: 85.0,
            disk_critical_percent: 92.0,
            refresh_interval_ms: 10_000,
            degraded_mode_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetentionTableStatus {
    #[default]
    Ok,
    Skipped,
    Error,
}

impl RetentionTableStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Skipped => "skipped",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetentionTableResult {
    pub table: String,
    pub time_column: String,
    pub status: RetentionTableStatus,
    pub deleted_rows: usize,
    pub duration_ms: u64,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub error_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WalCheckpointResult {
    pub attempted: bool,
    pub ok: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetentionRunHealth {
    pub ok: bool,
    pub total_deleted_rows: usize,
    pub failed_tables: Vec<String>,
    pub skipped_tables: Vec<String>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub finished_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageHealthStatus {
    #[default]
    Ok,
    Warn,
    Critical,
    Disabled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHealthSnapshot {
    pub db_path: String,
    pub db_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub shm_size_bytes: u64,
    pub disk_used_percent: f64,
    pub disk_free_bytes: u64,
    pub status: StorageHealthStatus,
    pub warnings: Vec<String>,
    pub degraded_mode_active: bool,
    pub degraded_writes: Vec<String>,
    pub last_runtime_retention: Option<RetentionRunHealth>,
    pub last_contract_whale_retention: Option<RetentionRunHealth>,
    pub last_wal_checkpoint: Option<WalCheckpointResult>,
    pub refreshed_at_ms: i64,
}

impl Default for StorageHealthSnapshot {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            db_size_bytes: 0,
            wal_size_bytes: 0,
            shm_size_bytes: 0,
            disk_used_percent: 0.0,
            disk_free_bytes: 0,
            status: StorageHealthStatus::Disabled,
            warnings: Vec::new(),
            degraded_mode_active: false,
            degraded_writes: degraded_snapshot_writes(),
            last_runtime_retention: None,
            last_contract_whale_retention: None,
            last_wal_checkpoint: None,
            refreshed_at_ms: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageHealthTracker {
    inner: Arc<RwLock<StorageHealthTrackerInner>>,
}

#[derive(Debug, Default)]
struct StorageHealthTrackerInner {
    db_path: Option<PathBuf>,
    config: StorageHealthGuardConfig,
    snapshot: StorageHealthSnapshot,
    last_log_signature: Option<(StorageHealthStatus, bool, Vec<String>)>,
}

impl StorageHealthTracker {
    pub fn new(db_path: Option<PathBuf>, config: StorageHealthGuardConfig) -> Self {
        let mut snapshot = StorageHealthSnapshot::default();
        snapshot.status = if config.enabled {
            StorageHealthStatus::Ok
        } else {
            StorageHealthStatus::Disabled
        };
        if let Some(path) = db_path.as_ref() {
            snapshot.db_path = path.display().to_string();
        }
        Self {
            inner: Arc::new(RwLock::new(StorageHealthTrackerInner {
                db_path,
                config,
                snapshot,
                last_log_signature: None,
            })),
        }
    }

    pub fn snapshot(&self) -> StorageHealthSnapshot {
        self.refresh_if_due(false)
    }

    pub fn refresh_now(&self) -> StorageHealthSnapshot {
        self.refresh_if_due(true)
    }

    pub fn refresh_if_due(&self, force: bool) -> StorageHealthSnapshot {
        let (db_path, config, current_snapshot) = {
            let inner = self.inner.read();
            (inner.db_path.clone(), inner.config, inner.snapshot.clone())
        };

        if !config.enabled {
            return current_snapshot;
        }

        let now = now_ms();
        if !force
            && current_snapshot.refreshed_at_ms > 0
            && now.saturating_sub(current_snapshot.refreshed_at_ms)
                < config.refresh_interval_ms as i64
        {
            return current_snapshot;
        }

        let mut next = current_snapshot.clone();
        next.refreshed_at_ms = now;
        if let Some(path) = db_path.as_ref() {
            next.db_path = path.display().to_string();
            next.db_size_bytes = file_size(path);
            next.wal_size_bytes = file_size(&sidecar_path(path, "-wal"));
            next.shm_size_bytes = file_size(&sidecar_path(path, "-shm"));
            if let Some(mount_path) = path.parent().or_else(|| Some(Path::new("."))) {
                let total_space = fs2::total_space(mount_path).unwrap_or_default();
                let available_space = fs2::available_space(mount_path).unwrap_or_default();
                next.disk_free_bytes = available_space;
                if total_space > 0 {
                    next.disk_used_percent =
                        ((total_space - available_space) as f64 / total_space as f64) * 100.0;
                }
            }
        }

        next.warnings = build_storage_warnings(&config, &next);
        next.status = derive_status(&config, &next);
        next.degraded_mode_active =
            config.degraded_mode_enabled && next.disk_used_percent >= config.disk_critical_percent;

        {
            let mut inner = self.inner.write();
            inner.snapshot = next.clone();
            emit_health_log_if_needed(&mut inner, &next);
        }

        next
    }

    pub fn record_runtime_retention(
        &self,
        health: RetentionRunHealth,
        checkpoint: Option<WalCheckpointResult>,
    ) {
        let mut inner = self.inner.write();
        inner.snapshot.last_runtime_retention = Some(health);
        if checkpoint.is_some() {
            inner.snapshot.last_wal_checkpoint = checkpoint;
        }
    }

    pub fn record_contract_whale_retention(
        &self,
        health: RetentionRunHealth,
        checkpoint: Option<WalCheckpointResult>,
    ) {
        let mut inner = self.inner.write();
        inner.snapshot.last_contract_whale_retention = Some(health);
        if checkpoint.is_some() {
            inner.snapshot.last_wal_checkpoint = checkpoint;
        }
    }
}

pub fn load_storage_health_guard_config_from_settings(
    settings: &config::Config,
) -> StorageHealthGuardConfig {
    fn bool_setting(
        settings: &config::Config,
        env_key: &str,
        settings_key: &str,
        default: bool,
    ) -> bool {
        std::env::var(env_key)
            .ok()
            .and_then(|value| parse_bool_text(&value))
            .or_else(|| settings.get_bool(settings_key).ok())
            .unwrap_or(default)
    }

    fn float_setting(
        settings: &config::Config,
        env_key: &str,
        settings_key: &str,
        default: f64,
    ) -> f64 {
        std::env::var(env_key)
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .or_else(|| settings.get_float(settings_key).ok())
            .unwrap_or(default)
    }

    fn int_setting(
        settings: &config::Config,
        env_key: &str,
        settings_key: &str,
        default: u64,
    ) -> u64 {
        std::env::var(env_key)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .or_else(|| {
                settings
                    .get_int(settings_key)
                    .ok()
                    .map(|value| value.max(0) as u64)
            })
            .unwrap_or(default)
    }

    StorageHealthGuardConfig {
        enabled: bool_setting(
            settings,
            "STORAGE_HEALTH_GUARD_ENABLED",
            "storage.health_guard.enabled",
            StorageHealthGuardConfig::default().enabled,
        ),
        db_warn_gb: float_setting(
            settings,
            "STORAGE_HEALTH_DB_WARN_GB",
            "storage.health_guard.db_warn_gb",
            StorageHealthGuardConfig::default().db_warn_gb,
        ),
        db_critical_gb: float_setting(
            settings,
            "STORAGE_HEALTH_DB_CRITICAL_GB",
            "storage.health_guard.db_critical_gb",
            StorageHealthGuardConfig::default().db_critical_gb,
        ),
        wal_warn_gb: float_setting(
            settings,
            "STORAGE_HEALTH_WAL_WARN_GB",
            "storage.health_guard.wal_warn_gb",
            StorageHealthGuardConfig::default().wal_warn_gb,
        ),
        wal_critical_gb: float_setting(
            settings,
            "STORAGE_HEALTH_WAL_CRITICAL_GB",
            "storage.health_guard.wal_critical_gb",
            StorageHealthGuardConfig::default().wal_critical_gb,
        ),
        disk_warn_percent: float_setting(
            settings,
            "STORAGE_HEALTH_DISK_WARN_PERCENT",
            "storage.health_guard.disk_warn_percent",
            StorageHealthGuardConfig::default().disk_warn_percent,
        ),
        disk_critical_percent: float_setting(
            settings,
            "STORAGE_HEALTH_DISK_CRITICAL_PERCENT",
            "storage.health_guard.disk_critical_percent",
            StorageHealthGuardConfig::default().disk_critical_percent,
        ),
        refresh_interval_ms: int_setting(
            settings,
            "STORAGE_HEALTH_REFRESH_INTERVAL_MS",
            "storage.health_guard.refresh_interval_ms",
            StorageHealthGuardConfig::default().refresh_interval_ms,
        ),
        degraded_mode_enabled: bool_setting(
            settings,
            "STORAGE_HEALTH_DEGRADED_MODE_ENABLED",
            "storage.health_guard.degraded_mode_enabled",
            StorageHealthGuardConfig::default().degraded_mode_enabled,
        ),
    }
}

pub fn storage_health_guard_config() -> StorageHealthGuardConfig {
    *storage_health_guard_lock().read()
}

pub fn set_storage_health_guard_config(config: StorageHealthGuardConfig) {
    *storage_health_guard_lock().write() = config;
}

pub fn reset_storage_health_guard_config() {
    set_storage_health_guard_config(StorageHealthGuardConfig::default());
}

pub fn classify_retention_error(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("no such table") {
        "table_missing".to_string()
    } else if lower.contains("no such column") {
        "time_column_missing".to_string()
    } else if lower.contains("database is locked") {
        "database_locked".to_string()
    } else if lower.contains("readonly") {
        "readonly".to_string()
    } else if lower.contains("disk full") {
        "disk_full".to_string()
    } else if lower.contains("busy timeout") || lower.contains("timeout") {
        "timeout".to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn degraded_snapshot_writes() -> Vec<String> {
    vec![
        "flow_snapshots".to_string(),
        "toxic_snapshots".to_string(),
        "venue_health_snapshots".to_string(),
    ]
}

fn storage_health_guard_lock() -> &'static RwLock<StorageHealthGuardConfig> {
    static CONFIG: OnceLock<RwLock<StorageHealthGuardConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(StorageHealthGuardConfig::default()))
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|meta| meta.len())
        .unwrap_or_default()
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(suffix);
    PathBuf::from(os)
}

fn derive_status(
    config: &StorageHealthGuardConfig,
    snapshot: &StorageHealthSnapshot,
) -> StorageHealthStatus {
    if !config.enabled {
        return StorageHealthStatus::Disabled;
    }

    if snapshot.db_size_bytes >= gb_to_bytes(config.db_critical_gb)
        || snapshot.wal_size_bytes >= gb_to_bytes(config.wal_critical_gb)
        || snapshot.disk_used_percent >= config.disk_critical_percent
    {
        StorageHealthStatus::Critical
    } else if snapshot.db_size_bytes >= gb_to_bytes(config.db_warn_gb)
        || snapshot.wal_size_bytes >= gb_to_bytes(config.wal_warn_gb)
        || snapshot.disk_used_percent >= config.disk_warn_percent
    {
        StorageHealthStatus::Warn
    } else {
        StorageHealthStatus::Ok
    }
}

fn build_storage_warnings(
    config: &StorageHealthGuardConfig,
    snapshot: &StorageHealthSnapshot,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if snapshot.db_size_bytes >= gb_to_bytes(config.db_critical_gb) {
        warnings.push("db_size_critical".to_string());
    } else if snapshot.db_size_bytes >= gb_to_bytes(config.db_warn_gb) {
        warnings.push("db_size_warn".to_string());
    }

    if snapshot.wal_size_bytes >= gb_to_bytes(config.wal_critical_gb) {
        warnings.push("wal_size_critical".to_string());
    } else if snapshot.wal_size_bytes >= gb_to_bytes(config.wal_warn_gb) {
        warnings.push("wal_size_warn".to_string());
    }

    if snapshot.disk_used_percent >= config.disk_critical_percent {
        warnings.push("disk_used_critical".to_string());
    } else if snapshot.disk_used_percent >= config.disk_warn_percent {
        warnings.push("disk_used_warn".to_string());
    }
    warnings
}

fn emit_health_log_if_needed(
    inner: &mut StorageHealthTrackerInner,
    snapshot: &StorageHealthSnapshot,
) {
    let signature = (
        snapshot.status,
        snapshot.degraded_mode_active,
        snapshot.warnings.clone(),
    );
    if inner.last_log_signature.as_ref() == Some(&signature)
        && snapshot.status == StorageHealthStatus::Ok
    {
        return;
    }

    inner.last_log_signature = Some(signature);
    match snapshot.status {
        StorageHealthStatus::Critical => tracing::error!(
            db_path = snapshot.db_path,
            db_size_gb = bytes_to_gb(snapshot.db_size_bytes),
            wal_size_gb = bytes_to_gb(snapshot.wal_size_bytes),
            disk_used_percent = snapshot.disk_used_percent,
            degraded_mode_active = snapshot.degraded_mode_active,
            warnings = ?snapshot.warnings,
            "storage_health status=critical"
        ),
        StorageHealthStatus::Warn => tracing::warn!(
            db_path = snapshot.db_path,
            db_size_gb = bytes_to_gb(snapshot.db_size_bytes),
            wal_size_gb = bytes_to_gb(snapshot.wal_size_bytes),
            disk_used_percent = snapshot.disk_used_percent,
            degraded_mode_active = snapshot.degraded_mode_active,
            warnings = ?snapshot.warnings,
            "storage_health status=warn"
        ),
        StorageHealthStatus::Ok if snapshot.degraded_mode_active => tracing::warn!(
            db_path = snapshot.db_path,
            disk_used_percent = snapshot.disk_used_percent,
            disabled_writes = ?snapshot.degraded_writes,
            "storage_guard_degraded_mode enabled"
        ),
        _ => {}
    }
}

fn gb_to_bytes(gb: f64) -> u64 {
    (gb.max(0.0) * 1024.0 * 1024.0 * 1024.0) as u64
}

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn parse_bool_text(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
