//! Unified Cockpit v2 interface — the 3×3 tiling dashboard (blocs U2–U7).
//!
//! Responsive grid + navigation: `ertdfgcvb` selects a pane, `jk` moves the
//! cursor, `l`/`h` drill in/out, `z` zooms, `Enter` opens the contextual menu.
//! Colours come from the active [`palette`] (config `theme`, F2 cycles). The
//! four original panes reuse their existing renderers (still on classic
//! colours until they get cockpit-native renderers); the five promoted panes
//! use the compact renderers in [`panes`].

mod grid;
mod menu;
mod panes;

use crate::app::{AppState, DrillLevel, Pane};
use crate::ui::panels::{
    render_inventory_panel, render_mannies_panel, render_probe_panel, render_scanner_panel,
};
use crate::ui::theme::{palette, pane_block, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let p = palette(state.color_mode);

    if state.booting {
        render_boot(frame, area, state, p);
        return;
    }

    let status_h = if state.hints_visible { 2 } else { 1 };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(status_h)])
        .split(area);

    let visible: Vec<Pane> = if state.zoomed {
        render_pane(frame, rows[0], state.active_pane, state, true, p);
        vec![state.active_pane]
    } else {
        let panes = grid::visible_panes(rows[0], state.active_pane);
        for (pane, rect) in &panes {
            render_pane(frame, *rect, *pane, state, *pane == state.active_pane, p);
        }
        panes.iter().map(|(pane, _)| *pane).collect()
    };
    render_status(frame, rows[1], state, p, &visible);

    // Contextual menu popup, then any active wizard overlay on top.
    if let crate::app::InputMode::Menu(m) = &state.mode {
        menu::render(frame, area, m, p);
    }
    crate::ui::overlays::render_active_overlays(frame, area, state);
}

/// Boot self-check: the probe boots first, then each subsystem pane comes
/// online centre-out and types out its own themed check lines.
fn render_boot(frame: &mut Frame, area: Rect, state: &AppState, p: Palette) {
    use crate::app::{BOOT_CHARS_PER_FRAME, BOOT_LINE_STRIDE};

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    for (pane, rect) in grid::visible_panes(rows[0], state.active_pane) {
        // The pane coming online this frame glows (bright accent border).
        let title = format!(" {} ", pane.label());
        let block = pane_block(&title, state.boot_leading(pane), p);
        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        if !state.boot_revealed(pane) {
            frame.render_widget(
                Paragraph::new(Line::styled("· · ·", Style::default().fg(p.dim)))
                    .alignment(Alignment::Center),
                inner,
            );
            continue;
        }

        // Type out the pane's self-check lines, one after another.
        let elapsed = state.boot_elapsed(pane);
        let width = inner.width as usize;
        let mut lines: Vec<Line> = Vec::new();
        for (i, (label, result)) in state.boot_check_lines(pane).iter().enumerate() {
            let start = i as u64 * BOOT_LINE_STRIDE;
            if elapsed < start {
                break;
            }
            let base_len = label.chars().count() + 1;
            let pad = width
                .saturating_sub(base_len + result.chars().count() + 1)
                .max(1);
            let dotted = format!("{label} {}", "·".repeat(pad));
            let revealed = (elapsed - start) as usize * BOOT_CHARS_PER_FRAME;
            if revealed >= dotted.chars().count() {
                lines.push(Line::from(vec![
                    Span::styled(dotted, Style::default().fg(p.dim)),
                    Span::styled(
                        format!(" {result}"),
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                let partial: String = dotted.chars().take(revealed).collect();
                lines.push(Line::from(Span::styled(partial, Style::default().fg(p.dim))));
            }
        }
        // Once the self-check is done, invite the pilot to continue — in the
        // centre probe pane.
        if pane == Pane::Probe && state.boot_complete() {
            let cur = if (state.boot_frame / 3).is_multiple_of(2) { "▌" } else { " " };
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "— WE ARE LEGION —",
                Style::default().fg(p.dim),
            )));
            lines.push(Line::from(Span::styled(
                format!("{cur} ANY KEY TO CONTINUE"),
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            )));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }

    // Bottom banner — GUPPI reporting.
    let status = if state.boot_complete() {
        "all systems online"
    } else {
        "self-check…"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" GUPPI — {status}"),
            Style::default().fg(p.accent),
        ))),
        rows[1],
    );
}

