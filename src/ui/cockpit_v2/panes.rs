//! Compact renderers for the five panes promoted from overlays (blocs U2–U7):
//! Map, Comms, Sector, Missions, Storage. The four original panes (Probe,
//! Inventory, Scanner, Mannies) reuse their existing renderers.
//!
//! Each shows a terse summary sized for a 1/3 grid cell; drilling in (`l`)
//! swaps a pane to its detail view (Missions → steps, Comms → message).
//! Colours come from the active [`Palette`].

use crate::api::types::{
    Manny, MannyLocationType, MannyTaskVisibility, MissionStatus, MissionStepStatus, SectorObject, SectorObjectType,
};
use crate::app::{AppState, CommsCategory, DrillLevel, MissionsCategory, Pane};
use crate::ui::panels::mannies::{
    manny_artificial_detection, manny_crafting_detail, manny_mining_detail, manny_task_eta, manny_task_label,
    manny_task_progress,
};
use crate::ui::panels::scanner::{resource_shares_line, sector_object_lines};
use crate::ui::theme::{
    block_gauge_line, object_color, object_icon, object_type_label, pane_block, ratio_color, Palette,
};
use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// Style for the selected row: highlighted only while the pane is active.
fn row_style(active: bool, selected: bool) -> Style {
    if active && selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}

/// Fill colour for a "how full" ratio, mapped through the palette so mono
/// modes stay single-hue.
fn fill_color(p: Palette, ratio: f64) -> ratatui::style::Color {
    if ratio > 0.5 {
        p.good
    } else if ratio > 0.25 {
        p.warn
    } else {
        p.crit
    }
}

fn cursor(state: &AppState, pane: Pane) -> usize {
    state.pane_nav[pane.index()].cursor
}

/// Vertical scroll offset that keeps `cursor_line` on screen within `height`
/// visible rows out of `total`. Scrolls only when the content overflows, keeps
/// the cursor on the last visible row when scrolling down, and never scrolls
/// past the end.
fn scroll_offset(cursor_line: usize, total: usize, height: usize) -> u16 {
    if height == 0 || total <= height {
        return 0;
    }
    let max_off = total - height;
    let off = if cursor_line < height {
        0
    } else {
        cursor_line + 1 - height
    };
    off.min(max_off) as u16
}

/// Render a pane body. When `cursor_line` is `Some`, the content is scrolled so
/// that absolute line index stays visible — the compact list panes render more
/// rows than a 1/3 cell can show, so without this the cursor (and the row
/// `Enter` acts on) can sit off-screen.
fn render_body(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    active: bool,
    p: Palette,
    lines: Vec<Line>,
    cursor_line: Option<usize>,
) {
    let block = pane_block(title, active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let offset = cursor_line
        .map(|c| scroll_offset(c, lines.len(), inner.height as usize))
        .unwrap_or(0);
    frame.render_widget(Paragraph::new(lines).scroll((offset, 0)), inner);
}

pub fn render_map(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    match state.probe_sector_coords() {
        Some((x, y, z)) => lines.push(Line::styled(
            format!("sector ({x}, {y}, {z})"),
            Style::default().fg(p.text),
        )),
        None => lines.push(Line::styled("sector unknown", dim)),
    }
    lines.push(Line::styled(
        format!("visited: {}", state.visited_sectors.len()),
        Style::default().fg(p.text),
    ));
    let nets = state.scut_coverage();
    if nets.is_empty() {
        lines.push(Line::styled("SCUT: no coverage", dim));
    } else {
        lines.push(Line::styled(
            format!("≣ SCUT: {} network(s)", nets.len()),
            Style::default().fg(p.accent),
        ));
    }
    lines.push(Line::styled("z open map · Enter travel", dim));
    render_body(frame, area, " MAP ", active, p, lines, None);
}

pub fn render_comms(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    match state.comms_drill() {
        Some(CommsCategory::Alerts) => {
            render_comms_feed(frame, area, state, active, p, "ALERTS", &state.alerts);
        }
        Some(CommsCategory::Warnings) => {
            render_comms_feed(frame, area, state, active, p, "WARNINGS", &state.damage_warnings);
        }
        _ => render_comms_root(frame, area, state, active, p),
    }
}

/// Comms root: each category shows its totals + unread count, plus a one-line
/// preview of its most recent entry. Only the category header rows are
/// selectable (the cursor drills the category); previews are decorative.
fn render_comms_root(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let cur = cursor(state, Pane::Comms);
    let latest_msg = state.messages.first().map(|m| {
        let icon = if m.status == crate::api::types::MessageStatus::Unread {
            "✉"
        } else {
            "·"
        };
        format!("{icon} {}: {}", m.sender.name, comms_preview(&m.body))
    });
    let latest_alert = state.alerts.first().map(|a| format!("✱ {}", comms_preview(&a.message)));
    let latest_warn = state
        .damage_warnings
        .first()
        .map(|w| format!("⚠ {}", comms_preview(&w.message)));
    let rows = [
        (
            "Messages",
            state.messages.len(),
            state.unread_message_count(),
            latest_msg,
        ),
        ("Alerts", state.alerts.len(), state.unread_alert_count(), latest_alert),
        (
            "Warnings",
            state.damage_warnings.len(),
            state.damage_warnings.iter().filter(|w| w.is_unread()).count(),
            latest_warn,
        ),
    ];
    let mut lines = Vec::new();
    let mut sel_line = None;
    for (i, (label, total, unread, preview)) in rows.iter().enumerate() {
        if i == cur {
            sel_line = Some(lines.len());
        }
        let mut spans = vec![
            Span::styled(format!("{label:<9} "), row_style(active, i == cur).patch(text)),
            Span::styled(format!("{total}"), dim),
        ];
        if *unread > 0 {
            spans.push(Span::styled(
                format!("  ({unread} unread)"),
                Style::default().fg(p.accent),
            ));
        }
        lines.push(Line::from(spans));
        // Preview of the most recent entry (dash when the category is empty).
        let preview = preview.clone().unwrap_or_else(|| "—".to_string());
        lines.push(Line::from(Span::styled(format!("  {preview}"), dim)));
    }
    render_body(frame, area, " COMMS ", active, p, lines, sel_line);
}

/// One-line preview: newlines flattened, truncated with an ellipsis.
fn comms_preview(body: &str) -> String {
    let one = body.replace('\n', " ");
    if one.chars().count() > 28 {
        format!("{}…", one.chars().take(28).collect::<String>())
    } else {
        one
    }
}

/// A drilled-in Comms feed (alerts or damage warnings): one compact row each,
/// unread marked, scrolled to the cursor. `Enter` acknowledges the selected one.
fn render_comms_feed(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
    p: Palette,
    label: &str,
    entries: &[crate::api::types::ProbeAlert],
) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let cur = cursor(state, Pane::Comms);
    let mut lines = Vec::new();
    let mut sel_line = None;
    if entries.is_empty() {
        lines.push(Line::styled(format!("no {}", label.to_lowercase()), dim));
    }
    for (i, a) in entries.iter().enumerate() {
        if i == cur {
            sel_line = Some(lines.len());
        }
        let unread = a.is_unread();
        let (mark, mark_style) = if unread {
            ("● ", Style::default().fg(p.accent))
        } else {
            ("○ ", dim)
        };
        let body_style = if unread { text } else { dim };
        lines.push(Line::from(vec![
            Span::styled(mark, mark_style),
            Span::styled(a.message.clone(), row_style(active, i == cur).patch(body_style)),
        ]));
    }
    render_body(frame, area, &format!(" COMMS › {label} "), active, p, lines, sel_line);
}

