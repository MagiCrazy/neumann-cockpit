use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{AppState, Fabricator, BASE_RESOURCES};
use crate::ui::theme::{format_duration, palette};

use super::{render_footer, FooterKey};

/// Full-screen tech-tree browser (`:tree`, #200): a left catalog of recipes,
/// each expandable into its ingredient sub-tree, and a right panel rolling the
/// selected node up to base resources.
pub(crate) fn render_tree_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" TECH TREE ".to_owned())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(layout[0]);

    render_catalog(frame, panels[0], state, p);
    render_detail(frame, panels[1], state, p);

    render_footer(
        frame,
        layout[1],
        p,
        &[
            FooterKey::nav("[↑/↓]", "move"),
            FooterKey::nav("[→/←]", "expand/collapse"),
            FooterKey::nav("[+/-]", "qty"),
            FooterKey::nav("[Esc]", "close"),
        ],
    );
}

/// The left catalog: sectioned recipe roots, expanded nodes indented beneath
/// them with a per-node absolute quantity and an expand caret.
fn render_catalog(frame: &mut Frame, area: Rect, state: &AppState, p: crate::ui::theme::Palette) {
    let rows = state.tree_rows();
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);

    // Scroll so the cursor stays visible in the available height.
    let height = area.height as usize;
    let cursor = state.tree.cursor;
    let start = cursor.saturating_sub(height.saturating_sub(1));

    let mut lines: Vec<Line> = Vec::new();
    if rows.is_empty() {
        lines.push(Line::from(Span::styled("no recipes loaded — F5 to refresh", dim)));
    }
    for (i, row) in rows.iter().enumerate().skip(start).take(height) {
        if row.is_header {
            lines.push(Line::from(Span::styled(
                row.label.clone(),
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        let selected = i == cursor;
        let indent = "  ".repeat(row.depth);
        let caret = if row.expandable {
            if row.expanded {
                "▾ "
            } else {
                "▸ "
            }
        } else if row.is_base {
            "· "
        } else {
            "  "
        };
        let qty = fmt_qty(row.qty_abs);
        let name_style = if selected {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else if row.is_base {
            dim
        } else {
            text
        };
        lines.push(Line::from(vec![
            Span::styled(if selected { "▶" } else { " " }, Style::default().fg(p.accent)),
            Span::raw(format!(" {indent}")),
            Span::styled(caret, Style::default().fg(p.accent)),
            Span::styled(format!("{qty:>5} "), dim),
            Span::styled(row.label.clone(), name_style),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

/// The right panel: the selected node's fabricator/duration, its direct
/// ingredients, and the roll-up to base resources for the current quantity.
fn render_detail(frame: &mut Frame, area: Rect, state: &AppState, p: crate::ui::theme::Palette) {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(p.dim));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let accent = Style::default().fg(p.accent);

    let rows = state.tree_rows();
    let Some(row) = rows.get(state.tree.cursor).filter(|r| !r.is_header) else {
        return;
    };
    let qty = state.tree.qty.max(1);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        row.label.clone(),
        accent.add_modifier(Modifier::BOLD),
    )));
    let builder = match row.fabricator {
        Some(Fabricator::AtomicPrinter) => "atomic printer",
        Some(Fabricator::Manny) => "Manny",
        None => "raw resource",
    };
    let mut meta = vec![Span::styled(format!("built by: {builder}"), dim)];
    if row.duration_seconds > 0 {
        meta.push(Span::styled(
            format!("  ·  {}", format_duration(row.duration_seconds)),
            dim,
        ));
    }
    lines.push(Line::from(meta));
    lines.push(Line::from(Span::styled(format!("quantity: ×{qty}"), dim)));
    lines.push(Line::default());

    // Direct ingredients of the selected recipe (if any).
    if let Some(recipe) = state.recipes.iter().find(|r| r.id == row.item || r.output.output_type == row.item) {
        lines.push(Line::from(Span::styled("DIRECT INGREDIENTS", dim)));
        for ing in &recipe.ingredients {
            let q = if ing.unit == "item" {
                format!("{}", (ing.quantity * qty as f64) as i64)
            } else {
                format!("{:.2}", ing.quantity * qty as f64)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {q:>7} "), text),
                Span::styled(ing.ingredient_type.clone(), text),
            ]));
        }
        lines.push(Line::default());
    }

    // Roll-up to base resources.
    let rollup = state.recipe_rollup(&row.item, qty as f64);
    lines.push(Line::from(Span::styled("── ROLLED UP TO BASE ──", dim)));
    for res in BASE_RESOURCES {
        let amount = rollup.base.get(res).copied().unwrap_or(0.0);
        if amount <= 0.0 {
            continue;
        }
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<18}", res), text),
            Span::styled(format!("{amount:.2}"), accent),
        ]));
    }
    // Any unmodelled leaf (an item with no recipe) surfaces here.
    for (res, amount) in &rollup.base {
        if !BASE_RESOURCES.contains(&res.as_str()) && *amount > 0.0 {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<18}", res), Style::default().fg(p.warn)),
                Span::styled(format!("{amount:.2}"), Style::default().fg(p.warn)),
            ]));
        }
    }
    lines.push(Line::default());
    let ops = rollup.craft_ops() as i64;
    lines.push(Line::from(Span::styled(
        format!("{ops} craft ops  ·  {} total", format_duration(rollup.duration_seconds as i64)),
        dim,
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Compact quantity for the catalog column: an integer for whole counts, else
/// two decimals (base-resource amounts).
fn fmt_qty(q: f64) -> String {
    if (q.fract()).abs() < 1e-9 {
        format!("{}", q as i64)
    } else {
        format!("{q:.2}")
    }
}
