use crate::watcher::{Watcher, types::*};
use crate::errors::{FileSentinelError, Result};
use inotify::{Inotify, WatchMask};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct InotifyWatcher {
    inotify: Inotify,
    watches: HashMap<inotify::WatchDescriptor, PathBuf>,
}

impl InotifyWatcher {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inotify: Inotify::init().map_err(|e| FileSentinelError::Generic(e.to_string()))?,
            watches: HashMap::new(),
        })
    }
}

impl Watcher for InotifyWatcher {
    fn watch(&mut self, path: &Path) -> Result<()> {
        let path_buf = path.to_path_buf();
        let wd = self.inotify
            .watches()
            .add(
                &path_buf,
                WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE | WatchMask::MOVED_TO | WatchMask::MOVED_FROM,
            )
            .map_err(|e| FileSentinelError::Io(e))?;
        
        self.watches.insert(wd, path_buf);
        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> Result<()> {
        let path_buf = path.to_path_buf();
        if let Some((&wd, _)) = self.watches.iter().find(|(_, p)| **p == path_buf) {
            self.inotify.watches().remove(wd).map_err(|e| FileSentinelError::Io(e))?;
            self.watches.remove(&wd);
        }
        Ok(())
    }

    fn events(&mut self) -> Result<Vec<DiffEvent>> {
        let mut events = Vec::new();
        let mut buffer = [0; 4096];
        
        match self.inotify.read_events(&mut buffer) {
            Ok(event_reader) => {
                for event in event_reader {
                    if let Some(base_path) = self.watches.get(&event.wd) {
                        let full_path = if let Some(name) = event.name {
                            base_path.join(name)
                        } else {
                            base_path.clone()
                        };

                        let change_type = if event.mask.contains(inotify::EventMask::CREATE) 
                            || event.mask.contains(inotify::EventMask::MOVED_TO) {
                            ChangeType::Created
                        } else if event.mask.contains(inotify::EventMask::DELETE)
                            || event.mask.contains(inotify::EventMask::MOVED_FROM) {
                            ChangeType::Deleted
                        } else if event.mask.contains(inotify::EventMask::MODIFY) {
                            ChangeType::Modified
                        } else {
                            continue; // Ignorer les autres événements
                        };

                        let new_state = if change_type == ChangeType::Created || change_type == ChangeType::Modified {
                            // Essayer de lire l'état du fichier
                            match std::fs::metadata(&full_path) {
                                Ok(metadata) => {
                                    match self.compute_file_state(&full_path, &metadata) {
                                        Ok(state) => Some(state),
                                        Err(_) => None,
                                    }
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        };

                        events.push(DiffEvent::new(
                            change_type,
                            full_path,
                            None, // Pour inotify, on ne garde pas l'ancien état
                            new_state,
                        ));
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Aucun événement disponible, c'est normal
            }
            Err(e) => return Err(FileSentinelError::Io(e)),
        }
        Ok(events)
    }

    /// Calcule l'état d'un fichier (méthode helper)
    fn compute_file_state(&self, path: &Path, metadata: &std::fs::Metadata) -> Result<FileState> {
        use std::io::Read;
        use md5::{Md5, Digest};

        let mut file = std::fs::File::open(path)?;
        let mut hasher = Md5::new();
        let mut buffer = [0; 8192];

        while let Ok(count) = file.read(&mut buffer) {
            if count == 0 { break; }
            hasher.update(&buffer[..count]);
        }

        let mut hash = [0u8; 16];
        hash.copy_from_slice(&hasher.finalize());

        Ok(FileState {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified()?,
            hash,
        })
    }
}
