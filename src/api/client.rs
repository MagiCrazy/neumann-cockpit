use super::types::{
    ContainerInventory, CraftingRecipe, DamageWarningRule, Manny, Mission, Probe, ProbeAlert,
    ProbeInventory, ProbeMovement, ScutNetwork, SectorObservation, StorageContainer, VisitedSector,
};
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: Url,
    api_key: String,
}

impl ApiClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("neumann-cockpit/0.1")
            .build()
            .context("Failed to build HTTP client")?;
        let base_url = Url::parse(&base_url).context("Invalid base_url in config")?;
        Ok(Self { client, base_url, api_key })
    }

    fn url(&self, path: &str) -> Url {
        self.base_url.join(path).expect("static paths are valid")
    }

    async fn send_with_body<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .client
            .request(method.clone(), self.url(path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .with_context(|| format!("{method} {path}"))?;

        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            anyhow::bail!("Unauthorized — check your api_key in config.toml");
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(String::from))
                .unwrap_or(text);
            anyhow::bail!("{msg}");
        }

        resp.json::<T>().await.with_context(|| format!("Parsing {method} {path}"))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        self.send_with_body(reqwest::Method::POST, path, body).await
    }

    async fn patch<T: for<'de> Deserialize<'de>, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        self.send_with_body(reqwest::Method::PATCH, path, body).await
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let resp = self
            .client
            .get(self.url(path))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .with_context(|| format!("GET {path}"))?;

        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            anyhow::bail!("Unauthorized — check your api_key in config.toml");
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {status} on GET {path}: {body}");
        }

        resp.json::<T>().await.with_context(|| format!("Parsing GET {path}"))
    }

    pub async fn get_api_version(&self) -> Result<u32> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp { api_version: u32 }
        Ok(self.get::<Resp>("/api/version").await?.api_version)
    }

    pub async fn get_probe(&self) -> Result<Probe> {
        #[derive(Deserialize)]
        struct Resp {
            probe: Probe,
        }
        Ok(self.get::<Resp>("/api/probe").await?.probe)
    }

    pub async fn get_mannies(&self) -> Result<Vec<Manny>> {
        #[derive(Deserialize)]
        struct Resp {
            mannies: Vec<Manny>,
        }
        Ok(self.get::<Resp>("/api/probe/mannies").await?.mannies)
    }

    pub async fn get_probe_sector(&self) -> Result<SectorObservation> {
        #[derive(Deserialize)]
        struct Resp {
            sector: SectorObservation,
        }
        Ok(self.get::<Resp>("/api/probe/sector").await?.sector)
    }

    pub async fn move_probe(&self, x: i32, y: i32, z: i32) -> Result<ProbeMovement> {
        #[derive(Serialize)]
        struct Target { x: i32, y: i32, z: i32 }
        #[derive(Serialize)]
        struct Body { target: Target }
        #[derive(Deserialize)]
        struct Resp { movement: ProbeMovement }
        Ok(self
            .post::<Resp, _>("/api/probe/move", &Body { target: Target { x, y, z } })
            .await?
            .movement)
    }

    pub async fn repair_manny(&self, manny_id: &str, integrity_percent: f64) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body { integrity_percent: f64 }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/repair");
        Ok(self.post::<Resp, _>(&path, &Body { integrity_percent }).await?.manny)
    }

    pub async fn mine_manny(
        &self,
        manny_id: &str,
        object_id: &str,
        resources: Vec<String>,
        target_amount: f64,
        target_container_id: Option<String>,
    ) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            object_id: String,
            resources: Vec<String>,
            target_amount: f64,
            #[serde(skip_serializing_if = "Option::is_none")]
            target_container_id: Option<String>,
        }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/mine");
        Ok(self
            .post::<Resp, _>(&path, &Body {
                object_id: object_id.to_string(),
                resources,
                target_amount,
                target_container_id,
            })
            .await?
            .manny)
    }

    pub async fn jettison_inventory(&self, item_id: &str, amount: Option<f64>) -> Result<ProbeInventory> {
        #[derive(Serialize)]
        struct Body {
            #[serde(skip_serializing_if = "Option::is_none")]
            amount: Option<f64>,
        }
        #[derive(Deserialize)]
        struct Resp { inventory: ProbeInventory }
        let path = format!("/api/probe/inventory/{item_id}/jettison");
        Ok(self.post::<Resp, _>(&path, &Body { amount }).await?.inventory)
    }

    pub async fn craft_manny(&self, manny_id: &str, recipe: &str) -> Result<Manny> {
        #[derive(Serialize)]
        struct Body<'a> { recipe: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/craft");
        Ok(self.post::<Resp, _>(&path, &Body { recipe }).await?.manny)
    }

    pub async fn get_sector(&self, x: i32, y: i32, z: i32) -> Result<SectorObservation> {
        #[derive(Deserialize)]
        struct Resp {
            sector: SectorObservation,
        }
        let path = format!("/api/sector?x={x}&y={y}&z={z}");
        Ok(self.get::<Resp>(&path).await?.sector)
    }

    pub async fn salvage_manny(&self, manny_id: &str, object_id: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> { object_id: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/salvage");
        Ok(self.post::<Resp, _>(&path, &Body { object_id }).await?.manny)
    }

    pub async fn recall_manny(&self, manny_id: &str) -> Result<Manny> {
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/recall");
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.manny)
    }

    /// Drop an additional storage container onto a planet in the current sector
    /// (`POST /api/probe/mannies/{id}/drop-storage-container`). Consumes an
    /// atmospheric_drop_kit; the container leaves the probe inventory.
    pub async fn drop_storage_container_on_planet(
        &self,
        manny_id: &str,
        container_id: &str,
        planet_id: &str,
    ) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> { container_id: &'a str, planet_id: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/drop-storage-container");
        Ok(self.post::<Resp, _>(&path, &Body { container_id, planet_id }).await?.manny)
    }

    /// Drop the cargo of a Manny waiting outside for storage space and retry
    /// docking (`POST /api/probe/mannies/{id}/drop-manny-cargo`). Resource cargo
    /// is lost; recoverable objects are restored to the current sector.
    pub async fn drop_manny_cargo(&self, manny_id: &str) -> Result<Manny> {
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/drop-manny-cargo");
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.manny)
    }

    /// Start a one-minute Manny task that refills the probe deuterium tank
    /// (`POST /api/probe/mannies/{id}/refill-deuterium-tank`). Requires a
    /// deuterium refuel station in the current sector.
    pub async fn refill_deuterium_tank(&self, manny_id: &str) -> Result<Manny> {
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/refill-deuterium-tank");
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.manny)
    }

    /// List the probe's active missions (`GET /api/probe/missions`).
    pub async fn get_missions(&self) -> Result<Vec<Mission>> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(default)]
            missions: Vec<Mission>,
        }
        Ok(self.get::<Resp>("/api/probe/missions").await?.missions)
    }

    /// Abandon an active mission (`POST /api/probe/missions/{id}/abandon`).
    pub async fn abandon_mission(&self, mission_id: &str) -> Result<Mission> {
        #[derive(Deserialize)]
        struct Resp { mission: Mission }
        let path = format!("/api/probe/missions/{mission_id}/abandon");
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.mission)
    }

    /// Inspect a SCUT relay network covering the current probe
    /// (`GET /api/probe/scut-network/{id}`).
    pub async fn get_scut_network(&self, network_id: i64) -> Result<ScutNetwork> {
        #[derive(Deserialize)]
        struct Resp { network: ScutNetwork }
        let path = format!("/api/probe/scut-network/{network_id}");
        Ok(self.get::<Resp>(&path).await?.network)
    }

    /// Send a Manny to turn on an inactive SCUT relay in the current sector
    /// (`POST /api/probe/mannies/{id}/turn-on-relay`). Requires a star in the
    /// sector and one integrated_circuit in inventory. `relay_id` is the
    /// relay's integer id (the sector object id parsed as an integer).
    pub async fn turn_on_relay(
        &self,
        manny_id: &str,
        relay_id: i64,
        network_name: Option<&str>,
    ) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            relay_id: i64,
            #[serde(skip_serializing_if = "Option::is_none")]
            network_name: Option<&'a str>,
        }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/turn-on-relay");
        Ok(self
            .post::<Resp, _>(&path, &Body { relay_id, network_name })
            .await?
            .manny)
    }

    /// Reassign the player's mind snapshot to a fresh probe chassis
    /// (`POST /api/probe/mind-snapshot/reassign`). Only valid when the current
    /// probe is dead or trapped by a black hole; resets the local reference
    /// frame to 0,0,0 and returns the new probe.
    pub async fn reassign_mind_snapshot(&self) -> Result<Probe> {
        #[derive(Deserialize)]
        struct Resp { probe: Probe }
        Ok(self
            .post::<Resp, _>("/api/probe/mind-snapshot/reassign", &serde_json::json!({}))
            .await?
            .probe)
    }

    pub async fn rename_manny(&self, manny_id: &str, name: &str) -> Result<Manny> {
        #[derive(Serialize)]
        struct Body<'a> { name: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}");
        Ok(self.patch::<Resp, _>(&path, &Body { name }).await?.manny)
    }

    pub async fn craft_atomic_printer(&self, recipe: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Body<'a> { recipe: &'a str }
        self.post::<serde_json::Value, _>("/api/probe/atomic-printer/craft", &Body { recipe }).await?;
        Ok(())
    }

    pub async fn detach_storage_container(
        &self,
        manny_id: &str,
        container_id: &str,
        mode: &str,
        object_id: Option<&str>,
    ) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            container_id: &'a str,
            mode: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            object_id: Option<&'a str>,
        }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/detach-storage-container");
        Ok(self.post::<Resp, _>(&path, &Body { container_id, mode, object_id }).await?.manny)
    }

    pub async fn inspect_asteroid(&self, manny_id: &str, object_id: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> { object_id: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/inspect-asteroid");
        Ok(self.post::<Resp, _>(&path, &Body { object_id }).await?.manny)
    }

    pub async fn recover_storage_container(&self, manny_id: &str, object_id: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> { object_id: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/recover-storage-container");
        Ok(self.post::<Resp, _>(&path, &Body { object_id }).await?.manny)
    }

    pub async fn get_visited_sectors(&self) -> Result<Vec<VisitedSector>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp { visited_sectors: Vec<VisitedSector> }
        Ok(self.get::<Resp>("/api/probe/visited-sectors").await?.visited_sectors)
    }

    pub async fn get_crafting_recipes(&self) -> Result<Vec<CraftingRecipe>> {
        #[derive(Deserialize)]
        struct Resp { recipes: Vec<CraftingRecipe> }
        Ok(self.get::<Resp>("/api/crafting-recipes").await?.recipes)
    }

    pub async fn install_bookmark_manny(&self, manny_id: &str, object_id: &str, name: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> { object_id: &'a str, name: &'a str }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/install-bookmark");
        Ok(self.post::<Resp, _>(&path, &Body { object_id, name }).await?.manny)
    }

    pub async fn get_damage_warnings(&self) -> Result<(Vec<ProbeAlert>, DamageWarningRule)> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp {
            #[serde(default)]
            damage_warnings: Vec<ProbeAlert>,
            #[serde(default)]
            rule: DamageWarningRule,
        }
        let resp = self.get::<Resp>("/api/probe/damage-warnings").await?;
        Ok((resp.damage_warnings, resp.rule))
    }

    /// Mark a damage warning as read (`PATCH /api/probe/damage-warnings/{id}`).
    pub async fn ack_damage_warning(&self, id: i64) -> Result<ProbeAlert> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp { damage_warning: ProbeAlert }
        let path = format!("/api/probe/damage-warnings/{id}");
        Ok(self.patch::<Resp, _>(&path, &serde_json::json!({})).await?.damage_warning)
    }

    pub async fn get_alerts(&self) -> Result<Vec<ProbeAlert>> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(default)]
            alerts: Vec<ProbeAlert>,
        }
        Ok(self.get::<Resp>("/api/probe/alerts").await?.alerts)
    }

    /// Mark an alert as read (`PATCH /api/probe/alerts/{id}`).
    pub async fn ack_alert(&self, id: i64) -> Result<ProbeAlert> {
        #[derive(Deserialize)]
        struct Resp { alert: ProbeAlert }
        let path = format!("/api/probe/alerts/{id}");
        Ok(self.patch::<Resp, _>(&path, &serde_json::json!({})).await?.alert)
    }

    /// Assign an idle Manny to move stock between containers
    /// (`POST /api/probe/storage-moves`). `manny` kind is intentionally
    /// unsupported here — only `resource` and `item` are wired in the UI.
    #[allow(clippy::too_many_arguments)]
    pub async fn storage_move(
        &self,
        actor_manny_id: &str,
        kind: &str,
        to_container_id: &str,
        from_container_id: Option<&str>,
        resource_type: Option<&str>,
        amount: Option<f64>,
        item_ids: Option<Vec<String>>,
    ) -> Result<(Manny, ProbeInventory)> {
        let mut body = serde_json::Map::new();
        body.insert("actorMannyId".into(), serde_json::json!(actor_manny_id));
        body.insert("kind".into(), serde_json::json!(kind));
        body.insert("toContainerId".into(), serde_json::json!(to_container_id));
        if let Some(v) = from_container_id {
            body.insert("fromContainerId".into(), serde_json::json!(v));
        }
        if let Some(v) = resource_type {
            body.insert("resourceType".into(), serde_json::json!(v));
        }
        if let Some(v) = amount {
            body.insert("amount".into(), serde_json::json!(v));
        }
        if let Some(v) = item_ids {
            body.insert("itemIds".into(), serde_json::json!(v));
        }
        #[derive(Deserialize)]
        struct Resp { manny: Manny, inventory: ProbeInventory }
        let r = self
            .post::<Resp, _>("/api/probe/storage-moves", &serde_json::Value::Object(body))
            .await?;
        Ok((r.manny, r.inventory))
    }

    pub async fn get_storage_containers(&self) -> Result<Vec<StorageContainer>> {
        #[derive(Deserialize)]
        struct Resp { containers: Vec<StorageContainer> }
        Ok(self.get::<Resp>("/api/probe/storage-containers").await?.containers)
    }

    pub async fn get_storage_container(&self, id: &str) -> Result<(StorageContainer, ContainerInventory)> {
        #[derive(Deserialize)]
        struct Resp { container: StorageContainer, inventory: ContainerInventory }
        let path = format!("/api/probe/storage-containers/{id}");
        let r = self.get::<Resp>(&path).await?;
        Ok((r.container, r.inventory))
    }

    pub async fn rename_storage_container(
        &self,
        id: &str,
        label: &str,
    ) -> Result<(StorageContainer, ProbeInventory)> {
        #[derive(Serialize)]
        struct Body<'a> { label: &'a str }
        #[derive(Deserialize)]
        struct Resp { container: StorageContainer, inventory: ProbeInventory }
        let path = format!("/api/probe/storage-containers/{id}");
        let r = self.patch::<Resp, _>(&path, &Body { label }).await?;
        Ok((r.container, r.inventory))
    }

    pub async fn update_container_rules(
        &self,
        id: &str,
        priority: Vec<String>,
        exclusion: Vec<String>,
        strict_exclusion: Vec<String>,
    ) -> Result<(StorageContainer, ProbeInventory)> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            priority: Vec<String>,
            exclusion: Vec<String>,
            strict_exclusion: Vec<String>,
        }
        #[derive(Deserialize)]
        struct Resp { container: StorageContainer, inventory: ProbeInventory }
        let path = format!("/api/probe/storage-containers/{id}/rules");
        let r = self
            .patch::<Resp, _>(&path, &Body { priority, exclusion, strict_exclusion })
            .await?;
        Ok((r.container, r.inventory))
    }
}
