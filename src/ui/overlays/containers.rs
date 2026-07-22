use crate::app::{ActiveWizard, AppState, ContainerRulesInput, RenameContainerInput};
use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};

pub(crate) fn render_rename_container_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::RenameContainer(RenameContainerInput::Typing {
        current_label,
        buf,
        error,
        ..
    }) = &state.active_wizard
    else {
        return;
    };

    let popup = centered_rect(50, 7, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" RENAME — {current_label} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled("Label: ", Style::default().fg(p.accent)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(p.accent)),
    ])];
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::commit("[Enter]", "RENAME"),
            FooterKey::nav("[Tab]", "suggest"),
            FooterKey::nav("[Esc]", "cancel"),
        ],
    );
}

pub(crate) fn render_container_rules_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
        container_label,
        types,
        priority,
        exclusion,
        strict_exclusion,
        selection,
        error,
        ..
    }) = &state.active_wizard
    else {
        return;
    };
    let selection = *selection;

    let height = (types.len() as u16 + 8).clamp(10, 24);
    let popup = centered_rect(70, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" ROUTING RULES — {container_label} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    // Directional wording: each tag says where the item goes, not the raw API
    // term — so [S] reads as "never here", not as a whitelist (issue #234).
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("P", Style::default().fg(p.good)),
            Span::raw(" prefer here  "),
            Span::styled("E", Style::default().fg(p.warn)),
            Span::raw(" avoid  "),
            Span::styled("S", Style::default().fg(p.crit)),
            Span::raw(" never here  "),
            Span::styled("[ ]", Style::default().fg(p.dim)),
            Span::raw(" any"),
        ])),
        rows[0],
    );

    let items: Vec<ListItem> = types
        .iter()
        .map(|ty| {
            // Tag + a plain-language effect per type, so a pilot reads what the
            // rule does without consulting the OpenAPI spec (issue #234).
            let (tag, color, effect) = if priority.iter().any(|t| t == ty) {
                ("[P]", p.good, "prefer here")
            } else if exclusion.iter().any(|t| t == ty) {
                ("[E]", p.warn, "avoid if possible")
            } else if strict_exclusion.iter().any(|t| t == ty) {
                ("[S]", p.crit, "never placed here")
            } else {
                ("[ ]", p.dim, "any container")
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{tag} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{ty:<20}"), Style::default().fg(p.text)),
                Span::styled(effect, Style::default().fg(p.dim)),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    let mut ls = ListState::default();
    if !types.is_empty() {
        ls.select(Some(selection.min(types.len() - 1)));
    }
    frame.render_stateful_widget(list, rows[1], &mut ls);

    if let Some(err) = error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("✗ {err}"),
                Style::default().fg(p.crit),
            ))),
            rows[2],
        );
    } else {
        render_footer(
            frame,
            rows[2],
            p,
            &[
                FooterKey::nav("[Space]", "cycle"),
                FooterKey::nav("[d]", "reserve"),
                FooterKey::nav("[Del]", "clear"),
                FooterKey::commit("[Enter]", "SAVE"),
                FooterKey::nav("[Esc]", "cancel"),
            ],
        );
    }
}