fn render_pane(frame: &mut Frame, area: Rect, pane: Pane, state: &AppState, active: bool, p: Palette) {
    match pane {
        // Reused classic renderers keep their own colours for now.
        Pane::Probe => render_probe_panel(frame, area, state, active),
        Pane::Inventory => render_inventory_panel(frame, area, state, active),
        Pane::Scanner => {
            if state.zoomed {
                panes::render_scanner_neighbors(frame, area, state, active);
            } else {
                render_scanner_panel(frame, area, state, active);
            }
        }
        Pane::Mannies => {
            if let Some(DrillLevel::Manny(id)) = state.pane_nav[Pane::Mannies.index()].drill.last() {
                panes::render_manny_detail(frame, area, state, id, active, p);
            } else if state.zoomed {
                // Zoom shows the whole fleet at a glance, one detail card each.
                panes::render_mannies_overview(frame, area, state, p);
            } else {
                render_mannies_panel(frame, area, state, active);
            }
        }
        // Promoted panes are palette-aware.
        Pane::Map => panes::render_map(frame, area, state, active, p),
        Pane::Comms => panes::render_comms(frame, area, state, active, p),
        Pane::Sector => panes::render_sector(frame, area, state, active, p),
        Pane::Missions => panes::render_missions(frame, area, state, active, p),
        Pane::Storage => panes::render_storage(frame, area, state, active, p),
    }
}

fn render_status(frame: &mut Frame, area: Rect, state: &AppState, p: Palette, visible: &[Pane]) {
    let (bar, hints) = if state.hints_visible && area.height >= 2 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    render_status_line(frame, bar, state, p, visible);
    if let Some(hints_area) = hints {
        let line = Line::from(Span::styled(
            format!(" {}", state.pane_hints()),
            Style::default().fg(p.dim),
        ));
        frame.render_widget(Paragraph::new(line), hints_area);
    }
}

fn render_status_line(frame: &mut Frame, area: Rect, state: &AppState, p: Palette, visible: &[Pane]) {
    let tag = if state.zoomed { "ZOOM" } else { state.mode.tag() };

    let mut left = vec![
        Span::styled(
            format!(" {tag} "),
            Style::default().fg(Color::Black).bg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", state.breadcrumb().join(" › ")), Style::default().fg(p.accent)),
    ];
    // Position mini-map: shown only when the grid is reduced (not all 9 panes
    // visible), so you know where the active pane sits. Groups of three keys
    // evoke the 3×3 rows on a single line.
    if visible.len() < Pane::ALL.len() {
        left.push(Span::styled("   ", Style::default()));
        for (i, pane) in Pane::ALL.iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                left.push(Span::styled(" ", Style::default().fg(p.dim)));
            }
            let key = pane.key_label().to_string();
            let style = if *pane == state.active_pane {
                Style::default().fg(Color::Black).bg(p.accent).add_modifier(Modifier::BOLD)
            } else if visible.contains(pane) {
                Style::default().fg(p.text)
            } else {
                Style::default().fg(p.dim)
            };
            left.push(Span::styled(key, style));
        }
    }
    // An error takes over the line until dismissed; otherwise a success toast.
    if let Some(err) = &state.error {
        left.push(Span::styled(format!("  ✗ {err}"), Style::default().fg(p.crit)));
    } else if let Some(toast) = state.active_toast() {
        left.push(Span::styled(format!("  ✓ {toast}"), Style::default().fg(p.good)));
    }

    let mut meta = Vec::new();
    // Sync status: spinner while a refresh is in flight, else the age since the
    // last successful sync (ticks live via the 1 s UI tick).
    if state.loading {
        meta.push(("⟳ sync".to_string(), p.accent));
    } else if let Some(age) = state.seconds_since_sync() {
        let label = if age < 60 { format!("⟳ {age}s") } else { format!("⟳ {}m", age / 60) };
        meta.push((label, p.dim));
    }
    if !state.scut_coverage().is_empty() {
        meta.push(("≣ SCUT".to_string(), p.accent));
    }
    let unread = state.unread_alert_count();
    if unread > 0 {
        meta.push((format!("! {unread}"), p.crit));
    }
    if let Some(v) = state.api_version {
        meta.push((format!("API v{v}"), p.dim));
    }
    // Live wall clock — ticks every second via the 1 s UI tick. (Sync recency
    // lives in the ⟳ indicator above, so this is a real clock, not last-sync.)
    meta.push((chrono::Local::now().format("%H:%M:%S").to_string(), p.dim));
    let meta_len: usize = meta.iter().map(|(s, _)| s.chars().count() + 3).sum();
    let meta_spans: Vec<Span> = meta
        .iter()
        .enumerate()
        .flat_map(|(i, (s, c))| {
            let sep = if i == 0 { "" } else { " · " };
            [
                Span::styled(sep, Style::default().fg(p.dim)),
                Span::styled(s.clone(), Style::default().fg(*c)),
            ]
        })
        .collect();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(meta_len as u16 + 1)])
        .split(area);
    frame.render_widget(Paragraph::new(Line::from(left)), cols[0]);
    frame.render_widget(
        Paragraph::new(Line::from(meta_spans)).alignment(Alignment::Right),
        cols[1],
    );
}