pub fn render_sector(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let Some(s) = state.current_sector() else {
        render_body(
            frame,
            area,
            " SECTOR ",
            active,
            p,
            vec![Line::styled("no sector scan yet", dim)],
            None,
        );
        return;
    };
    let v = &s.relative_coordinates;
    let header = format!(
        "({}, {}, {})  d{}",
        v.x as i32,
        v.y as i32,
        v.z as i32,
        s.active_distance(state.active_probe_id)
    );

    // Zoom: the science station — full per-object detail. Solar systems get a
    // merged per-body breakdown (star, each planet's class/habitability, each
    // mineable body's resources); standalone objects reuse the scanner's
    // verbose object lines.
    if state.zoomed {
        let mut lines = vec![Line::styled(header, text), Line::raw("")];
        match &s.objects {
            Some(objs) if !objs.is_empty() => {
                for obj in objs {
                    if obj.object_type == SectorObjectType::SolarSystem {
                        lines.extend(solar_system_zoom_lines(obj, p));
                    } else {
                        lines.extend(sector_object_lines(obj, false, p));
                    }
                    lines.push(Line::raw(""));
                }
            }
            _ => lines.push(Line::styled("empty sector", dim)),
        }
        render_body(frame, area, " SECTOR ", active, p, lines, None);
        return;
    }

    // Compact: navigable object list, each row tagged with its headline data
    // (system star/planet count, asteroid composition/resources, planet
    // habitability).
    let mut lines = vec![Line::styled(header, text)];
    let objs = state.scanner_objects();
    lines.push(Line::styled(format!("{} object(s)", objs.len()), dim));
    let cur = cursor(state, Pane::Sector);
    let mut sel_line = None;
    for (i, e) in objs.iter().enumerate() {
        let color = object_color(&e.object_type, p);
        let icon = object_icon(&e.object_type).0;
        // Keep the compact row width bounded, but mark a cut name with an
        // ellipsis so a long label (e.g. "Astéroïde contenant du Deutérium")
        // reads as truncated rather than broken mid-word.
        let name: String = if e.name.chars().count() > 16 {
            e.name.chars().take(15).chain(std::iter::once('…')).collect()
        } else {
            e.name.clone()
        };
        let mut spans: Vec<Span<'static>> = Vec::new();
        // Detached containers hidden on an asteroid render indented under their
        // host (see scanner_objects hierarchy ordering).
        if e.attached {
            spans.push(Span::styled("  └ ".to_string(), Style::default().fg(p.dim)));
        }
        spans.push(Span::styled(format!("{icon} "), Style::default().fg(color)));
        spans.push(Span::styled(name, row_style(active, i == cur).patch(text)));
        spans.extend(sector_entry_tags(state, &e.id, p));
        if i == cur {
            sel_line = Some(lines.len());
        }
        lines.push(Line::from(spans));
    }
    render_body(frame, area, " SECTOR ", active, p, lines, sel_line);
}

