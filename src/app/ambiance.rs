//! Cockpit ambiance: entropy, signal age, attract mode (issues #204, #205, #206).
//!
//! Atmospheric texture — a cosmic ray flipping a glyph, a relayed Manny whose
//! marker breathes with its distance, a starfield when the cockpit is left
//! alone. None of it is information; all of it is meant to make the cockpit
//! feel like a place.
//!
//! Three rules hold the whole module together, and each one is load-bearing:
//!
//! 1. **All of it switches off, together.** One `ambiance` config key and one
//!    `F6`, because an effect a pilot cannot stop is hostile — the same lesson
//!    the notification mute learned in #331. With it off, [`Ambiance::tick`]
//!    returns immediately and every query answers "nothing", so the cockpit
//!    renders exactly as it did before this module existed.
//! 2. **Entropy never touches a number.** A cockpit that garbles a fuel
//!    reading for effect is a cockpit you stop trusting. Bit-flips land on
//!    decorative glyphs — borders, dividers, the frame — and the renderer is
//!    handed the *cells*, never the data.
//! 3. **Nothing hides something that matters.** Attract mode refuses to start
//!    while anything needs attention, and any key dismisses it.

use std::time::{Duration, Instant};

/// How long the cockpit must sit untouched before the starfield takes over
/// (issue #206). Long enough that it never interrupts play.
pub const ATTRACT_AFTER: Duration = Duration::from_secs(5 * 60);

/// Odds of a decorative cell glitching on a given tick, as one-in-N. Rare on
/// purpose: entropy that is *noticed* every second is a strobe, not texture.
const GLITCH_ODDS: u32 = 14;

/// How many cells may glitch at once, however unlucky the roll.
const GLITCH_MAX: usize = 3;

/// Frames a flipped cell stays flipped. Two ticks reads as a flicker; longer
/// reads as a rendering bug.
const GLITCH_TICKS: u8 = 2;

/// The glyphs a cosmic ray may leave behind. All box-drawing or block, so a
/// flip always lands *on* the frame rather than punching a hole in it.
const GLITCH_GLYPHS: [&str; 6] = ["▚", "▞", "░", "▒", "╳", "▓"];

/// One decorative cell currently flipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glitch {
    /// Which cell of the pane's border run — the renderer decides what that
    /// means for its own frame, so this module never needs to know the layout.
    pub slot: u16,
    pub glyph: &'static str,
    ticks_left: u8,
}

/// Ambiance state. Lives in `AppState`; ticked by the 1 s UI tick.
#[derive(Debug, Clone)]
pub struct Ambiance {
    /// The single switch. `false` means this module does nothing at all.
    pub enabled: bool,
    /// Ticks since the cockpit started, driving every periodic effect.
    frame: u64,
    /// xorshift64* state — the same hand-rolled approach as the probe sigil's
    /// FNV, and for the same reason: one small deterministic generator beats a
    /// dependency for decoration.
    rng: u64,
    glitches: Vec<Glitch>,
    /// Last keypress. Attract mode measures its idleness from here.
    last_input: Instant,
    /// Whether the starfield is up.
    pub attract: bool,
}

impl Default for Ambiance {
    fn default() -> Self {
        Ambiance {
            // Off until the config says otherwise, so a cockpit that never
            // opts in is byte-for-byte the cockpit that existed before.
            enabled: false,
            frame: 0,
            rng: 0x2545_f491_4f6c_dd1d,
            glitches: Vec::new(),
            last_input: Instant::now(),
            attract: false,
        }
    }
}

impl Ambiance {
    /// Seed the generator so two cockpits started at the same second do not
    /// glitch in lockstep. Any non-zero seed will do.
    pub fn seeded(seed: u64) -> Self {
        Ambiance {
            rng: seed | 1,
            ..Default::default()
        }
    }

    /// xorshift64*: small, fast, and good enough for deciding where a
    /// cosmic ray lands.
    fn next(&mut self) -> u64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn roll(&mut self, n: u32) -> u32 {
        (self.next() % n.max(1) as u64) as u32
    }

    /// Advance one tick. `attract_allowed` is the caller's answer to "is it
    /// safe to cover the screen right now" — this module does not know what a
    /// probe alert is, and should not.
    pub fn tick(&mut self, attract_allowed: bool) {
        if !self.enabled {
            // Leaving the switch off must cost nothing and leave nothing
            // behind, including a starfield from before it was turned off.
            self.glitches.clear();
            self.attract = false;
            return;
        }
        self.frame = self.frame.wrapping_add(1);

        self.glitches.retain_mut(|g| {
            g.ticks_left = g.ticks_left.saturating_sub(1);
            g.ticks_left > 0
        });
        if self.glitches.len() < GLITCH_MAX && self.roll(GLITCH_ODDS) == 0 {
            let slot = self.roll(u16::MAX as u32) as u16;
            let glyph = GLITCH_GLYPHS[self.roll(GLITCH_GLYPHS.len() as u32) as usize];
            self.glitches.push(Glitch {
                slot,
                glyph,
                ticks_left: GLITCH_TICKS,
            });
        }

        self.attract = attract_allowed && self.last_input.elapsed() >= ATTRACT_AFTER;
    }

