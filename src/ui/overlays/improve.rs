use crate::app::{AppState, ImproveInput};
use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, render_pick_list, FooterKey, KeyTone};

/// Render whichever step of the probe-improvement wizard is active.
pub(crate) fn render_improve_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.improve {
        ImproveInput::PickImprovement { selection, ref error } => {
            render_catalog(frame, area, state, selection, error.as_deref());
        }
        ImproveInput::PickBuilder { ref improvement_name, ref mannies, selection, ref error, .. } => {
            let p = palette(state.color_mode);
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let prompt = format!("Install {improvement_name} with:");
            let height = (names.len() as u16 + 6).clamp(8, 20);
            render_pick_list(
                frame, area, p, " IMPROVE PROBE — SELECT BUILDER ", 48, height,
                Some(&prompt), &names, selection, error.as_deref(), "BUILD",
            );
        }
        ImproveInput::Inactive => {}
    }
}

/// Two-panel catalog: the improvement list on the left, the selected one's
/// detail (status, duration, ingredient have/need, description) on the right.
fn render_catalog(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selection: usize,
    error: Option<&str>,
) {
    let p = palette(state.color_mode);
    let items = &state.probe_improvements;
    let sel = items.get(selection);

    let popup = centered_rect(72, area.height.saturating_sub(6).clamp(10, 22), area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" IMPROVE PROBE ".to_owned())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(layout[0]);

    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);

    // ── Left: the improvement list.
    let mut lines: Vec<Line> = Vec::new();
    if items.is_empty() {
        lines.push(Line::from(Span::styled("loading…", dim)));
    }
    for (i, imp) in items.iter().enumerate() {
        let selected = i == selection;
        let (mark, mark_color) = if imp.done {
            ("✓", p.good)
        } else if imp.available && state.improvement_affordable(imp) {
            ("○", p.accent)
        } else {
            ("·", p.dim)
        };
        let name_style = if selected {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else if imp.done {
            dim
        } else {
            text
        };
        lines.push(Line::from(vec![
            Span::styled(if selected { " ▶ " } else { "   " }, Style::default().fg(p.accent)),
            Span::styled(format!("{mark} "), Style::default().fg(mark_color)),
            Span::styled(imp.name.clone(), name_style),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), panels[0]);

    // ── Right: detail of the selected improvement.
    let detail_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(p.dim));
    let detail_area = detail_block.inner(panels[1]);
    frame.render_widget(detail_block, panels[1]);

    let mut detail: Vec<Line> = Vec::new();
    if let Some(imp) = sel {
        detail.push(Line::from(Span::styled(
            imp.name.clone(),
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        )));
        let status = if imp.done {
            Span::styled("already installed", Style::default().fg(p.good))
        } else if !imp.available {
            Span::styled("locked", Style::default().fg(p.warn))
        } else {
            Span::styled(format!("⏲ {} min", imp.duration_seconds / 60), dim)
        };
        detail.push(Line::from(status));
        detail.push(Line::default());
        detail.push(Line::from(Span::styled("INGREDIENTS   have/need", dim)));
        for ing in &imp.ingredients {
            let have = state.recipe_ingredient_have(ing);
            let ok = have >= ing.quantity;
            let (need, have_str) = if ing.unit == "item" {
                (format!("{}", ing.quantity as u32), format!("{}", have as u32))
            } else {
                (format!("{:.2}", ing.quantity), format!("{have:.2}"))
            };
            detail.push(Line::from(vec![
                Span::styled(if ok { "✓ " } else { "✗ " }, Style::default().fg(if ok { p.good } else { p.crit })),
                Span::styled(format!("{:<19}", ing.ingredient_type), text),
                Span::styled(format!("{have_str}/{need}"), Style::default().fg(if ok { p.text } else { p.crit })),
            ]));
        }
        detail.push(Line::default());
        for line in wrap(&imp.description, detail_area.width as usize) {
            detail.push(Line::from(Span::styled(line, dim)));
        }
    }
    if let Some(err) = error {
        detail.push(Line::default());
        detail.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(detail), detail_area);

    // Footer — Enter dimmed unless the selected improvement can be installed now.
    let can = sel.map(|i| i.available && !i.done && state.improvement_affordable(i)).unwrap_or(false);
    let commit = if can {
        FooterKey::commit("[Enter]", "INSTALL")
    } else {
        FooterKey { key: "[Enter]", label: "INSTALL", tone: KeyTone::Disabled }
    };
    render_footer(frame, layout[1], p, &[
        FooterKey::nav("[↑/↓]", "select"),
        commit,
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

/// Greedy word-wrap to `width` columns (right detail panel is narrow).
fn wrap(s: &str, width: usize) -> Vec<String> {
    if width < 4 {
        return vec![s.to_string()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.chars().count() + 1 + word.chars().count() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

