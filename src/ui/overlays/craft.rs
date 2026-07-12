use crate::app::{ActiveWizard, AppState, FabFocus, FabricationInput, Fabricator, StepState};
use crate::ui::theme::{palette, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, render_pick_list, FooterKey, KeyTone};

/// Render whichever step of the production console is active.
pub(crate) fn render_fabrication_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ActiveWizard::Fabrication(fabrication) = &state.active_wizard else {
        return;
    };
    match fabrication {
        FabricationInput::PickRecipe {
            selection,
            qty,
            focus,
            queue_sel,
            error,
            ..
        } => {
            render_console(
                frame,
                area,
                state,
                *selection,
                *qty,
                *focus,
                *queue_sel,
                error.as_deref(),
            );
        }
        FabricationInput::PickBuilder {
            recipe_name,
            mannies,
            selection,
            error,
            ..
        } => {
            let p = palette(state.color_mode);
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let prompt = format!("Build {recipe_name} with:");
            let height = (names.len() as u16 + 6).clamp(8, 20);
            render_pick_list(
                frame,
                area,
                p,
                " FABRICATION — SELECT BUILDER ",
                46,
                height,
                Some(&prompt),
                &names,
                *selection,
                error.as_deref(),
                "queue",
            );
        }
    }
}

/// The production console: recipe catalog | selected-recipe detail | live queue.
/// The catalog sets a recipe + quantity (`Enter` queues it); `Tab` moves focus to
/// the queue panel to manage it.
#[allow(clippy::too_many_arguments)]
fn render_console(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selection: usize,
    qty: u32,
    focus: FabFocus,
    queue_sel: usize,
    error: Option<&str>,
) {
    let p = palette(state.color_mode);
    let rows = state.fabrication_recipes();
    let sel = rows.get(selection);

    let popup = centered_rect(96, area.height.saturating_sub(4).clamp(12, 30), area);
    frame.render_widget(Clear, popup);

    // Title banner reflects the queue's run state.
    let (done, total) = state.queue_progress();
    let banner = if total == 0 {
        String::new()
    } else if state.queue_paused {
        " · ‖ paused".into()
    } else {
        format!(" · ▶ {done}/{total}")
    };
    let block = Block::default()
        .title(format!(" PRODUCTION{banner} "))
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
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(34),
            Constraint::Percentage(28),
        ])
        .split(layout[0]);

    render_catalog_list(frame, panels[0], state, selection, qty, focus);
    render_detail(frame, panels[1], state, sel, error, &p);
    render_queue_panel(frame, panels[2], state, queue_sel, focus, &p);

    // Focus-dependent footer.
    let footer: Vec<FooterKey> = if focus == FabFocus::Catalog {
        let can = sel.map(|(_, r)| state.recipe_affordable(r)).unwrap_or(false);
        let add = if can {
            FooterKey::commit("[Enter]", "add")
        } else {
            FooterKey {
                key: "[Enter]",
                label: "add",
                tone: KeyTone::Disabled,
            }
        };
        vec![
            FooterKey::nav("[↑↓]", "recipe"),
            FooterKey::nav("[+/-]", "qty"),
            add,
            FooterKey::nav("[Tab]", "queue"),
            FooterKey::nav("[p]", if state.queue_paused { "resume" } else { "pause" }),
            FooterKey::nav("[Esc]", "close"),
        ]
    } else {
        vec![
            FooterKey::nav("[↑↓]", "step"),
            FooterKey::nav("[+/-]", "repeat"),
            FooterKey::danger("[x]", "remove"),
            FooterKey::danger("[c]", "clear"),
            FooterKey::nav("[Tab]", "catalog"),
            FooterKey::nav("[Esc]", "close"),
        ]
    };
    render_footer(frame, layout[1], p, &footer);
}

