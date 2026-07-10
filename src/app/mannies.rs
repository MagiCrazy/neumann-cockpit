use super::*;
use crate::api::types::{Manny, MannyTaskVisibility, SectorObject, SectorObjectType};

impl AppState {
    pub fn manny_next(&mut self) {
        if let Some(mannies) = &self.mannies {
            if !mannies.is_empty() {
                self.mannies_selection = (self.mannies_selection + 1) % mannies.len();
            }
        }
    }

    pub fn manny_prev(&mut self) {
        if let Some(mannies) = &self.mannies {
            if !mannies.is_empty() {
                self.mannies_selection = self.mannies_selection.checked_sub(1).unwrap_or(mannies.len() - 1);
            }
        }
    }

    pub fn repair_max_percent(&self) -> f64 {
        self.probe
            .as_ref()
            .and_then(|p| p.systems.as_ref())
            .and_then(|s| s.integrity_percent)
            .map(|i| (100.0_f64 - i).max(0.0))
            .unwrap_or(0.0)
    }

    pub fn repair_metals_stock(&self) -> f64 {
        self.probe
            .as_ref()
            .map(|p| {
                p.inventory
                    .resource_stocks
                    .iter()
                    .find(|s| s.stock_type == "metals")
                    .map(|s| s.amount)
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0)
    }

    pub fn repair_type_char(&mut self, c: char) {
        if let ActiveWizard::Repair(RepairInput::Typing { buf, error, .. }) = &mut self.active_wizard {
            if c.is_ascii_digit() || (c == '.' && !buf.contains('.')) {
                buf.push(c);
                *error = None;
            }
        }
    }

    pub fn repair_backspace(&mut self) {
        if let ActiveWizard::Repair(RepairInput::Typing { buf, .. }) = &mut self.active_wizard {
            buf.pop();
        }
    }

    pub fn repair_fill_max(&mut self) {
        let max = self.repair_max_percent();
        if let ActiveWizard::Repair(RepairInput::Typing { buf, error, .. }) = &mut self.active_wizard {
            *buf = format!("{:.2}", max);
            *error = None;
        }
    }

    pub fn set_repair_error(&mut self, msg: String) {
        if let ActiveWizard::Repair(RepairInput::Typing { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_mine_error(&mut self, msg: String) {
        if let ActiveWizard::Mine(MineInput::Configure { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    /// Surface a craft failure on whichever fabrication step is active.
    pub fn set_fabrication_error(&mut self, msg: String) {
        if let ActiveWizard::Fabrication(fab) = &mut self.active_wizard {
            match fab {
                FabricationInput::PickRecipe { error, .. } => *error = Some(msg),
                FabricationInput::PickBuilder { error, .. } => *error = Some(msg),
            }
        }
    }

    /// Surface a probe-improvement failure on whichever step is active.
    pub fn set_improve_error(&mut self, msg: String) {
        if let ActiveWizard::Improve(improve) = &mut self.active_wizard {
            match improve {
                ImproveInput::PickImprovement { error, .. } => *error = Some(msg),
                ImproveInput::PickBuilder { error, .. } => *error = Some(msg),
            }
        }
    }

    pub fn set_salvage_error(&mut self, msg: String) {
        if let ActiveWizard::Salvage(SalvageInput::Confirm { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_recall_error(&mut self, msg: String) {
        if let ActiveWizard::Recall(RecallInput::Confirm { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_refuel_error(&mut self, msg: String) {
        if let ActiveWizard::Refuel(RefuelInput::Confirm { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    /// Fleet probes eligible as a deuterium-transfer destination: every probe
    /// except the one currently piloted (the source). The same-sector
    /// constraint is enforced server-side, since the roster carries no
    /// coordinates (API v86).
    pub fn transfer_deuterium_targets(&self) -> Vec<(u64, String)> {
        let source = self.active_probe_id.or(self.default_probe_id);
        self.fleet
            .iter()
            .filter(|p| Some(p.id) != source)
            .map(|p| (p.id, p.name.clone()))
            .collect()
    }

    pub fn set_transfer_deuterium_error(&mut self, msg: String) {
        if let ActiveWizard::TransferDeuterium(TransferDeuteriumInput::EnterAmount { error, .. }) =
            &mut self.active_wizard
        {
            *error = Some(msg);
        }
    }

    pub fn set_mind_snapshot_error(&mut self, msg: String) {
        if let ActiveWizard::MindSnapshot(MindSnapshotInput::Confirm { error }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_scut_relay_error(&mut self, msg: String) {
        if let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_mission_abandon_error(&mut self, msg: String) {
        if let ActiveWizard::Missions(MissionsInput::ConfirmAbandon { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_message_send_error(&mut self, msg: String) {
        if let ActiveWizard::Messages(MessagesInput::Compose { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn unread_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.status == crate::api::types::MessageStatus::Unread)
            .count()
    }

    /// Message recipients reachable from the current sector: detected probes
    /// plus inhabited planets. Returns (kind, endpoint id, display name).
    pub fn collect_message_recipients(&self) -> Vec<(String, crate::api::types::EndpointId, String)> {
        use crate::api::types::EndpointId;
        let mut out = Vec::new();
        let Some(sector) = self.probe_current_sector_scan() else {
            return out;
        };
        if let Some(probes) = sector.probes.as_ref() {
            for p in probes {
                out.push(("probe".to_string(), EndpointId::Probe(p.id), p.name.clone()));
            }
        }
        if let Some(objects) = sector.objects.as_ref() {
            for o in objects {
                if o.habitability_score.unwrap_or(0.0) > 0.0 {
                    if let Some(id) = o.id.clone() {
                        let name = o.name.clone().unwrap_or_else(|| "planet".into());
                        out.push(("planet".to_string(), EndpointId::Planet(id), name));
                    }
                }
            }
        }
        out
    }

    /// The probe's terminal recovery alert (dead / black-hole), if any.
    pub fn probe_terminal_alert(&self) -> Option<&crate::api::types::ProbeTerminalAlert> {
        self.probe.as_ref().and_then(|p| p.alert.as_ref())
    }

    /// True when the probe's current sector exposes a deuterium refuel station.
    pub fn deuterium_station_in_current_sector(&self) -> bool {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .is_some_and(|objects| {
                objects
                    .iter()
                    .any(|o| matches!(o.object_type, SectorObjectType::DeuteriumRefuelStation))
            })
    }

    pub fn collect_mineable_candidates(&self) -> Vec<(String, String)> {
        let Some(sector) = self.probe_current_sector_scan() else {
            return Vec::new();
        };
        let Some(objects) = sector.objects.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<(String, String)> = Vec::new();
        for o in objects {
            // Asteroids nested under a parent object (e.g. a solar system).
            for t in o.minable_targets.iter().flatten() {
                if matches!(t.object_type, SectorObjectType::Asteroid) && !out.iter().any(|(id, _)| id == &t.id) {
                    out.push((t.id.clone(), t.name.clone().unwrap_or_else(|| "unnamed".into())));
                }
            }
            // Standalone top-level asteroids (e.g. a wandering asteroid) carry no
            // parent minableTargets, so they must be collected directly or they
            // never reach the mine picker.
            if matches!(o.object_type, SectorObjectType::Asteroid) {
                if let Some(id) = &o.id {
                    if !out.iter().any(|(i, _)| i == id) {
                        out.push((id.clone(), o.name.clone().unwrap_or_else(|| "unnamed".into())));
                    }
                }
            }
        }
        out
    }

    pub fn collect_asteroid_candidates(&self) -> Vec<(String, String)> {
        match self.probe_current_sector_scan() {
            Some(s) => self.collect_asteroid_candidates_in(s),
            None => Vec::new(),
        }
    }

    /// Objects a Manny can inspect in the probe's current sector (API v65):
    /// asteroids plus top-level dormant constructs and detached containers.
    pub fn collect_inspectable_candidates(&self) -> Vec<(String, String)> {
        let Some(sector) = self.probe_current_sector_scan() else {
            return Vec::new();
        };
        let mut out = self.collect_asteroid_candidates_in(sector);
        if let Some(objects) = sector.objects.as_ref() {
            for o in objects {
                let inspectable = matches!(
                    o.object_type,
                    SectorObjectType::DormantConstruct | SectorObjectType::DetachedContainer
                );
                if !inspectable {
                    continue;
                }
                if let Some(id) = &o.id {
                    if out.iter().all(|(existing, _)| existing != id) {
                        out.push((id.clone(), o.name.clone().unwrap_or_else(|| "unnamed".into())));
                    }
                }
            }
        }
        out
    }

    pub fn collect_asteroid_candidates_in(
        &self,
        sector: &crate::api::types::SectorObservation,
    ) -> Vec<(String, String)> {
        Some(sector)
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects
                    .iter()
                    .flat_map(|o| {
                        let direct = if matches!(o.object_type, SectorObjectType::Asteroid) {
                            o.id.as_ref()
                                .map(|id| vec![(id.clone(), o.name.clone().unwrap_or_else(|| "unnamed".into()))])
                                .unwrap_or_default()
                        } else {
                            vec![]
                        };
                        let nested: Vec<(String, String)> = o
                            .bookmark_targets
                            .iter()
                            .filter(|t| matches!(t.object_type, SectorObjectType::Asteroid))
                            .map(|t| (t.id.clone(), t.name.clone().unwrap_or_else(|| "unnamed".into())))
                            .collect();
                        [direct, nested].concat()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_salvage_candidates(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects
                    .iter()
                    .filter(|o| matches!(o.object_type, SectorObjectType::Manny))
                    .map(|o| {
                        let id = o.id.clone().unwrap_or_default();
                        let name = o.name.clone().unwrap_or_else(|| "unknown manny".into());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_deploy_error(&mut self, msg: String) {
        if let ActiveWizard::Deploy(DeployInput::EnterName { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn set_rename_manny_error(&mut self, msg: String) {
        if let ActiveWizard::RenameManny(RenameMannyInput::Typing { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        }
    }

    pub fn rename_manny_type_char(&mut self, c: char) {
        if let ActiveWizard::RenameManny(RenameMannyInput::Typing { buf, .. }) = &mut self.active_wizard {
            if buf.len() < 40 {
                buf.push(c);
            }
        }
    }

    pub fn rename_manny_backspace(&mut self) {
        if let ActiveWizard::RenameManny(RenameMannyInput::Typing { buf, .. }) = &mut self.active_wizard {
            buf.pop();
        }
    }

    pub fn deploy_type_char(&mut self, c: char) {
        if let ActiveWizard::Deploy(DeployInput::EnterName { name_buf, .. }) = &mut self.active_wizard {
            if name_buf.len() < 80 {
                name_buf.push(c);
            }
        }
    }

    pub fn deploy_backspace(&mut self) {
        if let ActiveWizard::Deploy(DeployInput::EnterName { name_buf, .. }) = &mut self.active_wizard {
            name_buf.pop();
        }
    }

    pub fn collect_idle_onboard_mannies(&self) -> Vec<(String, String)> {
        self.mannies
            .as_ref()
            .map(|ms| {
                ms.iter()
                    .filter(|m| {
                        m.location.location_type == crate::api::types::MannyLocationType::Probe && m.can_receive_orders
                    })
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Indices (into `mannies`) of idle Mannies — onboard and free to take an
    /// order. Backs the status-bar "N idle" indicator and the idle-cycling key.
    fn idle_manny_indices(&self) -> Vec<usize> {
        self.mannies.as_ref().map_or(Vec::new(), |ms| {
            ms.iter()
                .enumerate()
                .filter(|(_, m)| {
                    m.location.location_type == crate::api::types::MannyLocationType::Probe && m.can_receive_orders
                })
                .map(|(i, _)| i)
                .collect()
        })
    }

    /// How many Mannies are idle (onboard, awaiting orders).
    pub fn idle_manny_count(&self) -> usize {
        self.idle_manny_indices().len()
    }

    /// Focus the Mannies pane and move its cursor to the next idle Manny after
    /// the current selection, wrapping around. No-op when none are idle.
    pub fn cycle_to_next_idle_manny(&mut self) {
        let idle = self.idle_manny_indices();
        let Some(&first) = idle.first() else {
            self.set_toast("no idle mannies");
            return;
        };
        self.active_pane = crate::app::Pane::Mannies;
        let cur = self.mannies_selection;
        self.mannies_selection = idle.iter().copied().find(|&i| i > cur).unwrap_or(first);
    }

    pub fn collect_deploy_candidates(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects
                    .iter()
                    .filter(|o| o.id.is_some())
                    .map(|o: &SectorObject| {
                        let id = o.id.clone().unwrap();
                        let name = o
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("{:?}", o.object_type).to_lowercase());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_inspect_error(&mut self, msg: String) {
        if let ActiveWizard::Inspect(InspectInput::PickTarget { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        } else {
            // Inspect was dispatched without the picker overlay (single
            // candidate or object-first flow) — surface in the status bar.
            self.error = Some(format!("inspect: {msg}"));
        }
    }

    pub fn set_recover_error(&mut self, msg: String) {
        if let ActiveWizard::Recover(RecoverInput::PickContainer { error, .. }) = &mut self.active_wizard {
            *error = Some(msg);
        } else {
            self.error = Some(format!("recover: {msg}"));
        }
    }

    pub fn set_detach_error(&mut self, msg: String) {
        if let ActiveWizard::Detach(detach) = &mut self.active_wizard {
            match detach {
                DetachInput::PickMode { error, .. } => *error = Some(msg),
                DetachInput::PickAsteroid { error, .. } => *error = Some(msg),
                _ => {}
            }
        }
    }

    pub fn collect_detachable_containers(&self) -> Vec<(String, String)> {
        self.probe
            .as_ref()
            .map(|p| {
                p.inventory
                    .containers
                    .iter()
                    .filter(|c| c.kind != "probe")
                    .map(|c| (c.id.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Empty additional containers, eligible as probe-assembly ingredients
    /// (API v81 `assemble-probe` consumes two of them). Excludes the probe's
    /// own container and any that still hold cargo.
    pub fn collect_empty_containers(&self) -> Vec<(String, String)> {
        self.probe
            .as_ref()
            .map(|p| {
                p.inventory
                    .containers
                    .iter()
                    .filter(|c| c.kind != "probe" && c.used_capacity == 0.0)
                    .map(|c| (c.id.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_detached_containers(&self) -> Vec<(String, String)> {
        match self.probe_current_sector_scan() {
            Some(s) => self.collect_detached_containers_in(s),
            None => Vec::new(),
        }
    }

    pub fn collect_detached_containers_in(
        &self,
        sector: &crate::api::types::SectorObservation,
    ) -> Vec<(String, String)> {
        Some(sector)
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects
                    .iter()
                    .filter(|o| matches!(o.object_type, SectorObjectType::DetachedContainer))
                    .map(|o| {
                        let id = o.id.clone().unwrap_or_default();
                        let name = o.name.clone().unwrap_or_else(|| "unnamed container".into());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// When the awaited remote sector arrives, advance the remote-mine wizard
    /// to asteroid selection (or abort with an error if it has no asteroids).
    pub fn remote_mine_sector_loaded(&mut self, x: i32, y: i32, z: i32) {
        let (manny_id, manny_name) = match &self.active_wizard {
            ActiveWizard::RemoteMine(RemoteMineInput::Loading {
                manny_id,
                manny_name,
                x: lx,
                y: ly,
                z: lz,
            }) if (*lx, *ly, *lz) == (x, y, z) => (manny_id.clone(), manny_name.clone()),
            _ => return,
        };
        let candidates = match self.sector_observation_at(x, y, z) {
            Some(s) => self.collect_asteroid_candidates_in(s),
            None => return,
        };
        if candidates.is_empty() {
            self.close_wizard();
            self.error = Some("no mineable asteroid in the Manny's sector".into());
            return;
        }
        self.active_wizard = ActiveWizard::RemoteMine(RemoteMineInput::PickAsteroid {
            manny_id,
            manny_name,
            x,
            y,
            z,
            candidates,
            selection: 0,
        });
    }

    pub fn set_remote_mine_error(&mut self, msg: String) {
        if let ActiveWizard::RemoteMine(rm) = &mut self.active_wizard {
            match rm {
                RemoteMineInput::Configure { error, .. } => *error = Some(msg),
                RemoteMineInput::PickContainer { error, .. } => *error = Some(msg),
                _ => {}
            }
        }
    }

    /// True when a Manny is idle and in a remote sector reachable via a shared
    /// SCUT network — eligible for remote mining (API v60).
    pub fn manny_remote_minable(&self, manny: &Manny) -> bool {
        manny.current_task.is_none()
            && matches!(manny.task_visibility, Some(MannyTaskVisibility::ScutNetwork))
            && self.manny_sector_coords(manny).is_some()
    }

    /// Relative sector coords of a Manny from its location payload.
    pub fn manny_sector_coords(&self, manny: &Manny) -> Option<(i32, i32, i32)> {
        let v = manny.location.sector.as_ref()?.get("relative")?;
        Some((
            v.get("x")?.as_f64()? as i32,
            v.get("y")?.as_f64()? as i32,
            v.get("z")?.as_f64()? as i32,
        ))
    }
}
