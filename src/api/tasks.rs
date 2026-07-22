use crate::api::client::ApiClient;
use crate::api::types::EndpointId;
use crate::app::ApiMessage;
use std::fmt::Display;
use std::future::Future;
use tokio::sync::mpsc;

/// Spawn an **action** task: await `fut`, map its `Ok`/`Err` to an `ApiMessage`
/// and send the result. This is the shape of every wrapper whose failure must
/// reach the pilot — the error is stringified into the `on_err` message.
///
/// Callers pass an `async move { … }` that owns the client + args, so the future
/// is `'static` and self-contained. `on_ok` is often a variant constructor
/// (`ApiMessage::MoveStarted`) or `|_| ApiMessage::Started` when the value is
/// unused; `on_err` is the matching error-variant constructor.
fn spawn_action<T, E>(
    tx: mpsc::Sender<ApiMessage>,
    fut: impl Future<Output = Result<T, E>> + Send + 'static,
    on_ok: impl FnOnce(T) -> ApiMessage + Send + 'static,
    on_err: impl FnOnce(String) -> ApiMessage + Send + 'static,
) where
    T: Send + 'static,
    E: Display + Send + 'static,
{
    tokio::spawn(async move {
        let msg = match fut.await {
            Ok(v) => on_ok(v),
            Err(e) => on_err(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

/// Spawn a **non-fatal fetch** task: await `fut`, send `on_ok(v)` on success,
/// drop errors silently. For background refreshes (roster, alerts, missions…)
/// whose failure should not disturb the cockpit.
fn spawn_fetch<T, E>(
    tx: mpsc::Sender<ApiMessage>,
    fut: impl Future<Output = Result<T, E>> + Send + 'static,
    on_ok: impl FnOnce(T) -> ApiMessage + Send + 'static,
) where
    T: Send + 'static,
    E: Send + 'static,
{
    tokio::spawn(async move {
        if let Ok(v) = fut.await {
            let _ = tx.send(on_ok(v)).await;
        }
    });
}

pub fn fetch_api_version(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_api_version().await },
        ApiMessage::VersionFetched,
    );
}

pub fn fetch_all(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    // Probe is the one fatal fetch — a failure surfaces as an error toast and
    // drives the refresh backoff.
    let c = client.clone();
    spawn_action(
        tx.clone(),
        async move { c.get_probe().await },
        ApiMessage::ProbeUpdated,
        ApiMessage::Error,
    );

    // Mannies, sector, visited sectors and the fleet roster are all non-fatal.
    let c = client.clone();
    spawn_fetch(
        tx.clone(),
        async move { c.get_mannies().await },
        ApiMessage::ManniesUpdated,
    );
    let c = client.clone();
    spawn_fetch(
        tx.clone(),
        async move { c.get_probe_sector().await },
        ApiMessage::SectorUpdated,
    );
    let c = client.clone();
    spawn_fetch(
        tx.clone(),
        async move { c.get_visited_sectors().await },
        ApiMessage::VisitedSectorsFetched,
    );
    // Fleet roster (API v81 multi-probe): drives the probe switcher.
    let c = client.clone();
    spawn_fetch(
        tx.clone(),
        async move { c.get_probes().await },
        ApiMessage::FleetFetched,
    );

    // Alerts + damage warnings + missions + probe improvements: same non-fatal
    // pattern, each in its own wrapper.
    fetch_alerts(client.clone(), tx.clone());
    fetch_missions(client.clone(), tx.clone());
    fetch_probe_improvements(client.clone(), tx.clone());
    fetch_damage_warnings(client, tx);
}

pub fn fetch_probe_improvements(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_probe_improvements().await },
        ApiMessage::ProbeImprovementsFetched,
    );
}

pub fn fetch_improve_probe(manny_id: String, improvement: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.improve_probe(&manny_id, &improvement).await },
        |_| ApiMessage::ImproveProbeStarted,
        ApiMessage::ImproveProbeError,
    );
}

pub fn fetch_missions(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_missions().await },
        ApiMessage::MissionsFetched,
    );
}

