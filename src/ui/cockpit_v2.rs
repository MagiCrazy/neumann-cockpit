//! Unified Cockpit v2 interface (bloc U1 — scaffolding stub).
//!
//! This is the entry point for the new tiling dashboard. U1 only proves the
//! config gate and the render dispatch are wired: it draws the 3×3 grid of
//! pane labels with the active pane highlighted, plus a WIP banner. The real
//! responsive layout, per-pane content, zoom, menus and command line land in
//! blocs U2–U7.

use crate::app::{AppState, Pane};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const AMBER: Color = Color::Rgb(0xff, 0xb2, 0x4a);
const GREEN: Color = Color::Rgb(0x5e, 0xf0, 0x8f);
const DIM: Color = Color::Rgb(0x6f, 0x8c, 0x7d);

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let outer = Block::default()
        .title(" NEUMANN COCKPIT · UNIFIED (WIP) ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    render_grid(frame, rows[0], state);
    render_status(frame, rows[1], state);
}

fn render_grid(frame: &mut Frame, area: Rect, state: &AppState) {
    let grid_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 3); 3])
        .split(area);

    for (r, row_area) in grid_rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3); 3])
            .split(*row_area);
        for (c, cell) in cols.iter().enumerate() {
            let pane = Pane::ALL[r * 3 + c];
            render_cell(frame, *cell, pane, pane == state.active_pane);
        }
    }
}

fn render_cell(frame: &mut Frame, area: Rect, pane: Pane, active: bool) {
    let color = if active { AMBER } else { DIM };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            format!(" {} ", pane.label()),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let key = Line::from(Span::styled(
        format!("[{}]", pane.key_label()),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(Paragraph::new(key), inner);
}

fn render_status(frame: &mut Frame, area: Rect, state: &AppState) {
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", state.mode.tag()),
            Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} ", state.active_pane.label()),
            Style::default().fg(GREEN),
        ),
        Span::styled(
            "· ertdfgcvb select · z zoom · U1 scaffold",
            Style::default().fg(DIM),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
