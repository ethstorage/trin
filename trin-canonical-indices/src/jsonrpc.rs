use std::sync::Arc;

use discv5::enr::NodeId;
use ethportal_api::types::{
    constants::CONTENT_ABSENT, jsonrpc::endpoints::CanonicalIndicesEndpoint,
    jsonrpc::request::CanonicalIndicesJsonRpcRequest, query_trace::QueryTrace,
};
use ethportal_api::utils::bytes::hex_encode;
use ethportal_api::{
    types::portal::{AcceptInfo, ContentInfo, FindNodesInfo, PongInfo, TraceContentInfo},
    ContentValue, {CanonicalIndicesContentKey, OverlayContentKey, RawContentKey},
};
use portalnet::storage::ContentStore;
use portalnet::types::messages::Content;
use serde_json::{json, Value};
use ssz::Encode;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::error;

use crate::network::CanonicalIndicesNetwork;
use crate::utils::bucket_entries_to_json;

/// Handles CanonicalIndices network JSON-RPC requests
pub struct CanonicalIndicesRequestHandler {
    pub network: Arc<RwLock<CanonicalIndicesNetwork>>,
    pub blob_rx: Arc<Mutex<mpsc::UnboundedReceiver<CanonicalIndicesJsonRpcRequest>>>,
}

impl CanonicalIndicesRequestHandler {
    /// Complete RPC requests for the CanonicalIndices network.
    pub async fn handle_client_queries(&self) {
        let blob_rx = self.blob_rx.clone();
        while let Some(request) = blob_rx.lock().await.recv().await {
            let network = self.network.clone();
            tokio::spawn(async move { complete_request(network, request).await });
        }
    }
}

/// Generates a response for a given request and sends it to the receiver.
async fn complete_request(network: Arc<RwLock<CanonicalIndicesNetwork>>, request: CanonicalIndicesJsonRpcRequest) {
    let response: Result<Value, String> = match request.endpoint {
        CanonicalIndicesEndpoint::LocalContent(content_key) => local_content(network, content_key).await,
        CanonicalIndicesEndpoint::PaginateLocalContentKeys(offset, limit) => {
            paginate_local_content_keys(network, offset, limit).await
        }
        CanonicalIndicesEndpoint::Store(content_key, content_value) => {
            store(network, content_key, content_value).await
        }
        CanonicalIndicesEndpoint::AddEnr(enr) => add_enr(network, enr).await,
        CanonicalIndicesEndpoint::DataRadius => {
            let radius = network.read().await.overlay.data_radius();
            Ok(json!(*radius))
        }
        CanonicalIndicesEndpoint::DeleteEnr(node_id) => delete_enr(network, node_id).await,
        CanonicalIndicesEndpoint::FindContent(enr, content_key) => {
            find_content(network, enr, content_key).await
        }
        CanonicalIndicesEndpoint::FindNodes(enr, distances) => find_nodes(network, enr, distances).await,
        CanonicalIndicesEndpoint::GetEnr(node_id) => get_enr(network, node_id).await,
        CanonicalIndicesEndpoint::LookupEnr(node_id) => lookup_enr(network, node_id).await,
        CanonicalIndicesEndpoint::Offer(enr, content_key, content_value) => {
            offer(network, enr, content_key, content_value).await
        }
        CanonicalIndicesEndpoint::Ping(enr) => ping(network, enr).await,
        CanonicalIndicesEndpoint::RoutingTableInfo => Ok(bucket_entries_to_json(
            network.read().await.overlay.bucket_entries(),
        )),
        CanonicalIndicesEndpoint::RecursiveFindNodes(node_id) => recursive_find_nodes(network, node_id).await,
        CanonicalIndicesEndpoint::RecursiveFindContent(content_key) => {
            recursive_find_content(network, content_key, false).await
        },
        CanonicalIndicesEndpoint::TraceRecursiveFindContent(content_key) => {
            recursive_find_content(network, content_key, true).await
        }
        CanonicalIndicesEndpoint::Gossip(content_key, content_value) => {
            gossip(network, content_key, content_value).await
        }
    };
    let _ = request.resp.send(response);
}

