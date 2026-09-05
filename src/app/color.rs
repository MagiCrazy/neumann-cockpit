//! Cockpit colour mode and polarity (config `theme` / `polarity`; `F2` and
//! `F3` cycle them at runtime).
//!
//! The two axes are deliberately independent (issue #233): [`ColorMode`] is the
//! hue and character of the cockpit, [`Polarity`] is whether the terminal it
//! sits on is dark or light. Every mode is defined for both, because the
//! cockpit never paints a background — `Palette` has no `bg` slot, ratatui lets
//! the terminal's own ground show through, and painting it would override the
//! pilot's terminal look, transparency and blur. So the only way to be legible
//! on a pale ground is to pick foregrounds that contrast with *it*.

/// Colour mode for the unified Cockpit interface. The mono modes are
/// single-hue phosphor; `PhosphorSemantic` adds green/yellow/red status
/// colours; `Modern16` uses named ANSI colours for terminals without
/// truecolor; the three **lore** modes (`Culture`, `DeepSpace`, `RustBelt`)
/// dress the cockpit in the game's own universe rather than pastiching a CRT,
/// and are semantic throughout.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorMode {
    #[default]
    MonoGreen,
    MonoAmber,
    PhosphorSemantic,
    Modern16,
    Culture,
    DeepSpace,
    RustBelt,
}

impl ColorMode {
    /// Every mode, in `F2` cycle order. The single source the cycle and the
    /// tests both read, so a new mode cannot be added to one and not the other.
    pub const ALL: [ColorMode; 7] = [
        ColorMode::MonoGreen,
        ColorMode::MonoAmber,
        ColorMode::PhosphorSemantic,
        ColorMode::Modern16,
        ColorMode::Culture,
        ColorMode::DeepSpace,
        ColorMode::RustBelt,
    ];

    pub fn cycle(self) -> Self {
        let i = ColorMode::ALL.iter().position(|m| *m == self).unwrap_or(0);
        ColorMode::ALL[(i + 1) % ColorMode::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorMode::MonoGreen => "mono-green",
            ColorMode::MonoAmber => "mono-amber",
            ColorMode::PhosphorSemantic => "phosphor-semantic",
            ColorMode::Modern16 => "modern-16",
            ColorMode::Culture => "culture",
            ColorMode::DeepSpace => "deep-space",
            ColorMode::RustBelt => "rust-belt",
        }
    }

    /// Parse a config `theme` value; unknown labels fall back to the default.
    pub fn from_label(label: &str) -> Option<Self> {
        ColorMode::ALL.into_iter().find(|m| m.label() == label)
    }
}

/// Whether the terminal the cockpit sits on has a dark or a light background.
/// Detected at boot (OSC 11), overridable by config and by `F3`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    /// Light foregrounds on the terminal's dark ground — the historical
    /// assumption of every palette, and the fallback when detection is
    /// inconclusive.
    #[default]
    Dark,
    /// Ink foregrounds for a pale ground.
    Light,
}

impl Polarity {
    pub fn toggle(self) -> Self {
        match self {
            Polarity::Dark => Polarity::Light,
            Polarity::Light => Polarity::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Polarity::Dark => "dark",
            Polarity::Light => "light",
        }
    }
}

/// What the config `polarity` key asks for.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PolarityPref {
    /// Ask the terminal at boot (OSC 11) and take its answer.
    #[default]
    Auto,
    /// Skip detection; the pilot has decided.
    Forced(Polarity),
}

impl PolarityPref {
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "auto" => Some(PolarityPref::Auto),
            "dark" => Some(PolarityPref::Forced(Polarity::Dark)),
            "light" => Some(PolarityPref::Forced(Polarity::Light)),
            _ => None,
        }
    }
}
