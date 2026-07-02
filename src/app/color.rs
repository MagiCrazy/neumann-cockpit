//! Cockpit color mode (config `theme`, F2 cycles at runtime).

/// Color mode for the unified Cockpit interface. Mono modes are single-hue
/// phosphor; `PhosphorSemantic` adds green/yellow/red status colours;
/// `Modern16` uses named ANSI colours for terminals without truecolor.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ColorMode {
    #[default]
    MonoGreen,
    MonoAmber,
    PhosphorSemantic,
    Modern16,
}

impl ColorMode {
    pub fn cycle(self) -> Self {
        match self {
            ColorMode::MonoGreen => ColorMode::MonoAmber,
            ColorMode::MonoAmber => ColorMode::PhosphorSemantic,
            ColorMode::PhosphorSemantic => ColorMode::Modern16,
            ColorMode::Modern16 => ColorMode::MonoGreen,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorMode::MonoGreen => "mono-green",
            ColorMode::MonoAmber => "mono-amber",
            ColorMode::PhosphorSemantic => "phosphor-semantic",
            ColorMode::Modern16 => "modern-16",
        }
    }
}
