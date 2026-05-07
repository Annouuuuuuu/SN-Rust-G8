pub mod types;
pub mod polling;

use std::path::Path;
use crate::errors::Result;
use self::types::DiffEvent;

/// Trait commun pour tous les watchers
pub trait Watcher {
    /// Commence à surveiller un répertoire
    fn watch(&mut self, path: &Path) -> Result<()>;
    
    /// Arrête de surveiller un répertoire
    #[allow(dead_code)]
    fn unwatch(&mut self, path: &Path) -> Result<()>;
    
    /// Récupère les événements détectés
    fn events(&mut self) -> Result<Vec<DiffEvent>>;

}