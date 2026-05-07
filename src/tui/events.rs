use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::app::{App, InputMode, Tab};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Ctrl+C global
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // ─── Mode saisie (overlay) ───────────────────────────────────────────────
    if app.input_mode.is_active() {
        match key.code {
            KeyCode::Enter => app.confirm_input(),
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Backspace => app.input_pop(),
            KeyCode::Char(c) => app.input_push(c),
            _ => {}
        }
        return;
    }

    // ─── Navigation globale ──────────────────────────────────────────────────
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            return;
        }

        // Onglets par numéro
        KeyCode::Char('1') => { app.set_tab(Tab::Dashboard); return; }
        KeyCode::Char('2') => { app.set_tab(Tab::Surveillance); return; }
        KeyCode::Char('3') => { app.set_tab(Tab::Sync); return; }
        KeyCode::Char('4') => { app.set_tab(Tab::Versions); return; }
        KeyCode::Char('5') => { app.set_tab(Tab::Config); return; }
        KeyCode::Char('6') => { app.set_tab(Tab::Regles); return; }

        // Onglets séquentiels
        KeyCode::Tab => { let t = app.active_tab.next(); app.set_tab(t); return; }
        KeyCode::BackTab => { let t = app.active_tab.prev(); app.set_tab(t); return; }

        // Navigation liste
        KeyCode::Up | KeyCode::Char('k') => { app.scroll_up(); return; }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = list_max(app);
            app.scroll_down(max);
            return;
        }

        _ => {}
    }

    // ─── Actions par onglet ──────────────────────────────────────────────────
    match app.active_tab {
        Tab::Dashboard => handle_dashboard(app, key),
        Tab::Surveillance => handle_surveillance(app, key),
        Tab::Sync => handle_sync(app, key),
        Tab::Versions => handle_versions(app, key),
        Tab::Config | Tab::Regles => {}
    }
}

fn handle_dashboard(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('w') | KeyCode::Char('W') => app.toggle_watch(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.run_sync(),
        _ => {}
    }
}

fn handle_surveillance(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('w') | KeyCode::Char('W') => app.toggle_watch(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.run_sync(),
        // Ajouter un dossier
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.open_input(InputMode::AddWatchDir, "");
        }
        // Supprimer le dossier sélectionné
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            app.remove_watch_dir();
        }
        _ => {}
    }
}

fn handle_sync(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') | KeyCode::Char('S') => app.run_sync(),
        KeyCode::Char('x') | KeyCode::Char('X') => app.cancel_sync(),
        // Modifier la destination
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let current = app.config.sync.destination.clone();
            app.open_input(InputMode::EditDestination, &current);
        }
        _ => {}
    }
}

fn handle_versions(app: &mut App, key: KeyEvent) {
    match key.code {
        // Rechercher les versions d'un fichier
        KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
            let current = app.input_value.clone();
            app.open_input(InputMode::VersionSearch, &current);
        }
        // Restaurer à l'emplacement original
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.restore_selected_original();
        }
        // Restaurer dans un dossier au choix
        KeyCode::Char('f') | KeyCode::Char('F') => {
            if !app.version_list.is_empty() {
                app.open_input(InputMode::RestoreToFolder, "");
            }
        }
        // Supprimer la version sélectionnée
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            app.delete_selected_version();
        }
        // Nettoyer : garder seulement N versions
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if !app.version_list.is_empty() {
                app.open_input(InputMode::CleanVersions, "3");
            }
        }
        _ => {}
    }
}

fn list_max(app: &App) -> usize {
    match app.active_tab {
        Tab::Dashboard => app.events.len().min(50),
        Tab::Surveillance => app.config.watch.directories.len(),
        Tab::Versions => app.version_list.len(),
        Tab::Config => 30,
        Tab::Regles => app.config.filters.exclude_patterns.len() + 5,
        Tab::Sync => 0,
    }
}
