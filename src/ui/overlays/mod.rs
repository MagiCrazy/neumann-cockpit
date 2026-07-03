pub(crate) mod alerts;
pub(crate) mod containers;
pub(crate) mod craft;
pub(crate) mod drop_container;
pub(crate) mod help;
pub(crate) mod inventory_detail;
pub(crate) mod jettison;
pub(crate) mod map;
pub(crate) mod messages;
pub(crate) mod mine;
pub(crate) mod missions;
pub(crate) mod remote_mine;
pub(crate) mod object_actions;
pub(crate) mod scut_network;
pub(crate) mod pickers;
pub(crate) mod repair;
pub(crate) mod scanner;
pub(crate) mod storage_move;
pub(crate) mod travel;
pub(crate) mod waypoints;

pub(crate) use alerts::render_alerts_overlay;
pub(crate) use containers::{render_container_rules_overlay, render_rename_container_overlay};
pub(crate) use craft::{render_atomic_printer_craft_overlay, render_craft_overlay};
pub(crate) use drop_container::render_drop_container_overlay;
pub(crate) use help::render_help_overlay;
pub(crate) use inventory_detail::render_inventory_detail_overlay;
pub(crate) use jettison::render_jettison_overlay;
pub(crate) use map::{render_goto_visited_overlay, render_map_overlay};
pub(crate) use messages::render_messages_overlay;
pub(crate) use mine::render_mine_overlay;
pub(crate) use remote_mine::render_remote_mine_overlay;
pub(crate) use missions::render_missions_overlay;
pub(crate) use scut_network::render_scut_network_overlay;
pub(crate) use object_actions::render_object_action_overlay;
pub(crate) use pickers::{
    render_deploy_overlay, render_detach_overlay, render_drop_cargo_overlay, render_inspect_overlay,
    render_mind_snapshot_overlay, render_recall_overlay, render_recover_overlay,
    render_refuel_overlay, render_rename_manny_overlay, render_salvage_overlay,
    render_scut_relay_overlay,
};
pub(crate) use repair::render_repair_overlay;
pub(crate) use scanner::render_scan_input_overlay;
pub(crate) use storage_move::render_storage_move_overlay;
pub(crate) use travel::render_travel_overlay;
pub(crate) use waypoints::render_waypoints_overlay;

use crate::app::{AppState, GotoVisitedInput, ScanMode};
use crate::ui::theme::{palette, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// The wizard overlays, in z-order (back to front). Every one self-guards —
/// its render fn early-returns when its `*Input` field is `Inactive` — so they
/// can be painted unconditionally. Only one wizard is active at a time (the
/// input router routes keys to a single wizard and launch sites are mutually
/// exclusive), so the relative order only matters as documentation. Adding a
/// wizard overlay means adding one line here — the single source of truth for
/// the render side, replacing the hand-synchronized `if !matches!` ladder.
const WIZARD_OVERLAYS: &[fn(&mut Frame, Rect, &AppState)] = &[
    render_alerts_overlay,
    render_travel_overlay,
    render_repair_overlay,
    render_mine_overlay,
    render_remote_mine_overlay,
    render_jettison_overlay,
    render_craft_overlay,
    render_atomic_printer_craft_overlay,
    render_salvage_overlay,
    render_recall_overlay,
    render_refuel_overlay,
    render_mind_snapshot_overlay,
    render_missions_overlay,
    render_messages_overlay,
    render_scut_relay_overlay,
    render_scut_network_overlay,
    render_drop_cargo_overlay,
    render_deploy_overlay,
    render_rename_manny_overlay,
    render_inspect_overlay,
    render_recover_overlay,
    render_detach_overlay,
    render_drop_container_overlay,
    render_object_action_overlay,
    render_waypoints_overlay,
    render_rename_container_overlay,
    render_container_rules_overlay,
    render_storage_move_overlay,
];

/// Render whichever wizard overlays are active, on top of the cockpit grid.
pub(crate) fn render_active_overlays(frame: &mut Frame, area: Rect, state: &AppState) {
    for render in WIZARD_OVERLAYS {
        render(frame, area, state);
    }
    // Modals that don't follow the `*Input::Inactive` pattern (bool flags or a
    // distinct enum) stay explicit. `help` is topmost, so it renders last.
    if state.map.open {
        render_map_overlay(frame, area, state);
    }
    if matches!(state.goto_visited, GotoVisitedInput::Picking { .. }) {
        render_goto_visited_overlay(frame, area, state);
    }
    if state.inventory_detail_open {
        render_inventory_detail_overlay(frame, area, state);
    }
    if !matches!(state.scan_mode, ScanMode::Current) {
        render_scan_input_overlay(frame, area, state);
    }
    if state.help_open {
        render_help_overlay(frame, area, palette(state.color_mode));
    }
}

pub(crate) fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

/// Semantic weight of a footer key, driving its colour and emphasis.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyTone {
    /// Navigation / intermediate step (no game effect). Accent, not bold.
    Nav,
    /// Terminal commit with a game effect (POST/PATCH). Good + bold.
    Commit,
    /// Terminal commit that is destructive or irreversible. Crit + bold.
    Danger,
    /// A commit key that is currently unavailable (form invalid). Dim.
    Disabled,
}

/// One `[key] label` entry in a modal footer.
pub(crate) struct FooterKey<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub tone: KeyTone,
}

impl<'a> FooterKey<'a> {
    pub fn nav(key: &'a str, label: &'a str) -> Self {
        Self { key, label, tone: KeyTone::Nav }
    }
    pub fn commit(key: &'a str, label: &'a str) -> Self {
        Self { key, label, tone: KeyTone::Commit }
    }
    pub fn danger(key: &'a str, label: &'a str) -> Self {
        Self { key, label, tone: KeyTone::Danger }
    }
}

/// Render the unified modal footer into `area` (expected to be a single row).
///
/// Each key is drawn as a bracketed, coloured `[key]` followed by its raw
/// label, entries separated by two spaces. This is the single source of truth
/// for every modal's bottom hint line; `[Esc]` should always come last as a
/// `Nav` key.
pub(crate) fn render_footer(frame: &mut Frame, area: Rect, p: Palette, keys: &[FooterKey]) {
    let mut spans: Vec<Span> = Vec::with_capacity(keys.len() * 2);
    for (i, fk) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let key_style = match fk.tone {
            KeyTone::Nav => Style::default().fg(p.accent),
            KeyTone::Commit => Style::default().fg(p.good).add_modifier(Modifier::BOLD),
            KeyTone::Danger => Style::default().fg(p.crit).add_modifier(Modifier::BOLD),
            KeyTone::Disabled => Style::default().fg(p.dim),
        };
        spans.push(Span::styled(fk.key.to_owned(), key_style));
        spans.push(Span::raw(format!(" {}", fk.label)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_pick_list(
    frame: &mut Frame,
    area: Rect,
    p: Palette,
    title: &str,
    width: u16,
    height: u16,
    prompt: Option<&str>,
    items: &[&str],
    selection: usize,
    error: Option<&str>,
    action: &str,
) {
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(title.to_owned())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(prompt) = prompt {
        lines.push(Line::from(Span::styled(prompt.to_owned(), Style::default().fg(p.accent))));
        lines.push(Line::default());
    }
    for (i, name) in items.iter().enumerate() {
        if i == selection {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(p.accent)),
                Span::styled(name.to_string(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.to_string(), Style::default().fg(p.dim)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::nav("[↑/↓]", "select"),
            FooterKey::commit("[Enter]", action),
            FooterKey::nav("[Esc]", "cancel"),
        ],
    );
}

