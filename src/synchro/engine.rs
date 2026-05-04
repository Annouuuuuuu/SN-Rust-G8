use crate::errors::{FileSentinelError, Result};
use crate::watcher::types::{ChangeType, DiffEvent};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use log::{info, warn, error};
use serde::{Serialize, Deserialize};

/// Statistiques de synchronisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub files_copied: u32,
    pub files_created: u32,
    pub files_deleted: u32,
    pub files_skipped: u32,
    pub total_bytes_transferred: u64,
    pub duration_ms: u128,
    pub errors: Vec<String>,
}

impl SyncStats {
    pub fn new() -> Self {
        Self {
            files_copied: 0,
            files_created: 0,
            files_deleted: 0,
            files_skipped: 0,
            total_bytes_transferred: 0,
            duration_ms: 0,
            errors: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: &SyncStats) {
        self.files_copied += other.files_copied;
        self.files_created += other.files_created;
        self.files_deleted += other.files_deleted;
        self.files_skipped += other.files_skipped;
        self.total_bytes_transferred += other.total_bytes_transferred;
        self.duration_ms += other.duration_ms;
        self.errors.extend(other.errors.clone());
    }
}

impl std::fmt::Display for SyncStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Sync Statistics:")?;
        writeln!(f, "  Files copied:   {}", self.files_copied)?;
        writeln!(f, "  Files created:  {}", self.files_created)?;
        writeln!(f, "  Files deleted:  {}", self.files_deleted)?;
        writeln!(f, "  Files skipped:  {}", self.files_skipped)?;
        writeln!(
            f,
            "  Data transferred: {} MB",
            self.total_bytes_transferred as f64 / 1_000_000.0
        )?;
        writeln!(f, "  Duration: {}ms", self.duration_ms)?;
        if !self.errors.is_empty() {
            writeln!(f, "  Errors: {}", self.errors.len())?;
            // Correction: enlever le & devant self.errors.iter()
            for error in self.errors.iter().take(5) {
                writeln!(f, "    - {}", error)?;
            }
            if self.errors.len() > 5 {
                writeln!(f, "    ... and {} more", self.errors.len() - 5)?;
            }
        }
        Ok(())
    }
}

pub struct SyncEngine {
    source_root: PathBuf,
    dest_root: PathBuf,
}

impl SyncEngine {
    pub fn new<P: AsRef<Path>>(source: P, dest: P) -> Self {
        Self {
            source_root: source.as_ref().to_path_buf(),
            dest_root: dest.as_ref().to_path_buf(),
        }
    }

    /// Résout le chemin de destination pour un fichier source
    fn resolve_dest_path(&self, source_path: &Path) -> Result<PathBuf> {
        let relative = source_path.strip_prefix(&self.source_root).map_err(|e| {
            FileSentinelError::Sync(format!(
                "Impossible de calculer le chemin relatif pour {} : {}",
                source_path.display(),
                e
            ))
        })?;
        Ok(self.dest_root.join(relative))
    }

    /// Vérifie l'espace disque disponible (version simplifiée)
    fn check_disk_space(&self, _needed: u64) -> Result<()> {
        // Dans une version réelle, utiliser fs2 ou équivalent
        // Pour l'instant, on suppose qu'il y a assez d'espace
        Ok(())
    }

    /// Synchronise un seul événement
    pub fn sync_event(&mut self, event: &DiffEvent) -> Result<SyncStats> {
        let start = Instant::now();
        let mut stats = SyncStats::new();

        match event.change_type {
            ChangeType::Created | ChangeType::Modified => {
                let dest_path = self.resolve_dest_path(&event.file_path)?;

                // Créer le répertoire parent si nécessaire
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        FileSentinelError::Sync(format!(
                            "Impossible de créer le répertoire {} : {}",
                            parent.display(),
                            e
                        ))
                    })?;
                }

                // Vérifier l'espace disque
                if let Some(size) = event.file_size() {
                    self.check_disk_space(size)?;
                }

                // Copier le fichier
                match fs::copy(&event.file_path, &dest_path) {
                    Ok(bytes) => {
                        stats.files_copied += 1;
                        stats.total_bytes_transferred += bytes;

                        if event.change_type == ChangeType::Created {
                            stats.files_created += 1;
                        }

                        info!(
                            "Synchronisé : {} -> {} ({} octets)",
                            event.file_path.display(),
                            dest_path.display(),
                            bytes
                        );
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "Impossible de copier {} : {}",
                            event.file_path.display(),
                            e
                        );
                        error!("{}", error_msg);
                        stats.errors.push(error_msg);
                    }
                }
            }
            ChangeType::Deleted => {
                let dest_path = self.resolve_dest_path(&event.file_path)?;

                if dest_path.exists() {
                    match fs::remove_file(&dest_path) {
                        Ok(_) => {
                            stats.files_deleted += 1;
                            info!("Supprimé : {}", dest_path.display());
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Impossible de supprimer {} : {}",
                                dest_path.display(),
                                e
                            );
                            warn!("{}", error_msg);
                            stats.errors.push(error_msg);
                        }
                    }
                } else {
                    stats.files_skipped += 1;
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis();
        Ok(stats)
    }

    /// Synchronisation complète du répertoire
    pub fn full_sync(&mut self) -> Result<SyncStats> {
        let start = Instant::now();
        let mut stats = SyncStats::new();

        info!(
            "Début de la synchronisation complète : {} -> {}",
            self.source_root.display(),
            self.dest_root.display()
        );

        // Créer le répertoire de destination si nécessaire
        fs::create_dir_all(&self.dest_root)?;

        self.sync_directory(&self.source_root, &mut stats)?;

        stats.duration_ms = start.elapsed().as_millis();
        info!("Synchronisation complète terminée en {}ms", stats.duration_ms);

        Ok(stats)
    }

    fn sync_directory(&self, source: &Path, stats: &mut SyncStats) -> Result<()> {
        if !source.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let dest_path = self.resolve_dest_path(&path)?;

                // Créer le répertoire parent
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let should_copy = match fs::metadata(&dest_path) {
                    Ok(dest_meta) => {
                        let source_meta = fs::metadata(&path)?;
                        source_meta.len() != dest_meta.len()
                            || source_meta.modified()? != dest_meta.modified()?
                    }
                    Err(_) => true,
                };

                if should_copy {
                    match fs::copy(&path, &dest_path) {
                        Ok(bytes) => {
                            stats.files_copied += 1;
                            if !dest_path.exists() {
                                stats.files_created += 1;
                            }
                            stats.total_bytes_transferred += bytes;
                        }
                        Err(e) => {
                            stats.errors.push(format!(
                                "Erreur de copie {} : {}",
                                path.display(),
                                e
                            ));
                        }
                    }
                } else {
                    stats.files_skipped += 1;
                }
            } else if path.is_dir() {
                let dest_path = self.resolve_dest_path(&path)?;
                fs::create_dir_all(&dest_path)?;
                self.sync_directory(&path, stats)?;
            }
        }

        Ok(())
    }
}