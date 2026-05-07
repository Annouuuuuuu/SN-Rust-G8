use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table,
        Tabs, Wrap,
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

// ─── Point d'entrée ───────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    f.render_widget(Block::default().style(Style::default().bg(SURFACE)), area);

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
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
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

    // Overlay de saisie (rendu en dernier, par-dessus tout)
    if app.input_mode.is_active() {
        render_input_overlay(f, app, area);
    }
}

// ─── Header ───────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    let (watch_label, watch_color) = if app.watching {
        ("● ACTIF", SUCCESS)
    } else {
        ("○ EN VEILLE", MUTED)
    };

    let sync_part = if app.syncing {
        let sp = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
        format!("  {} SYNC EN COURS", sp[(app.tick_count as usize) % sp.len()])
    } else {
        String::new()
    };

    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled("◈ FileSentinel", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" v0.2.0", Style::default().fg(MUTED)),
        Span::raw("  │  "),
        Span::styled(watch_label, Style::default().fg(watch_color).add_modifier(Modifier::BOLD)),
        Span::styled(sync_part, Style::default().fg(WARNING).add_modifier(Modifier::BOLD)),
        Span::raw("  │  "),
        Span::styled(now, Style::default().fg(MUTED)),
    ]);

    f.render_widget(
        Paragraph::new(title).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(SURFACE2)),
        ),
        area,
    );
}

// ─── Tabs ─────────────────────────────────────────────────────────────────────

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            Line::from(vec![
                Span::styled(format!(" [{}] ", i + 1), Style::default().fg(MUTED)),
                Span::styled(t.title(), Style::default().fg(TEXT)),
                Span::raw(" "),
            ])
        })
        .collect();

    f.render_widget(
        Tabs::new(titles)
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
            .divider(Span::styled("│", Style::default().fg(BORDER))),
        area,
    );
}

// ─── Dashboard ────────────────────────────────────────────────────────────────

fn render_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(cols[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(cols[1]);

    render_status_panel(f, app, left[0]);
    render_dirs_panel(f, app, left[1]);
    render_stats_panel(f, app, right[0]);
    render_events_panel(f, app, right[1]);
}

fn render_status_panel(f: &mut Frame, app: &App, area: Rect) {
    let (ws, wc) = if app.watching { ("ACTIVE", SUCCESS) } else { ("EN VEILLE", MUTED) };
    let rows = vec![
        Row::new(vec![
            Cell::from("Surveillance").style(Style::default().fg(MUTED)),
            Cell::from(ws).style(Style::default().fg(wc).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from("Dossiers").style(Style::default().fg(MUTED)),
            Cell::from(app.config.watch.directories.len().to_string()).style(Style::default().fg(TEXT)),
        ]),
        Row::new(vec![
            Cell::from("Destination").style(Style::default().fg(MUTED)),
            Cell::from(truncate(&app.config.sync.destination, 24)).style(Style::default().fg(TEXT)),
        ]),
        Row::new(vec![
            Cell::from("Intervalle").style(Style::default().fg(MUTED)),
            Cell::from(format!("{}ms", app.config.watch.polling_interval_ms)).style(Style::default().fg(TEXT)),
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

    f.render_widget(
        Table::new(rows, [Constraint::Percentage(40), Constraint::Percentage(60)])
            .block(styled_block(" Statut ", BORDER)),
        area,
    );
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

    f.render_widget(
        List::new(items).block(styled_block(" Dossiers ", BORDER)),
        area,
    );
}

fn render_stats_panel(f: &mut Frame, app: &App, area: Rect) {
    let rows = if let Some(ref s) = app.last_stats {
        vec![
            Row::new(vec![
                Cell::from("Copiés").style(Style::default().fg(MUTED)),
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
            Cell::from("Aucune sync").style(Style::default().fg(MUTED)),
            Cell::from(""),
        ])]
    };

    f.render_widget(
        Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
            .block(styled_block(" Dernière sync ", BORDER)),
        area,
    );
}

fn render_events_panel(f: &mut Frame, app: &App, area: Rect) {
    let items = build_event_items(app, 50);
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.list_selected.min(items.len() - 1)));
    }

    f.render_stateful_widget(
        List::new(items)
            .block(styled_block(" Événements récents ", BORDER))
            .highlight_style(Style::default().bg(Color::Rgb(30, 45, 70)).add_modifier(Modifier::BOLD))
            .highlight_symbol("► "),
        area,
        &mut state,
    );
}

// ─── Surveillance ─────────────────────────────────────────────────────────────

fn render_surveillance(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[0]);

    // Dossiers surveillés (liste sélectionnable)
    let dir_items: Vec<ListItem> = app
        .config
        .watch
        .directories
        .iter()
        .map(|d| {
            let col = if app.watching { SUCCESS } else { MUTED };
            ListItem::new(Line::from(vec![
                Span::styled(if app.watching { "  ● " } else { "  ○ " }, Style::default().fg(col)),
                Span::styled(truncate(d, 38), Style::default().fg(TEXT)),
            ]))
        })
        .collect();

    let mut dir_state = ListState::default();
    if !dir_items.is_empty() {
        dir_state.select(Some(app.list_selected.min(dir_items.len() - 1)));
    }

    let dir_border = if app.watching { BORDER_ACTIVE } else { BORDER };
    f.render_stateful_widget(
        List::new(dir_items)
            .block(
                Block::default()
                    .title(Span::styled(
                        " Dossiers surveillés  [A] Ajouter  [D] Retirer ",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(dir_border))
                    .style(Style::default().bg(SURFACE2)),
            )
            .highlight_style(Style::default().bg(Color::Rgb(30, 45, 70)).add_modifier(Modifier::BOLD))
            .highlight_symbol("► "),
        top[0],
        &mut dir_state,
    );

    // Panneau commandes
    let (btn_w, btn_c) = if app.watching {
        ("[W] Arrêter la surveillance", ERROR)
    } else {
        ("[W] Démarrer la surveillance", SUCCESS)
    };

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(btn_w, Style::default().fg(btn_c).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("[S] Synchroniser maintenant", Style::default().fg(ACCENT))),
            Line::from(Span::styled("[A] Ajouter un dossier", Style::default().fg(SUCCESS))),
            Line::from(Span::styled("[D] Retirer le dossier sélectionné", Style::default().fg(ERROR))),
            Line::from(Span::styled("[↑↓] Sélectionner un dossier", Style::default().fg(MUTED))),
        ])
        .block(styled_block(" Commandes ", BORDER)),
        top[1],
    );

    // Flux d'événements (défile en continu, les plus récents en haut)
    let items = build_event_items(app, 300);
    let count = items.len();

    f.render_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!(" Flux d'événements ({}) — les plus récents en haut ", count),
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(if app.watching { BORDER_ACTIVE } else { BORDER }))
                    .style(Style::default().bg(SURFACE2)),
            ),
        rows[1],
    );
}

