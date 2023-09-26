#![warn(clippy::unwrap_used)]

pub mod events;
mod jsonrpc;
pub mod network;
pub mod utils;
pub mod validation;

use std::sync::Arc;

use discv5::TalkRequest;
use network::CanonicalIndicesNetwork;
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    task::JoinHandle,
    time::{interval, Duration},
};
use tracing::info;
use utp_rs::socket::UtpSocket;

use crate::{events::CanonicalIndicesEvents, jsonrpc::CanonicalIndicesRequestHandler};
use ethportal_api::types::enr::Enr;
use ethportal_api::types::jsonrpc::request::CanonicalIndicesJsonRpcRequest;
use portalnet::{
    discovery::{Discovery, UtpEnr},
    storage::PortalStorageConfig,
    types::messages::PortalnetConfig,
};
use trin_validation::oracle::HeaderOracle;


type CanonicalIndicesHandler = Option<CanonicalIndicesRequestHandler>;
type CanonicalIndicesNetworkTask = Option<JoinHandle<()>>;
type CanonicalIndicesEventTx = Option<mpsc::UnboundedSender<TalkRequest>>;
type CanonicalIndicesJsonRpcTx = Option<mpsc::UnboundedSender<CanonicalIndicesJsonRpcRequest>>;

pub async fn initialize_canonical_indices_network(
    discovery: &Arc<Discovery>,
    utp_socket: Arc<UtpSocket<UtpEnr>>,

    portalnet_config: PortalnetConfig,
    storage_config: PortalStorageConfig,
    header_oracle: Arc<RwLock<HeaderOracle>>,
) -> anyhow::Result<(
    CanonicalIndicesHandler,
    CanonicalIndicesNetworkTask,
    CanonicalIndicesEventTx,
    CanonicalIndicesJsonRpcTx,
)> {
    let (canonical_indices_jsonrpc_tx, canonical_indices_jsonrpc_rx) =
        mpsc::unbounded_channel::<CanonicalIndicesJsonRpcRequest>();
    let (canonical_indices_event_tx, canonical_indices_event_rx) = mpsc::unbounded_channel::<TalkRequest>();
    let canonical_indices_network = CanonicalIndicesNetwork::new(
        Arc::clone(discovery),
        utp_socket,
        storage_config,
        portalnet_config.clone(),
        header_oracle,
    )
    .await?;
    let canonical_indices_handler = CanonicalIndicesRequestHandler {
        network: Arc::new(RwLock::new(canonical_indices_network.clone())),
        blob_rx: Arc::new(Mutex::new(canonical_indices_jsonrpc_rx)),
    };
    let canonical_indices_network = Arc::new(canonical_indices_network);
    let canonical_indices_network_task =
        spawn_canonical_indices_network(canonical_indices_network.clone(), portalnet_config, canonical_indices_event_rx);
    spawn_canonical_indices_heartbeat(canonical_indices_network);
    Ok((
        Some(canonical_indices_handler),
        Some(canonical_indices_network_task),
        Some(canonical_indices_event_tx),
        Some(canonical_indices_jsonrpc_tx),
    ))
}

pub fn spawn_canonical_indices_network(
    network: Arc<CanonicalIndicesNetwork>,
    portalnet_config: PortalnetConfig,
    canonical_indices_event_rx: mpsc::UnboundedReceiver<TalkRequest>,
) -> JoinHandle<()> {
    let bootnode_enrs: Vec<Enr> = portalnet_config.bootnodes.into();
    info!(
        "About to spawn CanonicalIndices Network with {} boot nodes",
        bootnode_enrs.len()
    );

    tokio::spawn(async move {
        let canonical_indices_events = CanonicalIndicesEvents {
            network: Arc::clone(&network),
            event_rx: canonical_indices_event_rx,
        };

        // Spawn canonical_indices event handler
        tokio::spawn(canonical_indices_events.start());

        // hacky test: make sure we establish a session with the boot node
        network.overlay.ping_bootnodes().await;

        tokio::signal::ctrl_c()
            .await
            .expect("failed to pause until ctrl-c");
    })
}

pub fn spawn_canonical_indices_heartbeat(network: Arc<CanonicalIndicesNetwork>) {
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