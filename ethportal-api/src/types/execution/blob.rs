use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ssz::{Encode, SszDecoderBuilder, SszEncoder};

use crate::types::bytes::ByteList;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Blob {
    pub blob: Vec<u8>,
}

impl ssz::Encode for Blob {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = SszEncoder::container(buf, 0);
        encoder.append(&self.blob);
        encoder.finalize();
    }

    fn ssz_bytes_len(&self) -> usize {
        // TODO: prefix size?
        self.blob.len()
    }
}

impl ssz::Decode for Blob {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = SszDecoderBuilder::new(bytes);

        builder.register_type::<ByteList>()?;

        let mut decoder = builder.build()?;

        let blob: Vec<u8> = decoder.decode_next()?;

        Ok(Self { blob })
    }
}