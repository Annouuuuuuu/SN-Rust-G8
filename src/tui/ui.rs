use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
        Wrap,
    },
    Frame,
};

use super::app::{App, MsgLevel, Tab};
use crate::watcher::types::ChangeType;

// ─── Palette ──────────────────────────────────────────────────────────────────

const ACCENT: Color = Color::Rgb(0, 200, 255);
const SUCCESS: Color = Color::Rgb(80, 210, 100);
const WARNING: Color = Color::Rgb(255, 185, 0);
const ERROR: Color = Color::Rgb(255, 75, 75);
const SURFACE: Color = Color::Rgb(13, 17, 30);
const SURFACE2: Color = Color::Rgb(22, 28, 48);
const BORDER: Color = Color::Rgb(45, 80, 140);
const BORDER_ACTIVE: Color = Color::Rgb(0, 160, 220);
const MUTED: Color = Color::Rgb(100, 115, 140);
const TEXT: Color = Color::Rgb(210, 220, 235);

// ─── Point d'entrée ──────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Fond général
    f.render_widget(
        Block::default().style(Style::default().bg(SURFACE)),
        area,
    );

    // Guard taille minimale
    if area.height < 12 || area.width < 60 {
        f.render_widget(
            Paragraph::new("Terminal trop petit (min 60×12)")
                .style(Style::default().fg(WARNING).bg(SURFACE))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // tabs
            Constraint::Min(0),    // contenu
            Constraint::Length(1), // statut
            Constraint::Length(1), // footer
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_tabs(f, app, chunks[1]);

    match app.active_tab {
        Tab::Dashboard => render_dashboard(f, app, chunks[2]),
        Tab::Surveillance => render_surveillance(f, app, chunks[2]),
        Tab::Sync => render_sync(f, app, chunks[2]),
        Tab::Versions => render_versions(f, app, chunks[2]),
        Tab::Config => render_config(f, app, chunks[2]),
        Tab::Regles => render_regles(f, app, chunks[2]),
    }

    render_status_bar(f, app, chunks[3]);
    render_footer(f, app, chunks[4]);
}

// ─── Header ──────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    let (watch_label, watch_color) = if app.watching {
        ("● ACTIF", SUCCESS)
    } else {
        ("○ EN VEILLE", MUTED)
    };

    let sync_label = if app.syncing {
        let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
        let s = spinners[(app.tick_count as usize) % spinners.len()];
        format!(" {} SYNC ", s)
    } else {
        String::new()
    };

    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "◈ FileSentinel",
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " v0.2.0",
            Style::default().fg(MUTED),
        ),
        Span::raw("  │  "),
        Span::styled(watch_label, Style::default().fg(watch_color).add_modifier(Modifier::BOLD)),
        if !sync_label.is_empty() {
            Span::styled(sync_label, Style::default().fg(WARNING).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("")
        },
        Span::raw("  │  "),
        Span::styled(now, Style::default().fg(MUTED)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(SURFACE2));

    f.render_widget(Paragraph::new(title).block(block), area);
}

// ─── Tabs ─────────────────────────────────────────────────────────────────────

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            Line::from(vec![
                Span::styled(
                    format!(" [{}] ", i + 1),
                    Style::default().fg(MUTED),
                ),
                Span::styled(t.title(), Style::default().fg(TEXT)),
                Span::raw(" "),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .select(app.active_tab.index())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        )
        .style(Style::default().fg(MUTED).bg(SURFACE2))
        .highlight_style(
            Style::default()
                .fg(ACCENT)
                .bg(SURFACE)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("│", Style::default().fg(BORDER)));

    f.render_widget(tabs, area);
}

// ─── Dashboard ────────────────────────────────────────────────────────────────

fn render_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(cols[0]);

    render_status_panel(f, app, left_rows[0]);
    render_dirs_panel(f, app, left_rows[1]);

    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(cols[1]);

    render_stats_panel(f, app, right_rows[0]);
    render_events_panel(f, app, right_rows[1]);
}

