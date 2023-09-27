use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use ethportal_api::BlobContentKey;
use trin_validation::{oracle::HeaderOracle, validator::Validator};

pub struct BlobValidator {
    pub header_oracle: Arc<RwLock<HeaderOracle>>,
}

#[async_trait]
impl Validator<BlobContentKey> for BlobValidator {
    async fn validate_content(
        &self,
        _content_key: &BlobContentKey,
        _content: &[u8],
    ) -> anyhow::Result<()>
    where
        BlobContentKey: 'async_trait,
    {
        // todo: implement blob network validation
        Ok(())
    }
}
