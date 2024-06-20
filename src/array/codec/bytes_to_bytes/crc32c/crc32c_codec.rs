use crate::{
    array::{
        codec::{
            BytesPartialDecoderTraits, BytesToBytesCodecTraits, CodecError, CodecOptions,
            CodecTraits, RecommendedConcurrency,
        },
        ArrayMetadataOptions, BytesRepresentation,
    },
    metadata::v3::MetadataV3,
};

#[cfg(feature = "async")]
use crate::array::codec::AsyncBytesPartialDecoderTraits;

use super::{
    crc32c_partial_decoder, Crc32cCodecConfiguration, Crc32cCodecConfigurationV1, CHECKSUM_SIZE,
    IDENTIFIER,
};

/// A `crc32c` (CRC32C checksum) codec implementation.
#[derive(Clone, Debug, Default)]
pub struct Crc32cCodec;

impl Crc32cCodec {
    /// Create a new `crc32c` codec.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Create a new `crc32c` codec.
    #[must_use]
    pub const fn new_with_configuration(_configuration: &Crc32cCodecConfiguration) -> Self {
        Self {}
    }
}

impl CodecTraits for Crc32cCodec {
    fn create_metadata_opt(&self, _options: &ArrayMetadataOptions) -> Option<MetadataV3> {
        let configuration = Crc32cCodecConfigurationV1 {};
        Some(MetadataV3::new_with_serializable_configuration(IDENTIFIER, &configuration).unwrap())
    }

    fn partial_decoder_should_cache_input(&self) -> bool {
        false
    }

    fn partial_decoder_decodes_all(&self) -> bool {
        false
    }
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl BytesToBytesCodecTraits for Crc32cCodec {
    fn recommended_concurrency(
        &self,
        _decoded_representation: &BytesRepresentation,
    ) -> Result<RecommendedConcurrency, CodecError> {
        Ok(RecommendedConcurrency::new_maximum(1))
    }

    fn encode(
        &self,
        mut decoded_value: Vec<u8>,
        _options: &CodecOptions,
    ) -> Result<Vec<u8>, CodecError> {
        let checksum = crc32c::crc32c(&decoded_value).to_le_bytes();
        decoded_value.reserve_exact(checksum.len());
        decoded_value.extend(&checksum);
        Ok(decoded_value)
    }

    fn decode(
        &self,
        mut encoded_value: Vec<u8>,
        _decoded_representation: &BytesRepresentation,
        options: &CodecOptions,
    ) -> Result<Vec<u8>, CodecError> {
        if encoded_value.len() >= CHECKSUM_SIZE {
            if options.validate_checksums() {
                let decoded_value = &encoded_value[..encoded_value.len() - CHECKSUM_SIZE];
                let checksum = crc32c::crc32c(decoded_value).to_le_bytes();
                if checksum != encoded_value[encoded_value.len() - CHECKSUM_SIZE..] {
                    return Err(CodecError::InvalidChecksum);
                }
            }
            encoded_value.resize_with(encoded_value.len() - CHECKSUM_SIZE, Default::default);
            Ok(encoded_value)
        } else {
            Err(CodecError::Other(
                "crc32c decoder expects a 32 bit input".to_string(),
            ))
        }
    }

    fn partial_decoder<'a>(
        &'a self,
        input_handle: Box<dyn BytesPartialDecoderTraits + 'a>,
        _decoded_representation: &BytesRepresentation,
        _options: &CodecOptions,
    ) -> Result<Box<dyn BytesPartialDecoderTraits + 'a>, CodecError> {
        Ok(Box::new(crc32c_partial_decoder::Crc32cPartialDecoder::new(
            input_handle,
        )))
    }

    #[cfg(feature = "async")]
    async fn async_partial_decoder<'a>(
        &'a self,
        input_handle: Box<dyn AsyncBytesPartialDecoderTraits + 'a>,
        _decoded_representation: &BytesRepresentation,
        _options: &CodecOptions,
    ) -> Result<Box<dyn AsyncBytesPartialDecoderTraits + 'a>, CodecError> {
        Ok(Box::new(
            crc32c_partial_decoder::AsyncCrc32cPartialDecoder::new(input_handle),
        ))
    }

    fn compute_encoded_size(
        &self,
        decoded_representation: &BytesRepresentation,
    ) -> BytesRepresentation {
        match decoded_representation {
            BytesRepresentation::FixedSize(size) => {
                BytesRepresentation::FixedSize(size + core::mem::size_of::<u32>() as u64)
            }
            BytesRepresentation::BoundedSize(size) => {
                BytesRepresentation::BoundedSize(size + core::mem::size_of::<u32>() as u64)
            }
            BytesRepresentation::UnboundedSize => BytesRepresentation::UnboundedSize,
        }
    }
}