fn render_status_panel(f: &mut Frame, app: &App, area: Rect) {
    let (watch_str, watch_color) = if app.watching {
        ("ACTIVE", SUCCESS)
    } else {
        ("EN VEILLE", MUTED)
    };

    let rows = vec![
        Row::new(vec![
            Cell::from("Surveillance").style(Style::default().fg(MUTED)),
            Cell::from(watch_str).style(Style::default().fg(watch_color).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from("Dossiers").style(Style::default().fg(MUTED)),
            Cell::from(app.config.watch.directories.len().to_string())
                .style(Style::default().fg(TEXT)),
        ]),
        Row::new(vec![
            Cell::from("Destination").style(Style::default().fg(MUTED)),
            Cell::from(truncate(&app.config.sync.destination, 24))
                .style(Style::default().fg(TEXT)),
        ]),
        Row::new(vec![
            Cell::from("Intervalle").style(Style::default().fg(MUTED)),
            Cell::from(format!("{}ms", app.config.watch.polling_interval_ms))
                .style(Style::default().fg(TEXT)),
        ]),
        Row::new(vec![
            Cell::from("Versioning").style(Style::default().fg(MUTED)),
            Cell::from(if app.config.versioning.enabled { "Activé" } else { "Désactivé" })
                .style(Style::default().fg(if app.config.versioning.enabled { SUCCESS } else { MUTED })),
        ]),
        Row::new(vec![
            Cell::from("Événements").style(Style::default().fg(MUTED)),
            Cell::from(app.events.len().to_string()).style(Style::default().fg(TEXT)),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Percentage(40), Constraint::Percentage(60)])
        .block(
            Block::default()
                .title(Span::styled(" Statut ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        );

    f.render_widget(table, area);
}

fn render_dirs_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .config
        .watch
        .directories
        .iter()
        .map(|d| {
            ListItem::new(Line::from(vec![
                Span::styled("  📁 ", Style::default().fg(ACCENT)),
                Span::styled(truncate(d, 30), Style::default().fg(TEXT)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                " Dossiers surveillés ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(SURFACE2)),
    );

    f.render_widget(list, area);
}

fn render_stats_panel(f: &mut Frame, app: &App, area: Rect) {
    let rows = if let Some(ref s) = app.last_stats {
        vec![
            Row::new(vec![
                Cell::from("Fichiers copiés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_copied.to_string()).style(Style::default().fg(SUCCESS)),
            ]),
            Row::new(vec![
                Cell::from("Créés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_created.to_string()).style(Style::default().fg(ACCENT)),
            ]),
            Row::new(vec![
                Cell::from("Supprimés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_deleted.to_string()).style(Style::default().fg(ERROR)),
            ]),
            Row::new(vec![
                Cell::from("Ignorés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_skipped.to_string()).style(Style::default().fg(MUTED)),
            ]),
            Row::new(vec![
                Cell::from("Données").style(Style::default().fg(MUTED)),
                Cell::from(format!("{:.2} MB", s.total_bytes_transferred as f64 / 1_000_000.0))
                    .style(Style::default().fg(TEXT)),
            ]),
            Row::new(vec![
                Cell::from("Durée").style(Style::default().fg(MUTED)),
                Cell::from(format!("{}ms", s.duration_ms)).style(Style::default().fg(TEXT)),
            ]),
            Row::new(vec![
                Cell::from("Erreurs").style(Style::default().fg(MUTED)),
                Cell::from(s.errors.len().to_string())
                    .style(Style::default().fg(if s.errors.is_empty() { SUCCESS } else { ERROR })),
            ]),
        ]
    } else {
        vec![Row::new(vec![
            Cell::from("Aucune sync effectuée").style(Style::default().fg(MUTED)),
            Cell::from(""),
        ])]
    };

    let table = Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
        .block(
            Block::default()
                .title(Span::styled(
                    " Dernière synchronisation ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        );

    f.render_widget(table, area);
}

fn render_events_panel(f: &mut Frame, app: &App, area: Rect) {
    let items = build_event_items(app, 50);

    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.list_selected.min(items.len() - 1)));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    " Événements récents ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        )
        .highlight_style(Style::default().bg(Color::Rgb(30, 45, 70)).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, area, &mut state);
}

// ─── Surveillance ─────────────────────────────────────────────────────────────

fn render_surveillance(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Panel supérieur : dossiers + commande
    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    // Dossiers
    let dirs: Vec<ListItem> = app
        .config
        .watch
        .directories
        .iter()
        .map(|d| {
            let color = if app.watching { SUCCESS } else { MUTED };
            ListItem::new(Line::from(vec![
                Span::styled(if app.watching { "  ● " } else { "  ○ " }, Style::default().fg(color)),
                Span::styled(truncate(d, 35), Style::default().fg(TEXT)),
            ]))
        })
        .collect();

    f.render_widget(
        List::new(dirs).block(
            Block::default()
                .title(Span::styled(
                    " Dossiers surveillés ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if app.watching { BORDER_ACTIVE } else { BORDER }))
                .style(Style::default().bg(SURFACE2)),
        ),
        top_cols[0],
    );

    // Commandes rapides
    let (btn_label, btn_color) = if app.watching {
        ("[W] Arrêter la surveillance", ERROR)
    } else {
        ("[W] Démarrer la surveillance", SUCCESS)
    };
    let hint = Paragraph::new(vec![
        Line::from(vec![Span::styled(btn_label, Style::default().fg(btn_color).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("[S] Synchroniser maintenant", Style::default().fg(ACCENT))]),
        Line::from(vec![Span::styled("[↑↓] Naviguer les événements", Style::default().fg(MUTED))]),
    ])
    .block(
        Block::default()
            .title(Span::styled(" Commandes ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(SURFACE2)),
    );
    f.render_widget(hint, top_cols[1]);

    // Flux d'événements
    let items = build_event_items(app, 300);
    let count = items.len();
    let title = format!(" Flux d'événements ({}) ", count);

    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.list_selected.min(items.len() - 1)));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if app.watching { BORDER_ACTIVE } else { BORDER }))
                .style(Style::default().bg(SURFACE2)),
        )
        .highlight_style(Style::default().bg(Color::Rgb(30, 45, 70)).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, rows[1], &mut state);
}

// ─── Synchronisation ──────────────────────────────────────────────────────────

fn render_sync(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(cols[0]);

    // Info source/dest
    let source = app
        .config
        .watch
        .directories
        .first()
        .cloned()
        .unwrap_or_else(|| "-".to_string());
    let dest = &app.config.sync.destination;

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Source      ", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}", truncate(&source, 30)), Style::default().fg(ACCENT)),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Destination ", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}", truncate(dest, 30)), Style::default().fg(ACCENT)),
        ]),
    ])
    .block(
        Block::default()
            .title(Span::styled(" Source & Destination ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(SURFACE2)),
    );
    f.render_widget(info, left_rows[0]);

    // Bouton sync
    let (btn_label, btn_color, btn_bg) = if app.syncing {
        let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
        let s = spinners[(app.tick_count as usize) % spinners.len()];
        (format!("{} Synchronisation en cours...", s), WARNING, Color::Rgb(40, 35, 0))
    } else {
        ("[S] Lancer la synchronisation complète".to_string(), SUCCESS, Color::Rgb(0, 30, 10))
    };

    let btn = Paragraph::new(vec![
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            &btn_label,
            Style::default().fg(btn_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            "Copie les fichiers modifiés de la source",
            Style::default().fg(MUTED),
        )),
        Line::from(Span::styled(
            "vers la destination.",
            Style::default().fg(MUTED),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title(Span::styled(" Action ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if app.syncing { WARNING } else { BORDER }))
            .style(Style::default().bg(btn_bg)),
    );
    f.render_widget(btn, left_rows[1]);

    // Résultats de la dernière sync
    if let Some(ref s) = app.last_stats {
        let rows = vec![
            Row::new(vec![
                Cell::from("Fichiers copiés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_copied.to_string()).style(Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)),
            ]),
            Row::new(vec![
                Cell::from("Fichiers créés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_created.to_string()).style(Style::default().fg(ACCENT)),
            ]),
            Row::new(vec![
                Cell::from("Fichiers supprimés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_deleted.to_string()).style(Style::default().fg(ERROR)),
            ]),
            Row::new(vec![
                Cell::from("Fichiers ignorés").style(Style::default().fg(MUTED)),
                Cell::from(s.files_skipped.to_string()).style(Style::default().fg(MUTED)),
            ]),
            Row::new(vec![
                Cell::from("Données transférées").style(Style::default().fg(MUTED)),
                Cell::from(format!("{:.2} MB", s.total_bytes_transferred as f64 / 1_000_000.0))
                    .style(Style::default().fg(TEXT)),
            ]),
            Row::new(vec![
                Cell::from("Durée totale").style(Style::default().fg(MUTED)),
                Cell::from(format!("{}ms", s.duration_ms)).style(Style::default().fg(TEXT)),
            ]),
            Row::new(vec![
                Cell::from("Erreurs").style(Style::default().fg(MUTED)),
                Cell::from(s.errors.len().to_string())
                    .style(Style::default().fg(if s.errors.is_empty() { SUCCESS } else { ERROR })),
            ]),
        ];

        let table = Table::new(rows, [Constraint::Percentage(55), Constraint::Percentage(45)])
            .block(
                Block::default()
                    .title(Span::styled(
                        " Résultats ",
                        Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(SUCCESS))
                    .style(Style::default().bg(SURFACE2)),
            );

        f.render_widget(table, cols[1]);
    } else {
        let placeholder = Paragraph::new(vec![
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  Aucune synchronisation effectuée.",
                Style::default().fg(MUTED),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  Appuyez sur [S] pour lancer",
                Style::default().fg(MUTED),
            )),
            Line::from(Span::styled(
                "  la synchronisation complète.",
                Style::default().fg(MUTED),
            )),
        ])
        .block(
            Block::default()
                .title(Span::styled(
                    " Résultats ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        );
        f.render_widget(placeholder, cols[1]);
    }
}

// ─── Versions ─────────────────────────────────────────────────────────────────

fn render_versions(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Input
    let input_display = if app.version_input_mode {
        format!("{}│", app.version_input)
    } else if app.version_input.is_empty() {
        "Appuyez sur [E] ou [Entrée] pour saisir un chemin...".to_string()
    } else {
        app.version_input.clone()
    };

    let input_border_color = if app.version_input_mode {
        BORDER_ACTIVE
    } else {
        BORDER
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                input_display,
                Style::default().fg(if app.version_input_mode { ACCENT } else { MUTED }),
            ),
        ]))
        .block(
            Block::default()
                .title(Span::styled(
                    " Chemin du fichier ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(input_border_color))
                .style(Style::default().bg(SURFACE2)),
        ),
        rows[0],
    );

    // Liste des versions
    let items: Vec<ListItem> = app
        .version_list
        .iter()
        .map(|v| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  v{:<4}", v.version_number),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "  {}",
                        v.timestamp.format("%Y-%m-%d %H:%M:%S")
                    ),
                    Style::default().fg(MUTED),
                ),
                Span::styled(
                    format!("  {:>10}", v.format_size()),
                    Style::default().fg(TEXT),
                ),
                Span::styled(
                    format!("  #{:.8}", v.hash),
                    Style::default().fg(MUTED),
                ),
            ]))
        })
        .collect();

    let count = items.len();
    let list_title = if count > 0 {
        format!(" {} version(s) trouvée(s) ", count)
    } else {
        " Historique des versions ".to_string()
    };

    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.list_selected.min(items.len() - 1)));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    list_title,
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        )
        .highlight_style(Style::default().bg(Color::Rgb(30, 45, 70)).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, rows[1], &mut state);

    // Aide contextuelle
    let help = if app.version_input_mode {
        "[Entrée] Rechercher  [Échap] Annuler  [Retour arrière] Effacer"
    } else if !app.version_list.is_empty() {
        "[E] Nouveau chemin  [↑↓] Sélectionner  [R] Restaurer la version sélectionnée"
    } else {
        "[E] Saisir un chemin de fichier  [Entrée] Confirmer la saisie"
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            help,
            Style::default().fg(MUTED),
        )]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        ),
        rows[2],
    );
}

// ─── Configuration ────────────────────────────────────────────────────────────

fn render_config(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.config;

    let mut lines: Vec<Line> = Vec::new();

    let section = |name: &str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("[{}]", name),
                Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let kv = |key: &str, value: String| {
        Line::from(vec![
            Span::styled(format!("  {:<35}", key), Style::default().fg(MUTED)),
            Span::styled(value, Style::default().fg(TEXT)),
        ])
    };

    lines.push(section("watch"));
    for d in &c.watch.directories {
        lines.push(kv("  directories", format!("\"{}\"", d)));
    }
    lines.push(kv("  polling_interval_ms", c.watch.polling_interval_ms.to_string()));

    lines.push(Line::from(""));
    lines.push(section("sync"));
    lines.push(kv("  destination", format!("\"{}\"", c.sync.destination)));
    lines.push(kv("  create_backups", c.sync.create_backups.to_string()));
    lines.push(kv("  max_concurrent_operations", c.sync.max_concurrent_operations.to_string()));

    lines.push(Line::from(""));
    lines.push(section("versioning"));
    lines.push(kv("  enabled", c.versioning.enabled.to_string()));
    lines.push(kv("  max_versions", c.versioning.max_versions.to_string()));
    lines.push(kv("  versions_dir", format!("\"{}\"", c.versioning.versions_dir.display())));
    lines.push(kv("  auto_version_on_change", c.versioning.auto_version_on_change.to_string()));

    lines.push(Line::from(""));
    lines.push(section("compression"));
    lines.push(kv("  enabled", c.compression.enabled.to_string()));
    lines.push(kv("  level", c.compression.level.to_string()));
    lines.push(kv(
        "  min_file_size_for_compression",
        format!("{} B", c.compression.min_file_size_for_compression),
    ));

    lines.push(Line::from(""));
    lines.push(section("notifications"));
    lines.push(kv("  enabled", c.notifications.enabled.to_string()));
    lines.push(kv("  show_batch_summary", c.notifications.show_batch_summary.to_string()));
    lines.push(kv("  min_interval_seconds", c.notifications.min_interval_seconds.to_string()));

    if let Some(ref net) = c.network {
        lines.push(Line::from(""));
        lines.push(section("network"));
        lines.push(kv("  host", format!("\"{}\"", net.host)));
        lines.push(kv("  port", net.port.to_string()));
        lines.push(kv("  username", format!("\"{}\"", net.username)));
        lines.push(kv("  remote_path", format!("\"{}\"", net.remote_path.display())));
    }

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(
                    " Configuration (config.toml) ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

// ─── Règles ───────────────────────────────────────────────────────────────────

fn render_regles(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Patterns d'exclusion
    let patterns: Vec<ListItem> = app
        .config
        .filters
        .exclude_patterns
        .iter()
        .map(|p| {
            ListItem::new(Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(ERROR)),
                Span::styled(p.as_str(), Style::default().fg(TEXT)),
            ]))
        })
        .collect();

    f.render_widget(
        List::new(patterns).block(
            Block::default()
                .title(Span::styled(
                    " Patterns d'exclusion ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        ),
        cols[0],
    );

    // Autres règles
    let mut other_lines: Vec<Line> = Vec::new();

    other_lines.push(Line::from(vec![
        Span::styled("  Taille max fichier  ", Style::default().fg(MUTED)),
        Span::styled(
            app.config
                .filters
                .max_file_size_mb
                .map(|m| format!("{} MB", m))
                .unwrap_or_else(|| "illimitée".to_string()),
            Style::default().fg(TEXT),
        ),
    ]));

    other_lines.push(Line::from(""));

    if app.config.filters.include_extensions.is_empty() {
        other_lines.push(Line::from(vec![
            Span::styled("  Extensions incluses ", Style::default().fg(MUTED)),
            Span::styled("toutes", Style::default().fg(MUTED)),
        ]));
    } else {
        other_lines.push(Line::from(vec![Span::styled(
            "  Extensions incluses:",
            Style::default().fg(MUTED),
        )]));
        for ext in &app.config.filters.include_extensions {
            other_lines.push(Line::from(vec![
                Span::styled("    ✓ ", Style::default().fg(SUCCESS)),
                Span::styled(ext.as_str(), Style::default().fg(TEXT)),
            ]));
        }
    }

    other_lines.push(Line::from(""));
    other_lines.push(Line::from(vec![Span::styled(
        "  Patterns critiques (notifications):",
        Style::default().fg(MUTED),
    )]));
    for pat in app.config.notifications.critical_patterns.iter().take(10) {
        other_lines.push(Line::from(vec![
            Span::styled("    ⚠ ", Style::default().fg(WARNING)),
            Span::styled(pat.as_str(), Style::default().fg(TEXT)),
        ]));
    }
    if app.config.notifications.critical_patterns.len() > 10 {
        other_lines.push(Line::from(vec![Span::styled(
            format!(
                "    ... et {} autres",
                app.config.notifications.critical_patterns.len() - 10
            ),
            Style::default().fg(MUTED),
        )]));
    }

    f.render_widget(
        Paragraph::new(other_lines)
            .block(
                Block::default()
                    .title(Span::styled(
                        " Autres règles ",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER))
                    .style(Style::default().bg(SURFACE2)),
            )
            .wrap(Wrap { trim: false }),
        cols[1],
    );
}

// ─── Barre de statut ──────────────────────────────────────────────────────────

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (text, color) = if let Some((ref msg, level, _)) = app.status {
        let col = match level {
            MsgLevel::Info => ACCENT,
            MsgLevel::Success => SUCCESS,
            MsgLevel::Warning => WARNING,
            MsgLevel::Error => ERROR,
        };
        let prefix = match level {
            MsgLevel::Info => "ℹ ",
            MsgLevel::Success => "✓ ",
            MsgLevel::Warning => "⚠ ",
            MsgLevel::Error => "✗ ",
        };
        (format!(" {}{}  ", prefix, msg), col)
    } else {
        (String::new(), MUTED)
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            text,
            Style::default().fg(color),
        )]))
        .style(Style::default().bg(SURFACE2)),
        area,
    );
}