/// Join mineable resource types into a terse label (`metals ice carbon`).
fn resource_types_str(types: &[String]) -> String {
    types
        .iter()
        .map(|r| match r.as_str() {
            "carbon_compounds" => "carbon",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Terse headline tags for a compact Sector row, looked up from the raw object
/// (top-level or nested by id): system star/planet count, asteroid
/// composition/resources, planet habitability + class.
fn sector_entry_tags(state: &AppState, id: &str, p: Palette) -> Vec<Span<'static>> {
    let dim = Style::default().fg(p.dim);
    let mut out: Vec<Span<'static>> = Vec::new();
    let Some(s) = state.current_sector() else { return out };
    let Some(objs) = s.objects.as_ref() else { return out };

    if let Some(o) = objs.iter().find(|o| o.id.as_deref() == Some(id)) {
        match o.object_type {
            SectorObjectType::SolarSystem => {
                out.push(Span::styled("  ★".to_string(), Style::default().fg(p.warn)));
                out.push(Span::styled(
                    format!(" · {} planet(s)", o.planet_count.unwrap_or(0)),
                    dim,
                ));
            }
            SectorObjectType::Asteroid => {
                if let Some(c) = &o.composition {
                    out.push(Span::styled(format!("  {c}"), dim));
                } else if !o.resource_types.is_empty() {
                    out.push(Span::styled(
                        format!("  {}", resource_types_str(&o.resource_types)),
                        dim,
                    ));
                }
            }
            SectorObjectType::Planet => {
                if let Some(h) = o.habitability_score {
                    out.push(Span::styled(
                        format!("  hab {:.0}%", h * 100.0),
                        Style::default().fg(ratio_color(h, p)),
                    ));
                }
                if let Some(c) = &o.category {
                    out.push(Span::styled(format!(" {c}"), dim));
                }
            }
            SectorObjectType::DetachedContainer => {
                // Containers hidden on an asteroid are shown indented under their
                // host (scanner_objects hierarchy); only flag free-floating ones.
                if o.mode.as_deref() == Some("drifting") {
                    out.push(Span::styled("  drifting".to_string(), dim));
                }
            }
            _ => {
                if o.manny_mineable == Some(true) {
                    out.push(Span::styled("  ⛏".to_string(), Style::default().fg(p.warn)));
                }
            }
        }
        return out;
    }

    // Nested body of a solar system, matched by id.
    for o in objs {
        if let Some(t) = o.minable_targets.iter().flatten().find(|t| t.id == id) {
            if let Some(rt) = &t.resource_types {
                if !rt.is_empty() {
                    out.push(Span::styled(format!("  {}", resource_types_str(rt)), dim));
                }
            }
            return out;
        }
        if let Some(t) = o.bookmark_targets.iter().find(|t| t.id == id) {
            if let Some(h) = t.habitability_score {
                out.push(Span::styled(
                    format!("  hab {:.0}%", h * 100.0),
                    Style::default().fg(ratio_color(h, p)),
                ));
            }
            if let Some(c) = &t.category {
                out.push(Span::styled(format!(" {c}"), dim));
            }
            return out;
        }
    }
    out
}

/// Zoom breakdown of a solar system: header + body counts, then one entry per
/// nested body (union of bookmark + mineable targets, merged by id) with its
/// type, class, habitability and mineable resources.
fn solar_system_zoom_lines(obj: &SectorObject, p: Palette) -> Vec<Line<'static>> {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let mut lines: Vec<Line<'static>> = Vec::new();

    let name = obj
        .name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| object_type_label(&obj.object_type).to_string());
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} ", object_icon(&obj.object_type).0),
            Style::default().fg(object_color(&obj.object_type, p)),
        ),
        Span::styled(name, text),
    ]));
    lines.push(Line::styled(
        format!(
            "  ★ {} star(s) · {} planet(s) · {} bodies",
            obj.star_count.unwrap_or(0),
            obj.planet_count.unwrap_or(0),
            obj.orbital_body_count.unwrap_or(0),
        ),
        dim,
    ));

    // Union of nested body ids, bookmark targets first, then mineable-only.
    let mut ids: Vec<String> = Vec::new();
    for t in &obj.bookmark_targets {
        if !ids.contains(&t.id) {
            ids.push(t.id.clone());
        }
    }
    for t in obj.minable_targets.iter().flatten() {
        if !ids.contains(&t.id) {
            ids.push(t.id.clone());
        }
    }

    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for id in ids {
        let bt = obj.bookmark_targets.iter().find(|t| t.id == id);
        let mt = obj.minable_targets.iter().flatten().find(|t| t.id == id);
        let otype = bt
            .map(|b| b.object_type.clone())
            .or_else(|| mt.map(|m| m.object_type.clone()));
        let Some(otype) = otype else { continue };
        let label = object_type_label(&otype);
        let n = counts.entry(label).or_insert(0);
        *n += 1;
        let name = bt
            .and_then(|b| b.name.clone())
            .or_else(|| mt.and_then(|m| m.name.clone()))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("{label} #{n}"));

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", object_icon(&otype).0),
                Style::default().fg(object_color(&otype, p)),
            ),
            Span::styled(name, text),
            Span::styled(format!("  {label}"), dim),
        ]));
        if let Some(b) = bt {
            if let Some(h) = b.habitability_score {
                lines.push(Line::from(vec![
                    Span::styled("      habitability ", dim),
                    Span::styled(format!("{:.0}%", h * 100.0), Style::default().fg(ratio_color(h, p))),
                ]));
            }
            if let Some(c) = &b.category {
                lines.push(Line::from(vec![
                    Span::styled("      class ", dim),
                    Span::styled(c.clone(), text),
                ]));
            }
        }
        if let Some(m) = mt {
            if let Some(rt) = &m.resource_types {
                if !rt.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("      resources ", dim),
                        Span::styled(resource_types_str(rt), Style::default().fg(p.warn)),
                    ]));
                }
            }
            if let Some(line) = resource_shares_line("      shares ", m.resource_composition.as_ref(), true, p) {
                lines.push(line);
            }
        }
    }
    lines
}

