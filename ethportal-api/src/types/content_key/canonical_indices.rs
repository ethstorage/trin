use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as Sha2Digest, Sha256};
use ssz::{self, Decode, Encode};
use ssz_derive::{Decode, Encode};
use std::fmt;

use crate::types::content_key::error::ContentKeyError;
use crate::types::content_key::overlay::OverlayContentKey;
use crate::utils::bytes::{hex_decode, hex_encode, hex_encode_compact};

/// A content key in the BLOB overlay network.
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
#[ssz(enum_behaviour = "union")]
pub enum CanonicalIndicesContentKey {
    /// A transaction.
    Transaction(TransactionKey),
}

impl Serialize for CanonicalIndicesContentKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for CanonicalIndicesContentKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?.to_lowercase();

        if !data.starts_with("0x") {
            return Err(de::Error::custom(format!(
                "Hex strings must start with 0x, but found {}",
                &data[..2]
            )));
        }

        let ssz_bytes = hex_decode(&data).map_err(de::Error::custom)?;

        CanonicalIndicesContentKey::from_ssz_bytes(&ssz_bytes)
            .map_err(|e| ContentKeyError::DecodeSsz {
                decode_error: e,
                input: hex_encode(ssz_bytes),
            })
            .map_err(serde::de::Error::custom)
    }
}

/// A key for a block header.
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct TransactionKey {
    /// Hash of the transaction.
    pub transaction_hash: [u8; 32],
}

impl From<&CanonicalIndicesContentKey> for Vec<u8> {
    fn from(val: &CanonicalIndicesContentKey) -> Self {
        val.as_ssz_bytes()
    }
}

impl From<CanonicalIndicesContentKey> for Vec<u8> {
    fn from(val: CanonicalIndicesContentKey) -> Self {
        val.as_ssz_bytes()
    }
}

impl TryFrom<Vec<u8>> for CanonicalIndicesContentKey {
    type Error = ContentKeyError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        CanonicalIndicesContentKey::from_ssz_bytes(&value).map_err(|e| ContentKeyError::DecodeSsz {
            decode_error: e,
            input: hex_encode(value),
        })
    }
}

impl fmt::Display for CanonicalIndicesContentKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Transaction(transaction) => format!(
                "Transaction {{ transaction_hash: {} }}",
                hex_encode_compact(transaction.transaction_hash)
            ),
        };

        write!(f, "{s}")
    }
}

impl OverlayContentKey for CanonicalIndicesContentKey {
    fn content_id(&self) -> [u8; 32] {
        let mut sha256 = Sha256::new();
        sha256.update(self.as_ssz_bytes());
        sha256.finalize().into()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        match self {
            CanonicalIndicesContentKey::Transaction(k) => {
                bytes.push(0x00);
                bytes.extend_from_slice(&k.transaction_hash);
            }
        }

        bytes
    }
}

// TODO: Tests