use crate::api::types::{
    DangerLevel, SectorObjectType, SectorObservation,
};
use crate::app::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::{format_duration, map_cell_symbol, palette};
use super::{centered_rect, render_footer, render_pick_list, FooterKey};

/// Picker over visited sectors (most-recent first, as returned by the API):
/// coordinates, distance from the probe, and visit count.
pub(crate) fn render_goto_visited_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let crate::app::GotoVisitedInput::Picking { selection } = state.goto_visited else {
        return;
    };
    let p = palette(state.color_mode);
    let probe = state.probe_sector_coords();
    let labels: Vec<String> = state
        .visited_sectors
        .iter()
        .map(|v| {
            let c = &v.relative_coordinates;
            let (x, y, z) = (c.x.round() as i32, c.y.round() as i32, c.z.round() as i32);
            let dist = probe
                .map(|(px, py, pz)| (x - px).abs().max((y - py).abs()).max((z - pz).abs()))
                .map(|d| format!("  d{d}"))
                .unwrap_or_default();
            let times = if v.visit_count > 1 { format!("  ×{}", v.visit_count) } else { String::new() };
            format!("({x}, {y}, {z}){dist}{times}")
        })
        .collect();
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let height = (refs.len() as u16 + 6).clamp(8, 20);
    render_pick_list(
        frame, area, p, " JUMP TO VISITED SECTOR ", 46, height,
        Some("Travel to:"), &refs, selection, None, "travel",
    );
}

pub(crate) fn sector_brief(s: &SectorObservation) -> String {
    let Some(objects) = &s.objects else {
        return "estimated only".into();
    };
    if objects.is_empty() {
        return "empty".into();
    }
    let mut parts: Vec<String> = Vec::new();
    if objects.iter().any(|o| matches!(o.object_type, SectorObjectType::BlackHole)) {
        parts.push("black hole".into());
    }
    if objects.iter().any(|o| {
        matches!(o.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem)
    }) {
        parts.push("star".into());
    }
    let planets = objects.iter().filter(|o| matches!(o.object_type, SectorObjectType::Planet)).count()
        + objects.iter().flat_map(|o| &o.bookmark_targets)
            .filter(|t| matches!(t.object_type, SectorObjectType::Planet)).count();
    if planets > 0 {
        parts.push(format!("{planets} planet(s)"));
    }
    let minables: usize = objects.iter()
        .map(|o| o.minable_targets.as_ref().map(|t| t.len()).unwrap_or(0))
        .sum();
    if minables > 0 {
        parts.push(format!("{minables} minable"));
    }
    if objects.iter().any(|o| matches!(o.danger_level, Some(DangerLevel::Extreme))) {
        parts.push("extreme danger".into());
    }
    if parts.is_empty() {
        parts.push(format!("{} object(s)", objects.len()));
    }
    parts.join("  ")
}