/// Constructs a JSON call for the RecursiveFindContent method.
async fn recursive_find_content(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    content_key: CanonicalIndicesContentKey,
    is_trace: bool,
) -> Result<Value, String> {
    // Check whether we have the data locally.
    let overlay = network.read().await.overlay.clone();
    let local_content: Option<Vec<u8>> = match overlay.store.read().get(&content_key) {
        Ok(Some(data)) => Some(data),
        Ok(None) => None,
        Err(err) => {
            error!(
                error = %err,
                content.key = %content_key,
                "Error checking data store for content",
            );
            None
        }
    };
    let (possible_content_bytes, utp_transfer, trace) = match local_content {
        Some(val) => {
            let local_enr = overlay.local_enr();
            let mut trace = QueryTrace::new(
                &overlay.local_enr(),
                NodeId::new(&content_key.content_id()).into(),
            );
            trace.node_responded_with_content(&local_enr);
            (Some(val), false, if is_trace { Some(trace) } else { None })
        }
        None => overlay.lookup_content(content_key.clone(), is_trace).await,
    };

    // Format as string.
    let content_response_string = match possible_content_bytes {
        Some(bytes) => Value::String(hex_encode(bytes)),
        None => Value::String(CONTENT_ABSENT.to_string()), // "0x"
    };

    // If tracing is not required, return content.
    if !is_trace {
        return Ok(json!(ContentInfo::Content {
            content: serde_json::from_value(content_response_string).map_err(|e| e.to_string())?,
            utp_transfer,
        }));
    }
    if let Some(trace) = trace {
        Ok(json!(TraceContentInfo {
            content: serde_json::from_value(content_response_string).map_err(|e| e.to_string())?,
            utp_transfer,
            trace,
        }))
    } else {
        Err("Content query trace requested but none provided.".to_owned())
    }
}

/// Constructs a JSON call for the LocalContent method.
async fn local_content(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    content_key: CanonicalIndicesContentKey,
) -> Result<Value, String> {
    let store = network.read().await.overlay.store.clone();
    let response = match store.read().get(&content_key)
        {
            Ok(val) => match val {
                Some(val) => {
                    Ok(Value::String(hex_encode(val)))
                }
                None => {
                    Ok(Value::String(CONTENT_ABSENT.to_string()))
                }
            },
            Err(err) => Err(format!(
                "Database error while looking for content key in local storage: {content_key:?}, with error: {err}",
            )),
        };
    response
}

/// Constructs a JSON call for the PaginateLocalContentKeys method.
async fn paginate_local_content_keys(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    offset: u64,
    limit: u64,
) -> Result<Value, String> {
    let store = network.read().await.overlay.store.clone();
    let response = match store.read().paginate(&offset, &limit)
        {
            Ok(val) => Ok(json!(val)),
            Err(err) => Err(format!(
                "Database error while paginating local content keys with offset: {offset:?}, limit: {limit:?}. Error message: {err}"
            )),
        };
    response
}

/// Constructs a JSON call for the Store method.
async fn store(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    content_key: CanonicalIndicesContentKey,
    content_value: ethportal_api::CanonicalIndicesContentValue,
) -> Result<Value, String> {
    let data = content_value.encode();
    let store = network.read().await.overlay.store.clone();
    let response = match store
        .write()
        .put::<CanonicalIndicesContentKey, Vec<u8>>(content_key, data)
    {
        Ok(_) => Ok(Value::Bool(true)),
        Err(err) => Ok(Value::String(err.to_string())),
    };
    response
}

/// Constructs a JSON call for the AddEnr method.
async fn add_enr(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    enr: discv5::enr::Enr<discv5::enr::CombinedKey>,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    match overlay.add_enr(enr) {
        Ok(_) => Ok(json!(true)),
        Err(err) => Err(format!("AddEnr failed: {err:?}")),
    }
}

/// Constructs a JSON call for the GetEnr method.
async fn get_enr(network: Arc<RwLock<CanonicalIndicesNetwork>>, node_id: NodeId) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    match overlay.get_enr(node_id) {
        Ok(enr) => Ok(json!(enr)),
        Err(err) => Err(format!("GetEnr failed: {err:?}")),
    }
}

