use crate::api::types::CraftingRecipe;
use crate::app::{AppState, AtomicPrinterCraftInput, CraftInput};
use crate::ui::theme::{palette, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;

pub(crate) fn render_craft_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let CraftInput::PickRecipe { ref manny_name, selection, ref error, .. } = state.craft else {
        return;
    };
    let p = palette(state.color_mode);
    let recipes = state.manny_craft_recipes();
    render_recipe_picker(
        frame, area, state, p, &format!(" CRAFT — {manny_name} "), p.accent, &recipes, selection,
        error.as_deref(),
    );
}

pub(crate) fn render_atomic_printer_craft_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let AtomicPrinterCraftInput::PickRecipe { selection, ref error } = state.atomic_printer_craft else {
        return;
    };
    let p = palette(state.color_mode);
    let recipes = state.atomic_printer_recipes();
    render_recipe_picker(
        frame, area, state, p, " ATOMIC PRINTER — SELECT RECIPE ", p.crit, &recipes, selection,
        error.as_deref(),
    );
}

/// Shared recipe picker: a list marked by affordability, plus a detail block
/// for the selected recipe (output, duration, per-ingredient have/need).
#[allow(clippy::too_many_arguments)]
fn render_recipe_picker(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    p: Palette,
    title: &str,
    border: Color,
    recipes: &[&CraftingRecipe],
    selection: usize,
    error: Option<&str>,
) {
    let sel = recipes.get(selection);
    let ing_rows = sel.map_or(0, |r| r.ingredients.len());
    let height = (recipes.len() as u16 + ing_rows as u16 + 7).clamp(9, 24);
    let popup = centered_rect(60, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(title.to_owned())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let mut lines: Vec<Line> = Vec::new();

    if recipes.is_empty() {
        lines.push(Line::from(Span::styled("loading recipes…", dim)));
    }
    for (i, recipe) in recipes.iter().enumerate() {
        let selected = i == selection;
        let affordable = state.recipe_affordable(recipe);
        let (mark, mark_color) = if affordable { ("✓", p.good) } else { ("✗", p.crit) };
        let name_style = if selected {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else if affordable {
            text
        } else {
            dim
        };
        lines.push(Line::from(vec![
            Span::styled(if selected { "▶ " } else { "  " }, Style::default().fg(p.accent)),
            Span::styled(format!("{mark} "), Style::default().fg(mark_color)),
            Span::styled(format!("{:<20}", recipe.name), name_style),
            Span::styled(format!(" {}m", recipe.duration_seconds / 60), dim),
        ]));
    }

    // Detail block for the selected recipe.
    if let Some(recipe) = sel {
        lines.push(Line::default());
        let mut out = vec![
            Span::styled("→ ", dim),
            Span::styled(recipe.output.name.as_str(), Style::default().fg(p.accent)),
        ];
        if let Some(bonus) = recipe.output.capacity_bonus {
            if bonus != 0.0 {
                out.push(Span::styled(format!("  +{bonus:.2} cap"), dim));
            }
        }
        lines.push(Line::from(out));
        for ing in &recipe.ingredients {
            let have = state.recipe_ingredient_have(ing);
            let ok = have >= ing.quantity;
            let need = if ing.unit == "item" {
                format!("{}", ing.quantity as u32)
            } else {
                format!("{:.2}", ing.quantity)
            };
            let have_str = if ing.unit == "item" {
                format!("{}", have as u32)
            } else {
                format!("{have:.2}")
            };
            lines.push(Line::from(vec![
                Span::styled(if ok { "  ✓ " } else { "  ✗ " }, Style::default().fg(if ok { p.good } else { p.crit })),
                Span::styled(format!("{:<18}", ing.ingredient_type), text),
                Span::styled(format!("{have_str}/{need}"), Style::default().fg(if ok { p.text } else { p.crit })),
            ]));
        }
    }

    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);

    // Footer — Enter dimmed when the selected recipe isn't affordable.
    let can = sel.map(|r| state.recipe_affordable(r)).unwrap_or(false);
    let enter_style = if can {
        Style::default().fg(p.good).add_modifier(Modifier::BOLD)
    } else {
        dim
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(p.accent)),
            Span::raw(" select  "),
            Span::styled("[Enter]", enter_style),
            Span::raw(" start  "),
            Span::styled("[Esc]", Style::default().fg(p.accent)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}
