use crate::errors::RpcServeError;
use crate::serde::from_value;

use crate::jsonrpsee::core::{async_trait, RpcResult};
use discv5::enr::NodeId;
use ethportal_api::types::constants::CONTENT_ABSENT;
use ethportal_api::types::enr::Enr;
use ethportal_api::types::jsonrpc::endpoints::BlobEndpoint;
use ethportal_api::types::jsonrpc::request::BlobJsonRpcRequest;
use ethportal_api::types::portal::{
    AcceptInfo, DataRadius, FindNodesInfo, PongInfo,
};
use ethportal_api::BlobContentKey;
use ethportal_api::BlobContentValue;
use ethportal_api::BlobNetworkApiServer;
use ethportal_api::PossibleBlobContentValue;
use ethportal_api::RoutingTableInfo;
use serde_json::Value;
use tokio::sync::mpsc;

pub struct BlobNetworkApi {
    network: mpsc::UnboundedSender<BlobJsonRpcRequest>,
}

impl BlobNetworkApi {
    pub fn new(network: mpsc::UnboundedSender<BlobJsonRpcRequest>) -> Self {
        Self { network }
    }

    pub async fn proxy_query_to_blob_subnet(
        &self,
        endpoint: BlobEndpoint,
    ) -> Result<Value, RpcServeError> {
        let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<Result<Value, String>>();
        let message = BlobJsonRpcRequest {
            endpoint,
            resp: resp_tx,
        };
        let _ = self.network.send(message);

        match resp_rx.recv().await {
            Some(val) => match val {
                Ok(result) => Ok(result),
                Err(msg) => Err(RpcServeError::Message(msg)),
            },
            None => Err(RpcServeError::Message(
                "Internal error: No response from chain blob subnetwork".to_string(),
            )),
        }
    }
}

#[async_trait]
impl BlobNetworkApiServer for BlobNetworkApi {
    /// Returns meta information about overlay routing table.
    async fn routing_table_info(&self) -> RpcResult<RoutingTableInfo> {
        let endpoint = BlobEndpoint::RoutingTableInfo;
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: RoutingTableInfo = from_value(result)?;
        Ok(result)
    }

    /// Write an Ethereum Node Record to the overlay routing table.
    async fn add_enr(&self, enr: Enr) -> RpcResult<bool> {
        let endpoint = BlobEndpoint::AddEnr(enr);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: bool = from_value(result)?;
        Ok(result)
    }

    /// Fetch the latest ENR associated with the given node ID.
    async fn get_enr(&self, node_id: NodeId) -> RpcResult<Enr> {
        let endpoint = BlobEndpoint::GetEnr(node_id);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: Enr = from_value(result)?;
        Ok(result)
    }

    /// Delete Node ID from the overlay routing table.
    async fn delete_enr(&self, node_id: NodeId) -> RpcResult<bool> {
        let endpoint = BlobEndpoint::DeleteEnr(node_id);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: bool = from_value(result)?;
        Ok(result)
    }

    /// Fetch the ENR representation associated with the given Node ID.
    async fn lookup_enr(&self, node_id: NodeId) -> RpcResult<Enr> {
        let endpoint = BlobEndpoint::LookupEnr(node_id);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: Enr = from_value(result)?;
        Ok(result)
    }

    /// Send a PING message to the designated node and wait for a PONG response
    async fn ping(&self, enr: Enr) -> RpcResult<PongInfo> {
        let endpoint = BlobEndpoint::Ping(enr);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: PongInfo = from_value(result)?;
        Ok(result)
    }

    /// Send a FINDNODES request for nodes that fall within the given set of distances, to the designated
    /// peer and wait for a response
    async fn find_nodes(&self, enr: Enr, distances: Vec<u16>) -> RpcResult<FindNodesInfo> {
        let endpoint = BlobEndpoint::FindNodes(enr, distances);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: FindNodesInfo = from_value(result)?;
        Ok(result)
    }

    /// Lookup a target node within in the network
    async fn recursive_find_nodes(&self, node_id: NodeId) -> RpcResult<Vec<Enr>> {
        let endpoint = BlobEndpoint::RecursiveFindNodes(node_id);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: Vec<Enr> = from_value(result)?;
        Ok(result)
    }

