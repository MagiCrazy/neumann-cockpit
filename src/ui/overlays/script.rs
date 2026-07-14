use crate::app::{ActiveWizard, AppState, ScriptInput, StepState};
use crate::ui::theme::palette;
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{render_footer, FooterKey};

/// The action-scripting console (#198): a vim-style modal editor over the
/// session script. Shows the ordered step list (with run state) and, in insert
/// mode, the command line being typed.
pub(crate) fn render_script_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ActiveWizard::Script(script) = &state.active_wizard else {
        return;
    };
    let p = palette(state.color_mode);
    let (inserting, buf, error, selection) = match script {
        ScriptInput::Insert { buf, error } => (true, buf.as_str(), error.as_deref(), 0),
        ScriptInput::Normal { selection } => (false, "", None, *selection),
    };

    let height = (state.script.len() as u16 + 8).clamp(12, 26);
    let popup = crate::ui::overlays::centered_rect(78, height.min(area.height.saturating_sub(2)), area);
    frame.render_widget(Clear, popup);

    // Title banner reflects the run state.
    let (done, total) = state.script_progress();
    let banner = if total == 0 {
        " compose".into()
    } else if state.script_running {
        format!(" ▶ {done}/{total}")
    } else if state.script.iter().any(|s| matches!(s.state, StepState::Failed(_))) {
        format!(" ✗ {done}/{total}")
    } else {
        format!(" ‖ {done}/{total}")
    };
    let block = Block::default()
        .title(format!(" ACTION SCRIPT ·{banner} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // [ step list | insert line (insert mode) | footer ]
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(if inserting { 2 } else { 0 }),
            Constraint::Length(1),
        ])
        .split(inner);

    render_steps(frame, rows[0], state, inserting, selection, &p);
    if inserting {
        render_insert_line(frame, rows[1], buf, error, &p);
    }
    render_script_footer(frame, rows[2], inserting, &p);
}

fn render_steps(frame: &mut Frame, area: Rect, state: &AppState, inserting: bool, selection: usize, p: &Palette) {
    if state.script.is_empty() {
        let hint = Paragraph::new(Line::from(Span::styled(
            "empty — press [i] to add a command line",
            Style::default().fg(p.dim),
        )));
        frame.render_widget(hint, area);
        return;
    }

    let lines: Vec<Line> = state
        .script
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let (sym, sym_style) = match &step.state {
                StepState::Pending => ("·", Style::default().fg(p.dim)),
                StepState::Running { .. } => ("▶", Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
                StepState::Done => ("✓", Style::default().fg(p.good)),
                StepState::Failed(_) => ("✗", Style::default().fg(p.crit).add_modifier(Modifier::BOLD)),
            };
            // In Normal mode the cursor row is emphasised.
            let selected = !inserting && i == selection;
            let text_style = if selected {
                Style::default().fg(p.text).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(p.text)
            };
            let mut spans = vec![
                Span::styled(format!(" {sym} "), sym_style),
                Span::styled(format!("{}. {}", i + 1, step.raw), text_style),
            ];
            if let StepState::Failed(e) = &step.state {
                spans.push(Span::styled(format!("  — {e}"), Style::default().fg(p.crit)));
            }
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_insert_line(frame: &mut Frame, area: Rect, buf: &str, error: Option<&str>, p: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    let line = Line::from(vec![
        Span::styled("> ", Style::default().fg(p.accent)),
        Span::styled(buf.to_owned(), Style::default().fg(p.text)),
        Span::styled("▌", Style::default().fg(p.accent)),
    ]);
    frame.render_widget(Paragraph::new(line), rows[0]);
    if let Some(e) = error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(e.to_owned(), Style::default().fg(p.crit)))),
            rows[1],
        );
    }
}

fn render_script_footer(frame: &mut Frame, area: Rect, inserting: bool, p: &Palette) {
    let keys: Vec<FooterKey> = if inserting {
        vec![
            FooterKey::commit("[Enter]", "add line"),
            FooterKey::nav("[Esc]", "manage"),
        ]
    } else {
        vec![
            FooterKey::nav("[i]", "insert"),
            FooterKey::nav("[j/k]", "move"),
            FooterKey::danger("[x]", "remove"),
            FooterKey::danger("[c]", "clear"),
            FooterKey::commit("[R]", "run"),
            FooterKey::nav("[p]", "pause"),
            FooterKey::nav("[Esc]", "close"),
        ]
    };
    render_footer(frame, area, *p, &keys);
}
