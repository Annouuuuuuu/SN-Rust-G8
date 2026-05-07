use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::settings::Config;
use crate::filters::rules::{FilterChain, FilterRule};
use crate::synchro::engine::{SyncEngine, SyncStats};
use crate::versioning::{FileVersion, VersionManager};
use crate::watcher::polling::PollingWatcher;
use crate::watcher::types::DiffEvent;
use crate::watcher::Watcher;

// ─── Tabs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard = 0,
    Surveillance = 1,
    Sync = 2,
    Versions = 3,
    Config = 4,
    Regles = 5,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Dashboard,
        Tab::Surveillance,
        Tab::Sync,
        Tab::Versions,
        Tab::Config,
        Tab::Regles,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Surveillance => "Surveillance",
            Tab::Sync => "Synchronisation",
            Tab::Versions => "Versions",
            Tab::Config => "Configuration",
            Tab::Regles => "Règles",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    pub fn next(self) -> Tab {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Tab {
        let i = self.index();
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

// ─── Mode de saisie ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    None,
    AddWatchDir,
    EditDestination,
    VersionSearch,
    RestoreToFolder,
    CleanVersions,
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        *self != InputMode::None
    }

    pub fn prompt(&self) -> &str {
        match self {
            InputMode::AddWatchDir => "Chemin du dossier à surveiller",
            InputMode::EditDestination => "Nouveau dossier de destination",
            InputMode::VersionSearch => "Chemin du fichier à rechercher",
            InputMode::RestoreToFolder => "Dossier cible pour la restauration",
            InputMode::CleanVersions => "Nombre de versions à conserver",
            InputMode::None => "",
        }
    }
}

// ─── Messages inter-threads ───────────────────────────────────────────────────

pub enum WatcherCmd {
    Start(Vec<String>),
    Stop,
}

pub enum SyncCmd {
    FullSync { source: String, dest: String },
}

pub enum BackgroundMsg {
    FileEvent(DiffEvent),
    SyncOk(SyncStats),
    SyncErr(String),
    Tick,
}

