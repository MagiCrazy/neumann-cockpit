//! Color palette for the unified Cockpit interface (bloc U7).
//!
//! One [`Palette`] per [`ColorMode`]. Mono modes are single-hue phosphor —
//! there is no red, so `warn`/`crit` collapse to a brighter shade of the
//! accent. `PhosphorSemantic` keeps the green base but adds real
//! green/yellow/red status colours. `Modern16` uses named ANSI colours for
//! terminals without truecolor.

use crate::app::ColorMode;
use ratatui::style::Color;

#[derive(Clone, Copy)]
pub struct Palette {
    /// Active/selected accent (borders, tags, selection).
    pub accent: Color,
    /// Dim accent (inactive borders).
    pub accent_dim: Color,
    /// Primary readable text.
    pub text: Color,
    /// Secondary/muted text.
    pub dim: Color,
    pub good: Color,
    pub warn: Color,
    pub crit: Color,
}

pub fn palette(mode: ColorMode) -> Palette {
    match mode {
        ColorMode::MonoGreen => {
            let accent = Color::Rgb(0x5e, 0xf0, 0x8f);
            Palette {
                accent,
                accent_dim: Color::Rgb(0x2f, 0x7a, 0x52),
                text: Color::Rgb(0xb6, 0xd4, 0xc2),
                dim: Color::Rgb(0x3a, 0x5a, 0x48),
                good: accent,
                warn: Color::Rgb(0xc8, 0xff, 0xdd),
                crit: accent,
            }
        }
        ColorMode::MonoAmber => {
            let accent = Color::Rgb(0xff, 0xb2, 0x4a);
            Palette {
                accent,
                accent_dim: Color::Rgb(0x8a, 0x5e, 0x22),
                text: Color::Rgb(0xf0, 0xd8, 0xb0),
                dim: Color::Rgb(0x6e, 0x4a, 0x16),
                good: accent,
                warn: Color::Rgb(0xff, 0xe1, 0xad),
                crit: accent,
            }
        }
        ColorMode::PhosphorSemantic => Palette {
            accent: Color::Rgb(0x5e, 0xf0, 0x8f),
            accent_dim: Color::Rgb(0x2f, 0x7a, 0x52),
            text: Color::Rgb(0xb6, 0xd4, 0xc2),
            dim: Color::Rgb(0x3a, 0x5a, 0x48),
            good: Color::Rgb(0x5e, 0xf0, 0x8f),
            warn: Color::Rgb(0xff, 0xd2, 0x4a),
            crit: Color::Rgb(0xff, 0x5d, 0x6b),
        },
        ColorMode::Modern16 => Palette {
            accent: Color::Green,
            accent_dim: Color::DarkGray,
            text: Color::Gray,
            dim: Color::DarkGray,
            good: Color::Green,
            warn: Color::Yellow,
            crit: Color::Red,
        },
    }
}
