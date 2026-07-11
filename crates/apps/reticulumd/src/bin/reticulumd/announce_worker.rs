use super::announce_ingest::ingest_announce_event;
use super::announce_persistence::{
    spawn_path_table_persistence_worker, PathTablePersistenceContext,
};
use super::bridge::PeerCrypto;
use rns_rpc::RpcDaemon;
use rns_transport::destination::DestinationName;
use rns_transport::discovery::announce::decode_plain_announce;
use rns_transport::discovery::InterfaceDiscoveryStore;
use rns_transport::time::now_epoch_secs_u64;
use rns_transport::transport::AnnounceEvent;
use rns_transport::transport::Transport;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(super) struct DiscoveryWorkerConfig {
    pub storage_path: PathBuf,
    pub allowed_network_ids: Vec<String>,
    pub required_value: u32,
}

pub(super) fn spawn_announce_worker(
    daemon: Arc<RpcDaemon>,
    transport: Arc<Transport>,
    peer_crypto: Arc<Mutex<HashMap<String, PeerCrypto>>>,
    reticulum_storage_path: Option<PathBuf>,
    discovery: Option<DiscoveryWorkerConfig>,
) {
    let daemon_announce = daemon;
    let persist_tx = reticulum_storage_path.as_ref().map(|path| {
        spawn_path_table_persistence_worker(PathTablePersistenceContext::new(
            transport.clone(),
            path.clone(),
        ))
    });
    tokio::spawn(async move {
        let mut rx = transport.recv_announces().await;
        loop {
            if let Ok(event) = rx.recv().await {
                if let Some(config) = discovery.as_ref() {
                    ingest_discovery_announce(&event, config).await;
                }
                ingest_announce_event(daemon_announce.as_ref(), event, peer_crypto.as_ref()).await;
                if let Some(tx) = persist_tx.as_ref() {
                    if let Err(err) = tx.try_send(()) {
                        log::warn!("[daemon] dropped path-table persistence trigger: {err}");
                    }
                }
            }
        }
    });
}

async fn ingest_discovery_announce(event: &AnnounceEvent, config: &DiscoveryWorkerConfig) {
    let discovery_name = DestinationName::new("rnstransport", "discovery.interface");
    let destination = event.destination.lock().await;
    if destination.desc.name.as_name_hash_slice() != discovery_name.as_name_hash_slice() {
        return;
    }
    let network_id = hex::encode(destination.desc.identity.address_hash.as_slice());
    drop(destination);
    let record = match decode_plain_announce(
        event.app_data.as_slice(),
        &network_id,
        &config.allowed_network_ids,
        event.hops,
        now_epoch_secs_u64() as f64,
        config.required_value,
    ) {
        Ok(record) => record,
        Err(error) => {
            log::debug!("[daemon] ignored interface discovery announce: {error}");
            return;
        }
    };
    if let Err(error) = InterfaceDiscoveryStore::new(&config.storage_path).observe(record) {
        log::warn!("[daemon] failed to persist discovered interface: {error}");
    }
}
