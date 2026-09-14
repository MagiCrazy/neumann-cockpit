use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};
use crate::app::{ActiveWizard, AppState, FireMissileInput};

/// The fire confirmation (issue #361).
///
/// Every other wizard in the cockpit spends the pilot's own resources; this one
/// damages someone else's. So the screen states the whole bill — target, what
/// it is, who fires, what it costs — and, when the target belongs to another
/// **player**, says in crit what a hit can do: v121 caps damage at the
/// remaining integrity and any drop to 0 % marks the target `dead` immediately.
/// That paragraph is the confirmation; it is not decoration around `Enter`.
pub(crate) fn render_fire_missile_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = state.palette();
    let ActiveWizard::FireMissile(FireMissileInput::Confirm {
        manny_name,
        object_name,
        target,
        error,
        ..
    }) = &state.active_wizard
    else {
        return;
    };

    let foreign = target.is_another_players();
    let popup = centered_rect(54, if foreign { 46 } else { 34 }, area);
    frame.render_widget(Clear, popup);
    let border = if foreign { p.crit } else { p.accent };
    let block = Block::default()
        .title(" FIRE MISSILE ")
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
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" target   ", dim),
            Span::styled(object_name.clone(), text.add_modifier(Modifier::BOLD)),
            Span::styled(format!("  ({})", target.label()), dim),
        ]),
        Line::from(vec![
            Span::styled(" by       ", dim),
            Span::styled(manny_name.clone(), text),
        ]),
        Line::from(vec![
            Span::styled(" cost     ", dim),
            Span::styled("1 missile", Style::default().fg(p.accent)),
            Span::styled(" · 1m preparation", dim),
        ]),
    ];

    if foreign {
        let crit = Style::default().fg(p.crit);
        lines.push(Line::default());
        lines.push(Line::styled(
            " ⚠ ANOTHER PLAYER'S ASSET",
            crit.add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled("   damage is capped at its remaining integrity,", crit));
        lines.push(Line::styled("   and a drop to 0% marks it dead.", crit));
    }

    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::styled(format!(" ✗ {err}"), Style::default().fg(p.crit)));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[FooterKey::nav("[Enter]", "fire"), FooterKey::nav("[Esc]", "cancel")],
    );
}
