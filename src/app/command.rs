use super::*;

/// Command verbs recognised by `:` mode. Kept as a table so the input layer can
/// offer Tab-completion and a `:help`-style listing.
pub const COMMANDS: [&str; 10] =
    ["focus", "travel", "goto", "filter", "refresh", "theme", "zoom", "craft", "help", "quit"];

fn pane_from_name(name: &str) -> Option<Pane> {
    Pane::ALL.into_iter().find(|p| p.label().eq_ignore_ascii_case(name))
}

fn color_from_name(name: &str) -> Option<ColorMode> {
    [
        ColorMode::MonoGreen,
        ColorMode::MonoAmber,
        ColorMode::PhosphorSemantic,
        ColorMode::Modern16,
    ]
    .into_iter()
    .find(|m| m.label() == name)
}

fn filter_from_name(name: &str) -> Option<ScanFilter> {
    match name {
        "all" => Some(ScanFilter::All),
        "objects" => Some(ScanFilter::Objects),
        "minable" => Some(ScanFilter::Minable),
        "danger" => Some(ScanFilter::Danger),
        _ => None,
    }
}

/// Parse `x y z` (also accepting commas or a leading `+` for relative) into a
/// coordinate triple, joined from whatever arg tokens were given.
fn parse_coords(args: &[&str]) -> Option<(i32, i32, i32)> {
    let joined = args.join(" ").replace(',', " ");
    let parts: Vec<i32> = joined.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    match parts.as_slice() {
        [x, y, z] => Some((*x, *y, *z)),
        _ => None,
    }
}

impl AppState {
    /// Parse and execute a `:` command line. Returns `true` when the caller
    /// should trigger a full data refresh (`fetch_all`) — the one effect this
    /// method can't perform itself. Unknown or malformed commands set a toast.
    pub fn run_command(&mut self, line: &str) -> bool {
        let line = line.trim();
        let mut parts = line.split_whitespace();
        let Some(verb) = parts.next() else { return false };
        let args: Vec<&str> = parts.collect();

        match verb {
            "focus" => match args.first().and_then(|n| pane_from_name(n)) {
                Some(pane) => {
                    self.active_pane = pane;
                    self.zoomed = true;
                }
                None => self.set_toast("usage: focus <pane>"),
            },
            "travel" => {
                // Accept "x y z" or "+dx dy dz" (commas tolerated).
                let buf = args.join(" ").replace(',', " ");
                self.travel = TravelInput::Typing(buf);
                self.travel_submit();
            }
            "goto" => match parse_coords(&args) {
                Some((x, y, z)) => {
                    self.open_map();
                    self.map.center_x = x;
                    self.map.y_layer = y;
                    self.map.center_z = z;
                }
                None => self.set_toast("usage: goto <x y z>"),
            },
            "filter" => match args.first().and_then(|n| filter_from_name(n)) {
                Some(f) => self.set_scan_filter(f),
                None => self.set_toast("usage: filter <all|objects|minable|danger>"),
            },
            "refresh" => return true,
            "theme" => match args.first().and_then(|n| color_from_name(n)) {
                Some(m) => {
                    self.color_mode = m;
                    self.set_toast(format!("color mode: {}", m.label()));
                }
                None => self.set_toast("usage: theme <mono-green|mono-amber|phosphor-semantic|modern-16>"),
            },
            "zoom" => self.toggle_zoom(),
            "craft" => {
                if self.recipes.is_empty() {
                    self.set_toast("recipes loading — F5 to refresh");
                } else {
                    self.fabrication = FabricationInput::PickRecipe {
                        prefilled_manny: None,
                        selection: 0,
                        error: None,
                    };
                }
            }
            "help" => self.help_open = true,
            "q" | "quit" => self.set_quit(),
            other => self.set_toast(format!("unknown command: {other}")),
        }
        false
    }
}
