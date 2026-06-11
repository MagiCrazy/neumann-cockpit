use super::palette::block_gauge;
use super::radar::{blip_intensity, blip_polar, sweep_angle};
use super::ticker::tlm_segment;

#[test]
fn sweep_angle_wraps_at_360() {
    assert_eq!(sweep_angle(0), 0.0);
    assert_eq!(sweep_angle(10), 60.0);
    assert_eq!(sweep_angle(60), 0.0);
    assert!(sweep_angle(u64::MAX / 7) < 360.0);
}

#[test]
fn blip_polar_is_stable_and_in_range() {
    let (a1, r1) = blip_polar("ast-221");
    let (a2, r2) = blip_polar("ast-221");
    assert_eq!((a1, r1), (a2, r2));
    assert!((0.0..360.0).contains(&a1));
    assert!((0.30..=0.90).contains(&r1));
    // different ids land elsewhere
    assert_ne!(blip_polar("ast-221"), blip_polar("ast-222"));
}

#[test]
fn blip_intensity_trails_behind_sweep() {
    // beam just passed the blip → hot
    assert_eq!(blip_intensity(100.0, 90.0), 2);
    // a quarter turn later → fading
    assert_eq!(blip_intensity(180.0, 90.0), 1);
    // long after → at rest
    assert_eq!(blip_intensity(350.0, 90.0), 0);
    // wrap-around: sweep at 10°, blip at 350° → trail of 20°
    assert_eq!(blip_intensity(10.0, 350.0), 2);
}

#[test]
fn tlm_segment_is_deterministic_and_non_empty() {
    for slot in 0..32 {
        let s1 = tlm_segment(slot);
        assert_eq!(s1, tlm_segment(slot));
        assert!(!s1.is_empty());
    }
}

#[test]
fn block_gauge_fill_matches_ratio() {
    let cells = block_gauge(0.5, 10, 0);
    assert_eq!(cells.len(), 10);
    let filled = cells.iter().filter(|(c, _)| c == "▓").count();
    assert_eq!(filled, 5);
    let empty = block_gauge(0.0, 10, 0);
    assert!(empty.iter().all(|(c, _)| c == "░"));
    let full = block_gauge(1.5, 10, 0); // clamped
    assert!(full.iter().all(|(c, _)| c == "▓"));
}

#[test]
fn retro_render_smoke_test() {
    use crate::app::{AppState, UiTheme};
    use ratatui::{backend::TestBackend, Terminal};

    // No probe data, boot screen, then console — must not panic at any size.
    for (w, h) in [(100u16, 30u16), (80, 24), (40, 12), (10, 4)] {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            ui_theme: UiTheme::Retro,
            animations_enabled: true,
            ..Default::default()
        };
        state.anim.booting = true;
        terminal.draw(|f| super::render(f, &state)).unwrap();
        for _ in 0..50 {
            state.tick_anim();
        }
        assert!(!state.anim.booting);
        terminal.draw(|f| super::render(f, &state)).unwrap();
    }
}
