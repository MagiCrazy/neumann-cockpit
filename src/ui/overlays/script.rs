use crate::app::{ActiveWizard, AppState, CompletionState, ScriptInput, ScriptVerb, StepState};
use crate::ui::theme::palette;
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
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
    let (inserting, buf, error, completion, selection) = match script {
        ScriptInput::Insert { buf, error, completion } => {
            (true, buf.as_str(), error.as_deref(), completion.as_ref(), 0)
        }
        ScriptInput::Normal { selection } => (false, "", None, None, *selection),
    };

    let height = (state.script.len() as u16 + 10).clamp(12, 28);
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

    // Insert region: input line + a hint line (candidates / usage) + the error
    // wrapped over as many rows as it needs (capped).
    let inner_w = inner.width.max(1) as usize;
    let err_h = error.map_or(0, |e| (e.chars().count() / inner_w + 1).min(3) as u16);
    // input (1) + hint/candidates (2, may wrap) + error.
    let insert_h = if inserting { 3 + err_h } else { 0 };

    // [ step list | insert region (insert mode) | footer ]
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(insert_h), Constraint::Length(1)])
        .split(inner);

    render_steps(frame, rows[0], state, inserting, selection, &p);
    if inserting {
        render_insert_line(frame, rows[1], state, buf, error, completion, &p);
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

    // Wrap so a long command line, or a failed step's appended error, reads out
    // in full instead of being clipped at the pane edge.
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_insert_line(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    buf: &str,
    error: Option<&str>,
    completion: Option<&CompletionState>,
    p: &Palette,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // Input line, horizontally scrolled so the caret (end of buf) stays visible
    // when the line is longer than the box. `> ` prefix + one cell for the caret.
    let avail = (rows[0].width as usize).saturating_sub(3);
    let chars: Vec<char> = buf.chars().collect();
    let shown: String = if chars.len() > avail {
        chars[chars.len() - avail..].iter().collect()
    } else {
        buf.to_owned()
    };
    let input = Line::from(vec![
        Span::styled("> ", Style::default().fg(p.accent)),
        Span::styled(shown, Style::default().fg(p.text)),
        Span::styled("▌", Style::default().fg(p.accent)),
    ]);
    frame.render_widget(Paragraph::new(input), rows[0]);

    // Hint line: the active Tab-completion candidates (current one highlighted),
    // else the recognised verb's argument grammar as dim ghost-text.
    if let Some(c) = completion.filter(|c| c.candidates.len() > 1) {
        let mut spans = Vec::new();
        for (i, cand) in c.candidates.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            if i == c.index {
                // The active candidate (the one Tab will insert) shows its
                // resource hint, so unnamed asteroids are told apart.
                let label = match state.object_resource_hint(cand) {
                    Some(h) => format!("{cand} · {h}"),
                    None => cand.clone(),
                };
                spans.push(Span::styled(
                    label,
                    Style::default().fg(p.text).add_modifier(Modifier::REVERSED),
                ));
            } else {
                spans.push(Span::styled(cand.clone(), Style::default().fg(p.dim)));
            }
        }
        frame.render_widget(Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true }), rows[1]);
    } else if let Some(usage) = buf
        .split_whitespace()
        .next()
        .and_then(ScriptVerb::parse)
        .map(|v| v.usage())
    {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {usage}"),
                Style::default().fg(p.dim),
            ))),
            rows[1],
        );
    }

    if let Some(e) = error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(e.to_owned(), Style::default().fg(p.crit))))
                .wrap(Wrap { trim: true }),
            rows[2],
        );
    }
}

fn render_script_footer(frame: &mut Frame, area: Rect, inserting: bool, p: &Palette) {
    let keys: Vec<FooterKey> = if inserting {
        vec![
            FooterKey::commit("[Enter]", "add line"),
            FooterKey::nav("[Tab]", "complete"),
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
