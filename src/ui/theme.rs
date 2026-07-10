use crate::api::types::{DangerLevel, KnowledgeLevel, MovementPhase, ProbeStatus, SectorObjectType, SectorObservation};
use crate::app::ColorMode;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

/// Resolved colour palette for the cockpit, one per [`ColorMode`]. Mono modes
/// are single-hue phosphor; `PhosphorSemantic` adds real status colours;
/// `Modern16` uses named ANSI colours for terminals without truecolor.
#[derive(Clone, Copy)]
pub(crate) struct Palette {
    /// Active/selected accent (active borders, tags, selection).
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

pub(crate) fn palette(mode: ColorMode) -> Palette {
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

impl Palette {
    /// Style for at-a-glance urgency signals (the `! n` badge, the `✗` status
    /// line, the RECOVER banner). In the mono palettes `crit == accent`, so
    /// colour alone can't convey urgency — fall back to bold + REVERSED, which
    /// reads regardless of hue. Semantic palettes keep the red, bold.
    pub(crate) fn crit_style(&self) -> Style {
        let base = Style::default().fg(self.crit).add_modifier(Modifier::BOLD);
        if self.crit == self.accent {
            base.add_modifier(Modifier::REVERSED)
        } else {
            base
        }
    }
}

/// Palette-aware pane frame with retro double-line borders. Active panes get
/// the accent colour and a bold title; inactive ones the dim accent.
pub(crate) fn pane_block(title: &str, active: bool, p: Palette) -> Block<'_> {
    let color = if active { p.accent } else { p.accent_dim };
    let modifier = if active { Modifier::BOLD } else { Modifier::empty() };
    Block::default()
        .title(Span::styled(title, Style::default().fg(color).add_modifier(modifier)))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(color))
}