    /// Lookup a target node within in the network
    async fn radius(&self) -> RpcResult<DataRadius> {
        let endpoint = BlobEndpoint::DataRadius;
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: DataRadius = from_value(result)?;
        Ok(result)
    }

    /// Send FINDCONTENT message to get the content with a content key.
    async fn find_content(
        &self,
        enr: Enr,
        content_key: BlobContentKey,
    ) -> RpcResult<BlobContentValue> {
        let endpoint = BlobEndpoint::FindContent(enr, content_key);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: BlobContentValue = from_value(result)?;
        Ok(result)
    }

    /// Lookup a target content key in the network
    async fn recursive_find_content(
        &self,
        content_key: BlobContentKey,
    ) -> RpcResult<PossibleBlobContentValue> {
        let endpoint = BlobEndpoint::RecursiveFindContent(content_key);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        if result == serde_json::Value::String(CONTENT_ABSENT.to_string()) {
            return Ok(PossibleBlobContentValue::ContentAbsent);
        };
        let result: PossibleBlobContentValue = from_value(result)?;
        Ok(result)
    }

    /// Lookup a target content key in the network. Return tracing info.
    // async fn trace_recursive_find_content(
    //     &self,
    //     content_key: BlobContentKey,
    // ) -> RpcResult<TraceContentInfo> {
    //     let endpoint = BlobEndpoint::TraceRecursiveFindContent(content_key);
    //     let result = self.proxy_query_to_blob_subnet(endpoint).await?;
    //     let info: TraceContentInfo = from_value(result)?;
    //     Ok(info)
    // }

    // /// Pagination of local content keys
    // async fn paginate_local_content_keys(
    //     &self,
    //     offset: u64,
    //     limit: u64,
    // ) -> RpcResult<PaginateLocalContentInfo> {
    //     let endpoint = BlobEndpoint::PaginateLocalContentKeys(offset, limit);
    //     let result = self.proxy_query_to_blob_subnet(endpoint).await?;
    //     let result: PaginateLocalContentInfo = from_value(result)?;
    //     Ok(result)
    // }

    /// Send the provided content to interested peers. Clients may choose to send to some or all peers.
    /// Return the number of peers that the content was gossiped to.
    async fn gossip(
        &self,
        content_key: BlobContentKey,
        content_value: BlobContentValue,
    ) -> RpcResult<u32> {
        let endpoint = BlobEndpoint::Gossip(content_key, content_value);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: u32 = from_value(result)?;
        Ok(result)
    }

    /// Send an OFFER request with given ContentKey, to the designated peer and wait for a response.
    /// Returns the content keys bitlist upon successful content transmission or empty bitlist receive.
    async fn offer(
        &self,
        enr: Enr,
        content_key: BlobContentKey,
        content_value: Option<BlobContentValue>,
    ) -> RpcResult<AcceptInfo> {
        let endpoint = BlobEndpoint::Offer(enr, content_key, content_value);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: AcceptInfo = from_value(result)?;
        Ok(result)
    }

    /// Store content key with a content data to the local database.
    async fn store(
        &self,
        content_key: BlobContentKey,
        content_value: BlobContentValue,
    ) -> RpcResult<bool> {
        let endpoint = BlobEndpoint::Store(content_key, content_value);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        let result: bool = from_value(result)?;
        Ok(result)
    }

    /// Get a content from the local database.
    async fn local_content(
        &self,
        content_key: BlobContentKey,
    ) -> RpcResult<PossibleBlobContentValue> {
        let endpoint = BlobEndpoint::LocalContent(content_key);
        let result = self.proxy_query_to_blob_subnet(endpoint).await?;
        if result == serde_json::Value::String(CONTENT_ABSENT.to_string()) {
            return Ok(PossibleBlobContentValue::ContentAbsent);
        };
        let content: BlobContentValue = from_value(result)?;
        Ok(PossibleBlobContentValue::ContentPresent(content))
    }
}

impl std::fmt::Debug for BlobNetworkApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobNetworkApi").finish_non_exhaustive()
    }
}
