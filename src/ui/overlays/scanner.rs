use crate::app::{AppState, ScanMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;

/// Coordinate-entry and neighbor-scan prompts for the Scanner pane. Both modes
/// are handled by the shared scan-input router in `input/mod.rs`; this only
/// renders the prompt.
pub(crate) fn render_scan_input_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.scan_mode {
        ScanMode::Input(buf) => render_coord_input(frame, area, buf),
        ScanMode::DirectionPick => render_direction_pick(frame, area, state),
        ScanMode::Current => {}
    }
}

fn render_coord_input(frame: &mut Frame, area: Rect, buf: &str) {
    let popup = centered_rect(52, 7, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" OBSERVE SECTOR ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "relative coordinates, space-separated",
            Style::default().fg(Color::DarkGray),
        ))),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("x y z: ", Style::default().fg(Color::Cyan)),
            Span::raw(buf),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ])),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" observe  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[2],
    );
}

fn render_direction_pick(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(52, 7, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" SCAN NEIGHBORS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let origin = state
        .probe_sector_coords()
        .map(|(x, y, z)| format!("({x}, {y}, {z})"))
        .unwrap_or_else(|| "unknown".into());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("from ", Style::default().fg(Color::DarkGray)),
                Span::styled(origin, Style::default().fg(Color::White)),
            ]),
            Line::default(),
            Line::from("pick an axis to sweep its two faces:"),
        ]),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[x] [y] [z]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" scan  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}
