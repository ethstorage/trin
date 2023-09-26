#![warn(clippy::unwrap_used)]

mod beacon_rpc;
mod builder;
mod cors;
mod discv5_rpc;
mod errors;
mod eth_rpc;
mod history_rpc;
mod rpc_server;
mod serde;
mod web3_rpc;
mod canonical_indices_rpc;

use crate::jsonrpsee::server::ServerBuilder;
pub use crate::rpc_server::RpcServerHandle;
use beacon_rpc::BeaconNetworkApi;
pub use builder::{PortalRpcModule, RpcModuleBuilder, TransportRpcModuleConfig};
use discv5_rpc::Discv5Api;
use errors::RpcError;
use eth_rpc::EthApi;
use ethportal_api::jsonrpsee;
use ethportal_api::types::cli::{
    TrinConfig, Web3TransportType, BEACON_NETWORK, HISTORY_NETWORK, STATE_NETWORK, CANONICAL_INDICES_NETWORK
};
use ethportal_api::types::jsonrpc::request::{
    BeaconJsonRpcRequest, HistoryJsonRpcRequest, StateJsonRpcRequest, CanonicalIndicesJsonRpcRequest,
};
use history_rpc::HistoryNetworkApi;
use web3_rpc::Web3Api;
use canonical_indices_rpc::CanonicalIndicesNetworkApi;

use crate::rpc_server::RpcServerConfig;
use portalnet::discovery::Discovery;
use reth_ipc::server::Builder as IpcServerBuilder;
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn launch_jsonrpc_server(
    trin_config: TrinConfig,
    discv5: Arc<Discovery>,
    history_handler: Option<mpsc::UnboundedSender<HistoryJsonRpcRequest>>,
    state_handler: Option<mpsc::UnboundedSender<StateJsonRpcRequest>>,
    beacon_handler: Option<mpsc::UnboundedSender<BeaconJsonRpcRequest>>,
    canonical_indices_handler: Option<mpsc::UnboundedSender<CanonicalIndicesJsonRpcRequest>>,
) -> Result<RpcServerHandle, RpcError> {
    // Discv5 and Web3 modules are enabled with every network
    let mut modules = vec![PortalRpcModule::Discv5, PortalRpcModule::Web3];

    for network in trin_config.networks.iter() {
        match network.as_str() {
            HISTORY_NETWORK => {
                modules.push(PortalRpcModule::History);
                modules.push(PortalRpcModule::Eth);
            }
            STATE_NETWORK => {
                // not implemented
            }
            BEACON_NETWORK => modules.push(PortalRpcModule::Beacon),
            CANONICAL_INDICES_NETWORK => {
                modules.push(PortalRpcModule::CanonicalIndices);
            },
            _ => panic!("Unexpected network type: {}", network),
        }
    }

    let handle: RpcServerHandle = match trin_config.web3_transport {
        Web3TransportType::IPC => {
            let transport = TransportRpcModuleConfig::default().with_ipc(modules);
            let transport_modules = RpcModuleBuilder::new(discv5)
                .maybe_with_history(history_handler)
                .maybe_with_beacon(beacon_handler)
                .maybe_with_state(state_handler)
                .maybe_with_canonical_indices(canonical_indices_handler)
                .build(transport);

            RpcServerConfig::default()
                .with_ipc_endpoint(
                    trin_config
                        .web3_ipc_path
                        .to_str()
                        .expect("Path should be string"),
                )
                .with_ipc(IpcServerBuilder::default())
                .start(transport_modules)
                .await?
        }
        Web3TransportType::HTTP => {
            let transport = TransportRpcModuleConfig::default().with_http(modules);
            let transport_modules = RpcModuleBuilder::new(discv5)
                .maybe_with_history(history_handler)
                .maybe_with_beacon(beacon_handler)
                .maybe_with_state(state_handler)
                .maybe_with_canonical_indices(canonical_indices_handler)
                .build(transport);

            RpcServerConfig::default()
                .with_http_address(
                    trin_config
                        .web3_http_address
                        .socket_addrs(|| None)
                        .expect("Invalid socket address")[0],
                )
                .with_http(ServerBuilder::default())
                .with_ws(ServerBuilder::default())
                .start(transport_modules)
                .await?
        }
    };

    Ok(handle)
}
