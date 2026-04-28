use crate::errors::{FileSentinelError, Result};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{info, debug, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub key_path: Option<PathBuf>,
    pub remote_path: PathBuf,
    pub rsync_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSyncResult {
    pub success: bool,
    pub duration_ms: u128,
    pub files_synced: u32,
    pub bytes_transferred: u64,
    pub errors: Vec<String>,
}

impl std::fmt::Display for NetworkSyncResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Network sync: {} in {}ms - {} files, {} MB",
            if self.success { "SUCCESS" } else { "FAILED" },
            self.duration_ms,
            self.files_synced,
            self.bytes_transferred as f64 / 1_000_000.0
        )
    }
}

pub struct NetworkSync {
    config: SshConfig,
}

impl NetworkSync {
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }

    /// Vérifie la connexion SSH
    pub fn test_connection(&self) -> Result<bool> {
        info!(
            "Testing SSH connection to {}@{}:{}",
            self.config.username, self.config.host, self.config.port
        );

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-p", &self.config.port.to_string(),
            "-o", "ConnectTimeout=5",
            "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=accept-new",
        ]);

        if let Some(key_path) = &self.config.key_path {
            cmd.args(["-i", &key_path.to_string_lossy()]);
        }

        cmd.arg(format!("{}@{}", self.config.username, self.config.host));
        cmd.arg("echo connected");

        let output = cmd.output().map_err(|e| {
            FileSentinelError::Network(format!("Cannot execute SSH: {}", e))
        })?;

        Ok(output.status.success())
    }

    /// Synchronise vers le serveur distant
    pub fn sync_to_remote<P: AsRef<Path>>(&self, local_path: P) -> Result<NetworkSyncResult> {
        let local_path = local_path.as_ref();
        let start = std::time::Instant::now();
        let mut result = NetworkSyncResult {
            success: false,
            duration_ms: 0,
            files_synced: 0,
            bytes_transferred: 0,
            errors: Vec::new(),
        };

        info!(
            "Syncing to remote: {} -> {}@{}:{}",
            local_path.display(),
            self.config.username,
            self.config.host,
            self.config.remote_path.display()
        );

        // Vérifier que rsync est disponible
        if !Self::is_rsync_available() {
            return Err(FileSentinelError::Network(
                "rsync is not available. Please install rsync.".to_string()
            ));
        }

        // Construire la commande rsync
        let mut cmd = Command::new("rsync");

        // Options rsync
        cmd.args(&self.config.rsync_options);

        // Options SSH
        cmd.args([
            "-e",
            &format!("ssh -p {} -o StrictHostKeyChecking=accept-new", self.config.port),
        ]);

        // Clé SSH si spécifiée
        if let Some(key_path) = &self.config.key_path {
            cmd.args([
                "-e",
                &format!(
                    "ssh -p {} -i {} -o StrictHostKeyChecking=accept-new",
                    self.config.port,
                    key_path.display()
                ),
            ]);
        }

        // Source et destination
        cmd.arg(local_path);
        cmd.arg(format!(
            "{}@{}:{}",
            self.config.username,
            self.config.host,
            self.config.remote_path.display()
        ));

        // Exécuter la commande
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    result.success = true;
                    info!("Remote sync completed successfully");
                    
                    if !output.stdout.is_empty() {
                        debug!("rsync stdout: {}", String::from_utf8_lossy(&output.stdout));
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_msg = format!("rsync failed: {}", stderr);
                    error!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
            Err(e) => {
                let error_msg = format!("Cannot execute rsync: {}", e);
                error!("{}", error_msg);
                result.errors.push(error_msg);
            }
        }

        result.duration_ms = start.elapsed().as_millis();
        Ok(result)
    }

    /// Synchronise depuis le serveur distant
    pub fn sync_from_remote<P: AsRef<Path>>(&self, local_path: P) -> Result<NetworkSyncResult> {
        let local_path = local_path.as_ref();
        let start = std::time::Instant::now();
        let mut result = NetworkSyncResult {
            success: false,
            duration_ms: 0,
            files_synced: 0,
            bytes_transferred: 0,
            errors: Vec::new(),
        };

        info!(
            "Syncing from remote: {}@{}:{} -> {}",
            self.config.username,
            self.config.host,
            self.config.remote_path.display(),
            local_path.display()
        );

        if !Self::is_rsync_available() {
            return Err(FileSentinelError::Network(
                "rsync is not available. Please install rsync.".to_string()
            ));
        }

        let mut cmd = Command::new("rsync");
        cmd.args(&self.config.rsync_options);
        cmd.args([
            "-e",
            &format!("ssh -p {} -o StrictHostKeyChecking=accept-new", self.config.port),
        ]);

        if let Some(key_path) = &self.config.key_path {
            cmd.args([
                "-e",
                &format!(
                    "ssh -p {} -i {} -o StrictHostKeyChecking=accept-new",
                    self.config.port,
                    key_path.display()
                ),
            ]);
        }

        cmd.arg(format!(
            "{}@{}:{}",
            self.config.username,
            self.config.host,
            self.config.remote_path.display()
        ));
        cmd.arg(local_path);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    result.success = true;
                    info!("Remote sync completed successfully");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_msg = format!("rsync failed: {}", stderr);
                    error!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
            Err(e) => {
                result.errors.push(format!("Cannot execute rsync: {}", e));
            }
        }

        result.duration_ms = start.elapsed().as_millis();
        Ok(result)
    }

    fn is_rsync_available() -> bool {
        Command::new("rsync")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}