use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
// ── Help overlay ──────────────────────────────────────────────────────────────

pub(crate) fn help_key_line(key: &'static str, desc: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<10}"), Style::default().fg(Color::Cyan)),
        Span::raw(desc),
    ])
}

pub(crate) fn help_section(title: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ))
}

pub(crate) fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(76, 28, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" HELP — KEYBINDINGS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let left: Vec<Line> = vec![
        help_section("Navigate"),
        help_key_line("e r t", "Scanner · Map · Comms"),
        help_key_line("d f g", "Sector · Probe · Missions"),
        help_key_line("c v b", "Inventory · Storage · Mannies"),
        help_key_line("j k / ↑↓", "move cursor in pane"),
        help_key_line("l / h", "drill in / out (→ ←)"),
        help_key_line("Tab", "cycle panes (Shift-Tab reverse)"),
        help_key_line("z", "zoom active pane full screen"),
        help_key_line("Esc", "close / leave zoom / drill up"),
        Line::default(),
        help_section("Act & global"),
        help_key_line("Enter", "contextual action menu"),
        help_key_line("F1", "toggle hints line"),
        help_key_line("F2", "cycle color mode"),
        help_key_line("F5", "refresh"),
        help_key_line("?", "this help"),
        help_key_line("q", "quit"),
        Line::default(),
        help_section("In a menu"),
        help_key_line("j k", "move"),
        help_key_line("Enter", "fire selected"),
        help_key_line("Esc", "close"),
    ];

    let right: Vec<Line> = vec![
        help_section("Actions per pane (Enter)"),
        help_key_line("Mannies", "mine, craft, repair, salvage,"),
        help_key_line("", "inspect, recover, detach, refuel,"),
        help_key_line("", "drop cargo, recall/abandon, rename"),
        help_key_line("Inventory", "jettison, atomic craft, move stock"),
        help_key_line("Missions", "browse steps, abandon"),
        help_key_line("Comms", "messages inbox/sent/compose, alerts"),
        help_key_line("Storage", "rename, rules, recover, detach, move"),
        help_key_line("Sector", "object actions: mine, inspect,"),
        help_key_line("", "salvage, recover, deploy, relay"),
        Line::default(),
        help_section("Config"),
        help_key_line("theme", "color mode (mono-green/amber,"),
        help_key_line("", "phosphor-semantic, modern-16)"),
        help_key_line("hints", "show the hints line (F1)"),
    ];

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(Paragraph::new(right), cols[1]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc/?]", Style::default().fg(Color::Cyan)),
            Span::raw(" close"),
        ])),
        rows[1],
    );
}

