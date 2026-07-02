use crate::ui::theme::palette;
use crate::app::{AppState, AtomicPrinterCraftInput, CraftInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
pub(crate) fn render_craft_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let CraftInput::PickRecipe { ref manny_name, selection, ref error, .. } = state.craft else { return };

    let recipes = state.manny_craft_recipes();
    let height = (recipes.len() as u16 + 6).min(16);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);

    let title = format!(" CRAFT — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if recipes.is_empty() {
        lines.push(Line::from(Span::styled(
            "loading recipes…",
            Style::default().fg(p.dim),
        )));
    }
    for (i, recipe) in recipes.iter().enumerate() {
        let selected = i == selection;
        let duration_min = recipe.duration_seconds / 60;
        let ingredients: String = recipe.ingredients.iter().map(|ing| {
            if ing.unit == "item" {
                format!("{} × {}", ing.quantity as u32, ing.ingredient_type)
            } else {
                format!("{:.2} ECE {}", ing.quantity, ing.ingredient_type)
            }
        }).collect::<Vec<_>>().join(", ");
        let detail = format!("  {}m  {}", duration_min, ingredients);
        if selected {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(p.warn)),
                Span::styled(recipe.name.as_str(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
                Span::styled(detail, Style::default().fg(p.dim)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(recipe.name.as_str(), Style::default().fg(p.dim)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(p.accent)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(p.good).add_modifier(Modifier::BOLD)),
            Span::raw(" start  "),
            Span::styled("[Esc]", Style::default().fg(p.accent)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

pub(crate) fn render_atomic_printer_craft_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let AtomicPrinterCraftInput::PickRecipe { selection, ref error } = state.atomic_printer_craft else { return };

    let recipes = state.atomic_printer_recipes();
    let height = (recipes.len() as u16 + 6).min(16);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" ATOMIC PRINTER — SELECT RECIPE ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.crit));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if recipes.is_empty() {
        lines.push(Line::from(Span::styled(
            "loading recipes…",
            Style::default().fg(p.dim),
        )));
    }
    for (i, recipe) in recipes.iter().enumerate() {
        let selected = i == selection;
        let duration_min = recipe.duration_seconds / 60;
        let ingredients: String = recipe.ingredients.iter().map(|ing| {
            if ing.unit == "item" {
                format!("{} × {}", ing.quantity as u32, ing.ingredient_type)
            } else {
                format!("{:.2} ECE {}", ing.quantity, ing.ingredient_type)
            }
        }).collect::<Vec<_>>().join(", ");
        let detail = format!("  {}m  {}", duration_min, ingredients);
        if selected {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(p.warn)),
                Span::styled(recipe.name.as_str(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
                Span::styled(detail, Style::default().fg(p.dim)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(recipe.name.as_str(), Style::default().fg(p.dim)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(p.accent)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(p.good).add_modifier(Modifier::BOLD)),
            Span::raw(" start  "),
            Span::styled("[Esc]", Style::default().fg(p.accent)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

