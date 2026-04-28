use crate::errors::{FileSentinelError, Result};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use log::{info, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub original_path: PathBuf,
    pub version_number: u32,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub size: u64,
    pub hash: String,
    pub storage_path: PathBuf,
}

impl FileVersion {
    pub fn format_size(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        }
    }
}

pub struct VersionManager {
    versions_dir: PathBuf,
    max_versions: u32,
    versions_index: HashMap<PathBuf, Vec<FileVersion>>,
}

impl VersionManager {
    pub fn new<P: AsRef<Path>>(base_dir: P, max_versions: u32) -> Result<Self> {
        let versions_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&versions_dir)?;

        let mut manager = Self {
            versions_dir,
            max_versions,
            versions_index: HashMap::new(),
        };

        manager.load_index()?;
        info!(
            "Version manager initialized: {} versions max",
            manager.max_versions
        );

        Ok(manager)
    }

    /// Charge l'index des versions existantes
    fn load_index(&mut self) -> Result<()> {
        let index_path = self.versions_dir.join("versions_index.json");

        if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            self.versions_index = serde_json::from_str(&content).map_err(|e| {
                FileSentinelError::VersionStorage(format!("Failed to parse version index: {}", e))
            })?;

            debug!(
                "Loaded version index: {} files tracked",
                self.versions_index.len()
            );
        }

        Ok(())
    }

    /// Sauvegarde l'index des versions
    fn save_index(&self) -> Result<()> {
        let index_path = self.versions_dir.join("versions_index.json");
        let content = serde_json::to_string_pretty(&self.versions_index).map_err(|e| {
            FileSentinelError::VersionStorage(format!("Failed to serialize version index: {}", e))
        })?;

        fs::write(&index_path, content)?;
        Ok(())
    }

    /// Sauvegarde une nouvelle version d'un fichier
    pub fn save_version<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        hash: &[u8; 16],
    ) -> Result<FileVersion> {
        let file_path = file_path.as_ref();

        if !file_path.exists() {
            return Err(FileSentinelError::NotFound(format!(
                "Cannot version non-existent file: {}",
                file_path.display()
            )));
        }

        // Obtenir les métadonnées
        let metadata = fs::metadata(file_path)?;

        // Créer le nom de la version
        let timestamp = Utc::now();
        let version_filename = format!(
            "{}_{}",
            timestamp.format("%Y%m%d_%H%M%S_%3f"),
            file_path.file_name().unwrap().to_string_lossy()
        );

        let version_path = self.versions_dir.join(&version_filename);

        // Copier le fichier
        fs::copy(file_path, &version_path)?;

        // Déterminer le numéro de version
        let versions = self.versions_index
            .entry(file_path.to_path_buf())
            .or_insert_with(Vec::new);

        let version_number = (versions.len() as u32) + 1;

        // Créer l'entrée de version
        let version = FileVersion {
            original_path: file_path.to_path_buf(),
            version_number,
            timestamp,
            size: metadata.len(),
            hash: hex::encode(hash),
            storage_path: version_path,
        };

        versions.push(version.clone());

        // Nettoyer les anciennes versions si nécessaire
        if versions.len() > self.max_versions as usize {
            // Trier par timestamp (le plus ancien d'abord)
            versions.sort_by_key(|v| v.timestamp);
            
            // Supprimer l'excédent de versions
            while versions.len() > self.max_versions as usize {
                if let Some(old_version) = versions.first() {
                    let old_path = old_version.storage_path.clone();
                    if old_path.exists() {
                        fs::remove_file(&old_path)?;
                        debug!("Removed old version: {}", old_path.display());
                    }
                    versions.remove(0);
                }
            }
        }

        // Sauvegarder l'index
        self.save_index()?;

        info!(
            "Version {} saved for {} ({})",
            version_number,
            file_path.display(),
            version.format_size()
        );

        Ok(version)
    }

    /// Récupère toutes les versions d'un fichier
    pub fn get_versions<P: AsRef<Path>>(&self, file_path: P) -> Vec<&FileVersion> {
        self.versions_index
            .get(file_path.as_ref())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Restaure une version spécifique
    pub fn restore_version<P: AsRef<Path>>(
        &self,
        file_path: P,
        version_number: u32,
    ) -> Result<()> {
        let file_path = file_path.as_ref();

        let versions = self.versions_index.get(file_path).ok_or_else(|| {
            FileSentinelError::VersionStorage(format!(
                "No versions found for: {}",
                file_path.display()
            ))
        })?;

        let version = versions
            .iter()
            .find(|v| v.version_number == version_number)
            .ok_or_else(|| {
                FileSentinelError::VersionStorage(format!(
                    "Version {} not found for: {}",
                    version_number,
                    file_path.display()
                ))
            })?;

        // Restaurer le fichier
        fs::copy(&version.storage_path, file_path)?;

        info!(
            "Restored version {} of {}",
            version_number,
            file_path.display()
        );

        Ok(())
    }

    /// Obtient des statistiques globales
    pub fn get_stats(&self) -> VersionStats {
        let mut total_versions = 0u64;
        let mut total_size = 0u64;

        for versions in self.versions_index.values() {
            total_versions += versions.len() as u64;
            total_size += versions.iter().map(|v| v.size).sum::<u64>();
        }

        VersionStats {
            total_files_tracked: self.versions_index.len() as u64,
            total_versions,
            total_size_bytes: total_size,
            versions_dir: self.versions_dir.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionStats {
    pub total_files_tracked: u64,
    pub total_versions: u64,
    pub total_size_bytes: u64,
    pub versions_dir: PathBuf,
}

impl std::fmt::Display for VersionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Version Statistics:")?;
        writeln!(f, "  Files tracked: {}", self.total_files_tracked)?;
        writeln!(f, "  Total versions: {}", self.total_versions)?;
        writeln!(
            f,
            "  Total size: {:.1} MB",
            self.total_size_bytes as f64 / 1_000_000.0
        )?;
        writeln!(f, "  Storage directory: {}", self.versions_dir.display())?;
        Ok(())
    }
}