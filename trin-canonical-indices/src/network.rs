use std::sync::Arc;

use parking_lot::RwLock as PLRwLock;
use tokio::sync::RwLock;
use utp_rs::socket::UtpSocket;

use ethportal_api::types::distance::XorMetric;
use ethportal_api::types::enr::Enr;
use ethportal_api::CanonicalIndicesContentKey;
use portalnet::{
    discovery::{Discovery, UtpEnr},
    overlay::{OverlayConfig, OverlayProtocol},
    storage::{PortalStorage, PortalStorageConfig},
    types::messages::{PortalnetConfig, ProtocolId},
};
use trin_validation::oracle::HeaderOracle;

use crate::validation::CanonicalIndicesValidator;

/// CanonicalIndices network layer on top of the overlay protocol. Encapsulates CanonicalIndices network specific data and logic.
#[derive(Clone)]
pub struct CanonicalIndicesNetwork {
    pub overlay:
        Arc<OverlayProtocol<CanonicalIndicesContentKey, XorMetric, CanonicalIndicesValidator, PortalStorage>>,
}

impl CanonicalIndicesNetwork {
    pub async fn new(
        discovery: Arc<Discovery>,
        utp_socket: Arc<UtpSocket<UtpEnr>>,
        storage_config: PortalStorageConfig,
        portal_config: PortalnetConfig,
        header_oracle: Arc<RwLock<HeaderOracle>>,
    ) -> anyhow::Result<Self> {
        let bootnode_enrs: Vec<Enr> = portal_config.bootnodes.into();
        let config = OverlayConfig {
            bootnode_enrs,
            ..Default::default()
        };
        let storage = Arc::new(PLRwLock::new(PortalStorage::new(
            storage_config,
            ProtocolId::CanonicalIndices,
        )?));
        let validator = Arc::new(CanonicalIndicesValidator { header_oracle });
        let overlay = OverlayProtocol::new(
            config,
            discovery,
            utp_socket,
            storage,
            ProtocolId::CanonicalIndices,
            validator,
        )
        .await;

        Ok(Self {
            overlay: Arc::new(overlay),
        })
    }
}
