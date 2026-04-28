use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum FileSentinelError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Disk full: available {available} bytes, needed {needed} bytes")]
    DiskFull { available: u64, needed: u64 },

    #[error("File too large: {path} ({size} bytes, max {max_size} bytes)")]
    FileTooLarge { path: PathBuf, size: u64, max_size: u64 },

    #[error("Hash mismatch: {path}")]
    HashMismatch { path: PathBuf },

    #[error("Filter error: {0}")]
    Filter(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Watch error: {0}")]
    Watch(String),

    #[error("Version storage error: {0}")]
    VersionStorage(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Notification error: {0}")]
    Notification(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Generic(String),
}

impl FileSentinelError {
    /// Détermine si l'erreur est récupérable (peut être réessayée)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Io(e) if e.kind() == std::io::ErrorKind::Interrupted
                || e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::WouldBlock
        )
    }

    /// Retourne la sévérité de l'erreur
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::PermissionDenied { .. } | Self::DiskFull { .. } => ErrorSeverity::Critical,
            Self::HashMismatch { .. } => ErrorSeverity::Warning,
            Self::FileTooLarge { .. } => ErrorSeverity::Info,
            Self::Io(_) => ErrorSeverity::Error,
            _ => ErrorSeverity::Warning,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

pub type Result<T> = std::result::Result<T, FileSentinelError>;