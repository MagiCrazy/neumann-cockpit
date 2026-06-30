use crate::api::types::{
    DangerLevel, DataFreshness, KnowledgeLevel,
    MovementPhase, ProbeStatus, SectorObjectType, SectorObservation, SensorMode,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, LineGauge},
};

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
                let has_minable = obj.minable_targets.as_ref()
                    .is_some_and(|t| !t.is_empty());
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

/// Short content summary of a scanned sector for the map info line.
pub(crate) fn item_icon(item_type: &str) -> (&'static str, Color) {
    match item_type {
        "manny" => ("♟", Color::Green),
        "atomic_3d_printer" => ("⚙", Color::Magenta),
        "additional_container" => ("□", Color::Cyan),
        "waypoint_bookmark" => ("◎", Color::Cyan),
        "micro_conductor" | "ceramic_insulator" | "crystal_substrate"
        | "dopant_matrix" | "integrated_circuit" => ("◈", Color::Yellow),
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

pub(crate) fn knowledge_color(k: &KnowledgeLevel) -> Color {
    match k {
        KnowledgeLevel::Detailed => Color::Green,
        KnowledgeLevel::NeighborScan => Color::Cyan,
        KnowledgeLevel::DistantScan => Color::Yellow,
        KnowledgeLevel::LongRangeEstimation => Color::Red,
        KnowledgeLevel::Unknown => Color::DarkGray,
    }
}

pub(crate) fn freshness_label(f: &DataFreshness) -> &'static str {
    match f {
        DataFreshness::Live => "live",
        DataFreshness::DegradedLive => "degraded live",
        DataFreshness::Historical => "historical",
        DataFreshness::Unavailable => "unavailable",
        DataFreshness::Unknown => "?",
    }
}

pub(crate) fn freshness_color(f: &DataFreshness) -> Color {
    match f {
        DataFreshness::Live => Color::Green,
        DataFreshness::DegradedLive => Color::Yellow,
        DataFreshness::Historical => Color::DarkGray,
        DataFreshness::Unavailable => Color::Red,
        DataFreshness::Unknown => Color::DarkGray,
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
        SectorObjectType::Unknown => ("?", Color::DarkGray),
    }
}

pub(crate) fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let (border_color, title_modifier) = if focused {
        (Color::Cyan, Modifier::BOLD)
    } else {
        (Color::DarkGray, Modifier::empty())
    };
    Block::default()
        .title(Span::styled(title, Style::default().add_modifier(title_modifier)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
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
        ProbeStatus::Unknown => "?",
    }
}

pub(crate) fn probe_status_style(s: &ProbeStatus) -> Style {
    match s {
        ProbeStatus::Idle | ProbeStatus::Orbiting => Style::default().fg(Color::White),
        ProbeStatus::Preparing | ProbeStatus::Decelerating => Style::default().fg(Color::Yellow),
        ProbeStatus::Accelerating => {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        }
        ProbeStatus::Cruising => Style::default().fg(Color::Cyan),
        ProbeStatus::Disabled => Style::default().fg(Color::Red),
        ProbeStatus::Dead => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ProbeStatus::Unknown => Style::default().fg(Color::DarkGray),
    }
}

pub(crate) fn sensor_dot(m: &SensorMode) -> &'static str {
    match m {
        SensorMode::Normal | SensorMode::Degraded | SensorMode::Blind | SensorMode::Unknown => "●",
    }
}


pub(crate) fn sensor_style(m: &SensorMode) -> Style {
    match m {
        SensorMode::Normal => Style::default().fg(Color::Green),
        SensorMode::Degraded => Style::default().fg(Color::Yellow),
        SensorMode::Blind => Style::default().fg(Color::Red),
        SensorMode::Unknown => Style::default().fg(Color::DarkGray),
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

pub(crate) fn make_line_gauge(label: &str, ratio: f64, color: Color) -> LineGauge<'_> {
    LineGauge::default()
        .label(Line::raw(label.to_owned()))
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray))
        .ratio(ratio)
}

pub(crate) fn gauge_color(ratio: f64) -> Color {
    if ratio > 0.5 {
        Color::Green
    } else if ratio > 0.25 {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Compact human age: "just now", "5m ago", "3h ago", "2d ago".
pub fn format_age(secs: i64) -> String {
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
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
