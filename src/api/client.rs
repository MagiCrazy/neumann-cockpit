use super::types::{CraftingRecipe, Manny, Probe, ProbeInventory, ProbeMovement, SectorObservation};
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

    async fn patch<T: for<'de> Deserialize<'de>, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let resp = self
            .client
            .patch(self.url(path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .with_context(|| format!("PATCH {path}"))?;

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

        resp.json::<T>().await.with_context(|| format!("Parsing PATCH {path}"))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let resp = self
            .client
            .post(self.url(path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {path}"))?;

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

        resp.json::<T>().await.with_context(|| format!("Parsing POST {path}"))
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
    ) -> Result<Manny> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            object_id: String,
            resources: Vec<String>,
            target_amount: f64,
        }
        #[derive(Deserialize)]
        struct Resp { manny: Manny }
        let path = format!("/api/probe/mannies/{manny_id}/mine");
        Ok(self
            .post::<Resp, _>(&path, &Body {
                object_id: object_id.to_string(),
                resources,
                target_amount,
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
}