// ─── Footer ───────────────────────────────────────────────────────────────────

fn render_footer(f: &mut Frame, _app: &App, area: Rect) {
    let spans = vec![
        Span::styled(" Tab ", Style::default().fg(SURFACE2).bg(ACCENT)),
        Span::styled(" Onglets  ", Style::default().fg(MUTED)),
        Span::styled(" W ", Style::default().fg(SURFACE2).bg(SUCCESS)),
        Span::styled(" Surveiller  ", Style::default().fg(MUTED)),
        Span::styled(" S ", Style::default().fg(SURFACE2).bg(ACCENT)),
        Span::styled(" Synchroniser  ", Style::default().fg(MUTED)),
        Span::styled(" ↑↓ ", Style::default().fg(SURFACE2).bg(MUTED)),
        Span::styled(" Naviguer  ", Style::default().fg(MUTED)),
        Span::styled(" 1-6 ", Style::default().fg(SURFACE2).bg(Color::Rgb(80, 60, 140))),
        Span::styled(" Onglet direct  ", Style::default().fg(MUTED)),
        Span::styled(" Q ", Style::default().fg(SURFACE2).bg(ERROR)),
        Span::styled(" Quitter", Style::default().fg(MUTED)),
    ];

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE)),
        area,
    );
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn build_event_items(app: &App, limit: usize) -> Vec<ListItem<'static>> {
    app.events
        .iter()
        .take(limit)
        .map(|e| {
            let (symbol, color) = match e.change_type {
                ChangeType::Created => ("+", SUCCESS),
                ChangeType::Modified => ("~", WARNING),
                ChangeType::Deleted => ("✗", ERROR),
            };
            let filename = e
                .file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let parent = e
                .file_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| format!("{}/", n.to_string_lossy()))
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", symbol),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<10} ", e.change_type.to_string()),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("{}", e.timestamp.format("%H:%M:%S")),
                    Style::default().fg(MUTED),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(parent, Style::default().fg(MUTED)),
                Span::styled(filename, Style::default().fg(TEXT)),
            ]))
        })
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("...{}", &s[s.len().saturating_sub(max - 3)..])
    }
}
