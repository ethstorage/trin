use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use eth_trie::{EthTrie, MemoryDB, Trie};
use ethereum_types::H256;
use ssz::Decode;
use tokio::sync::RwLock;

use ethportal_api::{
    utils::bytes::hex_encode, CanonicalIndicesContentKey, Header, TransactionIndex,
};
use trin_validation::{oracle::HeaderOracle, validator::Validator};

pub struct CanonicalIndicesValidator {
    pub header_oracle: Arc<RwLock<HeaderOracle>>,
}

#[async_trait]
impl Validator<CanonicalIndicesContentKey> for CanonicalIndicesValidator {
    async fn validate_content(
        &self,
        content_key: &CanonicalIndicesContentKey,
        content: &[u8],
    ) -> anyhow::Result<()>
    where
        CanonicalIndicesContentKey: 'async_trait,
    {
        match content_key {
            CanonicalIndicesContentKey::Transaction(key) => {
                let idx = TransactionIndex::from_ssz_bytes(content)
                    .map_err(|msg| anyhow!("Block Body content has invalid encoding: {:?}", msg))?;
                let _trusted_header: Header = self
                    .header_oracle
                    .read()
                    .await
                    .recursive_find_header_with_proof(H256::from(idx.block_hash))
                    .await?
                    .header;

                let memdb = Arc::new(MemoryDB::new(true));
                let trie = EthTrie::new(memdb);

                let tx_key = rlp::encode(&idx.transaction_index).freeze().to_vec();
                let result =
                    trie.verify_proof(_trusted_header.transactions_root, &tx_key, idx.proof)?;

                match result {
                    None => Err(anyhow!("Content validation failed: Transaction not found in block body")),
                    Some(x) => {
                        let tx_hash = keccak_hash::keccak(x);
                        if tx_hash != H256::from(key.transaction_hash) {
                            return Err(anyhow!("Content validation failed: Invalid tx hash. Found: {tx_hash:?} - Expected: {:?}",
                            hex_encode(key.transaction_hash)));
                        }
                        Ok(())
                    }
                }
            }
        }
    }
}