pub fn render_missions(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    match state.missions_category() {
        None => return render_missions_root(frame, area, state, active, p),
        Some(MissionsCategory::ShipsLog) => return render_ship_log(frame, area, state, active, p),
        Some(MissionsCategory::Missions) => {}
    }
    if let Some(DrillLevel::Mission(id)) = state.pane_nav[Pane::Missions.index()].drill.last() {
        return render_mission_detail(frame, area, state, id, active, p);
    }
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    let mut sel_line = None;
    if state.missions.is_empty() {
        lines.push(Line::styled("no active missions", dim));
    } else {
        let cur = cursor(state, Pane::Missions);
        for (i, m) in state.missions.iter().enumerate() {
            let color = match m.status {
                MissionStatus::Active => p.accent,
                MissionStatus::Completed => p.good,
                MissionStatus::Failed | MissionStatus::Abandoned => p.crit,
                MissionStatus::Unknown => p.dim,
            };
            let done = m
                .steps
                .iter()
                .filter(|s| matches!(s.status, MissionStepStatus::Completed))
                .count();
            let title: String = m.title.chars().take(22).collect();
            if i == cur {
                sel_line = Some(lines.len());
            }
            lines.push(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(color)),
                Span::styled(title, row_style(active, i == cur).patch(Style::default().fg(p.text))),
                Span::styled(format!(" {done}/{}", m.steps.len()), dim),
            ]));
        }
    }
    render_body(frame, area, " MISSIONS ", active, p, lines, sel_line);
}

