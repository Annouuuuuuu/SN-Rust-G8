use crate::errors::{FileSentinelError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            watch: WatchConfig::default(),
            sync: SyncConfig::default(),
            filters: FiltersConfig::default(),
            reporting: ReportingConfig::default(),
            versioning: VersioningConfig::default(),
            compression: CompressionConfig::default(),
            notifications: NotificationsConfig::default(),
            network: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WatchConfig {
    #[serde(default = "default_directories")]
    pub directories: Vec<String>,
    #[serde(default = "default_polling_interval")]
    pub polling_interval_ms: u64,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            directories: default_directories(),
            polling_interval_ms: default_polling_interval(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SyncConfig {
    #[serde(default = "default_destination")]
    pub destination: String,
    #[serde(default = "default_true")]
    pub create_backups: bool,
    #[serde(default = "default_concurrent_ops")]
    pub max_concurrent_operations: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            destination: default_destination(),
            create_backups: true,
            max_concurrent_operations: default_concurrent_ops(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiltersConfig {
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    pub max_file_size_mb: Option<u64>,
    #[serde(default)]
    pub include_extensions: Vec<String>,
}

impl Default for FiltersConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: default_exclude_patterns(),
            max_file_size_mb: Some(100),
            include_extensions: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportingConfig {
    #[serde(default = "default_true")]
    pub show_progress: bool,
    pub log_file: Option<String>,
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            show_progress: true,
            log_file: Some(String::from("C:\\SAUVEGARDE\\filesentinel.log")),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_versions: 10,
            versions_dir: PathBuf::from("C:\\SAUVEGARDE\\.versions"),
            auto_version_on_change: true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompressionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_compression_level")]
    pub level: u32,
    #[serde(default = "default_min_size")]
    pub min_file_size_for_compression: u64,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: 6,
            min_file_size_for_compression: 10240,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_batch_summary: true,
            min_interval_seconds: 15,
            critical_patterns: default_critical_patterns(),
        }
    }
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

// ============================================
// VALEURS PAR DÉFAUT - WINDOWS
// ============================================

fn default_directories() -> Vec<String> {
    let username = whoami::username();
    vec![
        format!("C:\\Users\\{}\\Documents", username),
        format!("C:\\Users\\{}\\Desktop", username)
    ]
}

fn default_polling_interval() -> u64 {
    500
}

fn default_destination() -> String {
    "C:\\SAUVEGARDE".to_string()
}

fn default_concurrent_ops() -> usize {
    4
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        // Système Windows
        "**/AppData/**".to_string(),
        "**/Application Data/**".to_string(),
        "**/Cookies/**".to_string(),
        "**/Local Settings/**".to_string(),
        "**/Recent/**".to_string(),
        "**/SendTo/**".to_string(),
        "**/Start Menu/**".to_string(),
        "**/Templates/**".to_string(),
        "**/NTUSER.DAT*".to_string(),
        "**/ntuser.dat*".to_string(),
        "**/ntuser.ini".to_string(),
        "**/desktop.ini".to_string(),
        "**/Thumbs.db".to_string(),
        "**/$RECYCLE.BIN/**".to_string(),
        "**/System Volume Information/**".to_string(),
        // Fichiers temporaires
        "**/*.tmp".to_string(),
        "**/*.temp".to_string(),
        "**/~$*".to_string(),
        "**/*.swp".to_string(),
        "**/*.swo".to_string(),
        "**/*~".to_string(),
        // Cache navigateurs
        "**/Google/Chrome/User Data/*/Cache/**".to_string(),
        "**/Mozilla/Firefox/Profiles/*/cache2/**".to_string(),
        "**/MicrosoftEdge/User Data/*/Cache/**".to_string(),
        // OneDrive
        "**/OneDrive/**".to_string(),
        // Développement
        "**/node_modules/**".to_string(),
        "**/.git/**".to_string(),
        "**/target/**".to_string(),
        "**/vendor/**".to_string(),
        "**/__pycache__/**".to_string(),
        "**/*.pyc".to_string(),
        "**/build/**".to_string(),
        "**/dist/**".to_string(),
        // Logs
        "**/*.log".to_string(),
        "**/logs/**".to_string(),
        // IDE
        "**/.idea/**".to_string(),
        "**/.vscode/**".to_string(),
        "**/*.sublime-workspace".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_max_versions() -> u32 {
    10
}

fn default_versions_dir() -> PathBuf {
    PathBuf::from("C:\\SAUVEGARDE\\.versions")
}

fn default_compression_level() -> u32 {
    6
}

fn default_min_size() -> u64 {
    10240
}

fn default_min_interval() -> u64 {
    15
}

fn default_critical_patterns() -> Vec<String> {
    vec![
        "*.docx".to_string(),
        "*.xlsx".to_string(),
        "*.pptx".to_string(),
        "*.pdf".to_string(),
        "*.odt".to_string(),
        "*.ods".to_string(),
        "*.odp".to_string(),
        "*.pst".to_string(),
        "*.ost".to_string(),
        "*.db".to_string(),
        "*.sqlite".to_string(),
        "*.sql".to_string(),
        "*.conf".to_string(),
        "*.config".to_string(),
        "*.ini".to_string(),
        "*.xml".to_string(),
        "*.json".to_string(),
        "*.yaml".to_string(),
        "*.yml".to_string(),
        "*.env".to_string(),
        "*.toml".to_string(),
        "*.cfg".to_string(),
        "*.lock".to_string(),
        "*.key".to_string(),
        "*.pem".to_string(),
        "*.cert".to_string(),
        "*.crt".to_string(),
        "*.p12".to_string(),
        "*.pfx".to_string(),
        "*.zip".to_string(),
        "*.tar".to_string(),
        "*.gz".to_string(),
        "*.7z".to_string(),
        "*.rar".to_string(),
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
            FileSentinelError::Config(format!("Impossible de lire le fichier de configuration '{}' : {}", path, e))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            FileSentinelError::Config(format!("Impossible de parser le fichier de configuration : {}", e))
        })?;

        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let toml_string = toml::to_string_pretty(self).map_err(|e| {
            FileSentinelError::Config(format!("Impossible de sérialiser la configuration : {}", e))
        })?;

        fs::write(path, toml_string).map_err(|e| {
            FileSentinelError::Config(format!("Impossible d'écrire le fichier de configuration : {}", e))
        })?;

        Ok(())
    }
}

// ============================================
// CONFIGURATION LINUX (remplacez les fonctions ci-dessus)
// ============================================

// fn default_directories() -> Vec<String> {
//     let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
//     vec![
//         format!("{}/Documents", home),
//         format!("{}/Desktop", home),
//         format!("{}/Downloads", home),
//         format!("{}/Pictures", home),
//         format!("{}/Music", home),
//         format!("{}/Videos", home),
//         format!("{}/Templates", home),
//         format!("{}/Public", home),
//     ]
// }

// fn default_destination() -> String {
//     let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
//     format!("{}/SAUVEGARDE", home)
// }

// fn default_versions_dir() -> PathBuf {
//     let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
//     PathBuf::from(format!("{}/SAUVEGARDE/.versions", home))
// }

// fn default_exclude_patterns() -> Vec<String> {
//     vec![
//         "**/.cache/**".to_string(),
//         "**/.config/**".to_string(),
//         "**/.local/**".to_string(),
//         "**/.mozilla/**".to_string(),
//         "**/.thunderbird/**".to_string(),
//         "**/.ssh/**".to_string(),
//         "**/.gnupg/**".to_string(),
//         "**/snap/**".to_string(),
//         "**/Trash/**".to_string(),
//         "**/.Trash-*/**".to_string(),
//         "**/lost+found/**".to_string(),
//         "**/proc/**".to_string(),
//         "**/sys/**".to_string(),
//         "**/dev/**".to_string(),
//         "**/run/**".to_string(),
//         "**/tmp/**".to_string(),
//         "**/var/cache/**".to_string(),
//         "**/var/log/**".to_string(),
//         "**/var/tmp/**".to_string(),
//         "**/*.tmp".to_string(),
//         "**/*.temp".to_string(),
//         "**/*.swp".to_string(),
//         "**/*.swo".to_string(),
//         "**/*~".to_string(),
//         "**/node_modules/**".to_string(),
//         "**/.git/**".to_string(),
//         "**/target/**".to_string(),
//         "**/vendor/**".to_string(),
//         "**/__pycache__/**".to_string(),
//         "**/*.pyc".to_string(),
//         "**/build/**".to_string(),
//         "**/dist/**".to_string(),
//         "**/*.log".to_string(),
//         "**/logs/**".to_string(),
//         "**/.idea/**".to_string(),
//         "**/.vscode/**".to_string(),
//         "**/*.sublime-workspace".to_string()
//     ]
// }

// fn default_polling_interval() -> u64 {
//     100
// }