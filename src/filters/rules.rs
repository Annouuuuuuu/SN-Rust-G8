use crate::errors::{FileSentinelError, Result};
use crate::watcher::types::DiffEvent;
use glob::Pattern;
use std::path::Path;
use log::debug;

#[derive(Debug, Clone)]
pub struct FilterRule {
    pub name: String,
    pattern: Option<Pattern>,
    max_size: Option<u64>,
    extensions: Vec<String>,
    exclude: bool,
}

impl FilterRule {
    pub fn new_glob_pattern(name: &str, pattern: &str, exclude: bool) -> Result<Self> {
        let pattern = Pattern::new(pattern).map_err(|e| {
            FileSentinelError::Filter(format!("Modèle glob invalide '{}' : {}", pattern, e))
        })?;

        Ok(Self {
            name: name.to_string(),
            pattern: Some(pattern),
            max_size: None,
            extensions: Vec::new(),
            exclude,
        })
    }

    pub fn new_size_limit(max_bytes: u64) -> Self {
        Self {
            name: format!("SizeLimit({}B)", max_bytes),
            pattern: None,
            max_size: Some(max_bytes),
            extensions: Vec::new(),
            exclude: true,
        }
    }

    pub fn new_extension_filter(extensions: Vec<String>, exclude: bool) -> Self {
        Self {
            name: format!("Extensions({:?})", extensions),
            pattern: None,
            max_size: None,
            extensions,
            exclude,
        }
    }

    /// Vérifie si la règle correspond au chemin/fichier donné
    pub fn matches(&self, path: &Path, file_size: Option<u64>) -> bool {
        // Vérifier le pattern glob
        if let Some(pattern) = &self.pattern {
            if pattern.matches_path(path) {
                debug!("Correspondance de règle '{}' pour le chemin : {}", self.name, path.display());
                return true;
            }
        }

        // Vérifier la taille
        if let Some(max_size) = self.max_size {
            if let Some(size) = file_size {
                if size > max_size {
                    debug!(
                        "Correspondance de règle '{}' pour la taille : {} > {}",
                        self.name, size, max_size
                    );
                    return true;
                }
            }
        }

        // Vérifier l'extension
        if !self.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if self.extensions.iter().any(|e| e == ext) {
                    debug!("Correspondance de règle '{}' pour l'extension : {}", self.name, ext);
                    return true;
                }
            }
        }

        false
    }

    pub fn is_exclusion(&self) -> bool {
        self.exclude
    }
}

pub struct FilterChain {
    rules: Vec<FilterRule>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: FilterRule) {
        debug!("Adding filter rule: {}", rule.name);
        self.rules.push(rule);
    }

    /// Vérifie si un événement doit être inclus (filtré)
    pub fn should_include(&self, event: &DiffEvent) -> bool {
        for rule in &self.rules {
            let file_size = event.file_size();

            if rule.matches(&event.file_path, file_size) {
                return !rule.is_exclusion();
            }
        }

        // Par défaut, inclure
        true
    }

    /// Filtre une liste d'événements
    pub fn filter_events(&self, events: Vec<DiffEvent>) -> Vec<DiffEvent> {
        events
            .into_iter()
            .filter(|e| self.should_include(e))
            .collect()
    }

    /// Crée une chaîne de filtres avec les règles par défaut
    pub fn with_defaults() -> Result<Self> {
        let mut chain = Self::new();

        // Exclusions par défaut
        let default_excludes = vec![
            ("Git", "**/.git/**"),
            ("NodeModules", "**/node_modules/**"),
            ("RustTarget", "**/target/**"),
            ("TempFiles", "**/*.tmp"),
            ("SwapFiles", "**/*.swp"),
            ("BackupFiles", "**/*~"),
            ("DS_Store", "**/.DS_Store"),
            ("ThumbsDB", "**/Thumbs.db"),
        ];

        for (name, pattern) in default_excludes {
            chain.add_rule(FilterRule::new_glob_pattern(name, pattern, true)?);
        }

        Ok(chain)
    }
}