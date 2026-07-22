use super::types::{
    ContainerInventory, CraftingRecipe, DamageWarningRule, EndpointId, Manny, Mission, Pagination, Probe, ProbeAlert,
    ProbeImprovement, ProbeInventory, ProbeListResponse, ProbeMessage, ProbeMovement, ProbeSentMessage, ScutNetwork,
    SectorObservation, StorageContainer, VisitedSector,
};
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: Url,
    api_key: String,
    /// The probe every per-probe endpoint targets. `None` means the player's
    /// default probe and reproduces the pre-v81 paths (`/api/probe/…`) exactly;
    /// `Some(id)` targets a specific probe via the `/api/probe/{id}/…` mirrors
    /// (API v81 multi-probe). Player-level endpoints (missions, mind-snapshot,
    /// sent messages, version, sector, recipes) never use this.
    active_probe_id: Option<u64>,
}

/// Connect timeout: bound establishing the TCP/TLS connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Overall request timeout: bound the whole request/response.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

impl ApiClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        Self::build(base_url, api_key, CONNECT_TIMEOUT, REQUEST_TIMEOUT)
    }

    /// Build a client with explicit timeouts. `new` uses the production values;
    /// tests inject short ones to exercise the timeout path without a 30 s wait.
    ///
    /// reqwest has no default timeout: a half-open connection (suspend/resume,
    /// wifi dropping mid-request) would otherwise hang a fetch task forever —
    /// no ApiMessage ever arrives, `loading` stays true, and all three refresh
    /// paths are gated on `!loading`. Bounding both the connect and the overall
    /// request means a stuck fetch always fails and frees the refresh loop.
    fn build(base_url: String, api_key: String, connect_timeout: Duration, timeout: Duration) -> Result<Self> {
        // Identify the client to the game server: app version + platform.
        // e.g. "neumann-cockpit/63.1.0 (linux; x86_64)".
        let user_agent = format!(
            "neumann-cockpit/{} ({}; {})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );

        let client = Client::builder()
            .user_agent(user_agent)
            .connect_timeout(connect_timeout)
            .timeout(timeout)
            .build()
            .context("Failed to build HTTP client")?;
        let base_url = Url::parse(&base_url).context("Invalid base_url in config")?;
        Ok(Self {
            client,
            base_url,
            api_key,
            active_probe_id: None,
        })
    }

    /// Test-only constructor with a short overall timeout, for exercising the
    /// timeout path against a deliberately slow mock server.
    #[cfg(test)]
    fn new_with_timeout(base_url: String, api_key: String, timeout: Duration) -> Result<Self> {
        Self::build(base_url, api_key, timeout, timeout)
    }

    /// Return a clone of this client that targets `id` (or the default probe
    /// when `None`) for every per-probe endpoint. Cheap: `reqwest::Client` is
    /// internally reference-counted. Used by the cockpit to switch the active
    /// probe without touching the server-side default.
    pub fn with_active_probe(&self, id: Option<u64>) -> Self {
        Self {
            active_probe_id: id,
            ..self.clone()
        }
    }

    /// Which probe per-probe calls currently target (`None` = default).
    pub fn active_probe_id(&self) -> Option<u64> {
        self.active_probe_id
    }

    fn url(&self, path: &str) -> Url {
        self.base_url.join(path).expect("static paths are valid")
    }

    /// Build a per-probe endpoint path. `suffix` is everything after the probe
    /// segment (e.g. `"/mannies"`, `""`, `&format!("/mannies/{id}/mine")`).
    /// `None` → `/api/probe{suffix}` (default probe, pre-v81 behaviour);
    /// `Some(id)` → `/api/probe/{id}{suffix}`.
    fn probe_path(&self, suffix: &str) -> String {
        match self.active_probe_id {
            Some(id) => format!("/api/probe/{id}{suffix}"),
            None => format!("/api/probe{suffix}"),
        }
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

        resp.json::<T>()
            .await
            .with_context(|| format!("Parsing {method} {path}"))
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
        struct Resp {
            api_version: u32,
        }
        Ok(self.get::<Resp>("/api/version").await?.api_version)
    }

    pub async fn get_probe(&self) -> Result<Probe> {
        #[derive(Deserialize)]
        struct Resp {
            probe: Probe,
        }
        Ok(self.get::<Resp>(&self.probe_path("")).await?.probe)
    }

    /// List the player's probes (`GET /api/probes`, API v81). Player-level, not
    /// per-probe: never uses `probe_path`.
    pub async fn get_probes(&self) -> Result<ProbeListResponse> {
        self.get::<ProbeListResponse>("/api/probes").await
    }

    /// Rename a probe and/or promote it to the player's default
    /// (`PATCH /api/probe/{id}`, API v81); returns the refreshed fleet. The
    /// default can only change to a probe in the current default's sector or
    /// shared SCUT coverage — the server returns 422 otherwise (surfaced as the
    /// error message).
    pub async fn patch_probe(
        &self,
        probe_id: u64,
        name: Option<&str>,
        is_default: Option<bool>,
    ) -> Result<ProbeListResponse> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_default: Option<bool>,
        }
        let path = format!("/api/probe/{probe_id}");
        self.patch::<ProbeListResponse, _>(&path, &Body { name, is_default })
            .await
    }

    pub async fn get_mannies(&self) -> Result<Vec<Manny>> {
        #[derive(Deserialize)]
        struct Resp {
            mannies: Vec<Manny>,
        }
        Ok(self.get::<Resp>(&self.probe_path("/mannies")).await?.mannies)
    }

    pub async fn get_probe_sector(&self) -> Result<SectorObservation> {
        #[derive(Deserialize)]
        struct Resp {
            sector: SectorObservation,
        }
        Ok(self.get::<Resp>(&self.probe_path("/sector")).await?.sector)
    }

    pub async fn move_probe(&self, x: i32, y: i32, z: i32) -> Result<ProbeMovement> {
        #[derive(Serialize)]
        struct Target {
            x: i32,
            y: i32,
            z: i32,
        }
        #[derive(Serialize)]
        struct Body {
            target: Target,
        }
        #[derive(Deserialize)]
        struct Resp {
            movement: ProbeMovement,
        }
        Ok(self
            .post::<Resp, _>(
                &self.probe_path("/move"),
                &Body {
                    target: Target { x, y, z },
                },
            )
            .await?
            .movement)
    }

    pub async fn repair_manny(&self, manny_id: &str, integrity_percent: f64) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            integrity_percent: f64,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/repair"));
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
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/mine"));
        Ok(self
            .post::<Resp, _>(
                &path,
                &Body {
                    object_id: object_id.to_string(),
                    resources,
                    target_amount,
                    target_container_id,
                },
            )
            .await?
            .manny)
    }

    /// Start a Manny task that assembles a new drone probe from two empty
    /// additional containers plus fixed components (`POST
    /// /api/probe/mannies/{id}/assemble-probe`, API v81). Returns the updated
    /// Manny and probe inventory (the containers + components are consumed).
    pub async fn assemble_probe(&self, manny_id: &str, container_ids: &[String]) -> Result<(Manny, ProbeInventory)> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            container_ids: &'a [String],
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
            inventory: ProbeInventory,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/assemble-probe"));
        let r = self.post::<Resp, _>(&path, &Body { container_ids }).await?;
        Ok((r.manny, r.inventory))
    }

    pub async fn jettison_inventory(&self, item_id: &str, amount: Option<f64>) -> Result<ProbeInventory> {
        #[derive(Serialize)]
        struct Body {
            #[serde(skip_serializing_if = "Option::is_none")]
            amount: Option<f64>,
        }
        #[derive(Deserialize)]
        struct Resp {
            inventory: ProbeInventory,
        }
        let path = self.probe_path(&format!("/inventory/{item_id}/jettison"));
        Ok(self.post::<Resp, _>(&path, &Body { amount }).await?.inventory)
    }

    pub async fn craft_manny(&self, manny_id: &str, recipe: &str) -> Result<Manny> {
        #[derive(Serialize)]
        struct Body<'a> {
            recipe: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/craft"));
        Ok(self.post::<Resp, _>(&path, &Body { recipe }).await?.manny)
    }

    pub async fn get_probe_improvements(&self) -> Result<Vec<ProbeImprovement>> {
        #[derive(Deserialize)]
        struct Resp {
            improvements: Vec<ProbeImprovement>,
        }
        Ok(self
            .get::<Resp>(&self.probe_path("/probe-improvements-available"))
            .await?
            .improvements)
    }

    pub async fn improve_probe(&self, manny_id: &str, improvement: &str) -> Result<Manny> {
        #[derive(Serialize)]
        struct Body<'a> {
            improvement: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/improve-probe"));
        Ok(self.post::<Resp, _>(&path, &Body { improvement }).await?.manny)
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
        struct Body<'a> {
            object_id: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/salvage"));
        Ok(self.post::<Resp, _>(&path, &Body { object_id }).await?.manny)
    }

    pub async fn recall_manny(&self, manny_id: &str) -> Result<Manny> {
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/recall"));
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
        struct Body<'a> {
            container_id: &'a str,
            planet_id: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/drop-storage-container"));
        Ok(self
            .post::<Resp, _>(
                &path,
                &Body {
                    container_id,
                    planet_id,
                },
            )
            .await?
            .manny)
    }

    /// Drop the cargo of a Manny waiting outside for storage space and retry
    /// docking (`POST /api/probe/mannies/{id}/drop-manny-cargo`). Resource cargo
    /// is lost; recoverable objects are restored to the current sector.
    pub async fn drop_manny_cargo(&self, manny_id: &str) -> Result<Manny> {
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/drop-manny-cargo"));
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.manny)
    }

    /// Start a one-minute Manny task that refills the probe deuterium tank
    /// (`POST /api/probe/mannies/{id}/refill-deuterium-tank`). Requires a
    /// deuterium refuel station in the current sector.
    pub async fn refill_deuterium_tank(&self, manny_id: &str) -> Result<Manny> {
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/refill-deuterium-tank"));
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.manny)
    }

    /// Start a five-minute Manny task that transfers a reserved deuterium
    /// amount from the current probe to another fleet probe in the same sector
    /// (`POST /api/probe/mannies/{id}/transfer-deuterium-to-probe`, API v86).
    /// `amount` is a percentage-point reserve, strictly below the source
    /// reserve; on completion the target is topped up only to its capacity and
    /// any surplus returns to the source.
    pub async fn transfer_deuterium_to_probe(
        &self,
        manny_id: &str,
        target_probe_id: u64,
        amount: f64,
    ) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            target_probe_id: u64,
            amount: f64,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/transfer-deuterium-to-probe"));
        Ok(self
            .post::<Resp, _>(
                &path,
                &Body {
                    target_probe_id,
                    amount,
                },
            )
            .await?
            .manny)
    }

    /// Transfer a Manny to another owned probe (`POST .../transfer-to-probe`,
    /// API v93). The target must be in the Manny's sector (same sector, or a
    /// SCUT-reachable remote sector); the same-sector requirement is
    /// server-validated (422). Duration matches a container detachment.
    pub async fn transfer_manny_to_probe(&self, manny_id: &str, target_probe_id: u64) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            target_probe_id: u64,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/transfer-to-probe"));
        Ok(self.post::<Resp, _>(&path, &Body { target_probe_id }).await?.manny)
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

    /// List received messages (`GET /api/probe/messages`, newest first).
    pub async fn get_messages(&self) -> Result<(Vec<ProbeMessage>, Pagination)> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(default)]
            messages: Vec<ProbeMessage>,
            pagination: Pagination,
        }
        let r = self.get::<Resp>(&self.probe_path("/messages")).await?;
        Ok((r.messages, r.pagination))
    }

    /// List sent messages (`GET /api/probe/messages/sent`, newest first).
    pub async fn get_sent_messages(&self) -> Result<(Vec<ProbeSentMessage>, Pagination)> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(default)]
            messages: Vec<ProbeSentMessage>,
            pagination: Pagination,
        }
        let r = self.get::<Resp>("/api/probe/messages/sent").await?;
        Ok((r.messages, r.pagination))
    }

    /// Send a message to a probe or inhabited planet (`POST /api/probe/messages`).
    pub async fn send_message(
        &self,
        recipient_type: &str,
        recipient_id: &EndpointId,
        body: &str,
    ) -> Result<ProbeMessage> {
        #[derive(Deserialize)]
        struct Resp {
            message: ProbeMessage,
        }
        let payload = serde_json::json!({
            "recipient": { "type": recipient_type, "id": recipient_id },
            "body": body,
        });
        Ok(self
            .post::<Resp, _>(&self.probe_path("/messages"), &payload)
            .await?
            .message)
    }

    /// Mark a received message as read (`PATCH /api/probe/messages/{id}/read`).
    pub async fn mark_message_read(&self, message_id: i64) -> Result<ProbeMessage> {
        #[derive(Deserialize)]
        struct Resp {
            message: ProbeMessage,
        }
        let path = self.probe_path(&format!("/messages/{message_id}/read"));
        Ok(self.patch::<Resp, _>(&path, &serde_json::json!({})).await?.message)
    }

    /// Abandon an active mission (`POST /api/probe/missions/{id}/abandon`).
    pub async fn abandon_mission(&self, mission_id: &str) -> Result<Mission> {
        #[derive(Deserialize)]
        struct Resp {
            mission: Mission,
        }
        let path = format!("/api/probe/missions/{mission_id}/abandon");
        Ok(self.post::<Resp, _>(&path, &serde_json::json!({})).await?.mission)
    }

    /// Inspect a SCUT relay network covering the current probe
    /// (`GET /api/probe/scut-network/{id}`).
    pub async fn get_scut_network(&self, network_id: i64) -> Result<ScutNetwork> {
        #[derive(Deserialize)]
        struct Resp {
            network: ScutNetwork,
        }
        let path = self.probe_path(&format!("/scut-network/{network_id}"));
        Ok(self.get::<Resp>(&path).await?.network)
    }

    /// Send a Manny to turn on an inactive SCUT relay in the current sector
    /// (`POST /api/probe/mannies/{id}/turn-on-relay`). Requires a star in the
    /// sector and one integrated_circuit in inventory. `relay_id` is the
    /// relay's integer id (the sector object id parsed as an integer).
    pub async fn turn_on_relay(&self, manny_id: &str, relay_id: i64, network_name: Option<&str>) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            relay_id: i64,
            #[serde(skip_serializing_if = "Option::is_none")]
            network_name: Option<&'a str>,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/turn-on-relay"));
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
        struct Resp {
            probe: Probe,
        }
        Ok(self
            .post::<Resp, _>("/api/probe/mind-snapshot/reassign", &serde_json::json!({}))
            .await?
            .probe)
    }

    pub async fn rename_manny(&self, manny_id: &str, name: &str) -> Result<Manny> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}"));
        Ok(self.patch::<Resp, _>(&path, &Body { name }).await?.manny)
    }

    pub async fn craft_atomic_printer(&self, recipe: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Body<'a> {
            recipe: &'a str,
        }
        self.post::<serde_json::Value, _>(&self.probe_path("/atomic-printer/craft"), &Body { recipe })
            .await?;
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
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/detach-storage-container"));
        Ok(self
            .post::<Resp, _>(
                &path,
                &Body {
                    container_id,
                    mode,
                    object_id,
                },
            )
            .await?
            .manny)
    }

    /// Inspect a sector object (asteroid, dormant construct, or detached
    /// container). Replaces the deprecated `inspect-asteroid` endpoint (API v65).
    pub async fn inspect_sector_object(&self, manny_id: &str, object_id: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            object_id: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/inspect-sector-object"));
        Ok(self.post::<Resp, _>(&path, &Body { object_id }).await?.manny)
    }

    pub async fn recover_storage_container(&self, manny_id: &str, object_id: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            object_id: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/recover-storage-container"));
        Ok(self.post::<Resp, _>(&path, &Body { object_id }).await?.manny)
    }

    pub async fn get_visited_sectors(&self) -> Result<Vec<VisitedSector>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp {
            visited_sectors: Vec<VisitedSector>,
        }
        Ok(self
            .get::<Resp>(&self.probe_path("/visited-sectors"))
            .await?
            .visited_sectors)
    }

    pub async fn get_crafting_recipes(&self) -> Result<Vec<CraftingRecipe>> {
        #[derive(Deserialize)]
        struct Resp {
            recipes: Vec<CraftingRecipe>,
        }
        Ok(self.get::<Resp>("/api/crafting-recipes").await?.recipes)
    }

    pub async fn install_bookmark_manny(&self, manny_id: &str, object_id: &str, name: &str) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            object_id: &'a str,
            name: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            manny: Manny,
        }
        let path = self.probe_path(&format!("/mannies/{manny_id}/install-bookmark"));
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
        let resp = self.get::<Resp>(&self.probe_path("/damage-warnings")).await?;
        Ok((resp.damage_warnings, resp.rule))
    }

    /// Mark a damage warning as read (`PATCH /api/probe/damage-warnings/{id}`).
    pub async fn ack_damage_warning(&self, id: i64) -> Result<ProbeAlert> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp {
            damage_warning: ProbeAlert,
        }
        let path = self.probe_path(&format!("/damage-warnings/{id}"));
        Ok(self
            .patch::<Resp, _>(&path, &serde_json::json!({}))
            .await?
            .damage_warning)
    }

    pub async fn get_alerts(&self) -> Result<Vec<ProbeAlert>> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(default)]
            alerts: Vec<ProbeAlert>,
        }
        Ok(self.get::<Resp>(&self.probe_path("/alerts")).await?.alerts)
    }

    /// Mark an alert as read (`PATCH /api/probe/alerts/{id}`).
    pub async fn ack_alert(&self, id: i64) -> Result<ProbeAlert> {
        #[derive(Deserialize)]
        struct Resp {
            alert: ProbeAlert,
        }
        let path = self.probe_path(&format!("/alerts/{id}"));
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
        struct Resp {
            manny: Manny,
            inventory: ProbeInventory,
        }
        let r = self
            .post::<Resp, _>(&self.probe_path("/storage-moves"), &serde_json::Value::Object(body))
            .await?;
        Ok((r.manny, r.inventory))
    }

    pub async fn get_storage_containers(&self) -> Result<Vec<StorageContainer>> {
        #[derive(Deserialize)]
        struct Resp {
            containers: Vec<StorageContainer>,
        }
        Ok(self
            .get::<Resp>(&self.probe_path("/storage-containers"))
            .await?
            .containers)
    }

    pub async fn get_storage_container(&self, id: &str) -> Result<(StorageContainer, ContainerInventory)> {
        #[derive(Deserialize)]
        struct Resp {
            container: StorageContainer,
            inventory: ContainerInventory,
        }
        let path = self.probe_path(&format!("/storage-containers/{id}"));
        let r = self.get::<Resp>(&path).await?;
        Ok((r.container, r.inventory))
    }

    pub async fn rename_storage_container(&self, id: &str, label: &str) -> Result<(StorageContainer, ProbeInventory)> {
        #[derive(Serialize)]
        struct Body<'a> {
            label: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            container: StorageContainer,
            inventory: ProbeInventory,
        }
        let path = self.probe_path(&format!("/storage-containers/{id}"));
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
        struct Resp {
            container: StorageContainer,
            inventory: ProbeInventory,
        }
        let path = self.probe_path(&format!("/storage-containers/{id}/rules"));
        let r = self
            .patch::<Resp, _>(
                &path,
                &Body {
                    priority,
                    exclusion,
                    strict_exclusion,
                },
            )
            .await?;
        Ok((r.container, r.inventory))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(active: Option<u64>) -> ApiClient {
        ApiClient::new("https://example.test".into(), "vng_test".into())
            .unwrap()
            .with_active_probe(active)
    }

    #[test]
    fn probe_path_none_reproduces_pre_v81_paths() {
        let c = client(None);
        assert_eq!(c.probe_path(""), "/api/probe");
        assert_eq!(c.probe_path("/mannies"), "/api/probe/mannies");
        assert_eq!(c.probe_path("/mannies/m_1/mine"), "/api/probe/mannies/m_1/mine");
    }

    #[test]
    fn probe_path_some_targets_the_active_probe() {
        let c = client(Some(5));
        assert_eq!(c.probe_path(""), "/api/probe/5");
        assert_eq!(c.probe_path("/sector"), "/api/probe/5/sector");
        assert_eq!(c.probe_path("/mannies/m_1/mine"), "/api/probe/5/mannies/m_1/mine");
    }

    #[test]
    fn with_active_probe_sets_the_target() {
        let c = client(None);
        assert_eq!(c.active_probe_id(), None);
        assert_eq!(c.with_active_probe(Some(7)).active_probe_id(), Some(7));
    }

    // ── HTTP-level error/timeout paths (issue #213) ─────────────────────────
    // Exercised against a local wiremock server — offline, no network.
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> ApiClient {
        ApiClient::new(server.uri(), "vng_test".into()).unwrap()
    }

    #[tokio::test]
    async fn unauthorized_maps_to_api_key_hint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let err = client_for(&server).get_api_version().await.unwrap_err();
        assert!(
            err.to_string().contains("api_key"),
            "401 should hint at the api_key, got: {err}"
        );
    }

    #[tokio::test]
    async fn get_error_includes_status_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream boom"))
            .mount(&server)
            .await;
        let err = client_for(&server).get_api_version().await.unwrap_err().to_string();
        assert!(err.contains("500"), "GET error should carry the status: {err}");
        assert!(err.contains("upstream boom"), "GET error should carry the body: {err}");
    }

    #[tokio::test]
    async fn body_error_extracts_json_error_message() {
        // send_with_body path (PATCH): a JSON `error.message` is surfaced verbatim.
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/probe/9"))
            .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
                "error": { "message": "target must be in the same sector" }
            })))
            .mount(&server)
            .await;
        let err = client_for(&server)
            .patch_probe(9, Some("Falling Outside"), None)
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(err, "target must be in the same sector");
    }

    #[tokio::test]
    async fn body_error_without_json_falls_back_to_raw_text() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/probe/9"))
            .respond_with(ResponseTemplate::new(500).set_body_string("plain failure"))
            .mount(&server)
            .await;
        let err = client_for(&server)
            .patch_probe(9, Some("x"), None)
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(err, "plain failure");
    }

    #[tokio::test]
    async fn slow_response_times_out_rather_than_hanging() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;
        // A 200 ms overall timeout must fail fast, not wait for the 5 s response.
        let c = ApiClient::new_with_timeout(server.uri(), "vng_test".into(), Duration::from_millis(200)).unwrap();
        assert!(
            c.get_api_version().await.is_err(),
            "a slow response must time out to an error"
        );
    }
}
