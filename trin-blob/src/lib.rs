#![warn(clippy::unwrap_used)]

pub mod events;
mod jsonrpc;
pub mod network;
// mod trie;
pub mod utils;
pub mod validation;

use std::sync::Arc;

use discv5::TalkRequest;
use network::BlobNetwork;
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    task::JoinHandle,
    time::{interval, Duration},
};
use tracing::info;
use utp_rs::socket::UtpSocket;

use crate::{events::BlobEvents, jsonrpc::BlobRequestHandler};
use ethportal_api::types::enr::Enr;
use ethportal_api::types::jsonrpc::request::BlobJsonRpcRequest;
use portalnet::{
    discovery::{Discovery, UtpEnr},
    storage::PortalStorageConfig,
    types::messages::PortalnetConfig,
};
use trin_validation::oracle::HeaderOracle;


type BlobHandler = Option<BlobRequestHandler>;
type BlobNetworkTask = Option<JoinHandle<()>>;
type BlobEventTx = Option<mpsc::UnboundedSender<TalkRequest>>;
type BlobJsonRpcTx = Option<mpsc::UnboundedSender<BlobJsonRpcRequest>>;

pub async fn initialize_blob_network(
    discovery: &Arc<Discovery>,
    utp_socket: Arc<UtpSocket<UtpEnr>>,

    portalnet_config: PortalnetConfig,
    storage_config: PortalStorageConfig,
    header_oracle: Arc<RwLock<HeaderOracle>>,
) -> anyhow::Result<(
    BlobHandler,
    BlobNetworkTask,
    BlobEventTx,
    BlobJsonRpcTx,
)> {
    let (blob_jsonrpc_tx, blob_jsonrpc_rx) =
        mpsc::unbounded_channel::<BlobJsonRpcRequest>();
    // TODO:
    // header_oracle.write().await.blob_jsonrpc_tx = Some(blob_jsonrpc_tx.clone());
    let (blob_event_tx, blob_event_rx) = mpsc::unbounded_channel::<TalkRequest>();
    let blob_network = BlobNetwork::new(
        Arc::clone(discovery),
        utp_socket,
        storage_config,
        portalnet_config.clone(),
        header_oracle,
    )
    .await?;
    let blob_handler = BlobRequestHandler {
        network: Arc::new(RwLock::new(blob_network.clone())),
        blob_rx: Arc::new(Mutex::new(blob_jsonrpc_rx)),
    };
    let blob_network = Arc::new(blob_network);
    let blob_network_task =
        spawn_blob_network(blob_network.clone(), portalnet_config, blob_event_rx);
    spawn_blob_heartbeat(blob_network);
    Ok((
        Some(blob_handler),
        Some(blob_network_task),
        Some(blob_event_tx),
        Some(blob_jsonrpc_tx),
    ))
}

pub fn spawn_blob_network(
    network: Arc<BlobNetwork>,
    portalnet_config: PortalnetConfig,
    blob_event_rx: mpsc::UnboundedReceiver<TalkRequest>,
) -> JoinHandle<()> {
    let bootnode_enrs: Vec<Enr> = portalnet_config.bootnodes.into();
    info!(
        "About to spawn Blob Network with {} boot nodes",
        bootnode_enrs.len()
    );

    tokio::spawn(async move {
        let blob_events = BlobEvents {
            network: Arc::clone(&network),
            event_rx: blob_event_rx,
        };

        // Spawn blob event handler
        tokio::spawn(blob_events.start());

        // hacky test: make sure we establish a session with the boot node
        network.overlay.ping_bootnodes().await;

        tokio::signal::ctrl_c()
            .await
            .expect("failed to pause until ctrl-c");
    })
}

pub fn spawn_blob_heartbeat(network: Arc<BlobNetwork>) {
    tokio::spawn(async move {
        let mut heart_interval = interval(Duration::from_millis(30000));

        loop {
            // Don't want to wait to display 1st log, but a bug seems to skip the first wait, so put
            // this wait at the top. Otherwise, we get two log lines immediately on startup.
            heart_interval.tick().await;

            let storage_log = network.overlay.store.read().get_summary_info();
            let message_log = network.overlay.get_message_summary();
            let utp_log = network.overlay.get_utp_summary();
            info!("reports~ data: {storage_log}; msgs: {message_log}");
            info!("reports~ utp: {utp_log}");
        }
    });
}