    /// Note a keypress: it resets the idle clock and dismisses the starfield.
    /// Returns `true` when the key was consumed *by* the starfield, so the
    /// caller knows not to also act on it — waking a cockpit should not fire
    /// whatever the sleeping pilot happened to press.
    pub fn note_input(&mut self) -> bool {
        self.last_input = Instant::now();
        let dismissed = self.attract;
        self.attract = false;
        dismissed
    }

    /// Pretend the cockpit has been untouched long enough for the starfield.
    /// Test-only: the idle clock is otherwise driven by real keypresses, which
    /// a render test has no way to withhold for five minutes.
    #[cfg(test)]
    pub fn force_idle(&mut self) {
        self.last_input = Instant::now() - ATTRACT_AFTER;
    }

    /// The cells flipped this instant. Empty whenever ambiance is off.
    pub fn glitches(&self) -> &[Glitch] {
        &self.glitches
    }

    /// Where a glitch lands in a border run of `len` cells, or `None` when it
    /// falls outside this run. The slot is a raw draw, so a short border simply
    /// gets fewer hits than a long one — which is what a cosmic ray does.
    pub fn glitch_at(&self, len: u16) -> Option<(u16, &'static str)> {
        if len == 0 {
            return None;
        }
        self.glitches.first().map(|g| (g.slot % len, g.glyph))
    }

    /// Breathing intensity `0.0..=1.0` for the SCUT marker (issue #205), so a
    /// Manny watched through a relay reads as *watched from far away* rather
    /// than as one sitting next door. A slow triangle wave: no easing, because
    /// the terminal has four intensity steps at best.
    pub fn scut_pulse(&self) -> f64 {
        if !self.enabled {
            return 1.0;
        }
        const PERIOD: u64 = 8;
        let phase = self.frame % PERIOD;
        let half = PERIOD / 2;
        let up = if phase < half {
            phase as f64 / half as f64
        } else {
            1.0 - (phase - half) as f64 / half as f64
        };
        // Never fully dark: the marker still has to be readable at its dimmest.
        0.45 + 0.55 * up
    }

    /// A starfield for `w × h`, as `(x, y, glyph)`. Deterministic in the frame,
    /// so the drift is smooth rather than a new random field every tick.
    pub fn starfield(&self, w: u16, h: u16, count: usize) -> Vec<(u16, u16, &'static str)> {
        if !self.enabled || w == 0 || h == 0 {
            return Vec::new();
        }
        const STARS: [&str; 4] = ["·", "˙", "*", "✦"];
        (0..count)
            .map(|i| {
                // Each star gets its own hash, then drifts left at a speed
                // derived from it — near stars faster than far ones, which is
                // the whole of the parallax.
                let mut h64 = 0xcbf2_9ce4_8422_2325u64;
                for b in (i as u64).to_le_bytes() {
                    h64 ^= b as u64;
                    h64 = h64.wrapping_mul(0x0000_0100_0000_01b3);
                }
                let speed = 1 + (h64 % 3);
                let drift = (self.frame * speed / 2) % w.max(1) as u64;
                let x = ((h64 % w as u64) + w as u64 - drift) % w as u64;
                let y = (h64 >> 17) % h as u64;
                let glyph = STARS[((h64 >> 33) % STARS.len() as u64) as usize];
                (x as u16, y as u16, glyph)
            })
            .collect()
    }

