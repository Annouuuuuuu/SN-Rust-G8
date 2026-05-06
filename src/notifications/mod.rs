use crate::errors::{FileSentinelError, Result};
use crate::watcher::types::{ChangeType, DiffEvent};
use notify_rust::Notification;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use log::{debug, warn};

#[derive(Debug, Clone)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub batch_enabled: bool,
    pub min_interval: Duration,
    pub max_events_per_notification: usize,
    pub critical_patterns: Vec<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            batch_enabled: true,
            min_interval: Duration::from_secs(5),
            max_events_per_notification: 10,
            critical_patterns: vec![
                "*.conf".to_string(),
                "*.env".to_string(),
                "*.toml".to_string(),
                "*.lock".to_string(),
            ],
        }
    }
}

pub struct NotificationManager {
    config: NotificationConfig,
    event_buffer: VecDeque<DiffEvent>,
    last_notification: Option<Instant>,
}

impl NotificationManager {
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            config,
            event_buffer: VecDeque::new(),
            last_notification: None,
        }
    }

    /// Ajoute un événement et envoie une notification si nécessaire
    pub fn push_event(&mut self, event: DiffEvent) {
        if !self.config.enabled {
            return;
        }

        self.event_buffer.push_back(event);

        let should_notify = match self.last_notification {
            Some(last) => last.elapsed() >= self.config.min_interval,
            None => true,
        };

        if should_notify && !self.event_buffer.is_empty() {
            self.send_batch_notification();
        }
    }

    fn send_batch_notification(&mut self) {
        let events: Vec<&DiffEvent> = self
            .event_buffer
            .iter()
            .take(self.config.max_events_per_notification)
            .collect();

        if events.is_empty() {
            return;
        }

        if events.len() == 1 || !self.config.batch_enabled {
            self.send_single_notification(events[0]);
        } else {
            self.send_summary_notification(&events);
        }

        // Nettoyer le buffer
        let count = events.len().min(self.event_buffer.len());
        for _ in 0..count {
            self.event_buffer.pop_front();
        }

        self.last_notification = Some(Instant::now());
    }

    fn send_single_notification(&self, event: &DiffEvent) {
        let summary = format!(
            "File Sentinel - {}",
            match event.change_type {
                ChangeType::Created => "📄 Created",
                ChangeType::Modified => "✏️ Modified",
                ChangeType::Deleted => "🗑️ Deleted",
            }
        );

        let body = format!(
            "Fichier : {}\nStatut : {:?}\nHeure : {}",
            event.file_path.display(),
            event.change_type,
            event.timestamp.format("%H:%M:%S")
        );

        let icon = if self.is_critical(&event.file_path) {
            "dialog-warning"
        } else {
            "dialog-information"
        };

        if let Err(e) = Notification::new()
            .summary(&summary)
            .body(&body)
            .icon(icon)
            .timeout(5000)
            .show()
        {
            debug!("Impossible d'envoyer la notification : {}", e);
        }
    }

    fn send_summary_notification(&self, events: &[&DiffEvent]) {
        let created = events
            .iter()
            .filter(|e| matches!(e.change_type, ChangeType::Created))
            .count();
        let modified = events
            .iter()
            .filter(|e| matches!(e.change_type, ChangeType::Modified))
            .count();
        let deleted = events
            .iter()
            .filter(|e| matches!(e.change_type, ChangeType::Deleted))
            .count();

        let mut summary = String::from("FileSentinel - ");
        let mut parts = Vec::new();

        if created > 0 {
            parts.push(format!("{}crées", created));
        }
        if modified > 0 {
            parts.push(format!("{}modifiés", modified));
        }
        if deleted > 0 {
            parts.push(format!("{}supprimés", deleted));
        }

        summary.push_str(&parts.join(", "));

        let mut body = String::new();

        // Ajouter les fichiers critiques
        let critical: Vec<_> = events
            .iter()
            .filter(|e| self.is_critical(&e.file_path))
            .collect();

        if !critical.is_empty() {
            body.push_str("⚠️ Fichiers critiques:\n");
            for event in critical.iter().take(5) {
                body.push_str(&format!(
                    "  {} - {}\n",
                    event.change_type,
                    event.file_path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
            }
        }

        // Ajouter les fichiers modifiés récents
        let recent: Vec<_> = events
            .iter()
            .filter(|e| !self.is_critical(&e.file_path))
            .take(3)
            .collect();

        if !recent.is_empty() {
            body.push_str("\nModifications récentes:\n");
            for event in recent {
                body.push_str(&format!(
                    "  {} - {}\n",
                    event.change_type,
                    event.file_path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
            }
        }

        if let Err(e) = Notification::new()
            .summary(&summary)
            .body(&body)
            .icon("dialog-information")
            .timeout(7000)
            .show()
        {
            debug!("Failed to send batch notification: {}", e);
        }
    }

    /// Notifie la fin d'une synchronisation
    pub fn send_sync_complete_notification(&self, stats: &crate::synchro::engine::SyncStats) {
        if !self.config.enabled {
            return;
        }

        let summary = if stats.errors.is_empty() {
            "Sync complete ✅"
        } else {
            "Sync complete with errors ⚠️"
        };

        let body = format!(
            "Files: {} copied, {} created, {} deleted\nData: {:.1} MB in {}ms",
            stats.files_copied,
            stats.files_created,
            stats.files_deleted,
            stats.total_bytes_transferred as f64 / 1_000_000.0,
            stats.duration_ms
        );

        if let Err(e) = Notification::new()
            .summary(summary)
            .body(&body)
            .icon("emblem-ok")
            .timeout(5000)
            .show()
        {
            debug!("Failed to send sync notification: {}", e);
        }
    }

    /// Notifie une erreur
    pub fn send_error_notification(&self, error: &FileSentinelError) {
        if !self.config.enabled {
            return;
        }

        if let Err(e) = Notification::new()
            .summary("File Sentinel Error ❌")
            .body(&error.to_string())
            .icon("dialog-error")
            .urgency(notify_rust::Urgency::Critical)
            .timeout(0) // Pas de timeout pour les erreurs
            .show()
        {
            warn!("Failed to send error notification: {}", e);
        }
    }
      
      
    fn is_critical(&self, path: &std::path::Path) -> bool {
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");

        self.config
            .critical_patterns
            .iter()
            .any(|pattern| {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(filename))
                    .unwrap_or(false)
            })
    }
}