/// Constructs a JSON call for the deleteEnr method.
async fn delete_enr(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    node_id: NodeId,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    let is_deleted = overlay.delete_enr(node_id);
    Ok(json!(is_deleted))
}

/// Constructs a JSON call for the LookupEnr method.
async fn lookup_enr(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    node_id: NodeId,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    match overlay.lookup_enr(node_id).await {
        Ok(enr) => Ok(json!(enr)),
        Err(err) => Err(format!("LookupEnr failed: {err:?}")),
    }
}

/// Constructs a JSON call for the FindContent method.
async fn find_content(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    enr: discv5::enr::Enr<discv5::enr::CombinedKey>,
    content_key: CanonicalIndicesContentKey,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    match overlay.send_find_content(enr, content_key.into()).await {
        Ok((content, utp_transfer)) => match content {
            Content::ConnectionId(id) => Err(format!(
                "FindContent request returned a connection id ({id:?}) instead of conducting utp transfer."
            )),
            Content::Content(content) => Ok(json!({
                "content": hex_encode(content),
                "utpTransfer": utp_transfer,
            })),
            Content::Enrs(enrs) => Ok(json!({
                "enrs": enrs,
            })),
        },
        Err(msg) => Err(format!("FindContent request timeout: {msg:?}")),
    }
}

/// Constructs a JSON call for the FindNodes method.
async fn find_nodes(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    enr: discv5::enr::Enr<discv5::enr::CombinedKey>,
    distances: Vec<u16>,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    match overlay.send_find_nodes(enr, distances).await {
        Ok(nodes) => Ok(json!(nodes
            .enrs
            .into_iter()
            .map(|enr| enr.into())
            .collect::<FindNodesInfo>())),
        Err(msg) => Err(format!("FindNodes request timeout: {msg:?}")),
    }
}

/// Constructs a JSON call for the Gossip method.
async fn gossip(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    content_key: CanonicalIndicesContentKey,
    content_value: ethportal_api::CanonicalIndicesContentValue,
) -> Result<Value, String> {
    let data = content_value.encode();
    let content_values = vec![(content_key, data)];
    let overlay = network.read().await.overlay.clone();
    let num_peers = overlay.propagate_gossip(content_values);
    Ok(num_peers.into())
}

/// Constructs a JSON call for the Offer method.
async fn offer(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    enr: discv5::enr::Enr<discv5::enr::CombinedKey>,
    content_key: CanonicalIndicesContentKey,
    content_value: Option<ethportal_api::CanonicalIndicesContentValue>,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    if let Some(content_value) = content_value {
        let content_value = content_value.encode();
        match overlay
            .send_populated_offer(enr, content_key.into(), content_value)
            .await
        {
            Ok(accept) => Ok(json!(AcceptInfo {
                content_keys: accept.content_keys,
            })),
            Err(msg) => Err(format!("Populated Offer request timeout: {msg:?}")),
        }
    } else {
        let content_key: Vec<RawContentKey> = vec![content_key.as_ssz_bytes()];
        match overlay.send_offer(content_key, enr).await {
            Ok(accept) => Ok(json!(AcceptInfo {
                content_keys: accept.content_keys,
            })),
            Err(msg) => Err(format!("Offer request timeout: {msg:?}")),
        }
    }
}

/// Constructs a JSON call for the Ping method.
async fn ping(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    enr: discv5::enr::Enr<discv5::enr::CombinedKey>,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    match overlay.send_ping(enr).await {
        Ok(pong) => Ok(json!(PongInfo {
            enr_seq: pong.enr_seq as u32,
            data_radius: *overlay.data_radius(),
        })),
        Err(msg) => Err(format!("Ping request timeout: {msg:?}")),
    }
}

/// Constructs a JSON call for the RecursiveFindNodes method.
async fn recursive_find_nodes(
    network: Arc<RwLock<CanonicalIndicesNetwork>>,
    node_id: NodeId,
) -> Result<Value, String> {
    let overlay = network.read().await.overlay.clone();
    let nodes = overlay.lookup_node(node_id).await;
    Ok(json!(nodes))
}
