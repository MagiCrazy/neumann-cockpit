use crate::api::client::ApiClient;
use crate::app::ApiMessage;
use tokio::sync::mpsc;

pub fn fetch_api_version(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(v) = client.get_api_version().await {
            let _ = tx.send(ApiMessage::VersionFetched(v)).await;
        }
    });
}

pub fn fetch_all(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    let c1 = client.clone();
    let tx1 = tx.clone();
    tokio::spawn(async move {
        let msg = match c1.get_probe().await {
            Ok(p) => ApiMessage::ProbeUpdated(p),
            Err(e) => ApiMessage::Error(e.to_string()),
        };
        let _ = tx1.send(msg).await;
    });

    let c2 = client.clone();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        if let Ok(m) = c2.get_mannies().await {
            let _ = tx2.send(ApiMessage::ManniesUpdated(m)).await;
        }
    });

    let c3 = client;
    let tx3 = tx;
    tokio::spawn(async move {
        if let Ok(s) = c3.get_probe_sector().await {
            let _ = tx3.send(ApiMessage::SectorUpdated(s)).await;
        }
    });
}

pub fn fetch_mannies(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(m) = client.get_mannies().await {
            let _ = tx.send(ApiMessage::ManniesUpdated(m)).await;
        }
    });
}

pub fn fetch_move(x: i32, y: i32, z: i32, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.move_probe(x, y, z).await {
            Ok(mv) => ApiMessage::MoveStarted(mv),
            Err(e) => ApiMessage::MoveError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_repair(manny_id: String, integrity_percent: f64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.repair_manny(&manny_id, integrity_percent).await {
            Ok(_) => ApiMessage::RepairStarted,
            Err(e) => ApiMessage::RepairError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_mine(
    manny_id: String,
    object_id: String,
    resources: Vec<String>,
    target_amount: f64,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.mine_manny(&manny_id, &object_id, resources, target_amount).await {
            Ok(_) => ApiMessage::MineStarted,
            Err(e) => ApiMessage::MineError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_jettison(item_id: String, amount: Option<f64>, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.jettison_inventory(&item_id, amount).await {
            Ok(inv) => ApiMessage::JettisonDone(inv),
            Err(e) => ApiMessage::JettisonError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_craft(manny_id: String, recipe: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.craft_manny(&manny_id, &recipe).await {
            Ok(_) => ApiMessage::CraftStarted,
            Err(e) => ApiMessage::CraftError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_crafting_recipes(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(recipes) = client.get_crafting_recipes().await {
            let _ = tx.send(ApiMessage::RecipesFetched(recipes)).await;
        }
    });
}

pub fn fetch_atomic_printer_craft(recipe: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.craft_atomic_printer(&recipe).await {
            Ok(()) => ApiMessage::AtomicPrinterCraftStarted,
            Err(e) => ApiMessage::AtomicPrinterCraftError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_salvage(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.salvage_manny(&manny_id, &object_id).await {
            Ok(_) => ApiMessage::SalvageStarted,
            Err(e) => ApiMessage::SalvageError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_recall(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.recall_manny(&manny_id).await {
            Ok(_) => ApiMessage::RecallStarted,
            Err(e) => ApiMessage::RecallError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_sector(coords: Option<(i32, i32, i32)>, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let result = match coords {
            None => client.get_probe_sector().await,
            Some((x, y, z)) => client.get_sector(x, y, z).await,
        };
        let msg = match result {
            Ok(s) => ApiMessage::SectorUpdated(s),
            Err(e) => ApiMessage::ScanError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_deploy(manny_id: String, object_id: String, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.install_bookmark_manny(&manny_id, &object_id, &name).await {
            Ok(_) => ApiMessage::DeployStarted,
            Err(e) => ApiMessage::DeployError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_inspect(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.inspect_asteroid(&manny_id, &object_id).await {
            Ok(_) => ApiMessage::InspectStarted,
            Err(e) => ApiMessage::InspectError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_recover(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.recover_storage_container(&manny_id, &object_id).await {
            Ok(_) => ApiMessage::RecoverStarted,
            Err(e) => ApiMessage::RecoverError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_detach(
    manny_id: String,
    container_id: String,
    mode: String,
    object_id: Option<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.detach_storage_container(&manny_id, &container_id, &mode, object_id.as_deref()).await {
            Ok(_) => ApiMessage::DetachStarted,
            Err(e) => ApiMessage::DetachError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_rename_manny(manny_id: String, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.rename_manny(&manny_id, &name).await {
            Ok(manny) => ApiMessage::RenameMannyDone(manny),
            Err(e) => ApiMessage::RenameMannyError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}
