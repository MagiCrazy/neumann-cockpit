use crossterm::event::KeyCode;

// Retained for the Scanner pane's upcoming neighbour-scan action in the
// cockpit interface (its classic single-key trigger was removed with U8).
#[allow(dead_code)]
pub(super) fn neighbors_d1() -> Vec<(i32, i32, i32)> {
    let mut out = Vec::new();
    for a in -1i32..=1 {
        for b in -1i32..=1 {
            for c in -1i32..=1 {
                if a.abs().max(b.abs()).max(c.abs()) == 1 && (a + b + c) % 2 == 0 {
                    out.push((a, b, c));
                }
            }
        }
    }
    out
}

pub(super) fn face_d2(axis: u8) -> Vec<(i32, i32, i32)> {
    let mut out = Vec::new();
    for face in [-2i32, 2] {
        for u in -2i32..=2 {
            for v in -2i32..=2 {
                let coords = match axis {
                    b'x' => (face, u, v),
                    b'y' => (u, face, v),
                    _    => (u, v, face),
                };
                if (coords.0 + coords.1 + coords.2) % 2 == 0 {
                    out.push(coords);
                }
            }
        }
    }
    out
}

pub(super) fn list_nav(code: KeyCode, sel: usize, count: usize) -> Option<usize> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => Some(sel.checked_sub(1).unwrap_or(count.saturating_sub(1))),
        KeyCode::Down | KeyCode::Char('j') => Some((sel + 1) % count.max(1)),
        _ => None,
    }
}

/// Apply a pick-list nav key to `selection` in place. Returns `true` when the
/// key was a navigation key (`j`/`k`/arrows) — so a handler can consume it and
/// stop before its Esc/Enter arms, without re-matching the state to write back.
pub(super) fn list_move(code: KeyCode, selection: &mut usize, count: usize) -> bool {
    match list_nav(code, *selection, count) {
        Some(n) => {
            *selection = n;
            true
        }
        None => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    // ── list_nav ──────────────────────────────────────────────────────────────

    #[test]
    fn list_nav_down_increments() {
        assert_eq!(list_nav(KeyCode::Down, 0, 3), Some(1));
        assert_eq!(list_nav(KeyCode::Down, 1, 3), Some(2));
        assert_eq!(list_nav(KeyCode::Char('j'), 0, 3), Some(1));
    }

    #[test]
    fn list_nav_down_wraps_at_end() {
        assert_eq!(list_nav(KeyCode::Down, 2, 3), Some(0));
        assert_eq!(list_nav(KeyCode::Char('j'), 2, 3), Some(0));
    }

    #[test]
    fn list_nav_up_decrements() {
        assert_eq!(list_nav(KeyCode::Up, 2, 3), Some(1));
        assert_eq!(list_nav(KeyCode::Char('k'), 2, 3), Some(1));
    }

    #[test]
    fn list_nav_up_wraps_at_zero() {
        assert_eq!(list_nav(KeyCode::Up, 0, 3), Some(2));
        assert_eq!(list_nav(KeyCode::Char('k'), 0, 3), Some(2));
    }

    #[test]
    fn list_nav_returns_none_for_other_keys() {
        assert_eq!(list_nav(KeyCode::Enter, 0, 3), None);
        assert_eq!(list_nav(KeyCode::Esc, 1, 3), None);
        assert_eq!(list_nav(KeyCode::Char('x'), 0, 3), None);
    }

    #[test]
    fn list_nav_empty_list_stays_at_zero() {
        assert_eq!(list_nav(KeyCode::Down, 0, 0), Some(0));
        assert_eq!(list_nav(KeyCode::Up, 0, 0), Some(0));
    }

    // ── neighbors_d1 ─────────────────────────────────────────────────────────

    #[test]
    fn neighbors_d1_count() {
        assert_eq!(neighbors_d1().len(), 12);
    }

    #[test]
    fn neighbors_d1_all_even_sum() {
        for (a, b, c) in neighbors_d1() {
            assert_eq!((a + b + c) % 2, 0, "odd sum at ({a},{b},{c})");
        }
    }

    #[test]
    fn neighbors_d1_all_at_distance_1() {
        for (a, b, c) in neighbors_d1() {
            let dist = a.abs().max(b.abs()).max(c.abs());
            assert_eq!(dist, 1, "distance != 1 at ({a},{b},{c})");
        }
    }

    #[test]
    fn neighbors_d1_no_duplicates() {
        let v = neighbors_d1();
        let mut seen = std::collections::HashSet::new();
        for coord in &v {
            assert!(seen.insert(coord), "duplicate at {coord:?}");
        }
    }

    // ── face_d2 ───────────────────────────────────────────────────────────────

    #[test]
    fn face_d2_all_even_sum() {
        for axis in [b'x', b'y', b'z'] {
            for (a, b, c) in face_d2(axis) {
                assert_eq!((a + b + c) % 2, 0, "odd sum at ({a},{b},{c}) axis={}", axis as char);
            }
        }
    }

    #[test]
    fn face_d2_x_face_coordinate_is_pm2() {
        for (a, _b, _c) in face_d2(b'x') {
            assert!(a == 2 || a == -2, "x coord {a} not ±2");
        }
    }

    #[test]
    fn face_d2_y_face_coordinate_is_pm2() {
        for (_a, b, _c) in face_d2(b'y') {
            assert!(b == 2 || b == -2, "y coord {b} not ±2");
        }
    }

    #[test]
    fn face_d2_z_face_coordinate_is_pm2() {
        for (_a, _b, c) in face_d2(b'z') {
            assert!(c == 2 || c == -2, "z coord {c} not ±2");
        }
    }

    #[test]
    fn face_d2_each_axis_has_same_count() {
        let cx = face_d2(b'x').len();
        let cy = face_d2(b'y').len();
        let cz = face_d2(b'z').len();
        assert_eq!(cx, cy);
        assert_eq!(cy, cz);
        assert!(cx > 0);
    }
}
