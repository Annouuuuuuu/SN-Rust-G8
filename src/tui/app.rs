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

// ─── Tabs ────────────────────────────────────────────────────────────────────

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

    pub fn from_index(i: usize) -> Tab {
        Self::ALL.get(i).copied().unwrap_or(Tab::Dashboard)
    }
}

// ─── Messages inter-threads ───────────────────────────────────────────────────

pub enum WatcherCmd {
    Start,
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

// ─── Niveau de message de statut ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MsgLevel {
    Info,
    Success,
    Warning,
    Error,
}

// ─── App ─────────────────────────────────────────────────────────────────────

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
    pub last_stats: Option<SyncStats>,

    // Versions
    pub version_input: String,
    pub version_input_mode: bool,
    pub version_list: Vec<FileVersion>,
    version_manager: Option<Arc<Mutex<VersionManager>>>,

    // Message de statut (message, level, expiration)
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

        spawn_watcher_thread(
            bg_tx.clone(),
            watch_rx,
            config.clone(),
            version_manager.clone(),
        );
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
            last_stats: None,
            version_input: String::new(),
            version_input_mode: false,
            version_list: Vec::new(),
            version_manager,
            status: Some((
                "Bienvenue ! Appuyez sur W pour surveiller, S pour synchroniser, ? pour l'aide".to_string(),
                MsgLevel::Info,
                Instant::now(),
            )),
            bg_rx,
            watch_tx,
            sync_tx,
        }
    }

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
                    self.syncing = false;
                    let msg = format!(
                        "Sync OK : {} fichier(s), {:.1} KB en {}ms",
                        stats.files_copied,
                        stats.total_bytes_transferred as f64 / 1024.0,
                        stats.duration_ms
                    );
                    self.last_stats = Some(stats);
                    self.set_status(msg, MsgLevel::Success);
                }
                BackgroundMsg::SyncErr(e) => {
                    self.syncing = false;
                    self.set_status(format!("Erreur sync : {}", e), MsgLevel::Error);
                }
                BackgroundMsg::Tick => {}
            }
        }

        // Expirer le message de statut après 8 secondes
        if let Some((_, _, since)) = &self.status {
            if since.elapsed() > Duration::from_secs(8) {
                self.status = None;
            }
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, level: MsgLevel) {
        self.status = Some((msg.into(), level, Instant::now()));
    }

    pub fn toggle_watch(&mut self) {
        if self.watching {
            let _ = self.watch_tx.send(WatcherCmd::Stop);
            self.watching = false;
            self.set_status("Surveillance arrêtée".to_string(), MsgLevel::Info);
        } else {
            let _ = self.watch_tx.send(WatcherCmd::Start);
            self.watching = true;
            self.set_status(
                format!(
                    "Surveillance démarrée sur {} dossier(s)",
                    self.config.watch.directories.len()
                ),
                MsgLevel::Success,
            );
        }
    }

    pub fn run_sync(&mut self) {
        if self.syncing {
            return;
        }
        let source = self
            .config
            .watch
            .directories
            .first()
            .cloned()
            .unwrap_or_default();
        let dest = self.config.sync.destination.clone();
        let _ = self.sync_tx.send(SyncCmd::FullSync { source, dest });
        self.syncing = true;
        self.set_status("Synchronisation en cours...".to_string(), MsgLevel::Info);
    }

    pub fn search_versions(&mut self) {
        self.version_input_mode = false;
        let path = std::path::PathBuf::from(&self.version_input);
        if let Some(ref vm_arc) = self.version_manager {
            if let Ok(vm) = vm_arc.lock() {
                self.version_list = vm.get_versions(&path).into_iter().cloned().collect();
            }
        }
        self.list_selected = 0;
        let count = self.version_list.len();
        if count == 0 {
            self.set_status("Aucune version trouvée pour ce fichier".to_string(), MsgLevel::Warning);
        } else {
            self.set_status(format!("{} version(s) trouvée(s)", count), MsgLevel::Success);
        }
    }

    pub fn restore_selected_version(&mut self) {
        if self.version_list.is_empty() {
            return;
        }
        let idx = self.list_selected.min(self.version_list.len() - 1);
        let path = self.version_list[idx].original_path.clone();
        let vnum = self.version_list[idx].version_number;

        // Clone l'Arc pour libérer le borrow de self avant d'appeler set_status
        let result: Option<std::result::Result<(), String>> =
            if let Some(vm_arc) = self.version_manager.clone() {
                if let Ok(vm) = vm_arc.lock() {
                    Some(vm.restore_version(&path, vnum).map_err(|e| e.to_string()))
                } else {
                    Some(Err("Mutex poisonné".to_string()))
                }
            } else {
                None
            };

        match result {
            Some(Ok(_)) => self.set_status(
                format!("Version {} restaurée pour {}", vnum, path.display()),
                MsgLevel::Success,
            ),
            Some(Err(e)) => self.set_status(e, MsgLevel::Error),
            None => self.set_status("Versioning non activé".to_string(), MsgLevel::Warning),
        }
    }

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

    pub fn version_input_push(&mut self, c: char) {
        self.version_input.push(c);
    }

    pub fn version_input_pop(&mut self) {
        self.version_input.pop();
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
                Ok(WatcherCmd::Start) => {
                    let mut pw = PollingWatcher::new();
                    for dir in &config.watch.directories {
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