pub fn fetch_messages(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(tx, async move { client.get_messages().await }, |(messages, _pag)| {
        ApiMessage::MessagesFetched(messages)
    });
}

pub fn fetch_sent_messages(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_sent_messages().await },
        |(messages, _pag)| ApiMessage::SentMessagesFetched(messages),
    );
}

pub fn fetch_send_message(
    recipient_type: String,
    recipient_id: EndpointId,
    body: String,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move { client.send_message(&recipient_type, &recipient_id, &body).await },
        ApiMessage::MessageSent,
        ApiMessage::MessageSendError,
    );
}

pub fn fetch_mark_message_read(id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.mark_message_read(id).await },
        ApiMessage::MessageMarkedRead,
        ApiMessage::ActionError,
    );
}

pub fn fetch_abandon_mission(mission_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.abandon_mission(&mission_id).await },
        ApiMessage::MissionAbandoned,
        ApiMessage::MissionAbandonError,
    );
}

pub fn fetch_alerts(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(tx, async move { client.get_alerts().await }, ApiMessage::AlertsFetched);
}

pub fn fetch_damage_warnings(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_damage_warnings().await },
        |(warnings, rule)| ApiMessage::DamageWarningsFetched(warnings, rule),
    );
}

pub fn fetch_ack_alert(id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.ack_alert(id).await },
        ApiMessage::AlertAcknowledged,
        ApiMessage::ActionError,
    );
}

pub fn fetch_ack_damage_warning(id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.ack_damage_warning(id).await },
        ApiMessage::DamageWarningAcknowledged,
        ApiMessage::ActionError,
    );
}

/// Parameters of a storage-move task, grouped into a struct so the wrapper stays
/// under clippy's argument-count threshold (was a 9-arg signature).
pub struct StorageMoveArgs {
    pub actor_manny_id: String,
    pub kind: String,
    pub to_container_id: String,
    pub from_container_id: Option<String>,
    pub resource_type: Option<String>,
    pub amount: Option<f64>,
    pub item_ids: Option<Vec<String>>,
}

pub fn fetch_storage_move(args: StorageMoveArgs, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move {
            client
                .storage_move(
                    &args.actor_manny_id,
                    &args.kind,
                    &args.to_container_id,
                    args.from_container_id.as_deref(),
                    args.resource_type.as_deref(),
                    args.amount,
                    args.item_ids,
                )
                .await
        },
        |(m, inv)| ApiMessage::StorageMoveDone(m, inv),
        ApiMessage::StorageMoveError,
    );
}

pub fn fetch_storage_containers(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_storage_containers().await },
        ApiMessage::StorageContainersFetched,
    );
}

pub fn fetch_storage_container_detail(id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.get_storage_container(&id).await },
        |(c, inv)| ApiMessage::StorageContainerDetailFetched(c, inv),
        ApiMessage::StorageContainerDetailError,
    );
}

pub fn fetch_rename_container(id: String, label: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.rename_storage_container(&id, &label).await },
        |(c, inv)| ApiMessage::RenameContainerDone(c, inv),
        ApiMessage::RenameContainerError,
    );
}

pub fn fetch_update_container_rules(
    id: String,
    priority: Vec<String>,
    exclusion: Vec<String>,
    strict_exclusion: Vec<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move {
            client
                .update_container_rules(&id, priority, exclusion, strict_exclusion)
                .await
        },
        |(c, inv)| ApiMessage::UpdateContainerRulesDone(c, inv),
        ApiMessage::UpdateContainerRulesError,
    );
}

pub fn fetch_mannies(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_mannies().await },
        ApiMessage::ManniesUpdated,
    );
}

pub fn fetch_move(x: i32, y: i32, z: i32, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.move_probe(x, y, z).await },
        ApiMessage::MoveStarted,
        ApiMessage::MoveError,
    );
}

pub fn fetch_repair(manny_id: String, integrity_percent: f64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.repair_manny(&manny_id, integrity_percent).await },
        |_| ApiMessage::RepairStarted,
        ApiMessage::RepairError,
    );
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
    spawn_action(
        tx,
        async move {
            client
                .mine_manny(&manny_id, &object_id, resources, target_amount, target_container_id)
                .await
        },
        |_| ApiMessage::MineStarted,
        ApiMessage::MineError,
    );
}

