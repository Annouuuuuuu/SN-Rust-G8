mod cli;
mod compression;
mod config;
mod errors;
mod filters;
mod network;
mod notifications;
mod synchro;
mod versioning;
mod watcher;

use clap::Parser;
use cli::commands::{Cli, Commands, SyncDirection};
use config::settings::Config;
use errors::{FileSentinelError, Result};
use filters::rules::FilterChain;
use log::{error, info, warn, LevelFilter};
use notifications::{NotificationConfig, NotificationManager};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use synchro::engine::SyncEngine;
use versioning::VersionManager;
use watcher::{polling::PollingWatcher, Watcher};

#[cfg(target_os = "linux")]
use watcher::inotify_watcher::InotifyWatcher;

fn main() {
    // Initialiser le logger
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    // Parser la CLI
    let cli = Cli::parse();

    // Charger la configuration
    let config = match Config::from_file(&cli.config) {
        Ok(c) => {
            info!("Configuration chargée depuis {}", cli.config);
            c
        }
        Err(e) => {
            warn!("Impossible de charger la config ({}), utilisation des valeurs par défaut", e);
            Config::default()
        }
    };

    // Exécuter la commande
    if let Err(e) = run_command(cli, config) {
        error!("Erreur fatale : {}", e);
        // Envoyer une notification d'erreur
        let manager = NotificationManager::new(NotificationConfig::default());
        manager.send_error_notification(&e);
        std::process::exit(1);
    }
}

fn run_command(cli: Cli, config: Config) -> Result<()> {
    // Créer les composants de base
    let filter_chain = create_filter_chain(&config)?;

    let version_manager = if config.versioning.enabled {
        Some(VersionManager::new(
            &config.versioning.versions_dir,
            config.versioning.max_versions,
        )?)
    } else {
        None
    };

    // Utiliser Mutex pour permettre la modification
    let version_manager = Mutex::new(version_manager);

    // NotificationManager n'a pas besoin d'être Arc si on ne le partage pas entre threads
    let notification_manager = NotificationManager::new(NotificationConfig {
        enabled: config.notifications.enabled,
        batch_enabled: config.notifications.show_batch_summary,
        min_interval: Duration::from_secs(config.notifications.min_interval_seconds),
        max_events_per_notification: 10,
        critical_patterns: config.notifications.critical_patterns.clone(),
    });

    match &cli.command {
        Commands::Watch {
            directories,
            daemon,
        } => {
            run_watch_command(
                &config,
                directories,
                *daemon,
                filter_chain,
                version_manager,
                notification_manager,
            )?;
        }

        Commands::Sync { source, dest } => {
            run_sync_command(&config, source.as_deref(), dest.as_deref(), notification_manager)?;
        }

        Commands::VersionHistory { path } => {
            let vm = version_manager.lock().unwrap();
            show_version_history(&*vm, path)?;
        }

        Commands::Restore { path, version } => {
            let vm = version_manager.lock().unwrap();
            restore_file_version(&*vm, path, *version, &notification_manager)?;
        }

        Commands::NetworkSync { direction } => {
            run_network_sync(&config, *direction, &notification_manager)?;
        }

        Commands::ShowConfig => {
            show_config(&config);
        }

        Commands::Stats { period } => {
            let vm = version_manager.lock().unwrap();
            show_stats(&config, vm.as_ref(), period);
        }

        Commands::Rules => {
            show_rules(&config, &filter_chain);
        }

        Commands::Init => {
            generate_default_config()?;
        }
    }

    Ok(())
}

fn create_filter_chain(config: &Config) -> Result<FilterChain> {
    let mut chain = FilterChain::with_defaults()?;

    for pattern in &config.filters.exclude_patterns {
        match filters::rules::FilterRule::new_glob_pattern(
            &format!("Config-{}", pattern),
            pattern,
            true,
        ) {
            Ok(rule) => chain.add_rule(rule),
            Err(e) => warn!("Pattern de filtre invalide '{}' : {}", pattern, e),
        }
    }

    if let Some(max_mb) = config.filters.max_file_size_mb {
        let max_bytes = max_mb * 1024 * 1024;
        chain.add_rule(filters::rules::FilterRule::new_size_limit(max_bytes));
    }

    if !config.filters.include_extensions.is_empty() {
        chain.add_rule(filters::rules::FilterRule::new_extension_filter(
            config.filters.include_extensions.clone(),
            false,
        ));
    }

    Ok(chain)
}

