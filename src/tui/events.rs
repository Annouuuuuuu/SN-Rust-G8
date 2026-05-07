use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::app::{App, Tab};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Ignorer les key-release (Windows envoie Press + Release)
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Ctrl+C global
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Mode saisie (onglet Versions)
    if app.version_input_mode {
        match key.code {
            KeyCode::Enter => app.search_versions(),
            KeyCode::Esc => {
                app.version_input_mode = false;
            }
            KeyCode::Backspace => app.version_input_pop(),
            KeyCode::Char(c) => app.version_input_push(c),
            _ => {}
        }
        return;
    }

    // Navigation globale (sauf mode saisie)
    match key.code {
        // Quitter
        KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,

        // Navigation onglets par numéro
        KeyCode::Char('1') => app.set_tab(Tab::Dashboard),
        KeyCode::Char('2') => app.set_tab(Tab::Surveillance),
        KeyCode::Char('3') => app.set_tab(Tab::Sync),
        KeyCode::Char('4') => app.set_tab(Tab::Versions),
        KeyCode::Char('5') => app.set_tab(Tab::Config),
        KeyCode::Char('6') => app.set_tab(Tab::Regles),

        // Navigation onglets séquentielle
        KeyCode::Tab => {
            let next = app.active_tab.next();
            app.set_tab(next);
        }
        KeyCode::BackTab => {
            let prev = app.active_tab.prev();
            app.set_tab(prev);
        }

        // Navigation liste
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = list_max(app);
            app.scroll_down(max);
        }

        // Actions
        KeyCode::Char('w') | KeyCode::Char('W') => app.toggle_watch(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.run_sync(),

        // Onglet Versions
        KeyCode::Enter => match app.active_tab {
            Tab::Versions => {
                if app.version_list.is_empty() {
                    app.version_input_mode = true;
                } else {
                    app.version_input_mode = true;
                }
            }
            _ => {}
        },
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if app.active_tab == Tab::Versions {
                app.version_input_mode = true;
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if app.active_tab == Tab::Versions {
                app.restore_selected_version();
            }
        }

        _ => {}
    }
}

fn list_max(app: &App) -> usize {
    match app.active_tab {
        super::app::Tab::Dashboard | super::app::Tab::Surveillance => app.events.len(),
        super::app::Tab::Versions => app.version_list.len(),
        super::app::Tab::Config => 20,
        super::app::Tab::Regles => app.config.filters.exclude_patterns.len() + 2,
        super::app::Tab::Sync => 0,
    }
}
