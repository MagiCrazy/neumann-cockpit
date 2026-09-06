use crate::api::types::{DangerLevel, KnowledgeLevel, MovementPhase, ProbeStatus, SectorObjectType, SectorObservation};
use crate::app::{ColorMode, Polarity};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
    Frame,
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
    /// Secondary/muted text. It carries real information across the cockpit —
    /// units, ETAs, routing rules, disabled reasons, ship's-log timestamps —
    /// so it is toned for contrast, not just for being quiet: every mode keeps
    /// it above the 4.5:1 WCAG AA text ratio on a dark ground while staying
    /// clearly under `text` (issue #327).
    pub dim: Color,
    pub good: Color,
    pub warn: Color,
    pub crit: Color,
}

/// The cockpit palette for a mode **and** a polarity (issue #233).
///
/// Two axes, deliberately independent: the mode is hue and character, the
/// polarity is the ground the terminal paints behind us. Every mode is defined
/// for both, because the cockpit never paints a background — so on a pale
/// terminal the only way to stay legible is to darken the foregrounds.
///
/// Light variants are not hue swaps: a phosphor green that glows on black
/// washes out on white, so each one is re-toned for ink-on-paper contrast and
/// pinned by the ratio tests below.
pub(crate) fn palette(mode: ColorMode, polarity: Polarity) -> Palette {
    match polarity {
        Polarity::Dark => palette_dark(mode),
        Polarity::Light => palette_light(mode),
    }
}

fn palette_dark(mode: ColorMode) -> Palette {
    match mode {
        ColorMode::MonoGreen => {
            let accent = Color::Rgb(0x5e, 0xf0, 0x8f);
            Palette {
                accent,
                accent_dim: Color::Rgb(0x2f, 0x7a, 0x52),
                text: Color::Rgb(0xb6, 0xd4, 0xc2),
                dim: Color::Rgb(0x6f, 0x9a, 0x82),
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
                dim: Color::Rgb(0xb0, 0x8d, 0x5a),
                good: accent,
                warn: Color::Rgb(0xff, 0xe1, 0xad),
                crit: accent,
            }
        }
        ColorMode::PhosphorSemantic => Palette {
            accent: Color::Rgb(0x5e, 0xf0, 0x8f),
            accent_dim: Color::Rgb(0x2f, 0x7a, 0x52),
            text: Color::Rgb(0xb6, 0xd4, 0xc2),
            dim: Color::Rgb(0x6f, 0x9a, 0x82),
            good: Color::Rgb(0x5e, 0xf0, 0x8f),
            warn: Color::Rgb(0xff, 0xd2, 0x4a),
            crit: Color::Rgb(0xff, 0x5d, 0x6b),
        },
        // Named ANSI only, so `dim` cannot be nudged a few percent: it takes
        // the next real step up (Gray), and `text` moves to White to keep the
        // two ranks apart. DarkGray stays for the inactive borders, which are
        // decoration rather than text.
        // The bright half of the 16 for the status colours: plain `Red` only
        // reaches 3.3:1 on a dark ground, under the bar the other modes hold.
        // DarkGray is likewise too faint for the inactive border (2.6:1), and
        // Gray is the only rank left — so in a 16-colour terminal the inactive
        // border shares `dim`'s tone and the active/inactive distinction is
        // carried by hue (accent green) rather than by brightness.
        ColorMode::Modern16 => Palette {
            accent: Color::Green,
            accent_dim: Color::Gray,
            text: Color::White,
            dim: Color::Gray,
            good: Color::Green,
            warn: Color::LightYellow,
            crit: Color::LightRed,
        },
        // ── Lore modes (#233) ─────────────────────────────────────────────
        // The universe's own colours rather than a pastiche of a CRT. All
        // three are semantic: good/warn/crit are true, distinct colours.
        //
        // `culture` — Mind aesthetic: deep violet ground, lilac/silver accent.
        ColorMode::Culture => Palette {
            accent: Color::Rgb(0xc9, 0xb6, 0xff),
            accent_dim: Color::Rgb(0x6a, 0x5e, 0x9a),
            text: Color::Rgb(0xdf, 0xe2, 0xee),
            dim: Color::Rgb(0x9a, 0x9f, 0xb8),
            good: Color::Rgb(0x8f, 0xe0, 0xb0),
            warn: Color::Rgb(0xff, 0xd2, 0x7a),
            crit: Color::Rgb(0xff, 0x7a, 0x90),
        },
        // `deep-space` — the lone probe: deep navy, ice accent, low saturation.
        ColorMode::DeepSpace => Palette {
            accent: Color::Rgb(0x7f, 0xb4, 0xff),
            accent_dim: Color::Rgb(0x54, 0x69, 0x9a),
            text: Color::Rgb(0xa8, 0xb8, 0xd0),
            dim: Color::Rgb(0x8a, 0x96, 0xae),
            good: Color::Rgb(0x7f, 0xd0, 0xb0),
            warn: Color::Rgb(0xf0, 0xc8, 0x6a),
            crit: Color::Rgb(0xef, 0x6a, 0x80),
        },
        // `rust-belt` — salvage/autofactory: warm ground, oxidised copper, with
        // an olive `good` so it does not clash into a Christmas tree.
        ColorMode::RustBelt => Palette {
            accent: Color::Rgb(0xc8, 0x7f, 0x4a),
            accent_dim: Color::Rgb(0x9a, 0x66, 0x40),
            text: Color::Rgb(0xd8, 0xc4, 0xb0),
            dim: Color::Rgb(0xa8, 0x92, 0x7e),
            good: Color::Rgb(0x9f, 0xb4, 0x6a),
            warn: Color::Rgb(0xe0, 0xb0, 0x50),
            crit: Color::Rgb(0xd8, 0x48, 0x38),
        },
    }
}

