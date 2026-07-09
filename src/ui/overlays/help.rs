use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{render_footer, FooterKey};

// ── Help overlay ──────────────────────────────────────────────────────────────
//
// Content is static data (title + (key, desc) rows) so the row count is known
// without a palette — the input layer uses it to clamp scrolling. The overlay
// is near-fullscreen and scrolls vertically, so nothing is ever hidden however
// short the terminal.

/// One key/description row. An empty `key` continues the previous row's desc.
type Row = (&'static str, &'static str);
/// A titled group of rows.
type Section = (&'static str, &'static [Row]);

const LEFT: &[Section] = &[
    (
        "Navigate",
        &[
            ("e r t", "Scanner · Map · Comms"),
            ("d f g", "Sector · Probe · Missions"),
            ("c v b", "Inventory · Storage · Mannies"),
            ("j k / ↑↓", "move cursor in pane"),
            ("l / h", "drill in / out (→ ←)"),
            ("Tab", "cycle panes (Shift-Tab reverse)"),
            ("z", "zoom active pane full screen"),
            ("Esc", "close / leave zoom / drill up"),
        ],
    ),
    (
        "Act & global",
        &[
            ("Enter", "contextual action menu"),
            (":", "command line (see right)"),
            ("F1", "toggle hints line"),
            ("F2", "cycle color mode"),
            ("F5", "refresh"),
            ("?", "this help"),
            ("q", "quit"),
        ],
    ),
    (
        "In a menu",
        &[
            ("1-9", "fire the nth item"),
            ("j k", "move"),
            ("Enter", "fire selected"),
            ("Esc", "close"),
        ],
    ),
];

const RIGHT: &[Section] = &[
    (
        "Actions per pane (Enter)",
        &[
            ("Mannies", "mine, fabricate, repair, salvage,"),
            ("", "inspect, recover, detach, refuel,"),
            ("", "drop cargo, recall/abandon, rename"),
            ("Inventory", "fabricate, jettison, move stock"),
            ("Missions", "browse steps, abandon"),
            ("Comms", "categories: messages, alerts, warnings"),
            ("Storage", "rename, rules, recover, detach, move"),
            ("Sector", "object actions: mine, inspect,"),
            ("", "salvage, recover, deploy, relay"),
        ],
    ),
    (
        "Command mode  ( : )",
        &[
            ("Tab", "complete / cycle verb + argument"),
            ("↑ ↓", "browse command history"),
            (":focus", "<pane> — zoom that pane"),
            (":travel", "<x y z | +dx dy dz>"),
            (":goto", "<x y z> — center the map"),
            (":filter", "<all|objects|minable|danger>"),
            (":probe", "<id|name> — pilot a fleet probe"),
            (":craft", "open the fabrication catalog"),
            (":theme", "<mono-green|mono-amber|…>"),
            (":refresh", "reload all data"),
            (":zoom", "toggle zoom"),
            (":help  :q", "this help · quit"),
        ],
    ),
];

fn key_line(key: &str, desc: &str, p: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<10}"), Style::default().fg(p.accent)),
        Span::raw(desc.to_string()),
    ])
}

/// Render a column of sections to styled lines (title, its rows, a blank gap).
fn column_lines(sections: &[Section], p: Palette) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, (title, rows)) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(Line::default());
        }
        lines.push(Line::from(Span::styled(
            title.to_string(),
            Style::default().fg(p.warn).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in *rows {
            lines.push(key_line(key, desc, p));
        }
    }
    lines
}

/// Rendered line count of a column (palette-independent): one title + its rows
/// per section, plus a blank line between sections.
fn column_len(sections: &[Section]) -> usize {
    let rows: usize = sections.iter().map(|(_, r)| 1 + r.len()).sum();
    rows + sections.len().saturating_sub(1)
}

/// Tallest column — the scrollable content height, used by the input layer to
/// clamp the scroll offset.
pub(crate) fn help_row_count() -> usize {
    column_len(LEFT).max(column_len(RIGHT))
}

pub(crate) fn render_help_overlay(frame: &mut Frame, area: Rect, p: Palette, scroll: u16) {
    // Near-fullscreen: leave a thin margin so the grid peeks around it.
    let w = area.width.saturating_sub(4).clamp(20, 96);
    let h = area.height.saturating_sub(2).max(6);
    let popup = super::centered_rect(w, h, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" HELP — KEYBINDINGS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    // Clamp the applied scroll so it never runs into blank space.
    let body_h = rows[0].height;
    let max_scroll = (help_row_count() as u16).saturating_sub(body_h);
    let off = scroll.min(max_scroll);

    frame.render_widget(Paragraph::new(column_lines(LEFT, p)).scroll((off, 0)), cols[0]);
    frame.render_widget(Paragraph::new(column_lines(RIGHT, p)).scroll((off, 0)), cols[1]);

    // Only advertise scrolling when there is something below the fold.
    if max_scroll > 0 {
        render_footer(
            frame,
            rows[1],
            p,
            &[
                FooterKey::nav("[↑/↓]", "scroll"),
                FooterKey::nav("[g/G]", "top/end"),
                FooterKey::nav("[Esc/?]", "close"),
            ],
        );
    } else {
        render_footer(frame, rows[1], p, &[FooterKey::nav("[Esc/?]", "close")]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ColorMode;
    use crate::ui::theme::palette;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn row_count_matches_rendered_columns() {
        // Keeps the scroll clamp honest: column_len must track column_lines.
        let p = palette(ColorMode::MonoGreen);
        let expected = column_lines(LEFT, p).len().max(column_lines(RIGHT, p).len());
        assert_eq!(help_row_count(), expected);
    }

    #[test]
    fn renders_all_sections_including_command_mode() {
        let p = palette(ColorMode::MonoGreen);
        let mut t = Terminal::new(TestBackend::new(100, 40)).unwrap();
        t.draw(|f| {
            let a = f.area();
            render_help_overlay(f, a, p, 0);
        })
        .unwrap();
        let text: String =
            t.backend().buffer().content.iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Navigate"), "navigation section shown");
        assert!(text.contains("Command mode"), "command-mode section shown");
        assert!(text.contains(":travel"), "command verbs documented");
    }
}
