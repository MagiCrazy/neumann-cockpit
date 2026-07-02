use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
// ── Help overlay ──────────────────────────────────────────────────────────────

pub(crate) fn help_key_line(key: &'static str, desc: &'static str, p: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<10}"), Style::default().fg(p.accent)),
        Span::raw(desc),
    ])
}

pub(crate) fn help_section(title: &'static str, p: Palette) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default().fg(p.warn).add_modifier(Modifier::BOLD),
    ))
}

pub(crate) fn render_help_overlay(frame: &mut Frame, area: Rect, p: Palette) {
    let popup = centered_rect(76, 28, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" HELP — KEYBINDINGS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
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
        help_section("Navigate", p),
        help_key_line("e r t", "Scanner · Map · Comms", p),
        help_key_line("d f g", "Sector · Probe · Missions", p),
        help_key_line("c v b", "Inventory · Storage · Mannies", p),
        help_key_line("j k / ↑↓", "move cursor in pane", p),
        help_key_line("l / h", "drill in / out (→ ←)", p),
        help_key_line("Tab", "cycle panes (Shift-Tab reverse)", p),
        help_key_line("z", "zoom active pane full screen", p),
        help_key_line("Esc", "close / leave zoom / drill up", p),
        Line::default(),
        help_section("Act & global", p),
        help_key_line("Enter", "contextual action menu", p),
        help_key_line(":", "command line (travel, goto, focus…)", p),
        help_key_line("F1", "toggle hints line", p),
        help_key_line("F2", "cycle color mode", p),
        help_key_line("F5", "refresh", p),
        help_key_line("?", "this help", p),
        help_key_line("q", "quit", p),
        Line::default(),
        help_section("In a menu", p),
        help_key_line("j k", "move", p),
        help_key_line("Enter", "fire selected", p),
        help_key_line("Esc", "close", p),
    ];

    let right: Vec<Line> = vec![
        help_section("Actions per pane (Enter)", p),
        help_key_line("Mannies", "mine, craft, repair, salvage,", p),
        help_key_line("", "inspect, recover, detach, refuel,", p),
        help_key_line("", "drop cargo, recall/abandon, rename", p),
        help_key_line("Inventory", "jettison, atomic craft, move stock", p),
        help_key_line("Missions", "browse steps, abandon", p),
        help_key_line("Comms", "messages inbox/sent/compose, alerts", p),
        help_key_line("Storage", "rename, rules, recover, detach, move", p),
        help_key_line("Sector", "object actions: mine, inspect,", p),
        help_key_line("", "salvage, recover, deploy, relay", p),
        Line::default(),
        help_section("Config", p),
        help_key_line("theme", "color mode (mono-green/amber,", p),
        help_key_line("", "phosphor-semantic, modern-16, p)", p),
        help_key_line("hints", "show the hints line (F1)", p),
    ];

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(Paragraph::new(right), cols[1]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc/?]", Style::default().fg(p.accent)),
            Span::raw(" close"),
        ])),
        rows[1],
    );
}

