use crate::api::types::{Manny, MannyTaskVisibility, SectorObject, SectorObjectType};
use super::*;

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
                self.mannies_selection = self
                    .mannies_selection
                    .checked_sub(1)
                    .unwrap_or(mannies.len() - 1);
            }
        }
    }

    pub fn repair_max_percent(&self) -> f64 {
        self.probe.as_ref()
            .and_then(|p| p.systems.as_ref())
            .and_then(|s| s.integrity_percent)
            .map(|i| (100.0_f64 - i).max(0.0))
            .unwrap_or(0.0)
    }

    pub fn repair_metals_stock(&self) -> f64 {
        self.probe.as_ref()
            .map(|p| {
                p.inventory.resource_stocks.iter()
                    .find(|s| s.stock_type == "metals")
                    .map(|s| s.amount)
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0)
    }

    pub fn repair_type_char(&mut self, c: char) {
        if let RepairInput::Typing { ref mut buf, ref mut error, .. } = self.repair {
            if c.is_ascii_digit() || (c == '.' && !buf.contains('.')) {
                buf.push(c);
                *error = None;
            }
        }
    }

    pub fn repair_backspace(&mut self) {
        if let RepairInput::Typing { ref mut buf, .. } = self.repair {
            buf.pop();
        }
    }

    pub fn repair_fill_max(&mut self) {
        let max = self.repair_max_percent();
        if let RepairInput::Typing { ref mut buf, ref mut error, .. } = self.repair {
            *buf = format!("{:.2}", max);
            *error = None;
        }
    }

    pub fn set_repair_error(&mut self, msg: String) {
        if let RepairInput::Typing { ref mut error, .. } = self.repair {
            *error = Some(msg);
        }
    }

    pub fn set_mine_error(&mut self, msg: String) {
        if let MineInput::Configure { ref mut error, .. } = self.mine {
            *error = Some(msg);
        }
    }

    /// Surface a craft failure on whichever fabrication step is active.
    pub fn set_fabrication_error(&mut self, msg: String) {
        match self.fabrication {
            FabricationInput::PickRecipe { ref mut error, .. } => *error = Some(msg),
            FabricationInput::PickBuilder { ref mut error, .. } => *error = Some(msg),
            FabricationInput::Inactive => {}
        }
    }

    /// Surface a probe-improvement failure on whichever step is active.
    pub fn set_improve_error(&mut self, msg: String) {
        match self.improve {
            ImproveInput::PickImprovement { ref mut error, .. } => *error = Some(msg),
            ImproveInput::PickBuilder { ref mut error, .. } => *error = Some(msg),
            ImproveInput::Inactive => {}
        }
    }

    pub fn set_salvage_error(&mut self, msg: String) {
        if let SalvageInput::Confirm { ref mut error, .. } = self.salvage {
            *error = Some(msg);
        }
    }

    pub fn set_recall_error(&mut self, msg: String) {
        if let RecallInput::Confirm { ref mut error, .. } = self.recall {
            *error = Some(msg);
        }
    }

    pub fn set_refuel_error(&mut self, msg: String) {
        if let RefuelInput::Confirm { ref mut error, .. } = self.refuel {
            *error = Some(msg);
        }
    }

    pub fn set_mind_snapshot_error(&mut self, msg: String) {
        if let MindSnapshotInput::Confirm { ref mut error } = self.mind_snapshot {
            *error = Some(msg);
        }
    }

    pub fn set_scut_relay_error(&mut self, msg: String) {
        if let ScutRelayInput::EnterNetworkName { ref mut error, .. } = self.scut_relay {
            *error = Some(msg);
        }
    }

    pub fn set_mission_abandon_error(&mut self, msg: String) {
        if let MissionsInput::ConfirmAbandon { ref mut error, .. } = self.missions_input {
            *error = Some(msg);
        }
    }

    pub fn set_message_send_error(&mut self, msg: String) {
        if let MessagesInput::Compose { ref mut error, .. } = self.messages_input {
            *error = Some(msg);
        }
    }

    pub fn unread_message_count(&self) -> usize {
        self.messages.iter()
            .filter(|m| m.status == crate::api::types::MessageStatus::Unread)
            .count()
    }

    /// Message recipients reachable from the current sector: detected probes
    /// plus inhabited planets. Returns (kind, endpoint id, display name).
    pub fn collect_message_recipients(&self) -> Vec<(String, crate::api::types::EndpointId, String)> {
        use crate::api::types::EndpointId;
        let mut out = Vec::new();
        let Some(sector) = self.probe_current_sector_scan() else { return out };
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
                objects.iter().any(|o| {
                    matches!(o.object_type, SectorObjectType::DeuteriumRefuelStation)
                })
            })
    }

    pub fn collect_mineable_candidates(&self) -> Vec<(String, String)> {
        let sector = self.probe_current_sector_scan();
        sector
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .flat_map(|o| o.minable_targets.iter().flatten())
                    .filter(|t| matches!(t.object_type, SectorObjectType::Asteroid))
                    .map(|t| (t.id.clone(), t.name.clone().unwrap_or_else(|| "unnamed".into())))
                    .collect()
            })
            .unwrap_or_default()
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
        let Some(sector) = self.probe_current_sector_scan() else { return Vec::new() };
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
                objects.iter()
                    .flat_map(|o| {
                        let direct = if matches!(o.object_type, SectorObjectType::Asteroid) {
                            o.id.as_ref().map(|id| vec![(id.clone(), o.name.clone().unwrap_or_else(|| "unnamed".into()))])
                                .unwrap_or_default()
                        } else { vec![] };
                        let nested: Vec<(String, String)> = o.bookmark_targets.iter()
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
                objects.iter()
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
        if let DeployInput::EnterName { ref mut error, .. } = self.deploy {
            *error = Some(msg);
        }
    }

    pub fn set_rename_manny_error(&mut self, msg: String) {
        if let RenameMannyInput::Typing { ref mut error, .. } = self.rename_manny {
            *error = Some(msg);
        }
    }

    pub fn rename_manny_type_char(&mut self, c: char) {
        if let RenameMannyInput::Typing { ref mut buf, .. } = self.rename_manny {
            if buf.len() < 40 {
                buf.push(c);
            }
        }
    }

    pub fn rename_manny_backspace(&mut self) {
        if let RenameMannyInput::Typing { ref mut buf, .. } = self.rename_manny {
            buf.pop();
        }
    }

    pub fn deploy_type_char(&mut self, c: char) {
        if let DeployInput::EnterName { ref mut name_buf, .. } = self.deploy {
            if name_buf.len() < 80 {
                name_buf.push(c);
            }
        }
    }

    pub fn deploy_backspace(&mut self) {
        if let DeployInput::EnterName { ref mut name_buf, .. } = self.deploy {
            name_buf.pop();
        }
    }

    pub fn collect_idle_onboard_mannies(&self) -> Vec<(String, String)> {
        self.mannies.as_ref()
            .map(|ms| {
                ms.iter()
                    .filter(|m| {
                        m.location.location_type == crate::api::types::MannyLocationType::Probe
                            && m.can_receive_orders
                    })
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_deploy_candidates(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .filter(|o| o.id.is_some())
                    .map(|o: &SectorObject| {
                        let id = o.id.clone().unwrap();
                        let name = o.name.clone().unwrap_or_else(|| format!("{:?}", o.object_type).to_lowercase());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_inspect_error(&mut self, msg: String) {
        if let InspectInput::PickTarget { ref mut error, .. } = self.inspect {
            *error = Some(msg);
        } else {
            // Inspect was dispatched without the picker overlay (single
            // candidate or object-first flow) — surface in the status bar.
            self.error = Some(format!("inspect: {msg}"));
        }
    }

    pub fn set_recover_error(&mut self, msg: String) {
        if let RecoverInput::PickContainer { ref mut error, .. } = self.recover {
            *error = Some(msg);
        } else {
            self.error = Some(format!("recover: {msg}"));
        }
    }

    pub fn set_detach_error(&mut self, msg: String) {
        match self.detach {
            DetachInput::PickMode { ref mut error, .. } => *error = Some(msg),
            DetachInput::PickAsteroid { ref mut error, .. } => *error = Some(msg),
            _ => {}
        }
    }

    pub fn collect_detachable_containers(&self) -> Vec<(String, String)> {
        self.probe.as_ref()
            .map(|p| {
                p.inventory.containers.iter()
                    .filter(|c| c.kind != "probe")
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
                objects.iter()
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
        let (manny_id, manny_name) = match &self.remote_mine {
            RemoteMineInput::Loading { manny_id, manny_name, x: lx, y: ly, z: lz }
                if (*lx, *ly, *lz) == (x, y, z) =>
            {
                (manny_id.clone(), manny_name.clone())
            }
            _ => return,
        };
        let candidates = match self.sector_observation_at(x, y, z) {
            Some(s) => self.collect_asteroid_candidates_in(s),
            None => return,
        };
        if candidates.is_empty() {
            self.remote_mine = RemoteMineInput::Inactive;
            self.error = Some("no mineable asteroid in the Manny's sector".into());
            return;
        }
        self.remote_mine = RemoteMineInput::PickAsteroid {
            manny_id,
            manny_name,
            x,
            y,
            z,
            candidates,
            selection: 0,
        };
    }

    pub fn set_remote_mine_error(&mut self, msg: String) {
        match self.remote_mine {
            RemoteMineInput::Configure { ref mut error, .. } => *error = Some(msg),
            RemoteMineInput::PickContainer { ref mut error, .. } => *error = Some(msg),
            _ => {}
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
        let v = manny
            .location
            .sector
            .as_ref()?
            .get("relative")?;
        Some((
            v.get("x")?.as_f64()? as i32,
            v.get("y")?.as_f64()? as i32,
            v.get("z")?.as_f64()? as i32,
        ))
    }
}
