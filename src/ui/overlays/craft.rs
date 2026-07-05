use crate::app::{AppState, Fabricator, FabricationInput};
use crate::ui::theme::{palette, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, render_pick_list, FooterKey, KeyTone};

/// Render whichever step of the unified fabrication wizard is active.
pub(crate) fn render_fabrication_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.fabrication {
        FabricationInput::PickRecipe { selection, ref error, .. } => {
            render_catalog(frame, area, state, selection, error.as_deref());
        }
        FabricationInput::PickBuilder { ref recipe_name, ref mannies, selection, ref error, .. } => {
            let p = palette(state.color_mode);
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let prompt = format!("Build {recipe_name} with:");
            let height = (names.len() as u16 + 6).clamp(8, 20);
            render_pick_list(
                frame, area, p, " FABRICATION — SELECT BUILDER ", 46, height,
                Some(&prompt), &names, selection, error.as_deref(), "BUILD",
            );
        }
        FabricationInput::Inactive => {}
    }
}

/// The item-first catalog: every recipe sectioned by fabricator, with a detail
/// block (output, duration, per-ingredient have/need + the builder/assistant it
/// will use) for the selected recipe.
fn render_catalog(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selection: usize,
    error: Option<&str>,
) {
    let p = palette(state.color_mode);
    let rows = state.fabrication_recipes();
    let sel = rows.get(selection);

    // Roomy two-panel modal: the catalog grows long once both sections are
    // shown, so the list scrolls on the left while the detail stays pinned right.
    let popup = centered_rect(76, area.height.saturating_sub(4).clamp(12, 30), area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" FABRICATION ".to_owned())
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
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(layout[0]);
    let list_area = panels[0];

    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);

    // ── Left panel: the sectioned recipe list, scrolled to keep the cursor in
    // view. `sel_line` tracks where the selected row lands among the rendered
    // lines (headers + gaps included) so we can compute the scroll offset.
    let mut lines: Vec<Line> = Vec::new();
    let mut sel_line = 0usize;
    if rows.is_empty() {
        lines.push(Line::from(Span::styled("loading recipes…", dim)));
    }
    let mut prev: Option<Fabricator> = None;
    for (i, (fab, recipe)) in rows.iter().enumerate() {
        if prev != Some(*fab) {
            if prev.is_some() {
                lines.push(Line::default());
            }
            lines.push(section_header(*fab, p));
            prev = Some(*fab);
        }
        let selected = i == selection;
        if selected {
            sel_line = lines.len();
        }
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
            Span::styled(if selected { " ▶ " } else { "   " }, Style::default().fg(p.accent)),
            Span::styled(format!("{mark} "), Style::default().fg(mark_color)),
            Span::styled(format!("{:<16}", recipe.name), name_style),
            Span::styled(format!(" {}m", recipe.duration_seconds / 60), dim),
        ]));
    }
    let visible = list_area.height as usize;
    let scroll = if sel_line >= visible { (sel_line - visible + 1) as u16 } else { 0 };
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), list_area);

    // ── Right panel: the selected recipe's detail, divided by a left border.
    let detail_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(p.dim));
    let detail_area = detail_block.inner(panels[1]);
    frame.render_widget(detail_block, panels[1]);

    let mut detail: Vec<Line> = Vec::new();
    if let Some((fab, recipe)) = sel {
        detail.push(Line::from(Span::styled(
            recipe.name.as_str(),
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        )));
        detail.push(builder_line(*fab, state, p));
        detail.push(Line::default());
        let mut out = vec![
            Span::styled("→ ", dim),
            Span::styled(recipe.output.name.as_str(), text),
        ];
        if let Some(bonus) = recipe.output.capacity_bonus {
            if bonus != 0.0 {
                out.push(Span::styled(format!("  +{bonus:.2} cap"), dim));
            }
        }
        detail.push(Line::from(out));
        detail.push(Line::from(Span::styled(
            format!("⏲ {} min", recipe.duration_seconds / 60), dim,
        )));
        detail.push(Line::default());
        detail.push(Line::from(Span::styled("INGREDIENTS   have/need", dim)));
        if recipe.ingredients.is_empty() {
            detail.push(Line::from(Span::styled("  (none)", dim)));
        }
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
            detail.push(Line::from(vec![
                Span::styled(if ok { "✓ " } else { "✗ " }, Style::default().fg(if ok { p.good } else { p.crit })),
                Span::styled(format!("{:<19}", ing.ingredient_type), text),
                Span::styled(format!("{have_str}/{need}"), Style::default().fg(if ok { p.text } else { p.crit })),
            ]));
        }
    }
    if let Some(err) = error {
        detail.push(Line::default());
        detail.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(detail), detail_area);

    // Footer — Enter dimmed when the selected recipe isn't affordable. Manny
    // recipes may advance to a builder step, so the label reflects that.
    let can = sel.map(|(_, r)| state.recipe_affordable(r)).unwrap_or(false);
    let commit = if can {
        FooterKey::commit("[Enter]", "FABRICATE")
    } else {
        FooterKey { key: "[Enter]", label: "FABRICATE", tone: KeyTone::Disabled }
    };
    render_footer(frame, layout[1], p, &[
        FooterKey::nav("[↑/↓]", "select"),
        commit,
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

fn section_header(fab: Fabricator, p: Palette) -> Line<'static> {
    // The "reserves/occupies a manny" nuance lives in the detail panel's builder
    // line; the header stays short so it fits the narrow list column.
    let (glyph, name) = match fab {
        Fabricator::AtomicPrinter => ("⚛", "ATOMIC PRINTER"),
        Fabricator::Manny => ("⚙", "MANNY FABRICATION"),
    };
    Line::from(Span::styled(
        format!("{glyph} {name}"),
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    ))
}

/// The line describing which Manny will build/assist the selected recipe.
fn builder_line(fab: Fabricator, state: &AppState, p: Palette) -> Line<'static> {
    let dim = Style::default().fg(p.dim);
    match fab {
        Fabricator::AtomicPrinter => {
            if !state.has_atomic_printer() {
                Line::from(Span::styled("⚠ no atomic printer in inventory", Style::default().fg(p.crit)))
            } else if state.collect_idle_onboard_mannies().is_empty() {
                Line::from(Span::styled("⚠ no idle manny to assist the printer", Style::default().fg(p.warn)))
            } else {
                Line::from(Span::styled("⚛ printer reserves an idle manny as assistant", dim))
            }
        }
        Fabricator::Manny => {
            // A pre-chosen builder (opened from the Mannies pane) wins.
            if let FabricationInput::PickRecipe { prefilled_manny: Some((_, name)), .. } = &state.fabrication {
                return Line::from(vec![
                    Span::styled("⚙ builder: ", dim),
                    Span::styled(name.clone(), Style::default().fg(p.text)),
                ]);
            }
            match state.collect_idle_onboard_mannies().len() {
                0 => Line::from(Span::styled("⚠ no idle manny on board", Style::default().fg(p.warn))),
                1 => Line::from(Span::styled("⚙ built by the idle manny on board", dim)),
                n => Line::from(Span::styled(format!("⚙ choose 1 of {n} idle mannies"), dim)),
            }
        }
    }
}
