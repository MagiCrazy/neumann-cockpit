use super::*;

/// Command verbs recognised by `:` mode. Kept as a table so the input layer can
/// offer Tab-completion and a `:help`-style listing.
pub const COMMANDS: [&str; 12] = [
    "focus", "travel", "goto", "filter", "refresh", "theme", "zoom", "craft", "mine", "probe",
    "help", "quit",
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
        "craft" => "[recipe]",
        "mine" => "[res[,res]] [amount] [by <manny>] [at <asteroid>] [to <container>]",
        _ => return None,
    })
}

/// An action a `:` command wants to spawn but cannot itself — `run_command` owns
/// no `ApiClient`/sender, so it stages the fire here and the input layer drains
/// it (`input/command.rs`) with the client + channel in hand.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandFire {
    /// `atomic-printer/craft` (the printer auto-reserves a Manny).
    AtomicCraft { recipe_id: String },
    /// A Manny recipe on a resolved builder.
    MannyCraft { manny_id: String, recipe_id: String },
    /// A local mine on the probe's current sector.
    Mine {
        manny_id: String,
        object_id: String,
        resources: Vec<String>,
        amount: f64,
        container_id: Option<String>,
    },
}

/// Map a `:mine` resource token to its API name. Accepts the short `carbon`
/// alias for `carbon_compounds`.
fn mine_resource(token: &str) -> Option<&'static str> {
    Some(match token.to_lowercase().as_str() {
        "deuterium" => "deuterium",
        "metals" => "metals",
        "ice" => "ice",
        "carbon" | "carbon_compounds" => "carbon_compounds",
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
            "craft" => self.fabrication_recipes().iter().map(|(_, r)| r.name.clone()).collect(),
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
                } else if args.is_empty() {
                    // Bare `:craft` opens the wizard (unchanged).
                    self.fabrication = FabricationInput::PickRecipe {
                        prefilled_manny: None,
                        selection: 0,
                        error: None,
                    };
                } else {
                    // `:craft <recipe>` fires directly.
                    self.craft_command(&args.join(" "));
                }
            }
            "mine" => {
                if args.is_empty() {
                    self.open_mine_wizard();
                } else {
                    self.mine_command(&args);
                }
            }
            "help" => self.help_open = true,
            "q" | "quit" => self.set_quit(),
            other => self.set_toast(format!("unknown command: {other}")),
        }
        false
    }

    /// Idle onboard Mannies matching a query: exact id, then case-insensitive
    /// exact name, then case-insensitive substring. `None` when unresolved.
    fn resolve_idle_manny(&self, query: &str) -> Option<(String, String)> {
        let mannies = self.collect_idle_onboard_mannies();
        if let Some(m) = mannies.iter().find(|(id, _)| id == query) {
            return Some(m.clone());
        }
        if let Some(m) = mannies.iter().find(|(_, name)| name.eq_ignore_ascii_case(query)) {
            return Some(m.clone());
        }
        let q = query.to_lowercase();
        mannies.iter().find(|(_, name)| name.to_lowercase().contains(&q)).cloned()
    }

    /// `:craft <recipe>` — resolve the recipe by name (case-insensitive exact,
    /// then substring) and stage the fire. A Manny recipe with several idle
    /// builders falls back to the wizard's builder picker.
    fn craft_command(&mut self, query: &str) {
        let recipes = self.fabrication_recipes();
        let resolved = recipes
            .iter()
            .find(|(_, r)| r.name.eq_ignore_ascii_case(query))
            .or_else(|| {
                let q = query.to_lowercase();
                recipes.iter().find(|(_, r)| r.name.to_lowercase().contains(&q))
            })
            .map(|(fab, r)| (*fab, r.id.clone(), r.name.clone()));

        let Some((fab, recipe_id, recipe_name)) = resolved else {
            self.set_toast(format!("no recipe matching \"{query}\""));
            return;
        };

        match fab {
            Fabricator::AtomicPrinter => {
                if self.has_atomic_printer() {
                    self.pending_fire = Some(CommandFire::AtomicCraft { recipe_id });
                } else {
                    self.set_toast("no atomic printer in inventory");
                }
            }
            Fabricator::Manny => {
                let mannies = self.collect_idle_onboard_mannies();
                match mannies.len() {
                    0 => self.set_toast("no idle Manny on board"),
                    1 => {
                        let (manny_id, _) = mannies.into_iter().next().unwrap();
                        self.pending_fire = Some(CommandFire::MannyCraft { manny_id, recipe_id });
                    }
                    // Ambiguous builder → let the pilot pick in the wizard.
                    _ => {
                        self.fabrication = FabricationInput::PickBuilder {
                            recipe_id,
                            recipe_name,
                            mannies,
                            selection: 0,
                            error: None,
                        };
                    }
                }
            }
        }
    }

    /// Bare `:mine` — resolve the builder from context (the sole idle onboard
    /// Manny) and open the mine wizard, mirroring the Mannies-pane launcher.
    fn open_mine_wizard(&mut self) {
        let mannies = self.collect_idle_onboard_mannies();
        let (manny_id, manny_name) = match mannies.len() {
            0 => {
                self.set_toast("no idle Manny on board");
                return;
            }
            1 => mannies.into_iter().next().unwrap(),
            _ => {
                self.set_toast("multiple idle Mannies — use :mine by <manny>");
                return;
            }
        };
        let candidates = self.collect_mineable_candidates();
        match candidates.len() {
            0 => self.set_toast("no mineable objects in current sector — scan first"),
            1 => {
                let (object_id, object_name) = candidates.into_iter().next().unwrap();
                self.mine = MineInput::Configure {
                    manny_id,
                    manny_name,
                    object_id,
                    object_name,
                    resources: [false, true, false, false],
                    amount_buf: "0.30".into(),
                    amount_mode: false,
                    target_container: None,
                    error: None,
                };
            }
            _ => {
                self.mine = MineInput::PickAsteroid { manny_id, manny_name, candidates, selection: 0 };
            }
        }
    }

    /// `:mine [res[,res]] [amount] [by <manny>] [at <asteroid>] [to <container>]`
    /// — a local mine fired directly. Missing manny/asteroid default to the sole
    /// context candidate; `to` defaults to the probe.
    fn mine_command(&mut self, args: &[&str]) {
        // Split into the positional head (resources + amount) and the by/at/to
        // keyword buckets. Keyword values run to the next keyword, so names may
        // contain spaces.
        let (mut positional, mut by, mut at, mut to) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut bucket = 0u8; // 0 positional · 1 by · 2 at · 3 to
        for &tok in args {
            match tok {
                "by" => bucket = 1,
                "at" => bucket = 2,
                "to" => bucket = 3,
                _ => match bucket {
                    1 => by.push(tok),
                    2 => at.push(tok),
                    3 => to.push(tok),
                    _ => positional.push(tok),
                },
            }
        }

        // Positional tokens: resource list (comma-separated) and/or amount.
        let mut resources: Vec<String> = Vec::new();
        let mut amount: Option<f64> = None;
        for tok in positional {
            if let Ok(n) = tok.parse::<f64>() {
                amount = Some(n);
                continue;
            }
            for r in tok.split(',').filter(|s| !s.is_empty()) {
                match mine_resource(r) {
                    Some(name) => {
                        let name = name.to_string();
                        if !resources.contains(&name) {
                            resources.push(name);
                        }
                    }
                    None => {
                        self.set_toast(format!("unknown resource \"{r}\""));
                        return;
                    }
                }
            }
        }
        if resources.is_empty() {
            resources.push("metals".into());
        }
        let amount = amount.unwrap_or(0.30);
        if amount <= 0.0 {
            self.set_toast("amount must be positive");
            return;
        }

        // Builder: `by` override, else the sole idle onboard Manny.
        let (manny_id, _manny_name) = if by.is_empty() {
            let mannies = self.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => {
                    self.set_toast("no idle Manny on board");
                    return;
                }
                1 => mannies.into_iter().next().unwrap(),
                _ => {
                    self.set_toast("multiple idle Mannies — add by <manny>");
                    return;
                }
            }
        } else {
            match self.resolve_idle_manny(&by.join(" ")) {
                Some(m) => m,
                None => {
                    self.set_toast(format!("no idle Manny matching \"{}\"", by.join(" ")));
                    return;
                }
            }
        };

        // Asteroid: `at` override, else the sole mineable object in the sector.
        let candidates = self.collect_mineable_candidates();
        let (object_id, _object_name) = if at.is_empty() {
            match candidates.len() {
                0 => {
                    self.set_toast("no mineable objects in current sector — scan first");
                    return;
                }
                1 => candidates.into_iter().next().unwrap(),
                _ => {
                    self.set_toast("multiple asteroids — add at <asteroid>");
                    return;
                }
            }
        } else {
            let q = at.join(" ").to_lowercase();
            match candidates.iter().find(|(_, n)| n.to_lowercase().contains(&q)).cloned() {
                Some(o) => o,
                None => {
                    self.set_toast(format!("no asteroid matching \"{}\"", at.join(" ")));
                    return;
                }
            }
        };

        // Destination: `to` a detached container (or the literal `probe`), else
        // the probe by default.
        let container_id = if to.is_empty() {
            None
        } else {
            let target = to.join(" ");
            if target.eq_ignore_ascii_case("probe") {
                None
            } else {
                let q = target.to_lowercase();
                match self
                    .collect_detached_containers()
                    .into_iter()
                    .find(|(_, n)| n.to_lowercase().contains(&q))
                {
                    Some((id, _)) => Some(id),
                    None => {
                        self.set_toast(format!("no container matching \"{target}\""));
                        return;
                    }
                }
            }
        };

        self.pending_fire = Some(CommandFire::Mine {
            manny_id,
            object_id,
            resources,
            amount,
            container_id,
        });
    }
}