// ─── Niveau de statut ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MsgLevel {
    Info,
    Success,
    Warning,
    Error,
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub config: Config,
    pub active_tab: Tab,
    pub list_selected: usize,
    pub should_quit: bool,
    pub tick_count: u64,

    // Surveillance
    pub watching: bool,
    pub events: VecDeque<DiffEvent>,

    // Synchronisation
    pub syncing: bool,
    pub sync_cancelled: bool,
    pub last_stats: Option<SyncStats>,

    // Versions
    pub version_list: Vec<FileVersion>,
    version_manager: Option<Arc<Mutex<VersionManager>>>,

    // Saisie universelle (overlay)
    pub input_mode: InputMode,
    pub input_value: String,

    // Message de statut (message, niveau, horodatage)
    pub status: Option<(String, MsgLevel, Instant)>,

    // Channels
    bg_rx: Receiver<BackgroundMsg>,
    watch_tx: Sender<WatcherCmd>,
    sync_tx: Sender<SyncCmd>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (bg_tx, bg_rx) = mpsc::channel::<BackgroundMsg>();
        let (watch_tx, watch_rx) = mpsc::channel::<WatcherCmd>();
        let (sync_tx, sync_rx) = mpsc::channel::<SyncCmd>();

        let version_manager: Option<Arc<Mutex<VersionManager>>> =
            if config.versioning.enabled {
                VersionManager::new(
                    &config.versioning.versions_dir,
                    config.versioning.max_versions,
                )
                .ok()
                .map(|vm| Arc::new(Mutex::new(vm)))
            } else {
                None
            };

        spawn_watcher_thread(bg_tx.clone(), watch_rx, config.clone(), version_manager.clone());
        spawn_sync_thread(bg_tx.clone(), sync_rx);
        spawn_tick_thread(bg_tx);

        Self {
            config,
            active_tab: Tab::Dashboard,
            list_selected: 0,
            should_quit: false,
            tick_count: 0,
            watching: false,
            events: VecDeque::new(),
            syncing: false,
            sync_cancelled: false,
            last_stats: None,
            version_list: Vec::new(),
            version_manager,
            input_mode: InputMode::None,
            input_value: String::new(),
            status: Some((
                "Bienvenue ! W=surveiller  S=sync  A=ajouter dossier  E=modifier dest".to_string(),
                MsgLevel::Info,
                Instant::now(),
            )),
            bg_rx,
            watch_tx,
            sync_tx,
        }
    }

    // ─── Tick ─────────────────────────────────────────────────────────────────

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        while let Ok(msg) = self.bg_rx.try_recv() {
            match msg {
                BackgroundMsg::FileEvent(e) => {
                    self.events.push_front(e);
                    if self.events.len() > 300 {
                        self.events.pop_back();
                    }
                }
                BackgroundMsg::SyncOk(stats) => {
                    let was_syncing = self.syncing;
                    self.syncing = false;
                    self.sync_cancelled = false;
                    let msg = format!(
                        "Sync OK : {} fichier(s), {:.1} KB en {}ms",
                        stats.files_copied,
                        stats.total_bytes_transferred as f64 / 1024.0,
                        stats.duration_ms
                    );
                    self.last_stats = Some(stats);
                    if was_syncing {
                        self.set_status(msg, MsgLevel::Success);
                    }
                }
                BackgroundMsg::SyncErr(e) => {
                    let was_syncing = self.syncing;
                    self.syncing = false;
                    self.sync_cancelled = false;
                    if was_syncing {
                        self.set_status(format!("Erreur sync : {}", e), MsgLevel::Error);
                    }
                }
                BackgroundMsg::Tick => {}
            }
        }

        if let Some((_, _, since)) = &self.status {
            if since.elapsed() > Duration::from_secs(10) {
                self.status = None;
            }
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, level: MsgLevel) {
        self.status = Some((msg.into(), level, Instant::now()));
    }

    // ─── Saisie (overlay) ─────────────────────────────────────────────────────

    pub fn open_input(&mut self, mode: InputMode, initial: &str) {
        self.input_mode = mode;
        self.input_value = initial.to_string();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::None;
        self.input_value.clear();
    }

    pub fn input_push(&mut self, c: char) {
        self.input_value.push(c);
    }

    pub fn input_pop(&mut self) {
        self.input_value.pop();
    }

    pub fn confirm_input(&mut self) {
        let value = self.input_value.trim().to_string();
        let mode = self.input_mode.clone();
        self.input_mode = InputMode::None;
        self.input_value.clear();

        match mode {
            InputMode::AddWatchDir => self.add_watch_dir(value),
            InputMode::EditDestination => self.set_destination(value),
            InputMode::VersionSearch => self.load_versions_for(value),
            InputMode::RestoreToFolder => self.restore_to_folder(value),
            InputMode::CleanVersions => {
                if let Ok(n) = value.parse::<u32>() {
                    self.clean_versions_keep(n);
                } else {
                    self.set_status("Entrez un nombre valide".to_string(), MsgLevel::Error);
                }
            }
            InputMode::None => {}
        }
    }

    // ─── Surveillance ─────────────────────────────────────────────────────────

    pub fn toggle_watch(&mut self) {
        if self.watching {
            let _ = self.watch_tx.send(WatcherCmd::Stop);
            self.watching = false;
            self.set_status("Surveillance arrêtée".to_string(), MsgLevel::Info);
        } else {
            let dirs = self.config.watch.directories.clone();
            if dirs.is_empty() {
                self.set_status("Ajoutez d'abord un dossier avec [A]".to_string(), MsgLevel::Warning);
                return;
            }
            let _ = self.watch_tx.send(WatcherCmd::Start(dirs.clone()));
            self.watching = true;
            self.set_status(
                format!("Surveillance démarrée sur {} dossier(s)", dirs.len()),
                MsgLevel::Success,
            );
        }
    }

    fn add_watch_dir(&mut self, path: String) {
        if path.is_empty() {
            return;
        }
        let p = std::path::Path::new(&path);
        if !p.exists() || !p.is_dir() {
            self.set_status(format!("Dossier invalide ou inexistant : {}", path), MsgLevel::Error);
            return;
        }
        if self.config.watch.directories.contains(&path) {
            self.set_status(format!("Dossier déjà surveillé : {}", path), MsgLevel::Warning);
            return;
        }
        self.config.watch.directories.push(path.clone());
        self.save_config();
        self.set_status(format!("Dossier ajouté : {}", path), MsgLevel::Success);
        if self.watching {
            let dirs = self.config.watch.directories.clone();
            let _ = self.watch_tx.send(WatcherCmd::Stop);
            let _ = self.watch_tx.send(WatcherCmd::Start(dirs));
        }
    }

    pub fn remove_watch_dir(&mut self) {
        if self.config.watch.directories.is_empty() {
            return;
        }
        let idx = self.list_selected.min(self.config.watch.directories.len() - 1);
        let removed = self.config.watch.directories.remove(idx);
        self.list_selected = self.list_selected.saturating_sub(1);
        self.save_config();
        self.set_status(format!("Dossier retiré : {}", removed), MsgLevel::Success);
        if self.watching {
            let _ = self.watch_tx.send(WatcherCmd::Stop);
            if self.config.watch.directories.is_empty() {
                self.watching = false;
            } else {
                let dirs = self.config.watch.directories.clone();
                let _ = self.watch_tx.send(WatcherCmd::Start(dirs));
            }
        }
    }

    // ─── Synchronisation ──────────────────────────────────────────────────────

    pub fn run_sync(&mut self) {
        if self.syncing {
            return;
        }
        let source = self.config.watch.directories.first().cloned().unwrap_or_default();
        if source.is_empty() {
            self.set_status("Aucun dossier source configuré".to_string(), MsgLevel::Warning);
            return;
        }
        let dest = self.config.sync.destination.clone();
        let _ = self.sync_tx.send(SyncCmd::FullSync { source, dest });
        self.syncing = true;
        self.set_status("Synchronisation en cours...".to_string(), MsgLevel::Info);
    }

    pub fn cancel_sync(&mut self) {
        if self.syncing {
            self.syncing = false;
            self.sync_cancelled = true;
            self.set_status("Sync annulée (thread en arrière-plan se termine)".to_string(), MsgLevel::Warning);
        }
    }

    fn set_destination(&mut self, dest: String) {
        if dest.is_empty() {
            return;
        }
        self.config.sync.destination = dest.clone();
        self.save_config();
        self.set_status(format!("Destination mise à jour : {}", dest), MsgLevel::Success);
    }

    // ─── Versions ─────────────────────────────────────────────────────────────

    pub fn load_versions_for(&mut self, path: String) {
        let p = std::path::PathBuf::from(&path);
        let vm_arc_opt = self.version_manager.clone();
        let versions = if let Some(vm_arc) = vm_arc_opt {
            if let Ok(vm) = vm_arc.lock() {
                vm.get_versions(&p).into_iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Sauvegarder le chemin dans input_value pour référence
        self.input_value = path;
        self.version_list = versions;
        self.list_selected = 0;

        let count = self.version_list.len();
        if count == 0 {
            self.set_status("Aucune version trouvée pour ce fichier".to_string(), MsgLevel::Warning);
        } else {
            self.set_status(format!("{} version(s) trouvée(s)", count), MsgLevel::Success);
        }
    }

    pub fn restore_selected_original(&mut self) {
        if self.version_list.is_empty() {
            return;
        }
        let idx = self.list_selected.min(self.version_list.len() - 1);
        let path = self.version_list[idx].original_path.clone();
        let vnum = self.version_list[idx].version_number;

        let result = if let Some(vm_arc) = self.version_manager.clone() {
            if let Ok(vm) = vm_arc.lock() {
                vm.restore_version(&path, vnum).map_err(|e| e.to_string())
            } else {
                Err("Mutex poisonné".to_string())
            }
        } else {
            Err("Versioning non activé".to_string())
        };

        match result {
            Ok(_) => self.set_status(
                format!("Version {} restaurée → {}", vnum, path.display()),
                MsgLevel::Success,
            ),
            Err(e) => self.set_status(e, MsgLevel::Error),
        }
    }

    fn restore_to_folder(&mut self, folder: String) {
        if self.version_list.is_empty() {
            return;
        }
        if folder.is_empty() {
            self.set_status("Dossier cible vide".to_string(), MsgLevel::Error);
            return;
        }
        let idx = self.list_selected.min(self.version_list.len() - 1);
        let original = self.version_list[idx].original_path.clone();
        let vnum = self.version_list[idx].version_number;
        let filename = original
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "fichier".to_string());
        let target = std::path::PathBuf::from(&folder).join(&filename);

        let result = if let Some(vm_arc) = self.version_manager.clone() {
            if let Ok(vm) = vm_arc.lock() {
                vm.restore_to_path(&original, vnum, &target)
                    .map_err(|e| e.to_string())
            } else {
                Err("Mutex poisonné".to_string())
            }
        } else {
            // Fallback : copier directement depuis storage_path
            std::fs::create_dir_all(&folder)
                .and_then(|_| std::fs::copy(&self.version_list[idx].storage_path, &target).map(|_| ()))
                .map_err(|e| e.to_string())
        };

        match result {
            Ok(_) => self.set_status(
                format!("Version {} → {}", vnum, target.display()),
                MsgLevel::Success,
            ),
            Err(e) => self.set_status(format!("Erreur : {}", e), MsgLevel::Error),
        }
    }

    pub fn delete_selected_version(&mut self) {
        if self.version_list.is_empty() {
            return;
        }
        let idx = self.list_selected.min(self.version_list.len() - 1);
        let original = self.version_list[idx].original_path.clone();
        let vnum = self.version_list[idx].version_number;

        let result = if let Some(vm_arc) = self.version_manager.clone() {
            if let Ok(mut vm) = vm_arc.lock() {
                vm.delete_version(&original, vnum).map_err(|e| e.to_string())
            } else {
                Err("Mutex poisonné".to_string())
            }
        } else {
            Err("Versioning non activé".to_string())
        };

        match result {
            Ok(_) => {
                self.version_list.remove(idx);
                if self.list_selected > 0 && self.list_selected >= self.version_list.len() {
                    self.list_selected -= 1;
                }
                self.set_status(format!("Version {} supprimée", vnum), MsgLevel::Success);
            }
            Err(e) => self.set_status(format!("Erreur : {}", e), MsgLevel::Error),
        }
    }

    fn clean_versions_keep(&mut self, keep: u32) {
        if self.version_list.is_empty() {
            self.set_status("Aucune version chargée".to_string(), MsgLevel::Warning);
            return;
        }
        let original = self.version_list[0].original_path.clone();

        let result = if let Some(vm_arc) = self.version_manager.clone() {
            if let Ok(mut vm) = vm_arc.lock() {
                vm.clean_old_versions(&original, keep).map_err(|e| e.to_string())
            } else {
                Err("Mutex poisonné".to_string())
            }
        } else {
            Err("Versioning non activé".to_string())
        };

        match result {
            Ok(n) => {
                let path = self.input_value.clone();
                self.load_versions_for(path);
                self.set_status(format!("{} ancienne(s) version(s) supprimée(s)", n), MsgLevel::Success);
            }
            Err(e) => self.set_status(format!("Erreur : {}", e), MsgLevel::Error),
        }
    }

    // ─── Config ───────────────────────────────────────────────────────────────

    fn save_config(&mut self) {
        if let Err(e) = self.config.save_to_file("config.toml") {
            self.set_status(format!("Erreur sauvegarde config : {}", e), MsgLevel::Error);
        }
    }

    // ─── Navigation ───────────────────────────────────────────────────────────

    pub fn scroll_up(&mut self) {
        self.list_selected = self.list_selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, max: usize) {
        if max > 0 && self.list_selected + 1 < max {
            self.list_selected += 1;
        }
    }

    pub fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
        self.list_selected = 0;
    }
}