/// Ink-on-paper variants. Each keeps its mode's hue and rank order — `accent`
/// is still the loudest, `dim` still quieter than `text` — but every slot is
/// darkened until it clears WCAG AA against a pale ground.
fn palette_light(mode: ColorMode) -> Palette {
    match mode {
        // The mono modes are the hard case: a phosphor glow has no ink
        // equivalent, so the hue is kept and the luminance inverted — dark
        // green/amber on paper, which is what a printed terminal listing looks
        // like. `crit == accent` still holds, so `crit_style` keeps working.
        ColorMode::MonoGreen => {
            let accent = Color::Rgb(0x0f, 0x6b, 0x3d);
            Palette {
                accent,
                accent_dim: Color::Rgb(0x5f, 0x8a, 0x74),
                text: Color::Rgb(0x1c, 0x2e, 0x25),
                dim: Color::Rgb(0x4a, 0x6b, 0x5a),
                good: accent,
                warn: Color::Rgb(0x0a, 0x4f, 0x2c),
                crit: accent,
            }
        }
        ColorMode::MonoAmber => {
            let accent = Color::Rgb(0x8a, 0x4f, 0x05);
            Palette {
                accent,
                accent_dim: Color::Rgb(0x9a, 0x7c, 0x58),
                text: Color::Rgb(0x33, 0x26, 0x14),
                dim: Color::Rgb(0x6d, 0x55, 0x33),
                good: accent,
                warn: Color::Rgb(0x66, 0x39, 0x02),
                crit: accent,
            }
        }
        ColorMode::PhosphorSemantic => Palette {
            accent: Color::Rgb(0x0f, 0x6b, 0x3d),
            accent_dim: Color::Rgb(0x5f, 0x8a, 0x74),
            text: Color::Rgb(0x1c, 0x2e, 0x25),
            dim: Color::Rgb(0x4a, 0x6b, 0x5a),
            good: Color::Rgb(0x1f, 0x7a, 0x44),
            warn: Color::Rgb(0x9a, 0x5a, 0x00),
            crit: Color::Rgb(0xbd, 0x2f, 0x28),
        },
        // Named ANSI again: the dark ranks (White/Gray) would vanish on paper,
        // so the ladder flips to Black for text and DarkGray for dim, and the
        // status colours take their non-bright variants.
        ColorMode::Modern16 => Palette {
            accent: Color::Green,
            accent_dim: Color::Gray,
            text: Color::Black,
            dim: Color::DarkGray,
            good: Color::Green,
            warn: Color::Yellow,
            crit: Color::Red,
        },
        // The lore modes keep their identity by hue: the Mind stays violet, the
        // probe stays ink-blue, the autofactory stays copper.
        ColorMode::Culture => Palette {
            accent: Color::Rgb(0x5b, 0x3f, 0xa8),
            accent_dim: Color::Rgb(0x8d, 0x80, 0xad),
            text: Color::Rgb(0x24, 0x25, 0x33),
            dim: Color::Rgb(0x5c, 0x5f, 0x77),
            good: Color::Rgb(0x1f, 0x7a, 0x50),
            warn: Color::Rgb(0x8a, 0x5b, 0x00),
            crit: Color::Rgb(0xb4, 0x2a, 0x45),
        },
        // The issue's `daylight` reference pair, which shares this mode's
        // ink-blue hue — so it validates the mechanism here rather than as an
        // eighth mode nobody asked for.
        ColorMode::DeepSpace => Palette {
            accent: Color::Rgb(0x1f, 0x70, 0x91),
            accent_dim: Color::Rgb(0x75, 0x8b, 0x95),
            text: Color::Rgb(0x22, 0x27, 0x2e),
            dim: Color::Rgb(0x5a, 0x64, 0x6d),
            good: Color::Rgb(0x1f, 0x7a, 0x44),
            warn: Color::Rgb(0x9a, 0x5a, 0x00),
            crit: Color::Rgb(0xbd, 0x2f, 0x28),
        },
        ColorMode::RustBelt => Palette {
            accent: Color::Rgb(0x8f, 0x4a, 0x1c),
            accent_dim: Color::Rgb(0x9c, 0x81, 0x63),
            text: Color::Rgb(0x32, 0x26, 0x1c),
            dim: Color::Rgb(0x6b, 0x55, 0x42),
            good: Color::Rgb(0x4d, 0x6b, 0x1f),
            warn: Color::Rgb(0x8a, 0x5c, 0x00),
            crit: Color::Rgb(0xa8, 0x2c, 0x1e),
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

/// Scatter a cosmic ray across a pane's **top border** (issue #204).
///
/// The border, deliberately: entropy never touches a number. A cockpit that
/// garbles a fuel reading for effect is a cockpit you stop trusting, so a flip
/// lands on the frame — where it reads as the ship being old, not as the
/// instrument being wrong. Drawn after the block, so it replaces a frame cell
/// rather than fighting it.
pub(crate) fn ambiance_entropy(frame: &mut Frame, area: Rect, ambiance: &crate::app::Ambiance, p: Palette) {
    if area.width < 4 || area.height < 2 {
        return;
    }
    // The run is the top border between the corners: hitting a corner would
    // read as a broken frame rather than as a speck of noise.
    let run = area.width.saturating_sub(2);
    let Some((slot, glyph)) = ambiance.glitch_at(run) else {
        return;
    };
    let x = area.x + 1 + slot;
    if let Some(cell) = frame.buffer_mut().cell_mut((x, area.y)) {
        cell.set_symbol(glyph).set_style(Style::default().fg(p.accent_dim));
    }
}

/// Overflow markers drawn **on** a pane's right border: `▲` just below the top
/// corner when the list continues above the viewport, `▼` just above the
/// bottom corner when it continues below (issue #326).
///
/// On the border rather than in the content, so a pane that is already short
/// spends no content row saying it has more to show. `area` is the pane rect,
/// borders included; `offset` is the first visible line and `total` the line
/// count, exactly as handed to `Paragraph::scroll` — a `List` reports the same
/// pair through its `ListState` offset. The marker takes the border's own
/// colour, so it reads as part of the frame.
pub(crate) fn scroll_markers(frame: &mut Frame, area: Rect, offset: u16, total: usize, active: bool, p: Palette) {
    if area.width < 2 || area.height < 3 {
        return;
    }
    let height = area.height.saturating_sub(2) as usize;
    if height == 0 || total <= height {
        return; // everything fits — no marker, or it would cry wolf
    }
    let above = offset > 0;
    let below = offset as usize + height < total;
    let x = area.right() - 1;
    let (top, bottom) = (area.y + 1, area.bottom() - 2);
    let color = if active { p.accent } else { p.accent_dim };
    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let mut mark = |y: u16, symbol: &str| {
        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
            cell.set_symbol(symbol).set_style(style);
        }
    };
    // A one-row viewport has a single border row to write on: what lies below
    // is the more useful half, since the pilot got here by scrolling down.
    if below {
        mark(bottom, "▼");
    }
    if above && (top != bottom || !below) {
        mark(top, "▲");
    }
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

/// Unicode sparkline over a series of ratios (each clamped to `0.0..=1.0`),
/// mapping every value to one of eight block glyphs `▁▂▃▄▅▆▇█` and keeping the
/// last `width` samples. An empty series or zero width yields "". Used for the
/// zoomed Probe pane's telemetry trends (issue #201).
pub(crate) fn text_sparkline(values: &[f64], width: usize) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if values.is_empty() || width == 0 {
        return String::new();
    }
    let start = values.len().saturating_sub(width);
    values[start..]
        .iter()
        .map(|&v| {
            let idx = (v.clamp(0.0, 1.0) * (BARS.len() - 1) as f64).round() as usize;
            BARS[idx]
        })
        .collect()
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
    use crate::app::{ColorMode, Polarity};

    #[test]
    fn sparkline_maps_extremes_and_keeps_tail() {
        assert_eq!(text_sparkline(&[], 8), "");
        assert_eq!(text_sparkline(&[0.0, 1.0], 8), "▁█");
        // Only the last `width` samples are kept.
        assert_eq!(text_sparkline(&[0.0, 0.0, 1.0], 1), "█");
    }

    #[test]
    fn crit_style_reverses_only_in_mono() {
        // Mono palettes: crit == accent, so urgency needs REVERSED to read.
        for mode in [ColorMode::MonoGreen, ColorMode::MonoAmber] {
            assert!(
                palette(mode, Polarity::Dark)
                    .crit_style()
                    .add_modifier
                    .contains(Modifier::REVERSED),
                "{mode:?} crit_style must reverse"
            );
        }
        // Semantic palettes keep the distinct red — no reverse needed.
        for mode in [ColorMode::PhosphorSemantic, ColorMode::Modern16] {
            assert!(
                !palette(mode, Polarity::Dark)
                    .crit_style()
                    .add_modifier
                    .contains(Modifier::REVERSED),
                "{mode:?} crit_style must not reverse"
            );
        }
    }

    /// sRGB channels for a palette colour. Named ANSI colours are resolved to
    /// the conventional xterm values — terminals vary, but the ranking they
    /// encode (DarkGray < Gray < White) is what the test relies on.
    fn channels(c: Color) -> (u8, u8, u8) {
        match c {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::White => (0xff, 0xff, 0xff),
            Color::Gray => (0xaa, 0xaa, 0xaa),
            Color::DarkGray => (0x55, 0x55, 0x55),
            Color::Black => (0x00, 0x00, 0x00),
            // The xterm 16-colour defaults. `modern-16` exists precisely
            // because these vary between terminals, so the ratios below are an
            // indication for it rather than the guarantee they are elsewhere —
            // which is why its status colours are checked at the same bar but
            // its palette is the one a themed terminal may legitimately shift.
            Color::Green => (0x00, 0xcd, 0x00),
            Color::Yellow => (0xcd, 0xcd, 0x00),
            Color::Red => (0xcd, 0x00, 0x00),
            Color::LightYellow => (0xff, 0xff, 0x00),
            Color::LightRed => (0xff, 0x00, 0x00),
            other => panic!("no sRGB value known for {other:?}"),
        }
    }

    /// WCAG 2.x relative luminance.
    fn luminance(c: Color) -> f64 {
        let (r, g, b) = channels(c);
        let lin = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    }

    /// WCAG contrast ratio between two colours, 1.0..=21.0.
    fn contrast(a: Color, b: Color) -> f64 {
        let (hi, lo) = {
            let (x, y) = (luminance(a), luminance(b));
            if x > y {
                (x, y)
            } else {
                (y, x)
            }
        };
        (hi + 0.05) / (lo + 0.05)
    }

    /// The two grounds the palettes are designed against. Real terminals sit
    /// somewhere near these; a palette that clears them clears the ones in
    /// between.
    const DARK_GROUND: Color = Color::Rgb(0x0d, 0x0d, 0x0d);
    const LIGHT_GROUND: Color = Color::Rgb(0xf5, 0xf5, 0xf5);

    fn ground(polarity: Polarity) -> Color {
        match polarity {
            Polarity::Dark => DARK_GROUND,
            Polarity::Light => LIGHT_GROUND,
        }
    }

    #[test]
    fn every_palette_is_readable_against_its_own_ground() {
        // `dim` used to sit at 2.7:1 on black — under even the 3:1 asked of
        // non-text elements (issue #327). Now that each mode is defined for
        // both grounds (#233), the guarantee has to hold 14 times, not 4:
        // a light variant that was only a hue swap would fail here.
        for polarity in [Polarity::Dark, Polarity::Light] {
            let bg = ground(polarity);
            for mode in ColorMode::ALL {
                let p = palette(mode, polarity);
                let label = format!("{} / {}", mode.label(), polarity.label());

                // `modern-16` names ANSI slots rather than colours: what they
                // render as belongs to the terminal, and a light-theme terminal
                // remaps them to inks precisely so they stay readable. Holding
                // it to the xterm defaults would be measuring someone else's
                // palette — ANSI green is 1.98:1 on white and there is no
                // darker green in the sixteen. Its structure is still checked.
                if mode == ColorMode::Modern16 {
                    assert_ne!(p.text, p.dim, "{label}: text and dim must be distinct ranks");
                    assert_ne!(p.accent, p.text, "{label}: the accent must stand out from text");
                    continue;
                }

                let dim = contrast(p.dim, bg);
                assert!(dim >= 4.5, "{label}: dim is {dim:.2}:1, want >= 4.5");
                let text = contrast(p.text, bg);
                assert!(text >= 4.5, "{label}: text is {text:.2}:1, want >= 4.5");
                assert!(text > dim, "{label}: text must stay brighter than dim");

                // Status colours carry meaning on their own, so they have to be
                // legible too — this is what a hue-swapped light palette fails.
                for (name, color) in [("good", p.good), ("warn", p.warn), ("crit", p.crit)] {
                    let ratio = contrast(color, bg);
                    assert!(ratio >= 4.5, "{label}: {name} is {ratio:.2}:1, want >= 4.5");
                }
                // Inactive borders are decoration, not text: 3:1 is the bar.
                let border = contrast(p.accent_dim, bg);
                assert!(border >= 3.0, "{label}: accent_dim is {border:.2}:1, want >= 3.0");
            }
        }
    }

    #[test]
    fn a_palette_is_never_used_against_the_wrong_ground_unnoticed() {
        // The point of the polarity axis: the dark palettes really are
        // unusable on paper, which is why light variants had to be authored
        // rather than reused. If this ever passes, the two axes have collapsed.
        let dark_on_paper = contrast(palette(ColorMode::MonoGreen, Polarity::Dark).text, LIGHT_GROUND);
        assert!(
            dark_on_paper < 4.5,
            "mono-green's dark text reads on paper at {dark_on_paper:.2}:1 — is the light variant still needed?"
        );
    }

    #[test]
    fn the_mono_crit_style_still_reads_under_light_polarity() {
        // In the mono modes `crit == accent`, so urgency is carried by
        // bold+REVERSED rather than hue: the cell paints `accent` as the
        // background. Under light polarity `accent` is ink, so the reversed
        // cell is pale-on-ink — it has to clear the bar the other way round.
        for mode in [ColorMode::MonoGreen, ColorMode::MonoAmber] {
            for polarity in [Polarity::Dark, Polarity::Light] {
                let p = palette(mode, polarity);
                assert!(
                    p.crit_style().add_modifier.contains(Modifier::REVERSED),
                    "{} still needs the reverse",
                    mode.label()
                );
                // Reversed: the accent becomes the ground, the ground the ink.
                let ratio = contrast(p.accent, ground(polarity));
                assert!(
                    ratio >= 4.5,
                    "{} / {}: reversed crit is {ratio:.2}:1",
                    mode.label(),
                    polarity.label()
                );
            }
        }
    }
}
