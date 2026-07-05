use crate::api::client::ApiClient;
use crate::api::types::EndpointId;
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

    let c3 = client.clone();
    let tx3 = tx.clone();
    tokio::spawn(async move {
        if let Ok(s) = c3.get_probe_sector().await {
            let _ = tx3.send(ApiMessage::SectorUpdated(s)).await;
        }
    });

    // Non-fatal, like mannies and sector.
    let c4 = client.clone();
    let tx4 = tx.clone();
    tokio::spawn(async move {
        if let Ok(v) = c4.get_visited_sectors().await {
            let _ = tx4.send(ApiMessage::VisitedSectorsFetched(v)).await;
        }
    });

    // Alerts + damage warnings + missions + probe improvements: non-fatal, same
    // pattern as mannies/sector.
    fetch_alerts(client.clone(), tx.clone());
    fetch_missions(client.clone(), tx.clone());
    fetch_probe_improvements(client.clone(), tx.clone());
    fetch_damage_warnings(client, tx);
}

pub fn fetch_probe_improvements(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(improvements) = client.get_probe_improvements().await {
            let _ = tx.send(ApiMessage::ProbeImprovementsFetched(improvements)).await;
        }
    });
}

pub fn fetch_improve_probe(
    manny_id: String,
    improvement: String,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.improve_probe(&manny_id, &improvement).await {
            Ok(_) => ApiMessage::ImproveProbeStarted,
            Err(e) => ApiMessage::ImproveProbeError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_missions(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(m) = client.get_missions().await {
            let _ = tx.send(ApiMessage::MissionsFetched(m)).await;
        }
    });
}

pub fn fetch_messages(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok((messages, _pag)) = client.get_messages().await {
            let _ = tx.send(ApiMessage::MessagesFetched(messages)).await;
        }
    });
}

pub fn fetch_sent_messages(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok((messages, _pag)) = client.get_sent_messages().await {
            let _ = tx.send(ApiMessage::SentMessagesFetched(messages)).await;
        }
    });
}

pub fn fetch_send_message(
    recipient_type: String,
    recipient_id: EndpointId,
    body: String,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.send_message(&recipient_type, &recipient_id, &body).await {
            Ok(m) => ApiMessage::MessageSent(m),
            Err(e) => ApiMessage::MessageSendError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_mark_message_read(id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.mark_message_read(id).await {
            Ok(m) => ApiMessage::MessageMarkedRead(m),
            Err(e) => ApiMessage::ActionError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_abandon_mission(mission_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.abandon_mission(&mission_id).await {
            Ok(m) => ApiMessage::MissionAbandoned(m),
            Err(e) => ApiMessage::MissionAbandonError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_alerts(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(a) = client.get_alerts().await {
            let _ = tx.send(ApiMessage::AlertsFetched(a)).await;
        }
    });
}

pub fn fetch_damage_warnings(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok((warnings, rule)) = client.get_damage_warnings().await {
            let _ = tx.send(ApiMessage::DamageWarningsFetched(warnings, rule)).await;
        }
    });
}

pub fn fetch_ack_alert(id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.ack_alert(id).await {
            Ok(a) => ApiMessage::AlertAcknowledged(a),
            Err(e) => ApiMessage::ActionError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_ack_damage_warning(id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.ack_damage_warning(id).await {
            Ok(w) => ApiMessage::DamageWarningAcknowledged(w),
            Err(e) => ApiMessage::ActionError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

#[allow(clippy::too_many_arguments)]
pub fn fetch_storage_move(
    actor_manny_id: String,
    kind: String,
    to_container_id: String,
    from_container_id: Option<String>,
    resource_type: Option<String>,
    amount: Option<f64>,
    item_ids: Option<Vec<String>>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client
            .storage_move(
                &actor_manny_id,
                &kind,
                &to_container_id,
                from_container_id.as_deref(),
                resource_type.as_deref(),
                amount,
                item_ids,
            )
            .await
        {
            Ok((m, inv)) => ApiMessage::StorageMoveDone(m, inv),
            Err(e) => ApiMessage::StorageMoveError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_storage_containers(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(c) = client.get_storage_containers().await {
            let _ = tx.send(ApiMessage::StorageContainersFetched(c)).await;
        }
    });
}

pub fn fetch_storage_container_detail(id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.get_storage_container(&id).await {
            Ok((c, inv)) => ApiMessage::StorageContainerDetailFetched(c, inv),
            Err(e) => ApiMessage::StorageContainerDetailError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_rename_container(id: String, label: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.rename_storage_container(&id, &label).await {
            Ok((c, inv)) => ApiMessage::RenameContainerDone(c, inv),
            Err(e) => ApiMessage::RenameContainerError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_update_container_rules(
    id: String,
    priority: Vec<String>,
    exclusion: Vec<String>,
    strict_exclusion: Vec<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.update_container_rules(&id, priority, exclusion, strict_exclusion).await {
            Ok((c, inv)) => ApiMessage::UpdateContainerRulesDone(c, inv),
            Err(e) => ApiMessage::UpdateContainerRulesError(e.to_string()),
        };
        let _ = tx.send(msg).await;
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
    target_container_id: Option<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.mine_manny(&manny_id, &object_id, resources, target_amount, target_container_id).await {
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
        let msg = match client.inspect_sector_object(&manny_id, &object_id).await {
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

pub fn fetch_drop_storage_container(
    manny_id: String,
    container_id: String,
    planet_id: String,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client
            .drop_storage_container_on_planet(&manny_id, &container_id, &planet_id)
            .await
        {
            Ok(m) => ApiMessage::DropStorageContainerStarted(m),
            Err(e) => ApiMessage::DropStorageContainerError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_drop_manny_cargo(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.drop_manny_cargo(&manny_id).await {
            Ok(m) => ApiMessage::DropMannyCargoStarted(m),
            Err(e) => ApiMessage::DropMannyCargoError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_refill_deuterium(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.refill_deuterium_tank(&manny_id).await {
            Ok(_) => ApiMessage::DeuteriumRefuelStarted,
            Err(e) => ApiMessage::DeuteriumRefuelError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_scut_network(network_id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.get_scut_network(network_id).await {
            Ok(n) => ApiMessage::ScutNetworkFetched(n),
            Err(e) => ApiMessage::ScutNetworkError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_turn_on_relay(
    manny_id: String,
    relay_id: i64,
    network_name: Option<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.turn_on_relay(&manny_id, relay_id, network_name.as_deref()).await {
            Ok(_) => ApiMessage::ScutRelayTurnedOn,
            Err(e) => ApiMessage::ScutRelayTurnOnError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

pub fn fetch_reassign_mind_snapshot(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.reassign_mind_snapshot().await {
            Ok(probe) => ApiMessage::MindSnapshotReassigned(probe),
            Err(e) => ApiMessage::MindSnapshotReassignError(e.to_string()),
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