pub(crate) fn map_cell_symbol(s: &SectorObservation) -> (&'static str, Style) {
    if let Some(objects) = &s.objects {
        for obj in objects {
            if matches!(obj.object_type, SectorObjectType::BlackHole) {
                return ("◉", Style::default().fg(Color::Magenta));
            }
            if matches!(obj.danger_level, Some(DangerLevel::Extreme)) {
                return ("!", Style::default().fg(Color::Red));
            }
            if matches!(obj.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem) {
                let has_minable = obj.minable_targets.as_ref().is_some_and(|t| !t.is_empty());
                return if has_minable {
                    ("★", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else {
                    ("★", Style::default().fg(Color::Yellow))
                };
            }
        }
        return ("●", Style::default().fg(Color::Green));
    }
    ("·", Style::default().fg(Color::White))
}

/// Palette-aware version of [`map_cell_symbol`] for the phosphor cockpit.
pub(crate) fn map_cell_style(s: &SectorObservation, p: Palette) -> (&'static str, Style) {
    if let Some(objects) = &s.objects {
        for obj in objects {
            if matches!(obj.object_type, SectorObjectType::BlackHole) {
                return ("◉", Style::default().fg(p.crit));
            }
            if matches!(obj.danger_level, Some(DangerLevel::Extreme)) {
                return ("!", Style::default().fg(p.crit));
            }
            if matches!(obj.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem) {
                let has_minable = obj.minable_targets.as_ref().is_some_and(|t| !t.is_empty());
                let style = Style::default().fg(p.warn);
                return (
                    "★",
                    if has_minable {
                        style.add_modifier(Modifier::BOLD)
                    } else {
                        style
                    },
                );
            }
        }
        return ("●", Style::default().fg(p.good));
    }
    ("·", Style::default().fg(p.dim))
}

/// Short content summary of a scanned sector for the map info line.
pub(crate) fn item_icon(item_type: &str) -> (&'static str, Color) {
    match item_type {
        "manny" => ("♟", Color::Green),
        "atomic_3d_printer" => ("⚙", Color::Magenta),
        "additional_container" => ("□", Color::Cyan),
        "waypoint_bookmark" => ("◎", Color::Cyan),
        "micro_conductor" | "ceramic_insulator" | "crystal_substrate" | "dopant_matrix" | "integrated_circuit" => {
            ("◈", Color::Yellow)
        }
        _ => ("◈", Color::White),
    }
}

pub(crate) fn knowledge_label(k: &KnowledgeLevel) -> &'static str {
    match k {
        KnowledgeLevel::Detailed => "detailed",
        KnowledgeLevel::NeighborScan => "neighbor scan",
        KnowledgeLevel::DistantScan => "distant scan",
        KnowledgeLevel::LongRangeEstimation => "long range",
        KnowledgeLevel::Unknown => "?",
    }
}

pub(crate) fn knowledge_color(k: &KnowledgeLevel, p: Palette) -> Color {
    match k {
        KnowledgeLevel::Detailed => p.good,
        KnowledgeLevel::NeighborScan => p.accent,
        KnowledgeLevel::DistantScan => p.warn,
        KnowledgeLevel::LongRangeEstimation => p.crit,
        KnowledgeLevel::Unknown => p.dim,
    }
}

/// Palette-aware colour for a scanned object's type. The glyph still comes from
/// [`object_icon`]; this maps the *meaning* onto the active palette so mono
/// modes stay single-hue and semantic modes get green/yellow/red.
pub(crate) fn object_color(t: &SectorObjectType, p: Palette) -> Color {
    match t {
        SectorObjectType::Star | SectorObjectType::SolarSystem => p.warn,
        SectorObjectType::Planet => p.accent,
        SectorObjectType::Asteroid | SectorObjectType::DriftingItem => p.text,
        SectorObjectType::DustCloud => p.dim,
        SectorObjectType::BlackHole => p.crit,
        SectorObjectType::Manny | SectorObjectType::DeuteriumRefuelStation => p.good,
        SectorObjectType::DetachedContainer | SectorObjectType::ScutRelay => p.accent,
        SectorObjectType::DormantConstruct => p.warn,
        SectorObjectType::Unknown => p.dim,
    }
}

/// Human label for an object type, used to synthesize a name (`asteroid #2`)
/// when the API returns none.
pub(crate) fn object_type_label(t: &SectorObjectType) -> &'static str {
    match t {
        SectorObjectType::Star => "star",
        SectorObjectType::Planet => "planet",
        SectorObjectType::Asteroid => "asteroid",
        SectorObjectType::DustCloud => "dust cloud",
        SectorObjectType::BlackHole => "black hole",
        SectorObjectType::SolarSystem => "solar system",
        SectorObjectType::Manny => "manny",
        SectorObjectType::DriftingItem => "drifting item",
        SectorObjectType::DetachedContainer => "container",
        SectorObjectType::DeuteriumRefuelStation => "fuel station",
        SectorObjectType::ScutRelay => "SCUT relay",
        SectorObjectType::DormantConstruct => "dormant construct",
        SectorObjectType::Unknown => "object",
    }
}

pub(crate) fn object_icon(t: &SectorObjectType) -> (&'static str, Color) {
    match t {
        SectorObjectType::Star => ("★", Color::Yellow),
        SectorObjectType::Planet => ("●", Color::Cyan),
        SectorObjectType::Asteroid => ("◆", Color::White),
        SectorObjectType::DustCloud => ("~", Color::DarkGray),
        SectorObjectType::BlackHole => ("◉", Color::Magenta),
        SectorObjectType::SolarSystem => ("⊙", Color::Yellow),
        SectorObjectType::Manny => ("♟", Color::Green),
        SectorObjectType::DriftingItem => ("◌", Color::White),
        SectorObjectType::DetachedContainer => ("□", Color::Cyan),
        SectorObjectType::DeuteriumRefuelStation => ("⛽", Color::Green),
        SectorObjectType::ScutRelay => ("≣", Color::LightBlue),
        SectorObjectType::DormantConstruct => ("⍟", Color::Yellow),
        SectorObjectType::Unknown => ("?", Color::DarkGray),
    }
}

