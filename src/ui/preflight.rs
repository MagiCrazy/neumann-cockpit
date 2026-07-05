//! Boot preflight screen: the real startup checks (config → local archive →
//! remote link) run **inside the boot grid's centre Probe pane**, while the
//! eight surrounding subsystems stay dark until the link comes up. First-run
//! API-key onboarding happens in the same Probe pane. Rendered entirely inside
//! the alternate screen so a missing config never flashes a console and
//! vanishes.

use crate::app::{ColorMode, Pane};
use crate::preflight::{Status, Step};
use crate::ui::cockpit_v2::grid;
use crate::ui::theme::{palette, pane_block};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw the boot grid during preflight: the Probe pane shows the live check-list
/// (and the onboarding prompt / link-failure actions), the eight others sit dark
/// until preflight succeeds. `entry` is `Some(buf)` while collecting the API
/// key; `note` is an optional action/status line shown under the check-list.
pub(crate) fn render(
    frame: &mut Frame,
    area: Rect,
    steps: &[Step],
    entry: Option<&str>,
    note: Option<&str>,
    color: ColorMode,
) {
    let p = palette(color);
    let dim = Style::default().fg(p.dim);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    for (pane, rect) in grid::visible_panes(rows[0], Pane::Probe) {
        let is_probe = pane == Pane::Probe;
        let title = format!(" {} ", pane.label());
        let block = pane_block(&title, is_probe, p);
        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        if !is_probe {
            // Subsystems stay offline until the preflight clears.
            frame.render_widget(
                Paragraph::new(Line::styled("· · ·", dim)).alignment(Alignment::Center),
                inner,
            );
            continue;
        }
        frame.render_widget(Paragraph::new(probe_lines(steps, entry, note, p)), inner);
    }

    // Bottom banner — GUPPI narrating the preflight.
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(" GUPPI — preflight self-check", Style::default().fg(p.accent)))),
        rows[1],
    );
}

/// Build the Probe pane's content: the check-list, then either the onboarding
/// prompt or a status/action line. Kept compact so it fits the centre cell.
fn probe_lines(
    steps: &[Step],
    entry: Option<&str>,
    note: Option<&str>,
    p: crate::ui::theme::Palette,
) -> Vec<Line<'static>> {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let mut lines: Vec<Line> = Vec::new();

    for step in steps {
        let (mark, color, result) = match &step.status {
            Status::Pending => ("·", p.dim, String::new()),
            Status::Ok(m) => ("✓", p.good, m.clone()),
            Status::Warn(m) => ("⚠", p.warn, m.clone()),
            Status::Fail(m) => ("✗", p.crit, m.clone()),
        };
        let mut spans = vec![
            Span::styled(format!("{mark} "), Style::default().fg(color)),
            Span::styled(format!("{:<12}", step.label), text),
        ];
        if !result.is_empty() {
            spans.push(Span::styled(result, Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }

    if let Some(buf) = entry {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled("⚠ NO API KEY", Style::default().fg(p.warn).add_modifier(Modifier::BOLD))));
        lines.push(Line::from(Span::styled("get one at", dim)));
        lines.push(Line::from(Span::styled("neumann-probe.net", Style::default().fg(p.accent))));
        lines.push(Line::from(vec![
            Span::styled("KEY ›", Style::default().fg(p.accent)),
            Span::styled(buf.to_string(), text),
            Span::styled("▌", Style::default().fg(p.accent)),
        ]));
    }

    if let Some(note) = note {
        lines.push(Line::default());
        // Notes may carry several `\n`-separated lines (e.g. the link-failure
        // actions), so they fit the narrow centre cell.
        for part in note.split('\n') {
            let style = if part.starts_with('✗') {
                Style::default().fg(p.crit)
            } else {
                dim
            };
            lines.push(Line::from(Span::styled(part.to_string(), style)));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preflight::Status;
    use ratatui::{backend::TestBackend, Terminal};

    fn text(steps: &[Step], entry: Option<&str>, note: Option<&str>) -> String {
        // Large enough for the 3×3 boot grid (Probe pane centre).
        let mut t = Terminal::new(TestBackend::new(100, 33)).unwrap();
        t.draw(|f| render(f, f.area(), steps, entry, note, ColorMode::default())).unwrap();
        t.backend().buffer().content.iter().map(|c| c.symbol()).collect()
    }

    #[test]
    fn boot_grid_shows_probe_active_and_others_dark() {
        let steps = [Step { label: "CONFIG", status: Status::Pending }];
        let out = text(&steps, None, None);
        assert!(out.contains("PROBE"), "the centre Probe pane is framed");
        assert!(out.contains("SCANNER") && out.contains("MANNIES"), "surrounding panes are drawn too");
        assert!(out.contains("· · ·"), "the eight subsystems sit dark until preflight clears");
        assert!(out.contains("GUPPI"), "preflight banner");
    }

    #[test]
    fn onboarding_prompt_shows_where_to_get_the_key_and_the_buffer() {
        let steps = [Step { label: "CONFIG", status: Status::Pending }];
        let out = text(&steps, Some("vng_abc123"), None);
        assert!(out.contains("NO API KEY"), "onboarding heading");
        assert!(out.contains("neumann-probe.net"), "where to get the key");
        assert!(out.contains("vng_abc123"), "the typed key echoes");
    }

    #[test]
    fn link_failure_is_shown_with_actions() {
        let steps = [
            Step { label: "CONFIG", status: Status::Ok("loaded".into()) },
            Step { label: "REMOTE LINK", status: Status::Fail("401 unauthorized".into()) },
        ];
        let out = text(&steps, None, Some("[R]etry   [K] re-enter key\n[Enter] continue offline"));
        assert!(out.contains("REMOTE LINK") && out.contains("401 unauthorized"), "the bad-key error is visible");
        assert!(out.contains("[R]etry") && out.contains("re-enter key") && out.contains("continue offline"), "recovery actions offered");
    }
}