// ─── Synchronisation ──────────────────────────────────────────────────────────

fn render_sync(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Length(7), Constraint::Min(0)])
        .split(cols[0]);

    // Source
    let sources: Vec<ListItem> = app
        .config
        .watch
        .directories
        .iter()
        .map(|d| ListItem::new(Line::from(vec![
            Span::styled("  → ", Style::default().fg(ACCENT)),
            Span::styled(truncate(d, 28), Style::default().fg(TEXT)),
        ])))
        .collect();

    f.render_widget(
        List::new(sources).block(styled_block(" Source(s) ", BORDER)),
        left[0],
    );

    // Destination (éditable)
    let dest_border = BORDER;
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    truncate(&app.config.sync.destination, 30),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  [E] Modifier la destination",
                Style::default().fg(SUCCESS),
            )),
        ])
        .block(
            Block::default()
                .title(Span::styled(
                    " Destination de sync ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(dest_border))
                .style(Style::default().bg(SURFACE2)),
        ),
        left[1],
    );

    // Bouton sync / annuler
    let action_lines = if app.syncing {
        let sp = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
        let spinner = sp[(app.tick_count as usize) % sp.len()];
        vec![
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                format!("{} Synchronisation en cours...", spinner),
                Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "[X] Annuler la synchronisation",
                Style::default().fg(ERROR),
            )),
        ]
    } else {
        vec![
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "[S] Lancer la synchronisation complète",
                Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
            )),
        ]
    };

    let (action_bg, action_border) = if app.syncing {
        (Color::Rgb(40, 35, 0), WARNING)
    } else {
        (Color::Rgb(0, 30, 10), BORDER)
    };

    f.render_widget(
        Paragraph::new(action_lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(Span::styled(" Action ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(action_border))
                .style(Style::default().bg(action_bg)),
        ),
        left[2],
    );

    // Résultats
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
        ];

        f.render_widget(
            Table::new(rows, [Constraint::Percentage(55), Constraint::Percentage(45)])
                .block(
                    Block::default()
                        .title(Span::styled(" Résultats ", Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(SUCCESS))
                        .style(Style::default().bg(SURFACE2)),
                ),
            cols[1],
        );
    } else {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Aucune synchronisation effectuée.", Style::default().fg(MUTED))),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  [S] pour lancer la synchronisation", Style::default().fg(MUTED))),
                Line::from(Span::styled("  [E] pour changer la destination", Style::default().fg(MUTED))),
            ])
            .block(styled_block(" Résultats ", BORDER)),
            cols[1],
        );
    }
}

