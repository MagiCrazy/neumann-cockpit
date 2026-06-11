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
        help_section("Global"),
        help_key_line("r", "refresh all"),
        help_key_line("p i m s", "focus probe / inventory / mannies / scanner"),
        help_key_line("Tab", "cycle panel focus (Shift-Tab reverse)"),
        help_key_line("F2", "toggle retro/classic theme"),
        help_key_line("t", "travel to coordinates"),
        help_key_line("b", "map"),
        help_key_line("w", "waypoints"),
        help_key_line("?", "this help"),
        help_key_line("q", "quit"),
        help_key_line("Esc", "unfocus / close"),
        Line::default(),
        help_section("Inventory (focused)"),
        help_key_line("↑↓", "select row"),
        help_key_line("Enter", "row detail"),
        help_key_line("j", "jettison selection"),
        help_key_line("d", "deploy waypoint"),
        help_key_line("a", "atomic printer craft"),
        Line::default(),
        help_section("Map"),
        help_key_line("hjkl/←↓↑→", "pan"),
        help_key_line("u / d", "layer y ± 1"),
        help_key_line("0", "center on probe"),
        help_key_line("c", "jump to coordinates"),
        help_key_line("g", "travel to center"),
    ];

    let right: Vec<Line> = vec![
        help_section("Mannies (focused)"),
        help_key_line("↑↓/jk", "select manny"),
        help_key_line("Enter", "repair"),
        help_key_line("e", "mine"),
        help_key_line("c", "craft"),
        help_key_line("s", "salvage"),
        help_key_line("x", "inspect asteroid"),
        help_key_line("D", "detach container"),
        help_key_line("v", "recover container"),
        help_key_line("n", "rename"),
        help_key_line("R", "recall (busy manny)"),
        Line::default(),
        help_section("Scanner (focused)"),
        help_key_line("Enter", "rescan current sector"),
        help_key_line("c", "scan custom coordinates"),
        help_key_line("n", "scan neighbors (d=1)"),
        help_key_line("d", "deep scan (axis faces, d=2)"),
        help_key_line("↑↓/jk", "browse history"),
        help_key_line("J / K", "scroll detail"),
        help_key_line("o", "object mode (probe sector)"),
        help_key_line("g", "travel to displayed sector"),
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

