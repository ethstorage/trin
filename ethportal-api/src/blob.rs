use crate::types::content_key::blob::BlobContentKey;
use crate::types::enr::Enr;
use crate::types::portal::FindNodesInfo;
use crate::types::portal::{
    AcceptInfo, DataRadius, PongInfo,
};
use crate::RoutingTableInfo;
use crate::{BlobContentValue, PossibleBlobContentValue};
use discv5::enr::NodeId;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

/// Portal Blob JSON-RPC endpoints
#[rpc(client, server, namespace = "portal")]
pub trait BlobNetworkApi {
    /// Returns meta information about overlay routing table.
    #[method(name = "blobRoutingTableInfo")]
    async fn routing_table_info(&self) -> RpcResult<RoutingTableInfo>;

    /// Returns the node data radios
    #[method(name = "blobRadius")]
    async fn radius(&self) -> RpcResult<DataRadius>;

    /// Write an Ethereum Node Record to the overlay routing table.
    #[method(name = "blobAddEnr")]
    async fn add_enr(&self, enr: Enr) -> RpcResult<bool>;

    /// Fetch the latest ENR associated with the given node ID.
    #[method(name = "blobGetEnr")]
    async fn get_enr(&self, node_id: NodeId) -> RpcResult<Enr>;

    /// Delete Node ID from the overlay routing table.
    #[method(name = "blobDeleteEnr")]
    async fn delete_enr(&self, node_id: NodeId) -> RpcResult<bool>;

    /// Fetch the ENR representation associated with the given Node ID.
    #[method(name = "blobLookupEnr")]
    async fn lookup_enr(&self, node_id: NodeId) -> RpcResult<Enr>;

    /// Send a PING message to the designated node and wait for a PONG response
    #[method(name = "blobPing")]
    async fn ping(&self, enr: Enr) -> RpcResult<PongInfo>;

    /// Send a FINDNODES request for nodes that fall within the given set of distances, to the designated
    /// peer and wait for a response
    #[method(name = "blobFindNodes")]
    async fn find_nodes(&self, enr: Enr, distances: Vec<u16>) -> RpcResult<FindNodesInfo>;

    /// Lookup a target node within in the network
    #[method(name = "blobRecursiveFindNodes")]
    async fn recursive_find_nodes(&self, node_id: NodeId) -> RpcResult<Vec<Enr>>;

    /// Send FINDCONTENT message to get the content with a content key.
    #[method(name = "blobFindContent")]
    async fn find_content(
        &self,
        enr: Enr,
        content_key: BlobContentKey,
    ) -> RpcResult<BlobContentValue>;

    /// Lookup a target content key in the network
    #[method(name = "blobRecursiveFindContent")]
    async fn recursive_find_content(
        &self,
        content_key: BlobContentKey,
    ) -> RpcResult<PossibleBlobContentValue>;

    // /// Lookup a target content key in the network. Return tracing info.
    // #[method(name = "blobTraceRecursiveFindContent")]
    // async fn trace_recursive_find_content(
    //     &self,
    //     content_key: BlobContentKey,
    // ) -> RpcResult<TraceContentInfo>;

    /// Pagination of local content keys
    // #[method(name = "blobPaginateLocalContentKeys")]
    // async fn paginate_local_content_keys(
    //     &self,
    //     offset: u64,
    //     limit: u64,
    // ) -> RpcResult<PaginateLocalContentInfo>;

    /// Send the provided content value to interested peers. Clients may choose to send to some or all peers.
    /// Return the number of peers that the content was gossiped to.
    #[method(name = "blobGossip")]
    async fn gossip(
        &self,
        content_key: BlobContentKey,
        content_value: BlobContentValue,
    ) -> RpcResult<u32>;

    /// Send an OFFER request with given ContentKey, to the designated peer and wait for a response.
    /// Returns the content keys bitlist upon successful content transmission or empty bitlist receive.
    #[method(name = "blobOffer")]
    async fn offer(
        &self,
        enr: Enr,
        content_key: BlobContentKey,
        content_value: Option<BlobContentValue>,
    ) -> RpcResult<AcceptInfo>;

    /// Store content key with a content data to the local database.
    #[method(name = "blobStore")]
    async fn store(
        &self,
        content_key: BlobContentKey,
        content_value: BlobContentValue,
    ) -> RpcResult<bool>;

    /// Get a content from the local database
    #[method(name = "blobLocalContent")]
    async fn local_content(
        &self,
        content_key: BlobContentKey,
    ) -> RpcResult<PossibleBlobContentValue>;
}