// ─── Versions ─────────────────────────────────────────────────────────────────

fn render_versions(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(5)])
        .split(area);

    // Fichier courant
    let file_display = if app.input_value.is_empty() {
        "Appuyez sur [E] pour chercher les versions d'un fichier...".to_string()
    } else {
        app.input_value.clone()
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  📄 ", Style::default().fg(ACCENT)),
            Span::styled(file_display, Style::default().fg(TEXT)),
        ]))
        .block(styled_block(" Fichier  [E] Chercher ", BORDER)),
        layout[0],
    );

    // Liste des versions
    let items: Vec<ListItem> = app
        .version_list
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let marker = if i == app.list_selected { "► " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(ACCENT)),
                Span::styled(
                    format!("v{:<4}", v.version_number),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}  ", v.timestamp.format("%Y-%m-%d  %H:%M:%S")),
                    Style::default().fg(MUTED),
                ),
                Span::styled(
                    format!("{:>10}  ", v.format_size()),
                    Style::default().fg(TEXT),
                ),
                Span::styled(
                    format!("#{:.8}", v.hash),
                    Style::default().fg(Color::Rgb(70, 90, 120)),
                ),
            ]))
        })
        .collect();

    let count = items.len();
    let title = if count > 0 {
        format!(" {} version(s) — [↑↓] Sélectionner ", count)
    } else {
        " Historique des versions ".to_string()
    };

    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.list_selected.min(items.len() - 1)));
    }

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER))
                    .style(Style::default().bg(SURFACE2)),
            )
            .highlight_style(Style::default().bg(Color::Rgb(30, 45, 70)).add_modifier(Modifier::BOLD)),
        layout[1],
        &mut state,
    );

    // Panel d'actions
    let has_versions = !app.version_list.is_empty();
    let action_items = vec![
        Line::from(vec![
            Span::styled(" [R] ", Style::default().fg(SURFACE2).bg(if has_versions { SUCCESS } else { MUTED })),
            Span::styled(" Restaurer à l'emplacement original   ", Style::default().fg(if has_versions { TEXT } else { MUTED })),
            Span::styled(" [F] ", Style::default().fg(SURFACE2).bg(if has_versions { ACCENT } else { MUTED })),
            Span::styled(" Restaurer dans un dossier...", Style::default().fg(if has_versions { TEXT } else { MUTED })),
        ]),
        Line::from(vec![
            Span::styled(" [D] ", Style::default().fg(SURFACE2).bg(if has_versions { ERROR } else { MUTED })),
            Span::styled(" Supprimer cette version              ", Style::default().fg(if has_versions { TEXT } else { MUTED })),
            Span::styled(" [C] ", Style::default().fg(SURFACE2).bg(if has_versions { WARNING } else { MUTED })),
            Span::styled(" Nettoyer (garder N versions)", Style::default().fg(if has_versions { TEXT } else { MUTED })),
        ]),
        Line::from(vec![
            Span::styled(" [E] ", Style::default().fg(SURFACE2).bg(ACCENT)),
            Span::styled(" Chercher les versions d'un fichier", Style::default().fg(TEXT)),
        ]),
    ];

    f.render_widget(
        Paragraph::new(action_items).block(styled_block(" Actions ", BORDER)),
        layout[2],
    );
}

// ─── Configuration ────────────────────────────────────────────────────────────

