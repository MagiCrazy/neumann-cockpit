use super::*;

/// Command verbs recognised by `:` mode. Kept as a table so the input layer can
/// offer Tab-completion and a `:help`-style listing.
pub const COMMANDS: [&str; 11] = [
    "focus", "travel", "goto", "filter", "refresh", "theme", "zoom", "craft", "probe", "help",
    "quit",
];

/// One-line argument usage for a verb, shown as inline ghost-text while typing
/// (`None` for verbs that take no argument).
pub fn command_usage(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "focus" => "<pane>",
        "travel" => "<x y z | +dx dy dz>",
        "goto" => "<x y z>",
        "filter" => "<all|objects|minable|danger>",
        "theme" => "<mono-green|mono-amber|phosphor-semantic|modern-16>",
        "probe" => "<id|name>",
        _ => return None,
    })
}

fn pane_from_name(name: &str) -> Option<Pane> {
    Pane::ALL.into_iter().find(|p| p.label().eq_ignore_ascii_case(name))
}

/// Resolve a `:probe` argument to a fleet probe id: an exact id, then a
/// case-insensitive exact name, then a case-insensitive substring match.
fn fleet_probe_id(state: &AppState, args: &[&str]) -> Option<u64> {
    let first = args.first()?;
    if let Ok(id) = first.parse::<u64>() {
        if state.fleet.iter().any(|p| p.id == id) {
            return Some(id);
        }
    }
    let q = args.join(" ");
    if let Some(p) = state.fleet.iter().find(|p| p.name.eq_ignore_ascii_case(&q)) {
        return Some(p.id);
    }
    let ql = q.to_lowercase();
    state.fleet.iter().find(|p| p.name.to_lowercase().contains(&ql)).map(|p| p.id)
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
    /// Enumerable argument values for a verb, used by Tab-completion. Empty for
    /// verbs whose argument is free-form (coordinates) or absent.
    fn arg_candidates(&self, verb: &str) -> Vec<String> {
        match verb {
            "focus" => Pane::ALL.iter().map(|p| p.label().to_lowercase()).collect(),
            "filter" => ["all", "objects", "minable", "danger"].iter().map(|s| s.to_string()).collect(),
            "theme" => [
                ColorMode::MonoGreen,
                ColorMode::MonoAmber,
                ColorMode::PhosphorSemantic,
                ColorMode::Modern16,
            ]
            .iter()
            .map(|m| m.label().to_string())
            .collect(),
            "probe" => self.fleet.iter().map(|p| p.name.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// Compute Tab-completion candidates for the token under the caret. Returns
    /// `(token_start_byte, candidates)` where `token_start_byte` is the byte
    /// offset in `input` where the completed token begins, and candidates match
    /// the stem case-insensitively (an empty stem yields every candidate for the
    /// slot). Returns `None` when nothing is completable here.
    pub fn command_completions(&self, input: &str, cursor: usize) -> Option<(usize, Vec<String>)> {
        // Caret byte offset (`cursor` is a char index).
        let cbyte = input.char_indices().nth(cursor).map_or(input.len(), |(b, _)| b);
        let head = &input[..cbyte];
        let lead = head.len() - head.trim_start().len();

        match head[lead..].find(char::is_whitespace) {
            // Still on the first token → complete the verb.
            None => {
                let stem = head[lead..].to_lowercase();
                let cands: Vec<String> =
                    COMMANDS.iter().filter(|c| c.starts_with(&stem)).map(|c| c.to_string()).collect();
                (!cands.is_empty()).then_some((lead, cands))
            }
            // Verb typed → complete its (single) argument. The token spans the
            // whole arg region so names containing spaces still match.
            Some(verb_len) => {
                let verb = &head[lead..lead + verb_len];
                let all = self.arg_candidates(verb);
                if all.is_empty() {
                    return None;
                }
                let after_verb = lead + verb_len;
                let region = &head[after_verb..];
                let ts = after_verb + (region.len() - region.trim_start().len());
                let stem = input[ts..cbyte].to_lowercase();
                let cands: Vec<String> =
                    all.into_iter().filter(|c| c.to_lowercase().starts_with(&stem)).collect();
                (!cands.is_empty()).then_some((ts, cands))
            }
        }
    }

    /// Parse and execute a `:` command line. Returns `true` when the caller
    /// should trigger a full data refresh (`fetch_all`) — the one effect this
    /// method can't perform itself. Unknown or malformed commands set a toast.
    pub fn run_command(&mut self, line: &str) -> bool {
        let line = line.trim();
        // Record in history (dedup consecutive repeats) for ↑/↓ recall.
        if !line.is_empty() && self.command_history.last().map(String::as_str) != Some(line) {
            self.command_history.push(line.to_string());
        }
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
            "probe" => match fleet_probe_id(self, &args) {
                // Only sets the active probe; the event loop reconciles the
                // ApiClient and refetches (so no `return true` here).
                Some(id) => {
                    if let Some(p) = self.fleet.iter().find(|p| p.id == id) {
                        let (name, reachable) = (p.name.clone(), p.is_reachable);
                        if !reachable {
                            self.set_toast(format!("{name} is out of SCUT range — cannot pilot"));
                        } else if self.set_active_probe(id) {
                            self.set_toast(format!("piloting {name}"));
                        }
                    }
                }
                None => self.set_toast("usage: probe <id|name>"),
            },
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