/// Left panel: the sectioned recipe list, scrolled to keep the cursor in view.
fn render_catalog_list(frame: &mut Frame, area: Rect, state: &AppState, selection: usize, qty: u32, focus: FabFocus) {
    let p = palette(state.color_mode);
    let rows = state.fabrication_recipes();
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);

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
        // The cursor is only "live" while the catalog holds focus.
        let cursor = if selected && focus == FabFocus::Catalog {
            " ▶ "
        } else if selected {
            " · "
        } else {
            "   "
        };
        let name_style = if selected {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else if affordable {
            text
        } else {
            dim
        };
        let mut spans = vec![
            Span::styled(cursor, Style::default().fg(p.accent)),
            Span::styled(format!("{mark} "), Style::default().fg(mark_color)),
            Span::styled(format!("{:<16}", recipe.name), name_style),
        ];
        // Show the quantity-to-add on the highlighted row.
        if selected {
            spans.push(Span::styled(
                format!(" ×{qty}"),
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(format!(" {}m", recipe.duration_seconds / 60), dim));
        }
        lines.push(Line::from(spans));
    }
    let visible = area.height as usize;
    let scroll = if sel_line >= visible {
        (sel_line - visible + 1) as u16
    } else {
        0
    };
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

/// Middle panel: the selected recipe's detail (output, duration, ingredients).
fn render_detail(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    sel: Option<&(Fabricator, &crate::api::types::CraftingRecipe)>,
    error: Option<&str>,
    p: &Palette,
) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let block = Block::default().borders(Borders::LEFT).border_style(dim);
    let detail_area = block.inner(area);
    frame.render_widget(block, area);

    let mut detail: Vec<Line> = Vec::new();
    if let Some((fab, recipe)) = sel {
        detail.push(Line::from(Span::styled(
            recipe.name.as_str(),
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        )));
        detail.push(builder_line(*fab, state, *p));
        detail.push(Line::default());
        detail.push(Line::from(vec![
            Span::styled("→ ", dim),
            Span::styled(recipe.output.name.as_str(), text),
        ]));
        detail.push(Line::from(Span::styled(
            format!("⏲ {} min", recipe.duration_seconds / 60),
            dim,
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
                Span::styled(
                    if ok { "✓ " } else { "✗ " },
                    Style::default().fg(if ok { p.good } else { p.crit }),
                ),
                Span::styled(format!("{:<16}", ing.ingredient_type), text),
                Span::styled(
                    format!("{have_str}/{need}"),
                    Style::default().fg(if ok { p.text } else { p.crit }),
                ),
            ]));
        }
    }
    if let Some(err) = error {
        detail.push(Line::default());
        detail.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(detail), detail_area);
}

/// Right panel: the live production queue.
fn render_queue_panel(frame: &mut Frame, area: Rect, state: &AppState, queue_sel: usize, focus: FabFocus, p: &Palette) {
    let dim = Style::default().fg(p.dim);
    let block = Block::default()
        .title(" QUEUE ")
        .borders(Borders::LEFT)
        .border_style(if focus == FabFocus::Queue {
            Style::default().fg(p.accent)
        } else {
            dim
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    if state.craft_queue.is_empty() {
        lines.push(Line::from(Span::styled("empty — Enter a recipe", dim)));
    }
    for (i, step) in state.craft_queue.iter().enumerate() {
        let (icon, icon_color) = match &step.state {
            StepState::Pending => ("⏳", p.dim),
            StepState::Running { .. } => ("▶", p.accent),
            StepState::Done => ("✓", p.good),
            StepState::Failed(_) => ("✗", p.crit),
        };
        let here = focus == FabFocus::Queue && i == queue_sel;
        let name_style =
            Style::default()
                .fg(p.text)
                .add_modifier(if here { Modifier::BOLD } else { Modifier::empty() });
        let mut spans = vec![
            Span::styled(if here { "›" } else { " " }, Style::default().fg(p.accent)),
            Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
            Span::styled(step.recipe_name.clone(), name_style),
        ];
        if step.repeat > 1 {
            spans.push(Span::styled(format!(" {}/{}", step.completed, step.repeat), dim));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn section_header(fab: Fabricator, p: Palette) -> Line<'static> {
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
                Line::from(Span::styled("⚠ no atomic printer", Style::default().fg(p.crit)))
            } else if state.collect_idle_onboard_mannies().is_empty() {
                Line::from(Span::styled("⚠ no idle manny to assist", Style::default().fg(p.warn)))
            } else {
                Line::from(Span::styled("⚛ printer reserves a manny", dim))
            }
        }
        Fabricator::Manny => {
            if let ActiveWizard::Fabrication(FabricationInput::PickRecipe {
                prefilled_manny: Some((_, name)),
                ..
            }) = &state.active_wizard
            {
                return Line::from(vec![
                    Span::styled("⚙ builder: ", dim),
                    Span::styled(name.clone(), Style::default().fg(p.text)),
                ]);
            }
            match state.collect_idle_onboard_mannies().len() {
                0 => Line::from(Span::styled("⚠ no idle manny on board", Style::default().fg(p.warn))),
                1 => Line::from(Span::styled("⚙ built by the idle manny", dim)),
                n => Line::from(Span::styled(format!("⚙ choose 1 of {n} idle mannies"), dim)),
            }
        }
    }
}
