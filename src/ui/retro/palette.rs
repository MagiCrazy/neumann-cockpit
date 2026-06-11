use crate::app::Phosphor;
use ratatui::style::{Color, Modifier, Style};

/// Four-intensity monochrome palette emulating a phosphor CRT.
/// Everything in the retro theme is drawn from these levels; `alert`
/// is the only chromatic exception.
#[derive(Clone, Copy)]
pub(crate) struct Pal {
    pub dim: Color,
    pub norm: Color,
    pub bright: Color,
    pub alert: Color,
}

pub(crate) fn pal(phosphor: Phosphor) -> Pal {
    match phosphor {
        Phosphor::Green => Pal {
            dim: Color::Rgb(0, 88, 24),
            norm: Color::Rgb(0, 176, 64),
            bright: Color::Rgb(120, 255, 160),
            alert: Color::Rgb(255, 72, 48),
        },
        Phosphor::Amber => Pal {
            dim: Color::Rgb(112, 64, 0),
            norm: Color::Rgb(216, 136, 0),
            bright: Color::Rgb(255, 208, 112),
            alert: Color::Rgb(255, 72, 48),
        },
    }
}

impl Pal {
    pub fn dim(&self) -> Style {
        Style::default().fg(self.dim)
    }
    pub fn norm(&self) -> Style {
        Style::default().fg(self.norm)
    }
    pub fn bright(&self) -> Style {
        Style::default().fg(self.bright)
    }
    pub fn bold(&self) -> Style {
        Style::default().fg(self.bright).add_modifier(Modifier::BOLD)
    }
    pub fn alert(&self) -> Style {
        Style::default().fg(self.alert).add_modifier(Modifier::BOLD)
    }
}

/// Retro block gauge: `▓▓▓▓▓░░░░░`. A single bright cell sweeps along the
/// filled part every few seconds — the periodic "self-check" shimmer.
pub(crate) fn block_gauge(ratio: f64, width: usize, frame: u64) -> Vec<(String, bool)> {
    let ratio = ratio.clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let sweep = ((frame / 2) % (width as u64 * 4)) as usize; // sweeps 1 in 4 cycles
    let mut cells: Vec<(String, bool)> = Vec::with_capacity(width);
    for i in 0..width {
        if i < filled {
            cells.push(("▓".into(), i == sweep));
        } else {
            cells.push(("░".into(), false));
        }
    }
    cells
}