    /// Extra boot self-check lines — the chatter half of #204. Empty when the
    /// switch is off, so the boot grid stays exactly as it was.
    pub fn boot_chatter(&self) -> &'static [(&'static str, &'static str)] {
        if !self.enabled {
            return &[];
        }
        &[
            ("COSMIC RAY FLUX", "NOMINAL"),
            ("HULL CREAK", "WITHIN TOLERANCE"),
            ("VACUUM", "STILL THERE"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> Ambiance {
        Ambiance {
            enabled: true,
            ..Ambiance::seeded(0x1234_5678)
        }
    }

    #[test]
    fn switched_off_the_module_does_nothing_at_all() {
        // The point of the switch: a pilot who does not want this gets the
        // cockpit exactly as it was before the module existed.
        let mut a = Ambiance::default();
        for _ in 0..500 {
            a.tick(true);
        }
        assert!(a.glitches().is_empty(), "no entropy");
        assert!(!a.attract, "no starfield, however long it idles");
        assert_eq!(a.scut_pulse(), 1.0, "the SCUT marker holds its full intensity");
        assert!(a.starfield(80, 24, 40).is_empty());
        assert!(a.boot_chatter().is_empty(), "and no extra boot lines");
    }

    #[test]
    fn switching_off_clears_what_was_already_on_screen() {
        // Turning it off must take effect now, not at the next launch —
        // "j'en veux plus" is a request for silence immediately (#331's rule).
        let mut a = on();
        for _ in 0..200 {
            a.tick(true);
        }
        a.enabled = false;
        a.tick(true);
        assert!(a.glitches().is_empty());
        assert!(!a.attract);
    }

    #[test]
    fn entropy_is_rare_and_bounded() {
        // Texture, not a strobe: a handful of flickers over a few hundred
        // seconds, never more than a few cells at once.
        let mut a = on();
        let mut peak = 0;
        let mut ticks_with_glitch = 0;
        for _ in 0..600 {
            a.tick(false);
            peak = peak.max(a.glitches().len());
            if !a.glitches().is_empty() {
                ticks_with_glitch += 1;
            }
        }
        assert!(peak <= GLITCH_MAX, "at most {GLITCH_MAX} cells, saw {peak}");
        assert!(ticks_with_glitch > 0, "some entropy did happen");
        assert!(
            ticks_with_glitch < 400,
            "but not constantly — {ticks_with_glitch}/600 ticks"
        );
    }

    #[test]
    fn a_glitch_only_ever_replaces_a_frame_glyph() {
        // Rule 2: entropy never touches a number. Every glyph it can leave is
        // box-drawing or block, so a flip lands *on* the frame.
        for glyph in GLITCH_GLYPHS {
            assert!(
                glyph.chars().all(|c| !c.is_alphanumeric()),
                "{glyph} could be mistaken for data"
            );
        }
    }

    #[test]
    fn a_glitch_lands_inside_the_run_it_is_given() {
        let mut a = on();
        for _ in 0..200 {
            a.tick(false);
            if let Some((slot, glyph)) = a.glitch_at(30) {
                assert!(slot < 30, "slot {slot} outside a 30-cell border");
                assert!(GLITCH_GLYPHS.contains(&glyph));
            }
        }
        assert_eq!(a.glitch_at(0), None, "a zero-length border cannot be hit");
    }

    #[test]
    fn the_scut_marker_breathes_without_ever_going_dark() {
        // It carries meaning — this Manny is being watched through a relay —
        // so the dimmest point still has to read.
        let mut a = on();
        let mut seen: Vec<f64> = Vec::new();
        for _ in 0..16 {
            a.tick(false);
            seen.push(a.scut_pulse());
        }
        let min = seen.iter().cloned().fold(f64::MAX, f64::min);
        let max = seen.iter().cloned().fold(f64::MIN, f64::max);
        assert!(min >= 0.45, "never darker than 45%, got {min}");
        assert!(max <= 1.0);
        assert!(max - min > 0.3, "and it visibly moves: {min}..{max}");
    }

    #[test]
    fn attract_waits_for_real_idleness_and_any_key_dismisses_it() {
        let mut a = on();
        a.tick(true);
        assert!(!a.attract, "a cockpit just touched is not idle");

        a.last_input = Instant::now() - ATTRACT_AFTER;
        a.tick(true);
        assert!(a.attract, "left alone long enough, the starfield comes up");

        // The key that wakes the cockpit is consumed by the waking.
        assert!(a.note_input(), "the dismissing key is reported as consumed");
        assert!(!a.attract);
        assert!(!a.note_input(), "a later key is the pilot's, not the starfield's");
    }

    #[test]
    fn attract_never_covers_something_that_needs_attention() {
        // Rule 3. The caller owns the judgement; this only proves it is obeyed.
        let mut a = on();
        a.last_input = Instant::now() - ATTRACT_AFTER;
        a.tick(false);
        assert!(!a.attract, "not while something needs the pilot");
        a.tick(true);
        assert!(a.attract, "and up again once it is clear");
    }

    #[test]
    fn the_starfield_fills_its_area_and_drifts() {
        let mut a = on();
        a.tick(true);
        let first = a.starfield(60, 20, 40);
        assert_eq!(first.len(), 40);
        assert!(
            first.iter().all(|(x, y, _)| *x < 60 && *y < 20),
            "every star inside the frame"
        );
        for _ in 0..6 {
            a.tick(true);
        }
        assert_ne!(a.starfield(60, 20, 40), first, "and the field moves");
        assert!(a.starfield(0, 0, 40).is_empty(), "no area, no stars");
    }
}
