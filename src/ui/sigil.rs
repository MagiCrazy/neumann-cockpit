//! Probe sigil — a deterministic 7×7 identicon derived from a probe id, giving
//! each probe (and drone) a unique visual signature that never changes.
//!
//! The pattern is a visual hash: FNV-1a over the id diffuses sequential ids
//! (5, 6, 7…) into very different grids, and a plain multiplicative hash keeps
//! it **stable across runs and platforms** — unlike the standard library's
//! randomized default hasher, which would reshuffle every launch. The left four
//! columns are mirrored onto the right, so the grid reads as a balanced
//! identicon rather than noise, leaving 7×4 = 28 free bits (~268M patterns).

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::ui::theme::Palette;

/// Build the 7×7 mirror-symmetric grid for `id`. Deterministic and stable.
pub(crate) fn probe_sigil(id: u64) -> [[bool; 7]; 7] {
    // FNV-1a (64-bit): offset basis + prime.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in id.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut grid = [[false; 7]; 7];
    let mut bit = 0u32;
    for row in grid.iter_mut() {
        for x in 0..4 {
            let on = (h >> bit) & 1 == 1;
            bit += 1;
            row[x] = on;
            row[6 - x] = on; // mirror onto the right half
        }
    }
    grid
}

/// Render the sigil as terminal lines: two grid rows packed per text line with
/// `▀▄█` half-blocks (so cells read roughly square), drawn in the palette
/// accent. 7 grid rows collapse to 4 lines; `indent` prefixes each line.
pub(crate) fn sigil_lines(id: u64, p: Palette, indent: &str) -> Vec<Line<'static>> {
    let grid = probe_sigil(id);
    let style = Style::default().fg(p.accent);
    let mut lines = Vec::new();
    let mut y = 0;
    while y < 7 {
        let top = grid[y];
        let bot = if y + 1 < 7 { grid[y + 1] } else { [false; 7] };
        let mut s = String::from(indent);
        for x in 0..7 {
            s.push(match (top[x], bot[x]) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        lines.push(Line::from(Span::styled(s, style)));
        y += 2;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic() {
        assert_eq!(probe_sigil(5), probe_sigil(5));
        assert_eq!(probe_sigil(42), probe_sigil(42));
    }

    #[test]
    fn is_mirror_symmetric() {
        for id in [0u64, 5, 6, 42, 1000, u64::MAX] {
            let g = probe_sigil(id);
            for (y, row) in g.iter().enumerate() {
                for x in 0..7 {
                    assert_eq!(row[x], row[6 - x], "row {y} not mirrored at col {x}");
                }
            }
        }
    }

    #[test]
    fn sequential_ids_differ() {
        // FNV diffusion must make neighbouring ids look clearly different.
        assert_ne!(probe_sigil(5), probe_sigil(6));
        assert_ne!(probe_sigil(6), probe_sigil(7));
    }

    #[test]
    fn renders_four_lines() {
        let p = crate::ui::theme::palette(crate::app::ColorMode::MonoGreen);
        assert_eq!(sigil_lines(5, p, "  ").len(), 4);
    }
}
