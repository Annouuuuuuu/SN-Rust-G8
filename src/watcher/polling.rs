use super::{Watcher, types::*};
use crate::errors::{FileSentinelError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{BufReader, Read};
use md5::{Md5, Digest};
use walkdir::WalkDir;
use log::{debug, info, warn};

pub struct PollingWatcher {
    watched_dirs: Vec<PathBuf>,
    previous_state: HashMap<PathBuf, FileState>,
}

impl PollingWatcher {
    pub fn new() -> Self {
        Self {
            watched_dirs: Vec::new(),
            previous_state: HashMap::new(),
        }
    }

    /// Calcule le hash MD5 d'un fichier
    fn compute_md5(&self, path: &Path) -> Result<[u8; 16]> {
        let file = fs::File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileSentinelError::PermissionDenied {
                    path: path.to_path_buf(),
                }
            } else {
                FileSentinelError::Io(e)
            }
        })?;

        let mut reader = BufReader::new(file);
        let mut hasher = Md5::new();
        let mut buffer = [0; 8192]; // Buffer de 8KB

        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 { break; }
            hasher.update(&buffer[..count]);
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 16];
        hash.copy_from_slice(&result);
        Ok(hash)
    }

    /// Scan récursif d'un répertoire
    fn scan_directory(&self, dir: &Path) -> Result<Vec<FileState>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();
            
            if path.is_file() {
                match fs::metadata(&path) {
                    Ok(metadata) => {
                        match self.compute_md5(&path) {
                            Ok(hash) => {
                                match metadata.modified() {
                                    Ok(modified) => {
                                        files.push(FileState {
                                            path,
                                            size: metadata.len(),
                                            modified,
                                            hash,
                                        });
                                    }
                                    Err(e) => {
                                        warn!("Cannot get modified time for {}: {}", path.display(), e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Cannot hash {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Cannot read metadata for {}: {}", path.display(), e);
                    }
                }
            }
        }

        debug!("Scanned {} files in {}", files.len(), dir.display());
        Ok(files)
    }

    /// Compare l'ancien état et le nouvel état pour détecter les changements
    fn compare_states(
        &self,
        old_state: &HashMap<PathBuf, FileState>,
        new_state: Vec<FileState>,
    ) -> Vec<DiffEvent> {
        let mut events = Vec::new();
        let now = chrono::Utc::now();

        // Convertir en HashMap pour une recherche rapide
        let new_state_map: HashMap<&PathBuf, &FileState> = new_state
            .iter()
            .map(|fs| (&fs.path, fs))
            .collect();

        // Détecter les suppressions et modifications
        for (path, old_file) in old_state {
            match new_state_map.get(path) {
                Some(new_file) => {
                    // Vérifier si le fichier a changé
                    if old_file.hash != new_file.hash 
                        || old_file.size != new_file.size 
                    {
                        info!("File modified: {}", path.display());
                        events.push(DiffEvent::new(
                            ChangeType::Modified,
                            path.clone(),
                            Some(old_file.clone()),
                            Some((*new_file).clone()),
                        ));
                    }
                }
                None => {
                    info!("File deleted: {}", path.display());
                    events.push(DiffEvent::new(
                        ChangeType::Deleted,
                        path.clone(),
                        Some(old_file.clone()),
                        None,
                    ));
                }
            }
        }

        // Détecter les créations
        for new_file in &new_state {
            if !old_state.contains_key(&new_file.path) {
                info!("File created: {}", new_file.path.display());
                events.push(DiffEvent::new(
                    ChangeType::Created,
                    new_file.path.clone(),
                    None,
                    Some(new_file.clone()),
                ));
            }
        }

        events
    }
}

impl Watcher for PollingWatcher {
    fn watch(&mut self, path: &Path) -> Result<()> {
        let path = path.to_path_buf();

        if !path.exists() {
            return Err(FileSentinelError::NotFound(format!(
                "Directory not found: {}",
                path.display()
            )));
        }

        if !path.is_dir() {
            return Err(FileSentinelError::Generic(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        if !self.watched_dirs.contains(&path) {
            info!("Starting to watch: {}", path.display());
            
            // Scanner l'état initial
            let initial_state = self.scan_directory(&path)?;
            for file in initial_state {
                self.previous_state.insert(file.path.clone(), file);
            }

            self.watched_dirs.push(path);
        }

        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> Result<()> {
        let path = path.to_path_buf();
        info!("Stopping watch: {}", path.display());
        self.watched_dirs.retain(|p| p != &path);
        Ok(())
    }

    fn events(&mut self) -> Result<Vec<DiffEvent>> {
        let mut all_events = Vec::new();

        for dir in self.watched_dirs.clone() {
            match self.scan_directory(&dir) {
                Ok(current_state) => {
                    let events = self.compare_states(&self.previous_state, current_state.clone());

                    // Mettre à jour l'état précédent
                    let new_state: HashMap<PathBuf, FileState> = current_state
                        .into_iter()
                        .map(|fs| (fs.path.clone(), fs))
                        .collect();

                    self.previous_state = new_state;
                    all_events.extend(events);
                }
                Err(e) => {
                    warn!("Error scanning {}: {}", dir.display(), e);
                }
            }
        }

        Ok(all_events)
    }
}