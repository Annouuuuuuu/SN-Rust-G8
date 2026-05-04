use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum FileSentinelError {
    #[error("Erreur E/S : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur de configuration : {0}")]
    Config(String),

    #[error("Accès refusé : {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Disque plein : disponible {available} octets, nécessaire {needed} octets")]
    DiskFull { available: u64, needed: u64 },

    #[error("Fichier trop volumineux : {path} ({size} octets, max {max_size} octets)")]
    FileTooLarge { path: PathBuf, size: u64, max_size: u64 },

    #[error("Désaccord de hash : {path}")]
    HashMismatch { path: PathBuf },

    #[error("Erreur de filtre : {0}")]
    Filter(String),

    #[error("Erreur de synchronisation : {0}")]
    Sync(String),

    #[error("Erreur de surveillance : {0}")]
    Watch(String),

    #[error("Erreur de stockage des versions : {0}")]
    VersionStorage(String),

    #[error("Erreur de compression : {0}")]
    Compression(String),

    #[error("Erreur de notification : {0}")]
    Notification(String),

    #[error("Erreur réseau : {0}")]
    Network(String),

    #[error("Non trouvé : {0}")]
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
            Self::Warning => write!(f, "AVERTISSEMENT"),
            Self::Error => write!(f, "ERREUR"),
            Self::Critical => write!(f, "CRITIQUE"),
        }
    }
}

pub type Result<T> = std::result::Result<T, FileSentinelError>;