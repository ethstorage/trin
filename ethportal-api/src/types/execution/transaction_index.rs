use ethereum_types::H256;
use serde::{Serialize, Deserialize};
use ssz_derive::{Decode, Encode};


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct TransactionIndex {
    pub block_hash: H256,
    pub transaction_index: u64,
    pub proof: Vec<Vec<u8>>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::utils::bytes::hex_decode;

    use super::*;

    use ssz::{Decode, Encode};
    use test_log::test;

    #[test]
    fn decode_encode_transacion_index() {
        let idx = TransactionIndex {
            block_hash : H256::from_slice(
                // https://etherscan.io/block/1
                &hex_decode("0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6")
                    .unwrap()
            ),
            transaction_index: 0,
            proof: vec![vec![1, 2], vec![3, 4]],
        };
        let idx_bytes = idx.as_ssz_bytes();
        let idx1 = TransactionIndex::from_ssz_bytes(&idx_bytes).unwrap();

        assert_eq! {
            idx,
            idx1,
        };
    }
}
