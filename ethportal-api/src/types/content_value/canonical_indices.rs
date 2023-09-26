use crate::types::constants::CONTENT_ABSENT;
use crate::types::content_value::ContentValue;
use crate::utils::bytes::{hex_decode, hex_encode};
use crate::{TransactionIndex, ContentValueError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ssz::{Decode, Encode};

/// A Portal Transaction content value.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum CanonicalIndicesContentValue {
    TransactionIndex(TransactionIndex),
}

/// A content response from the RPC server.
///
/// This type allows the RPC response to be non-error,
/// functioning as an Option, but with None serializing to "0x"
/// rather than 'null'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PossibleCanonicalIndicesContentValue {
    ContentPresent(CanonicalIndicesContentValue),
    ContentAbsent,
}

impl Serialize for PossibleCanonicalIndicesContentValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            PossibleCanonicalIndicesContentValue::ContentPresent(content) => content.serialize(serializer),
            PossibleCanonicalIndicesContentValue::ContentAbsent => serializer.serialize_str(CONTENT_ABSENT),
        }
    }
}

impl<'de> Deserialize<'de> for PossibleCanonicalIndicesContentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.as_str() == CONTENT_ABSENT {
            return Ok(PossibleCanonicalIndicesContentValue::ContentAbsent);
        }

        let content_bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;

        if let Ok(value) = TransactionIndex::from_ssz_bytes(&content_bytes) {
            return Ok(Self::ContentPresent(CanonicalIndicesContentValue::TransactionIndex(value)));
        }

        Err(ContentValueError::UnknownContent {
            bytes: s,
            network: "canonicalIndices".to_string(),
        })
        .map_err(serde::de::Error::custom)
    }
}

impl ContentValue for CanonicalIndicesContentValue {
    fn encode(&self) -> Vec<u8> {
        match self {
            Self::TransactionIndex(value) => value.as_ssz_bytes(),
        }
    }

    fn decode(buf: &[u8]) -> Result<Self, ContentValueError> {
        // Catch any attempt to construct a content value from "0x" improperly.
        if buf == CONTENT_ABSENT.to_string().as_bytes() {
            return Err(ContentValueError::DecodeAbsentContent);
        }

        if let Ok(value) = TransactionIndex::from_ssz_bytes(buf) {
            return Ok(Self::TransactionIndex(value));
        }

        Err(ContentValueError::UnknownContent {
            bytes: hex_encode(buf),
            network: "canonicalIndices".to_string(),
        })
    }
}

impl Serialize for CanonicalIndicesContentValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = match self {
            Self::TransactionIndex(value) => value.as_ssz_bytes(),
        };
        serializer.serialize_str(&hex_encode(encoded))
    }
}

impl<'de> Deserialize<'de> for CanonicalIndicesContentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let content_bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;

        if let Ok(value) = TransactionIndex::from_ssz_bytes(&content_bytes) {
            return Ok(Self::TransactionIndex(value));
        }

        Err(ContentValueError::UnknownContent {
            bytes: s,
            network: "canonicalIndices".to_string(),
        })
        .map_err(serde::de::Error::custom)
    }
}

// TODO: test
#[cfg(test)]
mod test {
    use ethereum_types::H256;

    use super::*;

    use crate::CanonicalIndicesContentValue;

    #[test]
    fn content_value_deserialization_failure_displays_debuggable_data() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let item_result = CanonicalIndicesContentValue::decode(&data);
        let error = item_result.unwrap_err();
        // Test the error Debug representation
        assert_eq!(
            error,
            ContentValueError::UnknownContent {
                bytes: "0x010203040506070809".to_string(),
                network: "canonicalIndices".to_string()
            }
        );
        // Test the error Display representation.
        assert_eq!(
            error.to_string(),
            "could not determine content type of 0x010203040506070809 from canonicalIndices network"
        );
    }

    #[test]
    fn content_value_deserialization_displays_debuggable_data() {
        let item = CanonicalIndicesContentValue::TransactionIndex(TransactionIndex {
            block_hash:  H256::from_slice(&hex_decode("0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6").unwrap()),
            transaction_index: 1,
            proof: vec![vec![2, 3]]
        });
        let data = item.encode();
        let result = CanonicalIndicesContentValue::decode(&data);
        let item1 = result.unwrap();

        // Test decoded one equals the original one
        assert_eq!(item, item1);
        // Test the raw data
        assert_eq!(data, hex_decode("0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6").unwrap());
    }
}
