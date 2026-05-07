pub mod app;
mod events;
mod ui;

pub use app::App;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use crate::config::settings::Config;
use crate::errors::{FileSentinelError, Result};

pub fn run_dashboard(config: Config) -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(FileSentinelError::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(FileSentinelError::Io)?;

    let mut app = App::new(config);

    let result = run_loop(&mut terminal, &mut app);

    // Restauration du terminal — toujours effectuée même en cas d'erreur
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Traiter les messages des threads background
        app.tick();

        // Rendre l'interface
        terminal.draw(|f| ui::render(f, app)).map_err(FileSentinelError::Io)?;

        // Lire les événements clavier (non-bloquant, timeout 50ms)
        if event::poll(Duration::from_millis(50)).map_err(FileSentinelError::Io)? {
            if let Ok(Event::Key(key)) = event::read() {
                // Filtrer les key-release (Windows génère Press + Release)
                if key.kind == KeyEventKind::Press {
                    events::handle_key(app, key);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
