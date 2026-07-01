//! Responsive layout for the Cockpit v2 tiling grid (bloc U2).
//!
//! Chooses a layout by terminal size and returns the visible panes with
//! their `Rect`s. U2 implements the full 3×3 and a single-pane fallback for
//! small terminals; the intermediate 2×2 + pagination tier lands in a
//! follow-up.

use crate::app::Pane;
use ratatui::layout::{Constraint, Layout, Rect};

/// Minimum size for the full 3×3 grid; below this we fall back to a single
/// full-screen pane (the active one).
const FULL_MIN_W: u16 = 96;
const FULL_MIN_H: u16 = 27;

/// The panes to draw this frame, each with its cell rectangle.
///
/// - Large terminals → all nine panes in a 3×3 grid.
/// - Small terminals → only the active pane, full screen (navigation keys
///   still switch which pane is shown).
pub fn visible_panes(area: Rect, active: Pane) -> Vec<(Pane, Rect)> {
    if area.width < FULL_MIN_W || area.height < FULL_MIN_H {
        return vec![(active, area)];
    }

    let rows = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(area);
    let mut out = Vec::with_capacity(9);
    for (r, row) in rows.iter().enumerate() {
        let cols = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(*row);
        for (c, cell) in cols.iter().enumerate() {
            out.push((Pane::ALL[r * 3 + c], *cell));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_area_shows_all_nine() {
        let panes = visible_panes(Rect::new(0, 0, 120, 40), Pane::Probe);
        assert_eq!(panes.len(), 9);
        // Every pane appears exactly once, in grid order.
        for (i, (pane, _)) in panes.iter().enumerate() {
            assert_eq!(*pane, Pane::ALL[i]);
        }
    }

    #[test]
    fn small_area_shows_only_active_pane() {
        let panes = visible_panes(Rect::new(0, 0, 60, 20), Pane::Mannies);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].0, Pane::Mannies);
    }
}
