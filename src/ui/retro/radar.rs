use crate::app::{anim_hash, AppState, ObjectProvenance, Panel, ScanFilter};
use crate::ui::theme::object_icon;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette::Pal;
use super::section_title;

/// Sweep angle in degrees for a given animation frame (one revolution ≈ 6 s
/// at 10 fps).
pub(crate) fn sweep_angle(frame: u64) -> f64 {
    ((frame * 6) % 360) as f64
}

/// Polar position of a blip derived from its object id: stable across
/// frames, pseudo-uniform across the dial.
pub(crate) fn blip_polar(id: &str) -> (f64, f64) {
    let h = anim_hash(id.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)));
    let angle = (h % 360) as f64;
    let radius = 0.30 + ((h >> 9) % 60) as f64 / 100.0; // 0.30..0.90
    (angle, radius)
}

/// Intensity of a blip given the sweep position: 2 = just illuminated,
/// 1 = fading, 0 = at rest. The sweep rotates clockwise, so the afterglow
/// trails behind it.
pub(crate) fn blip_intensity(sweep_deg: f64, blip_deg: f64) -> u8 {
    let trail = (sweep_deg - blip_deg).rem_euclid(360.0);
    if trail < 25.0 {
        2
    } else if trail < 110.0 {
        1
    } else {
        0
    }
}

fn plot(frame: &mut Frame, x: i32, y: i32, area: Rect, sym: &str, style: ratatui::style::Style) {
    if x >= area.x as i32
        && x < (area.x + area.width) as i32
        && y >= area.y as i32
        && y < (area.y + area.height) as i32
    {
        frame.buffer_mut().set_string(x as u16, y as u16, sym, style);
    }
}

pub(super) fn render_radar(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let f = state.anim.frame;
    let focused = state.focused == Some(Panel::Scanner);

    let rows = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Min(3),
            ratatui::layout::Constraint::Length(2),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(section_title("SCANNER", focused, p)), rows[0]);
    let dial = rows[1];
    let info = rows[2];

    let cx = dial.x as f64 + dial.width as f64 / 2.0;
    let cy = dial.y as f64 + dial.height as f64 / 2.0;
    // Cells are ~2× taller than wide; stretch x to keep the dial round.
    let rx = (dial.width as f64 / 2.0 - 1.0).max(1.0);
    let ry = (dial.height as f64 / 2.0 - 1.0).max(1.0);
    let project = |deg: f64, r: f64| -> (i32, i32) {
        let rad = deg.to_radians();
        (
            (cx + rad.cos() * r * rx).round() as i32,
            (cy + rad.sin() * r * ry).round() as i32,
        )
    };

    // Range rings (dim dots) + twinkling background stars.
    for a in (0..360).step_by(12) {
        let (x, y) = project(a as f64, 1.0);
        plot(frame, x, y, dial, "·", p.dim());
    }
    for a in (0..360).step_by(30) {
        let (x, y) = project(a as f64, 0.55);
        plot(frame, x, y, dial, "·", p.dim());
    }
    let star_seed = state
        .current_sector()
        .map(|s| {
            (s.relative_coordinates.x as i64 * 73 + s.relative_coordinates.y as i64 * 179
                + s.relative_coordinates.z as i64 * 283) as u64
        })
        .unwrap_or(0);
    for i in 0..10u64 {
        let h = anim_hash(star_seed.wrapping_add(i.wrapping_mul(0x9e37)));
        let (x, y) = project((h % 360) as f64, 0.15 + ((h >> 10) % 80) as f64 / 100.0);
        // each star twinkles on its own cadence
        let bright = (f / 4).wrapping_add(h).is_multiple_of(7);
        plot(frame, x, y, dial, "·", if bright { p.norm() } else { p.dim() });
    }

    // Sweep beam.
    let sweep = sweep_angle(f);
    for step in 1..=18 {
        let r = step as f64 / 18.0;
        let (x, y) = project(sweep, r);
        plot(frame, x, y, dial, "·", if step >= 16 { p.bright() } else { p.norm() });
    }

    // Center: pulses while a scan request is in flight.
    let center_style = if state.scan_loading && (f / 2).is_multiple_of(2) {
        p.bold()
    } else {
        p.bright()
    };
    plot(frame, cx.round() as i32, cy.round() as i32, dial, "⊕", center_style);

    // Blips: actionable objects of the displayed sector.
    let entries = state.scanner_objects();
    let selected_idx = state.scanner_obj_selection;
    let nested_inward = |prov: ObjectProvenance, r: f64| match prov {
        ObjectProvenance::TopLevel => r,
        _ => r * 0.6,
    };
    for (i, e) in entries.iter().enumerate() {
        let (deg, r0) = blip_polar(&e.id);
        let r = nested_inward(e.provenance, r0);
        let (x, y) = project(deg, r);
        let (icon, _) = object_icon(&e.object_type);
        let style = match blip_intensity(sweep, deg) {
            2 => p.bold(),
            1 => p.norm(),
            _ => p.dim(),
        };
        let style = if selected_idx == Some(i) { p.bold() } else { style };
        plot(frame, x, y, dial, icon, style);
        if selected_idx == Some(i) {
            plot(frame, x - 1, y, dial, "[", p.bright());
            plot(frame, x + 1, y, dial, "]", p.bright());
        }
    }

    // Info lines under the dial.
    let mut l1: Vec<Span> = Vec::new();
    let mut l2: Vec<Span> = Vec::new();
    if let Some(sector) = state.current_sector() {
        let c = &sector.relative_coordinates;
        l1.push(Span::styled(
            format!("  SECTOR ({},{},{})", c.x as i64, c.y as i64, c.z as i64),
            p.bold(),
        ));
        l1.push(Span::styled(format!("  CONF {:.0}%", sector.confidence * 100.0), p.norm()));
        l1.push(Span::styled(format!("  SWEEP {:03.0}°", sweep), p.dim()));
        if state.scan_filter != ScanFilter::All {
            l1.push(Span::styled(
                format!("  FILTER:{}", state.scan_filter.label().to_uppercase()),
                p.bright(),
            ));
        }
        let objects = sector.objects.as_ref().map(|o| o.len()).unwrap_or(0);
        let minable: usize = sector
            .objects
            .iter()
            .flatten()
            .map(|o| o.minable_targets.as_ref().map(|t| t.len()).unwrap_or(0))
            .sum();
        if let Some(sel) = selected_idx.and_then(|i| entries.get(i)) {
            l2.push(Span::styled("  TARGET ", p.dim()));
            l2.push(Span::styled(sel.name.to_uppercase(), p.bold()));
            l2.push(Span::styled("  [ENTER] ACTIONS", p.norm()));
        } else {
            l2.push(Span::styled(
                format!("  {objects} CONTACTS · {minable} MINABLE"),
                p.norm(),
            ));
            if !entries.is_empty() {
                l2.push(Span::styled("  [O] TARGETING", p.dim()));
            }
        }
    } else {
        l1.push(Span::styled("  NO SCAN DATA", p.dim()));
        l2.push(Span::styled("  AWAITING SWEEP — [ENTER] SCAN SECTOR", p.norm()));
    }
    frame.render_widget(
        Paragraph::new(vec![Line::from(l1), Line::from(l2)]),
        info,
    );
}