fn render_mission_detail(frame: &mut Frame, area: Rect, state: &AppState, id: &str, active: bool, p: Palette) {
    let block = pane_block(" MISSION ", active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let dim = Style::default().fg(p.dim);
    let Some(m) = state.missions.iter().find(|m| m.id == id) else {
        frame.render_widget(Paragraph::new(Line::styled("mission not found", dim)), inner);
        return;
    };
    let mut lines = vec![Line::styled(
        m.title.clone(),
        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
    )];
    if let Some(d) = &m.description {
        lines.push(Line::styled(d.clone(), dim));
    }
    lines.push(Line::raw(""));
    let cur = cursor(state, Pane::Missions);
    for (i, step) in m.steps.iter().enumerate() {
        let (mark, color) = match step.status {
            MissionStepStatus::Completed => ("✓", p.good),
            MissionStepStatus::Failed => ("✗", p.crit),
            MissionStepStatus::Skipped => ("–", p.dim),
            MissionStepStatus::Pending => ("·", p.accent),
            MissionStepStatus::Unknown => ("?", p.dim),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{mark} "), Style::default().fg(color)),
            Span::styled(
                step.title.clone(),
                row_style(active, i == cur).patch(Style::default().fg(p.text)),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Missions root: two selectable rows (active missions, ship's log) with counts
/// and a one-line preview of the latest entry, mirroring the Comms root.
fn render_missions_root(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let cur = cursor(state, Pane::Missions);
    let active_missions = state
        .missions
        .iter()
        .filter(|m| matches!(m.status, MissionStatus::Active))
        .count();
    let mission_preview = state.missions.first().map(|m| format!("▸ {}", comms_preview(&m.title)));
    let log_preview = state
        .journal
        .first()
        .map(|e| format!("· {}", comms_preview(&e.summary.replace(['«', '»'], ""))));
    let rows = [
        ("Missions", state.missions.len(), Some(active_missions), mission_preview),
        ("Ship's log", state.journal.len(), None, log_preview),
    ];
    let mut lines = Vec::new();
    let mut sel_line = None;
    for (i, (label, total, active_count, preview)) in rows.iter().enumerate() {
        if i == cur {
            sel_line = Some(lines.len());
        }
        let mut spans = vec![
            Span::styled(format!("{label:<11} "), row_style(active, i == cur).patch(text)),
            Span::styled(format!("{total}"), dim),
        ];
        if let Some(n) = active_count {
            if *n > 0 {
                spans.push(Span::styled(format!("  ({n} active)"), Style::default().fg(p.accent)));
            }
        }
        lines.push(Line::from(spans));
        let preview = preview.clone().unwrap_or_else(|| "—".to_string());
        lines.push(Line::from(Span::styled(format!("  {preview}"), dim)));
    }
    render_body(frame, area, " MISSIONS ", active, p, lines, sel_line);
}

/// The ship's log: newest-first captain's-log lines. The timestamp is dimmed,
/// entities (wrapped in `«…»` by the constructors) take the accent colour, and
/// server-reconstructed events render in the warning colour. Scrolls with the
/// cursor.
fn render_ship_log(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    let mut sel_line = None;
    // Actions merged with reconstructed server events (alerts / warnings).
    let entries = state.ship_log_entries();
    if entries.is_empty() {
        lines.push(Line::styled("ship's log empty — your actions will appear here", dim));
    } else {
        let cur = cursor(state, Pane::Missions);
        // Compact: truncate each line to the pane width with an ellipsis.
        // Zoomed: full width, so the whole captain's-log sentence reads out.
        let width = area.width.saturating_sub(2) as usize;
        for (i, e) in entries.iter().enumerate() {
            if i == cur {
                sel_line = Some(lines.len());
            }
            let ts = e.occurred_at.with_timezone(&Local).format("%H:%M").to_string();
            let server = e.kind == crate::app::kind::ALERT;
            let base = Style::default().fg(if server { p.warn } else { p.text });
            let accent = Style::default().fg(if server { p.warn } else { p.accent });
            let mut spans = vec![Span::styled(format!("{ts} — "), dim)];
            spans.extend(narrative_spans(&e.summary, base, accent));
            if !state.zoomed {
                spans = truncate_spans(spans, width);
            }
            lines.push(Line::from(spans));
        }
    }
    render_body(frame, area, " SHIP'S LOG ", active, p, lines, sel_line);
}

/// Truncate a styled line to `max` display columns, appending an ellipsis when
/// it overflows, while preserving each span's colour.
fn truncate_spans(spans: Vec<Span<'static>>, max: usize) -> Vec<Span<'static>> {
    let total: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if total <= max || max == 0 {
        return spans;
    }
    let budget = max.saturating_sub(1); // leave a column for the ellipsis
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for s in spans {
        let len = s.content.chars().count();
        if used + len <= budget {
            used += len;
            out.push(s);
        } else {
            let remaining = budget - used;
            if remaining > 0 {
                let clipped: String = s.content.chars().take(remaining).collect();
                out.push(Span::styled(clipped, s.style));
            }
            out.push(Span::styled("…".to_string(), s.style));
            break;
        }
    }
    out
}

/// Split a captain's-log summary on `«…»` markers into styled spans: plain text
/// in `base`, the wrapped entities in `accent`.
fn narrative_spans(summary: &str, base: Style, accent: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = summary;
    while let Some(open) = rest.find('«') {
        if open > 0 {
            spans.push(Span::styled(rest[..open].to_string(), base));
        }
        rest = &rest[open + '«'.len_utf8()..];
        match rest.find('»') {
            Some(close) => {
                spans.push(Span::styled(rest[..close].to_string(), accent));
                rest = &rest[close + '»'.len_utf8()..];
            }
            None => {
                spans.push(Span::styled(rest.to_string(), base));
                rest = "";
                break;
            }
        }
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base));
    }
    spans
}

pub fn render_storage(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let cur = cursor(state, Pane::Storage);
    let zoomed = state.zoomed;
    let mut lines = Vec::new();
    let mut sel_line = None;

    // Drilled into a container: render its contents inline (fetched on drill-in).
    if let Some(DrillLevel::Container(id)) = state.pane_nav[Pane::Storage.index()].drill.last() {
        return render_container_contents(frame, area, state, id, active, p);
    }

    // Containers come with the probe (probe.inventory.containers), so the pane
    // fills as soon as the probe loads — Enter opens the full browser.
    match state.probe.as_ref().map(|pr| &pr.inventory.containers) {
        None => lines.push(Line::styled("no data", dim)),
        Some(cs) if cs.is_empty() => lines.push(Line::styled("no storage containers", dim)),
        Some(cs) => {
            const W: usize = 8;
            for (i, c) in cs.iter().enumerate() {
                let selected = active && i == cur;
                let ratio = if c.capacity > 0.0 {
                    (c.used_capacity / c.capacity).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let filled = (ratio * W as f64).round() as usize;
                let name_style = if selected {
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
                } else {
                    text
                };
                let sec = if selected { Style::default().fg(p.accent) } else { dim };
                let label: String = c.label.chars().take(12).collect();
                let mut spans = vec![
                    Span::styled(if selected { "▶ " } else { "  " }, Style::default().fg(p.accent)),
                    Span::styled(format!("{label:<12}   "), name_style),
                    Span::styled("▓".repeat(filled), Style::default().fg(fill_color(p, 1.0 - ratio))),
                    Span::styled("░".repeat(W - filled), dim),
                    Span::styled(format!(" {:.0}%", ratio * 100.0), sec),
                ];
                let rules = &c.rules;
                if !rules.priority.is_empty() || !rules.exclusion.is_empty() || !rules.strict_exclusion.is_empty() {
                    spans.push(Span::styled(" ⚙", Style::default().fg(p.accent)));
                }
                if i == cur {
                    sel_line = Some(lines.len());
                }
                lines.push(Line::from(spans));

                // Zoom: routing rules and free capacity per container.
                if zoomed {
                    if !rules.priority.is_empty() {
                        lines.push(Line::styled(
                            format!("    priority: {}", rules.priority.join(", ")),
                            dim,
                        ));
                    }
                    if !rules.exclusion.is_empty() {
                        lines.push(Line::styled(
                            format!("    exclude:  {}", rules.exclusion.join(", ")),
                            dim,
                        ));
                    }
                    if !rules.strict_exclusion.is_empty() {
                        lines.push(Line::styled(
                            format!("    strict:   {}", rules.strict_exclusion.join(", ")),
                            dim,
                        ));
                    }
                    lines.push(Line::styled(
                        format!("    free {:.2} of {:.2}", c.free_capacity, c.capacity),
                        dim,
                    ));
                }
            }
        }
    }
    render_body(frame, area, " STORAGE ", active, p, lines, sel_line);
}

/// Inline contents of a container (drill-in `l` on the Storage pane): capacity,
/// resource stocks, and unit items. Fetched on drill-in; shows a placeholder
/// until the detail arrives.
fn render_container_contents(frame: &mut Frame, area: Rect, state: &AppState, id: &str, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let mut lines: Vec<Line> = Vec::new();

    // Prefer the fetched detail; fall back to the summary while it loads.
    let summary = state.storage_container(id);
    let label = summary.map(|c| c.label.clone()).unwrap_or_else(|| "container".into());

    match state.storage_container_detail.as_ref().filter(|(c, _)| c.id == id) {
        None => {
            if let Some(err) = &state.storage_container_detail_error {
                lines.push(Line::styled(format!("✗ {err}"), Style::default().fg(p.crit)));
            } else {
                lines.push(Line::styled("fetching contents…", dim));
            }
        }
        Some((c, inv)) => {
            let ratio = if c.capacity > 0.0 {
                (c.used_capacity / c.capacity).clamp(0.0, 1.0)
            } else {
                0.0
            };
            lines.push(block_gauge_line(
                "USED",
                ratio,
                &format!("{:.0}%", ratio * 100.0),
                fill_color(p, 1.0 - ratio),
                p,
            ));
            lines.push(Line::styled(
                format!("free {:.2} of {:.2}", c.free_capacity, c.capacity),
                dim,
            ));

            let rules = &c.rules;
            if !rules.priority.is_empty() {
                lines.push(Line::styled(format!("priority: {}", rules.priority.join(", ")), dim));
            }
            if !rules.exclusion.is_empty() {
                lines.push(Line::styled(format!("exclude:  {}", rules.exclusion.join(", ")), dim));
            }
            if !rules.strict_exclusion.is_empty() {
                lines.push(Line::styled(
                    format!("strict:   {}", rules.strict_exclusion.join(", ")),
                    dim,
                ));
            }

            lines.push(Line::raw(""));
            if inv.resource_stocks.is_empty() && inv.items.is_empty() {
                lines.push(Line::styled("empty", dim));
            }
            for st in &inv.resource_stocks {
                lines.push(Line::from(vec![
                    Span::styled(format!("{:<12} ", st.name), text),
                    Span::styled(format!("{:.2}", st.amount), Style::default().fg(p.accent)),
                ]));
            }
            for it in &inv.items {
                lines.push(Line::from(vec![
                    Span::styled("• ", dim),
                    Span::styled(it.name.clone(), text),
                ]));
            }
        }
    }

    let title = format!(" {label} ");
    render_body(frame, area, &title, active, p, lines, None);
}

/// Detail view for a single manny (drill-in `l` on the Mannies pane): task,
/// progress, time remaining, cargo breakdown, and location.
/// The detail lines for one manny (task/%, ETA, location, cargo), shared by
/// the drill-in detail and the zoom overview cards. The name lives in the
/// block title, not here.
fn manny_detail_lines(state: &AppState, m: &Manny, p: Palette) -> Vec<Line<'static>> {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let mut lines = Vec::new();

    let task = m.current_task.as_ref();
    if task.is_some() {
        lines.push(Line::from(vec![
            Span::styled(manny_task_label(task), Style::default().fg(p.accent)),
            Span::styled(format!("  {:.0}%", manny_task_progress(m) * 100.0), text),
        ]));
        if let Some(eta) = manny_task_eta(m) {
            lines.push(Line::from(vec![Span::styled("ETA ", dim), Span::styled(eta, text)]));
        }
    } else {
        lines.push(Line::styled("idle", dim));
    }
    // Mining target: which asteroid, resources, and where the output goes.
    if let Some(d) = manny_mining_detail(m) {
        lines.push(Line::from(vec![
            Span::styled("⛏ ", Style::default().fg(p.accent)),
            Span::styled(d.target, text),
        ]));
        if let Some(r) = d.resources {
            lines.push(Line::styled(format!("  {r}"), dim));
        }
        lines.push(Line::from(vec![
            Span::styled("→ ", dim),
            Span::styled(d.destination, text),
        ]));
    }
    // Crafting target: which recipe is being fabricated.
    if let Some(recipe) = manny_crafting_detail(m) {
        lines.push(Line::from(vec![
            Span::styled("⚙ ", Style::default().fg(p.accent)),
            Span::styled(recipe, text),
        ]));
    }
    // A hidden container turned up by mining — flag it (recoverable).
    if manny_artificial_detection(m).is_some() {
        lines.push(Line::from(Span::styled(
            "⚠ hidden container found",
            Style::default().fg(p.warn).add_modifier(Modifier::BOLD),
        )));
    }
    match state.manny_sector_coords(m) {
        Some((x, y, z)) => lines.push(Line::from(vec![
            Span::styled("sector ", dim),
            Span::styled(format!("({x}, {y}, {z})"), text),
        ])),
        None => lines.push(Line::styled("on probe", dim)),
    }
    if matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork)) {
        lines.push(Line::styled("≣ via SCUT", Style::default().fg(p.accent)));
    }

    // Cargo — what it is carrying (proxy for what it is mining/hauling).
    lines.push(Line::raw(""));
    let c = &m.cargo;
    let used = c.metals + c.ice + c.deuterium + c.organic_compounds;
    let ratio = if c.capacity > 0.0 { used / c.capacity } else { 0.0 };
    lines.push(block_gauge_line(
        "CARGO",
        ratio,
        &format!("{:.0}%", ratio * 100.0),
        p.accent,
        p,
    ));
    lines.push(Line::styled(format!("metals {:.2}  ice {:.2}", c.metals, c.ice), text));
    lines.push(Line::styled(
        format!("deut {:.2}  org {:.2}", c.deuterium, c.organic_compounds),
        text,
    ));
    lines
}

pub fn render_manny_detail(frame: &mut Frame, area: Rect, state: &AppState, id: &str, active: bool, p: Palette) {
    let Some(m) = state.mannies.as_ref().and_then(|v| v.iter().find(|m| m.id == id)) else {
        let block = pane_block(" MANNY ", active, p);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Line::styled("manny gone", Style::default().fg(p.dim))),
            inner,
        );
        return;
    };
    let title = format!(" {} ", m.name);
    let block = pane_block(&title, active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(manny_detail_lines(state, m, p)).wrap(Wrap { trim: false }),
        inner,
    );
}

/// Zoomed Mannies pane: a vertical list where each manny is a summary line
/// with its details indented below — the whole fleet at a glance.
pub fn render_mannies_overview(frame: &mut Frame, area: Rect, state: &AppState, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let block = pane_block(" MANNIES ", true, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(mannies) = &state.mannies else {
        frame.render_widget(Paragraph::new(Line::styled("no data", dim)), inner);
        return;
    };
    if mannies.is_empty() {
        frame.render_widget(Paragraph::new(Line::styled("no mannies aboard", dim)), inner);
        return;
    }

    let sel = state.mannies_selection;
    let mut lines: Vec<Line> = Vec::new();
    let mut sel_line = None;
    for (i, m) in mannies.iter().enumerate() {
        let selected = i == sel;
        if selected {
            sel_line = Some(lines.len());
        }
        let name_style = if selected {
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.text)
        };
        let sec = if selected { Style::default().fg(p.accent) } else { dim };

        // Summary line: marker · loc · name · task % · ETA.
        let loc = match m.location.location_type {
            MannyLocationType::Probe => "●",
            MannyLocationType::Sector => "◌",
            MannyLocationType::Unknown => "?",
        };
        let task = m.current_task.as_ref();
        let mut header = vec![
            Span::styled(if selected { "▶ " } else { "  " }, Style::default().fg(p.accent)),
            Span::styled(format!("{loc} "), sec),
            Span::styled(format!("{:<12}", m.name), name_style),
            Span::styled(manny_task_label(task), if task.is_none() { sec } else { name_style }),
        ];
        if m.current_task.is_some() {
            header.push(Span::styled(format!(" {:.0}%", manny_task_progress(m) * 100.0), sec));
            if let Some(eta) = manny_task_eta(m) {
                header.push(Span::styled(format!(" · {eta}"), sec));
            }
        }
        lines.push(Line::from(header));

        // Mining target, when visible: asteroid · resources → destination.
        if let Some(d) = manny_mining_detail(m) {
            let mut s = format!("    ⛏ {}", d.target);
            if let Some(r) = &d.resources {
                s.push_str(&format!(" · {r}"));
            }
            s.push_str(&format!(" → {}", d.destination));
            lines.push(Line::styled(s, dim));
        }
        // Crafting recipe, when visible.
        if let Some(recipe) = manny_crafting_detail(m) {
            lines.push(Line::styled(format!("    ⚙ {recipe}"), dim));
        }
        if manny_artificial_detection(m).is_some() {
            lines.push(Line::from(Span::styled(
                "    ⚠ hidden container found",
                Style::default().fg(p.warn),
            )));
        }

        // Indented detail: cargo gauge, cargo breakdown, location.
        let c = &m.cargo;
        let used = c.metals + c.ice + c.deuterium + c.organic_compounds;
        let ratio = if c.capacity > 0.0 { used / c.capacity } else { 0.0 };
        lines.push(block_gauge_line(
            "    CARGO",
            ratio,
            &format!("{:.0}%", ratio * 100.0),
            p.accent,
            p,
        ));
        lines.push(Line::styled(
            format!(
                "    metals {:.2} · ice {:.2} · deut {:.2} · org {:.2}",
                c.metals, c.ice, c.deuterium, c.organic_compounds
            ),
            dim,
        ));
        let mut loc_line = match state.manny_sector_coords(m) {
            Some((x, y, z)) => format!("    sector ({x}, {y}, {z})"),
            None => "    on probe".to_string(),
        };
        if matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork)) {
            loc_line.push_str("  ≣ via SCUT");
        }
        lines.push(Line::styled(loc_line, dim));
        lines.push(Line::raw(""));
    }

    let offset = sel_line
        .map(|c| scroll_offset(c, lines.len(), inner.height as usize))
        .unwrap_or(0);
    frame.render_widget(Paragraph::new(lines).scroll((offset, 0)), inner);
}

