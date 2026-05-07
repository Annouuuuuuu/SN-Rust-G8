use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "filesentinel")]
#[command(version = "0.2.0")]
#[command(about = "Outil de surveillance et synchronisation de fichiers", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Fichier de configuration
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    /// Activer le mode verbeux
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Démarrer la surveillance des répertoires
    Watch {
        /// Répertoires à surveiller (utilise la config par défaut si non spécifié)
        #[arg(short, long)]
        directories: Vec<String>,

        /// Mode daemon (continue en arrière-plan)
        #[arg(short, long)]
        daemon: bool,
    },

    /// Effectuer une synchronisation manuelle
    Sync {
        /// Répertoire source
        #[arg(short, long)]
        source: Option<String>,

        /// Répertoire de destination
        #[arg(short, long)]
        dest: Option<String>,
    },

    /// Afficher l'historique des versions d'un fichier
    VersionHistory {
        /// Chemin du fichier
        path: PathBuf,
    },

    /// Pour restaurer une version spécifique
    Restore {
        /// Chemin du fichier
        path: PathBuf,

        /// Numéro de version à restaurer
        #[arg(short, long)]
        version: u32,
    },

    /// Synchronisation réseau
    NetworkSync {
        /// Direction de la synchronisation
        #[arg(value_enum)]
        direction: SyncDirection,
    },

    /// Afficher la configuration actuelle
    ShowConfig,

    /// Afficher les statistiques
    Stats {
        /// Période d'analyse
        #[arg(short, long, default_value = "24h")]
        period: String,
    },

    /// Lister les règles de filtrage actives
    Rules,

    /// Générer un fichier de configuration par défaut
    Init,

    /// Ouvrir le tableau de bord interactif (TUI)
    Dashboard,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum SyncDirection {
    /// Vers le serveur distant
    ToRemote,
    /// Depuis le serveur distant
    FromRemote,
}