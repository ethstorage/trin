use crate::types::constants::CONTENT_ABSENT;
use crate::types::content_value::ContentValue;
use crate::utils::bytes::{hex_decode, hex_encode};
use crate::{ContentValueError, Blob};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ssz::{Decode, Encode};

/// A Portal Blob content value.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum BlobContentValue {
    Blob(Blob),
}

/// A content response from the RPC server.
///
/// This type allows the RPC response to be non-error,
/// functioning as an Option, but with None serializing to "0x"
/// rather than 'null'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PossibleBlobContentValue {
    ContentPresent(BlobContentValue),
    ContentAbsent,
}

impl Serialize for PossibleBlobContentValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            PossibleBlobContentValue::ContentPresent(content) => content.serialize(serializer),
            PossibleBlobContentValue::ContentAbsent => serializer.serialize_str(CONTENT_ABSENT),
        }
    }
}

impl<'de> Deserialize<'de> for PossibleBlobContentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.as_str() == CONTENT_ABSENT {
            return Ok(PossibleBlobContentValue::ContentAbsent);
        }

        let content_bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;

        if let Ok(value) = Blob::from_ssz_bytes(&content_bytes) {
            return Ok(Self::ContentPresent(
                BlobContentValue::Blob(value),
            ));
        }

        Err(ContentValueError::UnknownContent {
            bytes: s,
            network: "blob".to_string(),
        })
        .map_err(serde::de::Error::custom)
    }
}

impl ContentValue for BlobContentValue {
    fn encode(&self) -> Vec<u8> {
        match self {
            Self::Blob(value) => value.as_ssz_bytes(),
        }
    }

    fn decode(buf: &[u8]) -> Result<Self, ContentValueError> {
        // Catch any attempt to construct a content value from "0x" improperly.
        if buf == CONTENT_ABSENT.to_string().as_bytes() {
            return Err(ContentValueError::DecodeAbsentContent);
        }

        if let Ok(value) = Blob::from_ssz_bytes(buf) {
            return Ok(Self::Blob(value));
        }

        Err(ContentValueError::UnknownContent {
            bytes: hex_encode(buf),
            network: "blob".to_string(),
        })
    }
}

impl Serialize for BlobContentValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = match self {
            Self::Blob(value) => value.as_ssz_bytes(),
        };
        serializer.serialize_str(&hex_encode(encoded))
    }
}

impl<'de> Deserialize<'de> for BlobContentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let content_bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;

        if let Ok(value) = Blob::from_ssz_bytes(&content_bytes) {
            return Ok(Self::Blob(value));
        }

        Err(ContentValueError::UnknownContent {
            bytes: s,
            network: "blob".to_string(),
        })
        .map_err(serde::de::Error::custom)
    }
}


// TODO: test