/// Zoom view for the Scanner pane: a spatial mini-map of the six sectors
/// adjacent to the probe, coloured by interest, with a legend. Focuses the
/// Scanner on its real job — knowing what lies *around* the current sector.
pub fn render_scanner_neighbors(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    use crate::ui::panels::scanner::sector_interest_color;
    use crate::ui::theme::{knowledge_label, map_cell_style};

    let p = crate::ui::theme::palette(state.color_mode);
    let block = pane_block(" SCANNER · NEIGHBORS ", active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let dim = Style::default().fg(p.dim);

    let Some((px, py, pz)) = state.probe_sector_coords() else {
        frame.render_widget(Paragraph::new(Line::styled("unknown probe position", dim)), inner);
        return;
    };

    // Look up a scanned observation at exact relative coordinates.
    let at = |x: i32, y: i32, z: i32| {
        state.scan_history.iter().find(|s| {
            let c = &s.relative_coordinates;
            c.x.round() as i32 == x && c.y.round() as i32 == y && c.z.round() as i32 == z
        })
    };
    // (symbol, color) for a neighbor cell — dim dot when never scanned.
    let cell = |x: i32, y: i32, z: i32| match at(x, y, z) {
        Some(s) => {
            let (sym, st) = map_cell_style(s, p);
            (sym.to_string(), st)
        }
        None => ("·".to_string(), dim),
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("from ", dim),
        Span::styled(format!("({px},{py},{pz})"), Style::default().fg(p.text)),
    ]));
    lines.push(Line::raw(""));

    // XY plane cross around the probe (P), with the +X/-X row centred.
    let (uy, us) = cell(px, py + 1, pz);
    let (dy, ds) = cell(px, py - 1, pz);
    let (lx, ls) = cell(px - 1, py, pz);
    let (rx, rs) = cell(px + 1, py, pz);
    lines.push(Line::from(vec![
        Span::raw("      "),
        Span::styled(uy, us),
        Span::styled("  +Y", dim),
    ]));
    lines.push(Line::from(vec![
        Span::styled(lx, ls),
        Span::styled(" -X   ", dim),
        Span::styled("P", Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        Span::styled("   +X ", dim),
        Span::styled(rx, rs),
    ]));
    lines.push(Line::from(vec![
        Span::raw("      "),
        Span::styled(dy, ds),
        Span::styled("  -Y", dim),
    ]));
    lines.push(Line::raw(""));

    // Z axis on its own row (out-of-plane).
    let (zu, zus) = cell(px, py, pz + 1);
    let (zd, zds) = cell(px, py, pz - 1);
    lines.push(Line::from(vec![
        Span::styled("+Z ", dim),
        Span::styled(zu, zus),
        Span::styled("    -Z ", dim),
        Span::styled(zd, zds),
    ]));
    lines.push(Line::raw(""));

    // Legend: one row per direction with coords, symbol, and what's known.
    let dirs = [
        ("+X", px + 1, py, pz),
        ("-X", px - 1, py, pz),
        ("+Y", px, py + 1, pz),
        ("-Y", px, py - 1, pz),
        ("+Z", px, py, pz + 1),
        ("-Z", px, py, pz - 1),
    ];
    for (tag, x, y, z) in dirs {
        let (sym, st, label, coord_color) = match at(x, y, z) {
            Some(s) => {
                let (sym, st) = map_cell_style(s, p);
                (
                    sym.to_string(),
                    st,
                    knowledge_label(&s.knowledge_level),
                    sector_interest_color(s, p),
                )
            }
            None => ("·".to_string(), dim, "unscanned", p.dim),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{tag} "), dim),
            Span::styled(sym, st),
            Span::styled(format!(" ({x},{y},{z}) "), Style::default().fg(coord_color)),
            Span::styled(label, dim),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::scroll_offset;

    #[test]
    fn no_scroll_when_content_fits() {
        assert_eq!(scroll_offset(0, 3, 5), 0);
        assert_eq!(scroll_offset(4, 5, 5), 0, "exactly fills → no scroll");
    }

    #[test]
    fn no_scroll_while_cursor_on_first_page() {
        // 10 rows, 4 visible: cursor within the first 4 stays put.
        assert_eq!(scroll_offset(0, 10, 4), 0);
        assert_eq!(scroll_offset(3, 10, 4), 0);
    }

    #[test]
    fn scrolls_to_keep_cursor_on_last_visible_row() {
        // 10 rows, 4 visible: cursor 4 → window shows rows 1..=4.
        assert_eq!(scroll_offset(4, 10, 4), 1);
        assert_eq!(scroll_offset(6, 10, 4), 3);
    }

    #[test]
    fn never_scrolls_past_the_end() {
        // 10 rows, 4 visible: last cursor pins to the final window (offset 6).
        assert_eq!(scroll_offset(9, 10, 4), 6);
    }

    #[test]
    fn zero_height_is_safe() {
        assert_eq!(scroll_offset(5, 10, 0), 0);
    }
}
