//! The aiming wizard's screens (API v116, issue #308).
//!
//! Every screen is written so the pilot can tell the two modes apart without
//! having read the spec: an impact has a target and a speed, a transfer has a
//! heading and no destination at all.

use crate::app::{ActiveWizard, AimAsteroidInput, AppState};
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{centered_rect, render_footer, render_pick_list, FooterKey};

pub(crate) fn render_aim_asteroid_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = state.palette();
    let ActiveWizard::AimAsteroid(input) = &state.active_wizard else {
        return;
    };
    match input {
        AimAsteroidInput::PickMode {
            asteroid_name,
            selection,
            ..
        } => {
            let items = [
                "system impact — hit a body in this system",
                "sector transfer — set a heading and let it run",
            ];
            let items: Vec<&str> = items.to_vec();
            render_pick_list(
                frame,
                area,
                p,
                &format!(" LAUNCH «{asteroid_name}» "),
                62,
                9,
                Some("the tank is spent either way"),
                &items,
                *selection,
                None,
                "choose",
            );
        }
        AimAsteroidInput::PickImpactTarget { targets, selection, .. } => {
            let rows: Vec<String> = targets
                .iter()
                .map(|t| format!("{:<24} {}", t.name, object_kind(&t.object_type)))
                .collect();
            let items: Vec<&str> = rows.iter().map(String::as_str).collect();
            render_pick_list(
                frame,
                area,
                p,
                " IMPACT TARGET ",
                62,
                (targets.len() as u16 + 6).clamp(9, 20),
                Some("a body in this system — the asteroid arrives"),
                &items,
                *selection,
                None,
                "aim",
            );
        }
        AimAsteroidInput::PickHeading {
            headings, selection, ..
        } => {
            let from = state.probe_relative_coords().unwrap_or((0, 0, 0));
            let rows: Vec<String> = headings
                .iter()
                .map(|h| {
                    let (dx, dy, dz) = h.delta(from);
                    let seen = if h.visited { "visited" } else { "never scanned" };
                    format!("({:>3},{:>3},{:>3})  {dx:+} {dy:+} {dz:+}   {seen}", h.x, h.y, h.z)
                })
                .collect();
            let items: Vec<&str> = rows.iter().map(String::as_str).collect();
            render_pick_list(
                frame,
                area,
                p,
                " HEADING ",
                62,
                18,
                // Said on the picker as well as on the confirm: this is the
                // screen where a pilot forms the wrong idea.
                Some("a direction, not a destination"),
                &items,
                *selection,
                None,
                "set",
            );
        }
        AimAsteroidInput::EnterSpeed { target, buf, error, .. } => {
            render_speed(frame, area, p, &target.name, buf, error.as_deref())
        }
        AimAsteroidInput::Confirm {
            asteroid_name,
            summary,
            warning,
            ..
        } => render_confirm(frame, area, p, asteroid_name, summary, warning.as_deref()),
    }
}

fn object_kind(t: &crate::api::types::SectorObjectType) -> &'static str {
    use crate::api::types::SectorObjectType as T;
    match t {
        T::Star => "star",
        T::Planet => "planet",
        T::Asteroid => "asteroid",
        _ => "",
    }
}

fn render_speed(frame: &mut Frame, area: Rect, p: Palette, target: &str, buf: &str, error: Option<&str>) {
    let popup = centered_rect(58, 9, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" IMPACT SPEED ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines = vec![
        Line::styled(format!("aiming at «{target}»"), Style::default().fg(p.dim)),
        Line::default(),
        Line::styled(
            format!("{buf}█ c"),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!("fractions of light speed, up to {}c", crate::app::MAX_TARGET_SPEED_C),
            Style::default().fg(p.dim),
        ),
    ];
    if let Some(err) = error {
        lines.push(Line::styled(format!("✗ {err}"), Style::default().fg(p.crit)));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[FooterKey::commit("[Enter]", "confirm"), FooterKey::nav("[Esc]", "back")],
    );
}

fn render_confirm(frame: &mut Frame, area: Rect, p: Palette, asteroid: &str, summary: &str, warning: Option<&str>) {
    let popup = centered_rect(64, 10, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" LAUNCH ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.crit));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let mut lines = vec![
        Line::styled(
            format!("«{asteroid}»"),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ),
        Line::styled(summary.to_string(), Style::default().fg(p.text)),
        Line::default(),
    ];
    if let Some(w) = warning {
        lines.push(Line::styled(format!("⚠ {w}"), Style::default().fg(p.warn)));
    }
    lines.push(Line::styled(
        "mannies mining into a container on it are sent home",
        Style::default().fg(p.dim),
    ));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::danger("[Enter/y]", "launch"),
            FooterKey::nav("[Esc]", "cancel"),
        ],
    );
}