pub(crate) fn render_map_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    use std::collections::HashMap;

    let popup = centered_rect(66, 24, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(
            format!(
                " MAP  ({},{},{})  y:{} ",
                state.map.center_x, state.map.y_layer, state.map.center_z,
                state.map.y_layer,
            ),
            Style::default().fg(p.accent),
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let map_inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(map_inner);
    let map_area = rows_layout[0];
    let info_area = rows_layout[1];
    let legend_area = rows_layout[2];
    let hint_area = rows_layout[3];

    // Fast lookup: (x,y,z) → sector
    let sector_lookup: HashMap<(i32, i32, i32), &SectorObservation> = state
        .scan_history
        .iter()
        .map(|s| {
            (
                (
                    s.relative_coordinates.x as i32,
                    s.relative_coordinates.y as i32,
                    s.relative_coordinates.z as i32,
                ),
                s,
            )
        })
        .collect();

    let probe_coords = state
        .probe
        .as_ref()
        .and_then(|p| p.sector.as_ref())
        .and_then(|s| s.relative.as_ref())
        .map(|r| (r.x as i32, r.y as i32, r.z as i32));

    let visited: std::collections::HashSet<(i32, i32, i32)> = state
        .visited_sectors
        .iter()
        .map(|v| {
            (
                v.relative_coordinates.x.round() as i32,
                v.relative_coordinates.y.round() as i32,
                v.relative_coordinates.z.round() as i32,
            )
        })
        .collect();

    let cx = state.map.center_x;
    let cz = state.map.center_z;
    let y = state.map.y_layer;

    // ── Center cell info ──
    let mut info_spans: Vec<Span> = vec![
        Span::styled(
            format!("({cx},{y},{cz})"),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(d) = state.map_center_distance() {
        info_spans.push(Span::styled(
            format!("  d:{d}"),
            Style::default().fg(p.dim),
        ));
        if d > 0 {
            info_spans.push(Span::styled(
                format!("  ETA ~{}", format_duration((5 + 35 * d) * 60)),
                Style::default().fg(p.warn),
            ));
        }
    }
    let brief = sector_lookup
        .get(&(cx, y, cz))
        .map(|s| sector_brief(s))
        .unwrap_or_else(|| {
            if let Some(v) = state.visited_sectors.iter().find(|v| {
                (
                    v.relative_coordinates.x.round() as i32,
                    v.relative_coordinates.y.round() as i32,
                    v.relative_coordinates.z.round() as i32,
                ) == (cx, y, cz)
            }) {
                format!("visited ×{} — no scan data", v.visit_count)
            } else {
                "unscanned".into()
            }
        });
    info_spans.push(Span::styled(
        format!("  {brief}"),
        Style::default().fg(p.dim),
    ));
    frame.render_widget(Paragraph::new(Line::from(info_spans)), info_area);

    // ── Legend ──
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("⊕", Style::default().fg(p.accent)),
            Span::styled(" probe  ", Style::default().fg(p.dim)),
            Span::styled("★", Style::default().fg(p.warn)),
            Span::styled(" star  ", Style::default().fg(p.dim)),
            Span::styled("●", Style::default().fg(p.good)),
            Span::styled(" objects  ", Style::default().fg(p.dim)),
            Span::styled("◉", Style::default().fg(p.crit)),
            Span::styled(" black hole  ", Style::default().fg(p.dim)),
            Span::styled("!", Style::default().fg(p.crit)),
            Span::styled(" danger  ", Style::default().fg(p.dim)),
            Span::styled("○", Style::default().fg(p.accent)),
            Span::styled(" visited  ", Style::default().fg(p.dim)),
            Span::styled("·", Style::default().fg(p.text)),
            Span::styled(" empty  ", Style::default().fg(p.dim)),
            Span::styled("·", Style::default().fg(p.dim)),
            Span::styled(" unscanned", Style::default().fg(p.dim)),
        ])),
        legend_area,
    );

    // ── Hint / coordinate input ──
    if let Some(buf) = &state.map.coord_input {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("go to (x y z): ", Style::default().fg(p.accent)),
                Span::raw(buf.as_str()),
                Span::styled("█", Style::default().fg(p.accent)),
                Span::raw("  "),
                Span::styled("[Enter]", Style::default().fg(p.good)),
                Span::raw(" center  "),
                Span::styled("[Esc]", Style::default().fg(p.accent)),
                Span::raw(" cancel"),
            ])),
            hint_area,
        );
    } else {
        render_footer(frame, hint_area, p, &[
            FooterKey::nav("[hjkl]", "pan"),
            FooterKey::nav("[u/d]", "y±1"),
            FooterKey::nav("[0]", "probe"),
            FooterKey::nav("[c]", "go to"),
            FooterKey::commit("[g]", "TRAVEL"),
            FooterKey::nav("[b/Esc]", "close"),
        ]);
    }
    let w = map_area.width as i32;
    let h = map_area.height as i32;
    let center_col = map_area.x as i32 + w / 2;
    let center_row = map_area.y as i32 + h / 2;

    // Projection:
    //   term_col = center_col + (dx - dz) * 2
    //   term_row = center_row - (dx + dz) / 2
    // Valid cells: (dx+dz) even, col_off multiple of 4.

    let buf = frame.buffer_mut();

    for tr in map_area.y..(map_area.y + map_area.height) {
        let row_off = tr as i32 - center_row;
        let dsum = -2 * row_off; // dx + dz

        let col_off_min = map_area.x as i32 - center_col;
        let rem = col_off_min.rem_euclid(4);
        let col_off_start = if rem == 0 { col_off_min } else { col_off_min + (4 - rem) };
        let col_off_max = (map_area.x as i32 + w - 1) - center_col;

        let mut col_off = col_off_start;
        while col_off <= col_off_max {
            let tc = (center_col + col_off) as u16;
            let ddiff = col_off / 2; // dx - dz
            let dx = (dsum + ddiff) / 2;
            let dz = (dsum - ddiff) / 2;
            let x = cx + dx;
            let z = cz + dz;

            let is_probe = probe_coords == Some((x, y, z));
            let is_center = dx == 0 && dz == 0;

            let (sym, style) = if is_probe {
                ("⊕", Style::default().fg(p.accent).add_modifier(Modifier::BOLD))
            } else if let Some(sector) = sector_lookup.get(&(x, y, z)) {
                let (s, st) = map_cell_symbol(sector);
                if is_center {
                    (s, st.add_modifier(Modifier::REVERSED))
                } else {
                    (s, st)
                }
            } else if visited.contains(&(x, y, z)) {
                let st = Style::default().fg(p.accent);
                if is_center {
                    ("○", st.add_modifier(Modifier::REVERSED))
                } else {
                    ("○", st)
                }
            } else if is_center {
                ("+", Style::default().fg(p.dim).add_modifier(Modifier::REVERSED))
            } else {
                ("·", Style::default().fg(p.dim))
            };

            buf.set_string(tc, tr, sym, style);
            col_off += 4;
        }
    }
}

