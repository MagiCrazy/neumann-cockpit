//! Unified Cockpit v2 interface — the 3×3 tiling dashboard (blocs U2–U7).
//!
//! Responsive grid + navigation: `ertdfgcvb` selects a pane, `jk` moves the
//! cursor, `l`/`h` drill in/out, `z` zooms, `Enter` opens the contextual menu.
//! Colours come from the active [`palette`] (config `theme`, F2 cycles). The
//! four original panes reuse their existing renderers (still on classic
//! colours until they get cockpit-native renderers); the five promoted panes
//! use the compact renderers in [`panes`].

pub(crate) mod grid;
mod menu;
mod panes;

use crate::app::{AppState, DrillLevel, Pane};
use crate::ui::panels::{render_inventory_panel, render_mannies_panel, render_probe_panel, render_scanner_panel};
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
                Paragraph::new(Line::styled("· · ·", Style::default().fg(p.dim))).alignment(Alignment::Center),
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
            let pad = width.saturating_sub(base_len + result.chars().count() + 1).max(1);
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
            let cur = if (state.boot_frame / 3).is_multiple_of(2) {
                "▌"
            } else {
                " "
            };
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
    // The command line always claims the second row while `:` is open, even if
    // the hints line is toggled off.
    let command = if let crate::app::InputMode::Command(cmd) = &state.mode {
        Some(cmd)
    } else {
        None
    };
    let want_second = state.hints_visible || command.is_some();
    let (bar, second) = if want_second && area.height >= 2 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    render_status_line(frame, bar, state, p, visible);
    if let Some(second_area) = second {
        let line = if let Some(cmd) = command {
            // `:verb args` with the caret at cmd.cursor (accent = text-entry
            // mode). The caret is a REVERSED cell over the character it sits on,
            // or a block when it's past the end — so mid-line edits are visible.
            let chars: Vec<char> = cmd.input.chars().collect();
            let cur = cmd.cursor.min(chars.len());
            let accent = Style::default().fg(p.accent);
            let before: String = chars[..cur].iter().collect();
            let mut spans = vec![Span::styled(format!(" :{before}"), accent)];
            if cur < chars.len() {
                spans.push(Span::styled(
                    chars[cur].to_string(),
                    accent.add_modifier(Modifier::REVERSED),
                ));
                spans.push(Span::styled(chars[cur + 1..].iter().collect::<String>(), accent));
            } else {
                spans.push(Span::styled("█", accent));
            }
            // Trailing helper: the active Tab-completion candidate list (when
            // cycling more than one), else the recognised verb's argument usage
            // as dim ghost-text.
            if let Some(comp) = cmd.completion.as_ref().filter(|c| c.candidates.len() > 1) {
                spans.push(Span::styled("   ", Style::default()));
                for (i, cand) in comp.candidates.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::styled(" ", Style::default().fg(p.dim)));
                    }
                    let style = if i == comp.index {
                        accent.add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().fg(p.dim)
                    };
                    spans.push(Span::styled(cand.clone(), style));
                }
            } else if let Some(usage) = cmd.input.split_whitespace().next().and_then(crate::app::command_usage) {
                spans.push(Span::styled(format!("   {usage}"), Style::default().fg(p.dim)));
            }
            Line::from(spans)
        } else {
            Line::from(Span::styled(
                format!(" {}", state.pane_hints()),
                Style::default().fg(p.dim),
            ))
        };
        frame.render_widget(Paragraph::new(line), second_area);
    }
}

fn render_status_line(frame: &mut Frame, area: Rect, state: &AppState, p: Palette, visible: &[Pane]) {
    let tag = if state.zoomed { "ZOOM" } else { state.mode.tag() };

    let mut left = vec![
        Span::styled(
            format!(" {tag} "),
            Style::default()
                .fg(Color::Black)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}", state.breadcrumb().join(" › ")),
            Style::default().fg(p.accent),
        ),
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
                Style::default()
                    .fg(Color::Black)
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else if visible.contains(pane) {
                Style::default().fg(p.text)
            } else {
                Style::default().fg(p.dim)
            };
            left.push(Span::styled(key, style));
        }
    }
    // An error takes over the line until dismissed; otherwise a success toast.
    // Keep the leading gap in a plain span — `crit_style()` is REVERSED in the
    // mono palettes, which would otherwise highlight the spaces before the mark.
    if let Some(err) = &state.error {
        left.push(Span::styled("  ", Style::default()));
        left.push(Span::styled(format!("✗ {err}"), p.crit_style()));
    } else if let Some(toast) = state.active_toast() {
        left.push(Span::styled(format!("  ✓ {toast}"), Style::default().fg(p.good)));
    }

    let mut meta: Vec<(String, Style)> = Vec::new();
    let dim = Style::default().fg(p.dim);
    let accent = Style::default().fg(p.accent);
    // Sync status: spinner while a refresh is in flight, else the age since the
    // last successful sync (ticks live via the 1 s UI tick).
    if state.loading {
        meta.push(("⟳ sync".to_string(), accent));
    } else if let Some(age) = state.seconds_since_sync() {
        let label = if age < 60 {
            format!("⟳ {age}s")
        } else {
            format!("⟳ {}m", age / 60)
        };
        meta.push((label, dim));
    }
    // Active probe (multi-probe, API v81): shown only when the fleet has more
    // than one probe, or the pilot is on a non-default probe — single-probe
    // setups stay uncluttered. A non-default active probe is accented to flag
    // that the cockpit is off the default; an unreachable one is marked.
    if let Some(active) = state.active_probe_summary() {
        if state.fleet.len() > 1 || !active.is_default {
            let icon = if active.is_reachable { "⏻" } else { "⚠" };
            let style = if active.is_default { dim } else { accent };
            meta.push((format!("{icon} {}", active.name), style));
        }
    }
    if !state.scut_coverage().is_empty() {
        meta.push(("≣ SCUT".to_string(), accent));
    }
    // Idle-Manny nudge: bold warning colour so free workers don't go unnoticed
    // (`i` cycles to the next one).
    let idle = state.idle_manny_count();
    if idle > 0 {
        meta.push((
            format!("⚙ {idle} idle"),
            Style::default().fg(p.warn).add_modifier(Modifier::BOLD),
        ));
    }
    let unread = state.unread_alert_count();
    if unread > 0 {
        // Urgency signal: crit_style survives the mono palettes (bold+REVERSED).
        meta.push((format!("! {unread}"), p.crit_style()));
    }
    if let Some(v) = state.api_version {
        meta.push((format!("API v{v}"), dim));
    }
    // Live wall clock — ticks every second via the 1 s UI tick. (Sync recency
    // lives in the ⟳ indicator above, so this is a real clock, not last-sync.)
    meta.push((chrono::Local::now().format("%H:%M:%S").to_string(), dim));
    let meta_len: usize = meta.iter().map(|(s, _)| s.chars().count() + 3).sum();
    let meta_spans: Vec<Span> = meta
        .iter()
        .enumerate()
        .flat_map(|(i, (s, st))| {
            let sep = if i == 0 { "" } else { " · " };
            [
                Span::styled(sep, Style::default().fg(p.dim)),
                Span::styled(s.clone(), *st),
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
