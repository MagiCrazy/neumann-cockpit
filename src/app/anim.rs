use super::AppState;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum UiTheme {
    #[default]
    Classic,
    Retro,
    /// Unified Cockpit v2 interface — opt-in preview during the U-series
    /// migration (config `ui = "cockpit"`), becomes the default at bloc U8.
    Cockpit,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Phosphor {
    #[default]
    Green,
    Amber,
}

/// Boot sequence length in animation frames (~10 fps → ~3.5 s).
pub const BOOT_TOTAL_FRAMES: u64 = 36;

#[derive(Default)]
pub struct AnimState {
    /// Monotonic frame counter, advanced by the render tick. All retro
    /// animations are pure functions of this value.
    pub frame: u64,
    pub booting: bool,
    pub boot_frame: u64,
}

/// Deterministic 64-bit mixer (splitmix-style finalizer) used to derive
/// animation noise — blip placement, telemetry ticker, star twinkle —
/// without a rand dependency.
pub fn anim_hash(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    x
}

impl AppState {
    /// Advance animation state by one render tick. Never triggers I/O.
    pub fn tick_anim(&mut self) {
        self.anim.frame = self.anim.frame.wrapping_add(1);
        if self.anim.booting {
            self.anim.boot_frame += 1;
            if self.anim.boot_frame >= BOOT_TOTAL_FRAMES {
                self.anim.booting = false;
            }
        }
    }

    pub fn toggle_theme(&mut self) {
        self.ui_theme = match self.ui_theme {
            UiTheme::Classic => UiTheme::Retro,
            UiTheme::Retro => UiTheme::Classic,
            // Cockpit is a config-gated dev preview; F2 stays put until the
            // color-mode cycling lands in U7.
            UiTheme::Cockpit => UiTheme::Cockpit,
        };
    }

    pub fn skip_boot(&mut self) {
        self.anim.booting = false;
    }

    /// True when the render tick should run: retro theme with animations on,
    /// or a boot sequence still playing.
    pub fn anim_tick_active(&self) -> bool {
        (self.ui_theme == UiTheme::Retro && self.animations_enabled) || self.anim.booting
    }
}
