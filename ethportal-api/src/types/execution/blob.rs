use serde::Deserialize;
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
        let offset = <ByteList as Encode>::ssz_fixed_len();
        let mut encoder = SszEncoder::container(buf, offset);

        let bytes: ByteList = ByteList::from(self.blob.clone());
        encoder.append(&bytes);
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use ssz::{Decode, Encode};
    use test_log::test;

    #[test]
    fn decode_encode_blob_with_proofs() {
        let blob = Blob {
            blob: vec![1, 2, 3],
        };
        let blob_bytes = blob.as_ssz_bytes();
        let blob1 = Blob::from_ssz_bytes(&blob_bytes).unwrap();
        assert_eq!(blob_bytes, vec![4, 0, 0, 0, 1, 2, 3]);

        assert_eq! {
            blob,
            blob1,
        };
    }
}
