//! Responsive layout for the Cockpit v2 tiling grid.
//!
//! Adapts to the terminal: it fits as many whole panes as the area allows
//! (a `rows × cols` window, each dimension 1..=3 based on a minimum cell
//! size) and slides that window so the active pane is always visible. Large
//! terminals show the full 3×3; a half-screen shows 2×2; a short wide split
//! shows a single row; a tiny terminal shows just the active pane.

use crate::app::Pane;
use ratatui::layout::{Constraint, Layout, Rect};

/// Minimum readable cell size (double border + a few content lines). Drives
/// how many pane columns/rows fit.
const MIN_CELL_W: u16 = 30;
const MIN_CELL_H: u16 = 9;

/// The panes to draw this frame, each with its cell rectangle. Always
/// includes the active pane; navigation slides the window to keep it visible.
pub fn visible_panes(area: Rect, active: Pane) -> Vec<(Pane, Rect)> {
    let cols = (area.width / MIN_CELL_W).clamp(1, 3) as usize;
    let rows = (area.height / MIN_CELL_H).clamp(1, 3) as usize;

    // Slide the window so the active pane sits inside it.
    let (ar, ac) = active.grid_pos();
    let r0 = (ar as usize).min(3 - rows);
    let c0 = (ac as usize).min(3 - cols);

    let row_rects = Layout::vertical(vec![Constraint::Ratio(1, rows as u32); rows]).split(area);
    let mut out = Vec::with_capacity(rows * cols);
    for (i, rr) in row_rects.iter().enumerate() {
        let col_rects =
            Layout::horizontal(vec![Constraint::Ratio(1, cols as u32); cols]).split(*rr);
        for (j, cr) in col_rects.iter().enumerate() {
            out.push((Pane::ALL[(r0 + i) * 3 + (c0 + j)], *cr));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_area_shows_all_nine_in_order() {
        let panes = visible_panes(Rect::new(0, 0, 120, 40), Pane::Probe);
        assert_eq!(panes.len(), 9);
        for (i, (pane, _)) in panes.iter().enumerate() {
            assert_eq!(*pane, Pane::ALL[i]);
        }
    }

    #[test]
    fn tiny_area_shows_only_active_pane() {
        let panes = visible_panes(Rect::new(0, 0, 20, 8), Pane::Mannies);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].0, Pane::Mannies);
    }

    #[test]
    fn medium_area_shows_a_2x2_window_around_active() {
        // 60×20 → 2 cols, 2 rows.
        let panes = visible_panes(Rect::new(0, 0, 60, 20), Pane::Mannies);
        assert_eq!(panes.len(), 4);
        // The active pane must be in the window…
        assert!(panes.iter().any(|(p, _)| *p == Pane::Mannies));
        // …and Mannies is bottom-right, so the window is the bottom-right 2×2.
        let set: Vec<Pane> = panes.iter().map(|(p, _)| *p).collect();
        for expected in [Pane::Probe, Pane::Missions, Pane::Storage, Pane::Mannies] {
            assert!(set.contains(&expected), "{expected:?} should be visible");
        }
    }

    #[test]
    fn short_wide_area_shows_a_single_row() {
        // 200×10 → 3 cols, 1 row.
        let panes = visible_panes(Rect::new(0, 0, 200, 10), Pane::Scanner);
        assert_eq!(panes.len(), 3);
        // Scanner is top-left → top row visible.
        let set: Vec<Pane> = panes.iter().map(|(p, _)| *p).collect();
        assert_eq!(set, vec![Pane::Scanner, Pane::Map, Pane::Comms]);
    }
}