pub fn fetch_jettison(item_id: String, amount: Option<f64>, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.jettison_inventory(&item_id, amount).await },
        ApiMessage::JettisonDone,
        ApiMessage::JettisonError,
    );
}

pub fn fetch_craft(manny_id: String, recipe: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.craft_manny(&manny_id, &recipe).await },
        |_| ApiMessage::CraftStarted,
        ApiMessage::CraftError,
    );
}

pub fn fetch_crafting_recipes(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_fetch(
        tx,
        async move { client.get_crafting_recipes().await },
        ApiMessage::RecipesFetched,
    );
}

pub fn fetch_atomic_printer_craft(recipe: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.craft_atomic_printer(&recipe).await },
        |()| ApiMessage::AtomicPrinterCraftStarted,
        ApiMessage::AtomicPrinterCraftError,
    );
}

pub fn fetch_salvage(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.salvage_manny(&manny_id, &object_id).await },
        |_| ApiMessage::SalvageStarted,
        ApiMessage::SalvageError,
    );
}

pub fn fetch_recall(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.recall_manny(&manny_id).await },
        |_| ApiMessage::RecallStarted,
        ApiMessage::RecallError,
    );
}

pub fn fetch_sector(coords: Option<(i32, i32, i32)>, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move {
            match coords {
                None => client.get_probe_sector().await,
                Some((x, y, z)) => client.get_sector(x, y, z).await,
            }
        },
        ApiMessage::SectorUpdated,
        ApiMessage::ScanError,
    );
}

pub fn fetch_deploy(
    manny_id: String,
    object_id: String,
    name: String,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move { client.install_bookmark_manny(&manny_id, &object_id, &name).await },
        |_| ApiMessage::DeployStarted,
        ApiMessage::DeployError,
    );
}

pub fn fetch_inspect(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.inspect_sector_object(&manny_id, &object_id).await },
        |_| ApiMessage::InspectStarted,
        ApiMessage::InspectError,
    );
}

pub fn fetch_recover(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.recover_storage_container(&manny_id, &object_id).await },
        |_| ApiMessage::RecoverStarted,
        ApiMessage::RecoverError,
    );
}

pub fn fetch_detach(
    manny_id: String,
    container_id: String,
    mode: String,
    object_id: Option<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move {
            client
                .detach_storage_container(&manny_id, &container_id, &mode, object_id.as_deref())
                .await
        },
        |_| ApiMessage::DetachStarted,
        ApiMessage::DetachError,
    );
}

pub fn fetch_drop_storage_container(
    manny_id: String,
    container_id: String,
    planet_id: String,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move {
            client
                .drop_storage_container_on_planet(&manny_id, &container_id, &planet_id)
                .await
        },
        ApiMessage::DropStorageContainerStarted,
        ApiMessage::DropStorageContainerError,
    );
}

pub fn fetch_assemble_probe(
    manny_id: String,
    container_ids: Vec<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move { client.assemble_probe(&manny_id, &container_ids).await },
        |(m, inv)| ApiMessage::AssembleProbeStarted(m, inv),
        ApiMessage::AssembleProbeError,
    );
}

pub fn fetch_drop_manny_cargo(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.drop_manny_cargo(&manny_id).await },
        ApiMessage::DropMannyCargoStarted,
        ApiMessage::DropMannyCargoError,
    );
}

pub fn fetch_refill_deuterium(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.refill_deuterium_tank(&manny_id).await },
        |_| ApiMessage::DeuteriumRefuelStarted,
        ApiMessage::DeuteriumRefuelError,
    );
}

pub fn fetch_transfer_deuterium(
    manny_id: String,
    target_probe_id: u64,
    amount: f64,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move {
            client
                .transfer_deuterium_to_probe(&manny_id, target_probe_id, amount)
                .await
        },
        |_| ApiMessage::DeuteriumTransferStarted,
        ApiMessage::DeuteriumTransferError,
    );
}

pub fn fetch_transfer_manny(manny_id: String, target_probe_id: u64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.transfer_manny_to_probe(&manny_id, target_probe_id).await },
        |_| ApiMessage::MannyTransferStarted,
        ApiMessage::MannyTransferError,
    );
}