pub(crate) fn probe_status_label(s: &ProbeStatus) -> &'static str {
    match s {
        ProbeStatus::Idle => "idle",
        ProbeStatus::Preparing => "preparing",
        ProbeStatus::Accelerating => "accelerating",
        ProbeStatus::Cruising => "cruising",
        ProbeStatus::Decelerating => "decelerating",
        ProbeStatus::Orbiting => "orbiting",
        ProbeStatus::Disabled => "disabled",
        ProbeStatus::Dead => "DEAD",
        ProbeStatus::TrappedByBlackHole => "TRAPPED",
        ProbeStatus::Unknown => "?",
    }
}

pub(crate) fn probe_status_style(s: &ProbeStatus) -> Style {
    match s {
        ProbeStatus::Idle | ProbeStatus::Orbiting => Style::default().fg(Color::White),
        ProbeStatus::Preparing | ProbeStatus::Decelerating => Style::default().fg(Color::Yellow),
        ProbeStatus::Accelerating => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ProbeStatus::Cruising => Style::default().fg(Color::Cyan),
        ProbeStatus::Disabled => Style::default().fg(Color::Red),
        ProbeStatus::Dead | ProbeStatus::TrappedByBlackHole => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        ProbeStatus::Unknown => Style::default().fg(Color::DarkGray),
    }
}

pub(crate) fn movement_phase_label(p: &MovementPhase) -> &'static str {
    match p {
        MovementPhase::Idle => "idle",
        MovementPhase::Preparing => "preparing",
        MovementPhase::Accelerating => "accelerating",
        MovementPhase::Cruising => "cruising",
        MovementPhase::Decelerating => "decelerating",
        MovementPhase::Arrived => "arrived",
        MovementPhase::Failed => "failed",
        MovementPhase::Destroyed => "destroyed",
        MovementPhase::Unknown => "?",
    }
}

/// Palette colour for a "how full" ratio: good > 50 %, warn 25–50 %, crit below.
pub(crate) fn ratio_color(ratio: f64, p: Palette) -> Color {
    if ratio > 0.5 {
        p.good
    } else if ratio > 0.25 {
        p.warn
    } else {
        p.crit
    }
}

/// Retro block gauge: `LABEL ▓▓▓▓▓▓░░░░  62%` — filled cells in `fill`, empty
/// cells dim, value on the right. Static (no animation); the phosphor-CRT
/// "squares" look.
pub(crate) fn block_gauge_line(label: &str, ratio: f64, value: &str, fill: Color, p: Palette) -> Line<'static> {
    const WIDTH: usize = 10;
    let ratio = ratio.clamp(0.0, 1.0);
    let filled = (ratio * WIDTH as f64).round() as usize;
    Line::from(vec![
        Span::styled(format!("{label:<9} "), Style::default().fg(p.dim)),
        Span::styled("▓".repeat(filled), Style::default().fg(fill)),
        Span::styled("░".repeat(WIDTH - filled), Style::default().fg(p.dim)),
        Span::styled(format!(" {value:>5}"), Style::default().fg(p.text)),
    ])
}

pub fn format_duration(secs: i64) -> String {
    if secs <= 0 {
        return "arriving…".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {:02}m {:02}s", h, m, s)
    } else if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ColorMode;

    #[test]
    fn crit_style_reverses_only_in_mono() {
        // Mono palettes: crit == accent, so urgency needs REVERSED to read.
        for mode in [ColorMode::MonoGreen, ColorMode::MonoAmber] {
            assert!(
                palette(mode).crit_style().add_modifier.contains(Modifier::REVERSED),
                "{mode:?} crit_style must reverse"
            );
        }
        // Semantic palettes keep the distinct red — no reverse needed.
        for mode in [ColorMode::PhosphorSemantic, ColorMode::Modern16] {
            assert!(
                !palette(mode).crit_style().add_modifier.contains(Modifier::REVERSED),
                "{mode:?} crit_style must not reverse"
            );
        }
    }
}
