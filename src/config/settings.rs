use crate::errors::{FileSentinelError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub watch: WatchConfig,
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub filters: FiltersConfig,
    #[serde(default)]
    pub reporting: ReportingConfig,
    #[serde(default)]
    pub versioning: VersioningConfig,
    #[serde(default)]
    pub compression: CompressionConfig,
    #[serde(default)]
    pub notifications: NotificationsConfig,
    #[serde(default)]
    pub network: Option<NetworkConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct WatchConfig {
    #[serde(default = "default_directories")]
    pub directories: Vec<String>,
    #[serde(default = "default_polling_interval")]
    pub polling_interval_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SyncConfig {
    #[serde(default = "default_destination")]
    pub destination: String,
    #[serde(default)]
    pub create_backups: bool,
    #[serde(default = "default_concurrent_ops")]
    pub max_concurrent_operations: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FiltersConfig {
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    pub max_file_size_mb: Option<u64>,
    #[serde(default)]
    pub include_extensions: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ReportingConfig {
    #[serde(default = "default_true")]
    pub show_progress: bool,
    pub log_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct VersioningConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_versions")]
    pub max_versions: u32,
    #[serde(default = "default_versions_dir")]
    pub versions_dir: PathBuf,
    #[serde(default = "default_true")]
    pub auto_version_on_change: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CompressionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_compression_level")]
    pub level: u32,
    #[serde(default = "default_min_size")]
    pub min_file_size_for_compression: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct NotificationsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub show_batch_summary: bool,
    #[serde(default = "default_min_interval")]
    pub min_interval_seconds: u64,
    #[serde(default = "default_critical_patterns")]
    pub critical_patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub key_path: Option<PathBuf>,
    pub remote_path: PathBuf,
    #[serde(default = "default_rsync_options")]
    pub rsync_options: Vec<String>,
    pub auto_sync_interval_minutes: Option<u64>,
}

// Valeurs par défaut
fn default_directories() -> Vec<String> {
    vec![".".to_string()]
}

fn default_polling_interval() -> u64 {
    1000
}

fn default_destination() -> String {
    "./sync_dest".to_string()
}

fn default_concurrent_ops() -> usize {
    4
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "**/.git/**".to_string(),
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/*.tmp".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_max_versions() -> u32 {
    5
}

fn default_versions_dir() -> PathBuf {
    PathBuf::from(".versions")
}

fn default_compression_level() -> u32 {
    6
}

fn default_min_size() -> u64 {
    1024
}

fn default_min_interval() -> u64 {
    5
}

fn default_critical_patterns() -> Vec<String> {
    vec![
        "*.conf".to_string(),
        "*.env".to_string(),
        "*.toml".to_string(),
        "*.lock".to_string(),
    ]
}

fn default_port() -> u16 {
    22
}

fn default_rsync_options() -> Vec<String> {
    vec!["-avz".to_string(), "--progress".to_string(), "--partial".to_string()]
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            FileSentinelError::Config(format!("Cannot read config file '{}': {}", path, e))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            FileSentinelError::Config(format!("Cannot parse config file: {}", e))
        })?;

        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let toml_string = toml::to_string_pretty(self).map_err(|e| {
            FileSentinelError::Config(format!("Cannot serialize config: {}", e))
        })?;

        fs::write(path, toml_string).map_err(|e| {
            FileSentinelError::Config(format!("Cannot write config file: {}", e))
        })?;

        Ok(())
    }
}