fn run_watch_command(
    config: &Config,
    directories: &[String],
    daemon: bool,
    filter_chain: FilterChain,
    version_manager: Mutex<Option<VersionManager>>,
    mut notification_manager: NotificationManager,
) -> Result<()> {
    let dirs_to_watch = if directories.is_empty() {
        config.watch.directories.clone()
    } else {
        directories.to_vec()
    };

    if dirs_to_watch.is_empty() {
        return Err(FileSentinelError::Config(
            "Aucun répertoire spécifié pour la surveillance".to_string(),
        ));
    }

    println!("FileSentinel - Début de la surveillance...");
    println!("Répertoires surveillés : {:?}", dirs_to_watch);
    println!("Destination de synchronisation : {}", config.sync.destination);

    if daemon {
        println!("Fonctionnement en mode démon (Ctrl+C pour arrêter)");
    }

    let mut sync_engine = SyncEngine::new(&dirs_to_watch[0], &config.sync.destination);
    
    // Choisir le watcher selon la plateforme
    #[cfg(target_os = "linux")]
    let mut watcher: Box<dyn Watcher> = {
        println!(" Utilisation du watcher inotify (Linux)");
        Box::new(InotifyWatcher::new()?)
    };
    
    #[cfg(not(target_os = "linux"))]
    let mut watcher: Box<dyn Watcher> = {
        println!("  📊 Utilisation du watcher par scrutation");
        Box::new(PollingWatcher::new())
    };

    for dir in &dirs_to_watch {
        watcher.watch(std::path::Path::new(dir))?;
        println!("  📁 Surveillance : {}", dir);
    }

    println!("\nAppuyez sur Ctrl+C pour arrêter...\n");

    loop {
        match watcher.events() {
            Ok(events) => {
                if events.is_empty() {
                    std::thread::sleep(Duration::from_millis(config.watch.polling_interval_ms));
                    continue;
                }

                let filtered_events = filter_chain.filter_events(events);

                for event in &filtered_events {
                    println!("  {}", event);

                    // Versionnement automatique
                    if config.versioning.enabled && config.versioning.auto_version_on_change {
                        if let Ok(mut vm) = version_manager.lock() {
                            if let Some(ref mut version_mgr) = *vm {
                                if let Some(hash) = event.file_hash() {
                                    if let Err(e) = version_mgr.save_version(&event.file_path, hash) {
                                        warn!("Impossible de sauvegarder la version : {}", e);
                                    }
                                }
                            }
                        }
                    }

                    // Synchroniser l'événement
                    match sync_engine.sync_event(event) {
                        Ok(stats) => {
                            if config.reporting.show_progress && stats.files_copied > 0 {
                                println!(
                                    "    ✅ Synchronisé : {} fichiers, {} octets en {}ms",
                                    stats.files_copied,
                                    stats.total_bytes_transferred,
                                    stats.duration_ms
                                );
                            }

                            if !stats.errors.is_empty() {
                                for error in &stats.errors {
                                    eprintln!("    ❌ {}", error);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Erreur de synchronisation : {}", e);
                            notification_manager.send_error_notification(&e);
                        }
                    }

                    // Notification
                    notification_manager.push_event(event.clone());
                }
            }
            Err(e) => {
                error!("Erreur de surveillance : {}", e);
                notification_manager.send_error_notification(&e);
                std::thread::sleep(Duration::from_secs(1));
            }
        }

        std::thread::sleep(Duration::from_millis(config.watch.polling_interval_ms));
    }
}

fn run_sync_command(
    config: &Config,
    source: Option<&str>,
    dest: Option<&str>,
    notification_manager: NotificationManager,
) -> Result<()> {
    let source = source.unwrap_or(&config.watch.directories[0]);
    let dest = dest.unwrap_or(&config.sync.destination);

    println!("🔄 Lancement de la synchronisation complète...");
    println!("Source : {}", source);
    println!("Destination : {}", dest);

    let mut engine = SyncEngine::new(source, dest);

    match engine.full_sync() {
        Ok(stats) => {
            println!("\n✅ Synchronisation terminée avec succès !");
            println!("{}", stats);
            notification_manager.send_sync_complete_notification(&stats);
        }
        Err(e) => {
            error!("Sync failed: {}", e);
            notification_manager.send_error_notification(&e);
            return Err(e);
        }
    }

    Ok(())
}

fn show_version_history(
    version_manager: &Option<VersionManager>,
    path: &std::path::Path,
) -> Result<()> {
    match version_manager {
        Some(vm) => {
            let versions = vm.get_versions(path);

            if versions.is_empty() {
                println!("Aucune version trouvée pour : {}", path.display());
            } else {
                println!("Historique des versions pour : {}", path.display());
                println!("{:-<60}", "");

                for version in versions.iter().rev() {
                    println!(
                        "Version {} - {} - {}",
                        version.version_number,
                        version.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        version.format_size()
                    );
                }
            }
        }
        None => {
            println!("Versioning is not enabled in configuration");
        }
    }

    Ok(())
}

fn restore_file_version(
    version_manager: &Option<VersionManager>,
    path: &std::path::Path,
    version: u32,
    notification_manager: &NotificationManager,
) -> Result<()> {
    match version_manager {
        Some(vm) => {
            println!(
                "Restauration de la version {} de {}...",
                version,
                path.display()
            );

            vm.restore_version(path, version)?;

            println!(" Version restaurée avec succès !");

            // Créer un événement pour notification
            use watcher::types::{DiffEvent, ChangeType};
            let event = DiffEvent::new(
                ChangeType::Modified,
                path.to_path_buf(),
                None,
                None,
            );
            // Note: on ne peut pas push car notification_manager n'est pas mutable ici
            let _ = event; // Pour éviter warning
        }
        None => {
            println!("Versioning is not enabled in configuration");
        }
    }

    Ok(())
}

fn run_network_sync(
    config: &Config,
    direction: SyncDirection,
    _notification_manager: &NotificationManager,
) -> Result<()> {
    match &config.network {
        Some(net_config) => {
            let ssh_config = network::SshConfig {
                host: net_config.host.clone(),
                port: net_config.port,
                username: net_config.username.clone(),
                key_path: net_config.key_path.clone(),
                remote_path: net_config.remote_path.clone(),
                rsync_options: net_config.rsync_options.clone(),
            };

            let sync = network::NetworkSync::new(ssh_config);

            println!("Test de la connexion SSH...");
            match sync.test_connection() {
                Ok(true) => println!(" Connexion SSH réussie\n"),
                Ok(false) => {
                    println!("Échec de la connexion SSH");
                    return Ok(());
                }
                Err(e) => {
                    println!(" Erreur de connexion SSH : {}", e);
                    return Ok(());
                }
            }

            let local_path = &config.watch.directories[0];

            let result = match direction {
                SyncDirection::ToRemote => {
                    println!("Synchronisation vers le serveur distant...");
                    sync.sync_to_remote(local_path)?
                }
                SyncDirection::FromRemote => {
                    println!("Synchronisation depuis le serveur distant...");
                    sync.sync_from_remote(local_path)?
                }
            };

            println!("\n{}", result);

            if !result.success {
                // Note: notification_manager n'est pas mutable ici non plus
                println!("Network sync failed");
            }
        }
        None => {
            println!("La configuration réseau n'est pas définie. Ajoutez une section [network] à config.toml");
        }
    }

    Ok(())
}

fn show_config(config: &Config) {
    println!("\n📋 Configuration actuelle :");
    println!("{:#?}", config);
}

fn show_stats(_config: &Config, version_manager: Option<&VersionManager>, period: &str) {
    println!("\n📊 Statistiques pour la période : {}", period);
    println!("{:=<50}", "");

    if let Some(vm) = version_manager {
        let stats = vm.get_stats();
        println!("{}", stats);
    } else {
        println!("Le versionnage n'est pas activé");
    }
}

fn show_rules(config: &Config, _filter_chain: &FilterChain) {
    println!("\n🔧 Règles de filtrage actives :");
    println!("{:=<50}", "");

    println!("Modèles d'exclusion :");
    for pattern in &config.filters.exclude_patterns {
        println!("  - {}", pattern);
    }

    if let Some(max_size) = config.filters.max_file_size_mb {
        println!("\nTaille maximale de fichier : {} Mo", max_size);
    }

    if !config.filters.include_extensions.is_empty() {
        println!("\nExténsions incluses : {:?}", config.filters.include_extensions);
    }
}

fn generate_default_config() -> Result<()> {
    let config = Config::default();
    config.save_to_file("config.toml")?;
    println!(" ✅ Configuration par défaut générée : config.toml");
    Ok(())
}
