use super::{centered_rect, render_footer, FooterKey};
use crate::app::{ActiveWizard, AppState, QueueInput, StepState};
use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// The production-queue overlay (`:queue`, #197): the crafting steps with their
/// status, a run/pause banner, and management keys.
pub(crate) fn render_queue_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::Queue(QueueInput::Browsing { selection }) = &state.active_wizard else {
        return;
    };
    let selection = *selection;

    let popup = centered_rect(68, 20, area);
    frame.render_widget(Clear, popup);
    let title = if state.queue_running {
        " PRODUCTION QUEUE · ▶ RUNNING "
    } else {
        " PRODUCTION QUEUE · ‖ PAUSED "
    };
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
    if state.craft_queue.is_empty() {
        lines.push(Line::from(Span::styled(
            "queue empty — press [Q] on a recipe in the fabrication wizard to add one",
            Style::default().fg(p.dim),
        )));
    } else {
        for (i, step) in state.craft_queue.iter().enumerate() {
            let (icon, icon_color) = match &step.state {
                StepState::Pending => ("⏳", p.dim),
                StepState::Running { .. } => ("▶", p.accent),
                StepState::Done => ("✓", p.good),
                StepState::Failed(_) => ("✗", p.crit),
            };
            let name_style = Style::default()
                .fg(p.text)
                .add_modifier(if i == selection { Modifier::BOLD } else { Modifier::empty() });
            let builder = step.builder_manny_name.as_deref().unwrap_or("atomic printer");
            let mut spans = vec![
                Span::styled(if i == selection { "› " } else { "  " }, Style::default().fg(p.accent)),
                Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                Span::styled(step.recipe_name.clone(), name_style),
            ];
            if step.repeat > 1 {
                spans.push(Span::styled(format!(" ×{}", step.repeat), Style::default().fg(p.text)));
                spans.push(Span::styled(
                    format!(" ({}/{})", step.completed, step.repeat),
                    Style::default().fg(p.dim),
                ));
            }
            spans.push(Span::styled(format!("  · {builder}"), Style::default().fg(p.dim)));
            if let StepState::Failed(e) = &step.state {
                spans.push(Span::styled(format!("  {e}"), Style::default().fg(p.crit)));
            }
            lines.push(Line::from(spans));
        }
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);

    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::commit("[r]", if state.queue_running { "pause" } else { "run" }),
            FooterKey::nav("[+/-]", "repeat"),
            FooterKey::danger("[x]", "remove"),
            FooterKey::danger("[c]", "clear"),
            FooterKey::nav("[Esc]", "close"),
        ],
    );
}