fn render_config(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.config;
    let mut lines: Vec<Line> = Vec::new();

    let section = |name: &str| -> Line {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("[{}]", name), Style::default().fg(WARNING).add_modifier(Modifier::BOLD)),
        ])
    };

    let kv = |key: &str, value: String| -> Line {
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

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  Les modifications via [A][E] sont sauvegardées automatiquement dans config.toml",
        Style::default().fg(Color::Rgb(60, 80, 60)),
    )]));

    f.render_widget(
        Paragraph::new(lines)
            .block(styled_block(" Configuration (config.toml) ", BORDER))
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ─── Règles ───────────────────────────────────────────────────────────────────

fn render_regles(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

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
        List::new(patterns).block(styled_block(" Patterns d'exclusion ", BORDER)),
        cols[0],
    );

    let mut other: Vec<Line> = Vec::new();

    other.push(Line::from(vec![
        Span::styled("  Taille max  ", Style::default().fg(MUTED)),
        Span::styled(
            app.config
                .filters
                .max_file_size_mb
                .map(|m| format!("{} MB", m))
                .unwrap_or_else(|| "illimitée".to_string()),
            Style::default().fg(TEXT),
        ),
    ]));

    other.push(Line::from(""));

    if app.config.filters.include_extensions.is_empty() {
        other.push(Line::from(vec![
            Span::styled("  Extensions  ", Style::default().fg(MUTED)),
            Span::styled("toutes", Style::default().fg(MUTED)),
        ]));
    } else {
        other.push(Line::from(Span::styled("  Extensions incluses :", Style::default().fg(MUTED))));
        for ext in &app.config.filters.include_extensions {
            other.push(Line::from(vec![
                Span::styled("    ✓ ", Style::default().fg(SUCCESS)),
                Span::styled(ext.as_str(), Style::default().fg(TEXT)),
            ]));
        }
    }

    other.push(Line::from(""));
    other.push(Line::from(Span::styled("  Patterns critiques (notifs) :", Style::default().fg(MUTED))));
    for pat in app.config.notifications.critical_patterns.iter().take(10) {
        other.push(Line::from(vec![
            Span::styled("    ⚠ ", Style::default().fg(WARNING)),
            Span::styled(pat.as_str(), Style::default().fg(TEXT)),
        ]));
    }
    if app.config.notifications.critical_patterns.len() > 10 {
        other.push(Line::from(Span::styled(
            format!("    ... et {} autres", app.config.notifications.critical_patterns.len() - 10),
            Style::default().fg(MUTED),
        )));
    }

    f.render_widget(
        Paragraph::new(other)
            .block(styled_block(" Autres règles ", BORDER))
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
            MsgLevel::Info => "ℹ  ",
            MsgLevel::Success => "✓  ",
            MsgLevel::Warning => "⚠  ",
            MsgLevel::Error => "✗  ",
        };
        (format!(" {}{}", prefix, msg), col)
    } else {
        (String::new(), MUTED)
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(text, Style::default().fg(color))))
            .style(Style::default().bg(SURFACE2)),
        area,
    );
}

// ─── Footer ───────────────────────────────────────────────────────────────────

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    // Raccourcis contextuels selon l'onglet
    let ctx: Vec<(&str, Color, &str)> = match app.active_tab {
        Tab::Surveillance => vec![
            ("W", SUCCESS, "Watch"),
            ("A", ACCENT, "Ajouter"),
            ("D", ERROR, "Retirer"),
            ("S", ACCENT, "Sync"),
        ],
        Tab::Sync => if app.syncing {
            vec![
                ("X", ERROR, "Annuler"),
                ("E", ACCENT, "Modifier dest"),
            ]
        } else {
            vec![
                ("S", SUCCESS, "Lancer sync"),
                ("E", ACCENT, "Modifier dest"),
            ]
        },
        Tab::Versions => vec![
            ("E", ACCENT, "Chercher"),
            ("R", SUCCESS, "Restaurer"),
            ("F", ACCENT, "→ Dossier"),
            ("D", ERROR, "Supprimer"),
            ("C", WARNING, "Nettoyer"),
        ],
        _ => vec![
            ("W", SUCCESS, "Watch"),
            ("S", ACCENT, "Sync"),
        ],
    };

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" Tab ", Style::default().fg(SURFACE2).bg(BORDER)));
    spans.push(Span::styled(" Onglets  ", Style::default().fg(MUTED)));

    for (key, color, label) in ctx {
        spans.push(Span::styled(format!(" {} ", key), Style::default().fg(SURFACE2).bg(color)));
        spans.push(Span::styled(format!(" {}  ", label), Style::default().fg(MUTED)));
    }

    spans.push(Span::styled(" Q ", Style::default().fg(SURFACE2).bg(ERROR)));
    spans.push(Span::styled(" Quitter", Style::default().fg(MUTED)));

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE)),
        area,
    );
}

// ─── Overlay de saisie ────────────────────────────────────────────────────────

fn render_input_overlay(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_popup(70, 7, area);

    // Effacer la zone sous le popup
    f.render_widget(Clear, popup);

    let prompt = app.input_mode.prompt();
    let display = format!("{}│", app.input_value);

    let content = vec![
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(display, Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled(
                "  [Entrée] Confirmer   [Échap] Annuler   [⌫] Effacer",
                Style::default().fg(MUTED),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(content).block(
            Block::default()
                .title(Span::styled(
                    format!(" {} ", prompt),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_ACTIVE))
                .style(Style::default().bg(Color::Rgb(15, 20, 40))),
        ),
        popup,
    );
}

fn centered_popup(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_w = (area.width * percent_x / 100).max(30);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, popup_w.min(area.width), height.min(area.height))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn styled_block(title: &str, border_color: Color) -> Block<'_> {
    Block::default()
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(SURFACE2))
}

fn build_event_items(app: &App, limit: usize) -> Vec<ListItem<'static>> {
    app.events
        .iter()
        .take(limit)
        .map(|e| {
            let (sym, col) = match e.change_type {
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
                Span::styled(format!(" {} ", sym), Style::default().fg(col).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:<10} ", e.change_type.to_string()),
                    Style::default().fg(col),
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
