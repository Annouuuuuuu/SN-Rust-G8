use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fmt;
use serde::{Serialize, Deserialize};

/// Représente l'état d'un fichier à un instant donné
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileState {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub hash: [u8; 16],
}

/// Type de changement détecté
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "CREATED"),
            Self::Modified => write!(f, "MODIFIED"),
            Self::Deleted => write!(f, "DELETED"),
        }
    }
}

/// Événement de différence détecté
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEvent {
    pub change_type: ChangeType,
    pub file_path: PathBuf,
    pub old_state: Option<FileState>,
    pub new_state: Option<FileState>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DiffEvent {
    /// Crée un nouvel événement
    pub fn new(
        change_type: ChangeType,
        file_path: PathBuf,
        old_state: Option<FileState>,
        new_state: Option<FileState>,
    ) -> Self {
        Self {
            change_type,
            file_path,
            old_state,
            new_state,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Retourne la taille du fichier si disponible
    pub fn file_size(&self) -> Option<u64> {
        self.new_state
            .as_ref()
            .or(self.old_state.as_ref())
            .map(|s| s.size)
    }

    /// Retourne le hash si disponible
    pub fn file_hash(&self) -> Option<&[u8; 16]> {
        self.new_state
            .as_ref()
            .or(self.old_state.as_ref())
            .map(|s| &s.hash)
    }
}

impl fmt::Display for DiffEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {}",
            self.timestamp.format("%H:%M:%S"),
            self.change_type,
            self.file_path.display()
        )
    }
}