// ─── Threads background ───────────────────────────────────────────────────────

fn build_filter_chain(config: &Config) -> FilterChain {
    let mut chain = FilterChain::with_defaults().unwrap_or_else(|_| FilterChain::new());
    for pattern in &config.filters.exclude_patterns {
        if let Ok(rule) =
            FilterRule::new_glob_pattern(&format!("cfg-{}", pattern), pattern, true)
        {
            chain.add_rule(rule);
        }
    }
    if let Some(max_mb) = config.filters.max_file_size_mb {
        chain.add_rule(FilterRule::new_size_limit(max_mb * 1024 * 1024));
    }
    chain
}

fn spawn_watcher_thread(
    tx: Sender<BackgroundMsg>,
    cmd_rx: Receiver<WatcherCmd>,
    config: Config,
    version_manager: Option<Arc<Mutex<VersionManager>>>,
) {
    thread::spawn(move || {
        let filter_chain = build_filter_chain(&config);
        let interval = Duration::from_millis(config.watch.polling_interval_ms);
        let mut watcher: Option<PollingWatcher> = None;

        loop {
            match cmd_rx.try_recv() {
                Ok(WatcherCmd::Start(dirs)) => {
                    let mut pw = PollingWatcher::new();
                    for dir in &dirs {
                        let _ = pw.watch(std::path::Path::new(dir));
                    }
                    watcher = Some(pw);
                }
                Ok(WatcherCmd::Stop) => {
                    watcher = None;
                }
                Err(_) => {}
            }

            if let Some(ref mut pw) = watcher {
                if let Ok(evts) = pw.events() {
                    for event in filter_chain.filter_events(evts) {
                        if config.versioning.auto_version_on_change {
                            if let Some(ref vm_arc) = version_manager {
                                if let Ok(mut vm) = vm_arc.lock() {
                                    if let Some(hash) = event.file_hash() {
                                        let _ = vm.save_version(&event.file_path, hash);
                                    }
                                }
                            }
                        }
                        if tx.send(BackgroundMsg::FileEvent(event)).is_err() {
                            return;
                        }
                    }
                }
            }

            thread::sleep(interval);
        }
    });
}

fn spawn_sync_thread(tx: Sender<BackgroundMsg>, cmd_rx: Receiver<SyncCmd>) {
    thread::spawn(move || {
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                SyncCmd::FullSync { source, dest } => {
                    let mut engine = SyncEngine::new(&source, &dest);
                    let msg = match engine.full_sync() {
                        Ok(stats) => BackgroundMsg::SyncOk(stats),
                        Err(e) => BackgroundMsg::SyncErr(e.to_string()),
                    };
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

fn spawn_tick_thread(tx: Sender<BackgroundMsg>) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(200));
        if tx.send(BackgroundMsg::Tick).is_err() {
            break;
        }
    });
}