pub fn fetch_scut_network(network_id: i64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.get_scut_network(network_id).await },
        ApiMessage::ScutNetworkFetched,
        ApiMessage::ScutNetworkError,
    );
}

pub fn fetch_turn_on_relay(
    manny_id: String,
    relay_id: i64,
    network_name: Option<String>,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    spawn_action(
        tx,
        async move { client.turn_on_relay(&manny_id, relay_id, network_name.as_deref()).await },
        |_| ApiMessage::ScutRelayTurnedOn,
        ApiMessage::ScutRelayTurnOnError,
    );
}

/// Promote a probe to the player's default (`PATCH /api/probe/{id}`). The
/// server refuses an out-of-reach target with 422, surfaced as `ActionError`.
pub fn fetch_set_default_probe(probe_id: u64, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.patch_probe(probe_id, None, Some(true)).await },
        move |list| ApiMessage::DefaultProbeSet(list, name),
        ApiMessage::ActionError,
    );
}

/// Rename a probe (`PATCH /api/probe/{id}` with `name`, API v81).
pub fn fetch_rename_probe(probe_id: u64, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    // `name` is needed both in the request and in the success message; clone so
    // the future can borrow one copy while the `on_ok` closure owns the other.
    let label = name.clone();
    spawn_action(
        tx,
        async move { client.patch_probe(probe_id, Some(&name), None).await },
        move |list| ApiMessage::ProbeRenamed(list, label),
        ApiMessage::RenameProbeError,
    );
}

pub fn fetch_reassign_mind_snapshot(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.reassign_mind_snapshot().await },
        ApiMessage::MindSnapshotReassigned,
        ApiMessage::MindSnapshotReassignError,
    );
}

pub fn fetch_rename_manny(manny_id: String, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    spawn_action(
        tx,
        async move { client.rename_manny(&manny_id, &name).await },
        ApiMessage::RenameMannyDone,
        ApiMessage::RenameMannyError,
    );
}

#[cfg(test)]
mod tests {
    //! The two spawn helpers are the shared plumbing behind every fetch wrapper
    //! (see #209). These lock the Result -> ApiMessage mapping: actions surface
    //! both branches, fetches surface success and swallow errors.
    use super::*;

    #[tokio::test]
    async fn spawn_action_maps_ok_to_the_success_message() {
        let (tx, mut rx) = mpsc::channel(1);
        spawn_action(
            tx,
            async { Ok::<_, anyhow::Error>(7u32) },
            ApiMessage::VersionFetched,
            ApiMessage::Error,
        );
        assert!(matches!(rx.recv().await, Some(ApiMessage::VersionFetched(7))));
    }

    #[tokio::test]
    async fn spawn_action_maps_err_to_the_stringified_error_message() {
        let (tx, mut rx) = mpsc::channel(1);
        spawn_action(
            tx,
            async { Err::<u32, _>(anyhow::anyhow!("boom")) },
            ApiMessage::VersionFetched,
            ApiMessage::Error,
        );
        let msg = rx.recv().await;
        assert!(
            matches!(&msg, Some(ApiMessage::Error(e)) if e.contains("boom")),
            "err is stringified into the Error message",
        );
    }

    #[tokio::test]
    async fn spawn_fetch_sends_on_ok() {
        let (tx, mut rx) = mpsc::channel(1);
        spawn_fetch(tx, async { Ok::<_, anyhow::Error>(9u32) }, ApiMessage::VersionFetched);
        assert!(matches!(rx.recv().await, Some(ApiMessage::VersionFetched(9))));
    }

    #[tokio::test]
    async fn spawn_fetch_drops_errors_silently() {
        let (tx, mut rx) = mpsc::channel(1);
        spawn_fetch(
            tx,
            async { Err::<u32, _>(anyhow::anyhow!("x")) },
            ApiMessage::VersionFetched,
        );
        // No message is sent; the only sender is dropped, so the channel closes.
        assert!(rx.recv().await.is_none(), "a non-fatal fetch swallows the error");
    }
}
