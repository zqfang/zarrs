//! The `gzip` bytes to bytes codec.
//!
//! Applies [gzip](https://datatracker.ietf.org/doc/html/rfc1952) compression.
//!
//! See <https://zarr-specs.readthedocs.io/en/latest/v3/codecs/gzip/v1.0.html>.

mod gzip_codec;
mod gzip_partial_decoder;

pub use crate::metadata::v3::codec::gzip::{
    GzipCodecConfiguration, GzipCodecConfigurationV1, GzipCompressionLevel,
    GzipCompressionLevelError,
};
pub use gzip_codec::GzipCodec;

use crate::{
    array::codec::{Codec, CodecPlugin},
    metadata::v3::{codec::gzip, MetadataV3},
    plugin::{PluginCreateError, PluginMetadataInvalidError},
};

pub use gzip::IDENTIFIER;

// Register the codec.
inventory::submit! {
    CodecPlugin::new(IDENTIFIER, is_name_gzip, create_codec_gzip)
}

fn is_name_gzip(name: &str) -> bool {
    name.eq(IDENTIFIER)
}

pub(crate) fn create_codec_gzip(metadata: &MetadataV3) -> Result<Codec, PluginCreateError> {
    let configuration: GzipCodecConfiguration = metadata
        .to_configuration()
        .map_err(|_| PluginMetadataInvalidError::new(IDENTIFIER, "codec", metadata.clone()))?;
    let codec = Box::new(GzipCodec::new_with_configuration(&configuration));
    Ok(Codec::BytesToBytes(codec))
}

#[cfg(test)]
mod tests {
    use crate::{
        array::{
            codec::{BytesToBytesCodecTraits, CodecOptions},
            BytesRepresentation,
        },
        byte_range::ByteRange,
    };

    use super::*;

    const JSON_VALID: &str = r#"{
        "level": 1
    }"#;

    #[test]
    fn codec_gzip_configuration_valid() {
        assert!(serde_json::from_str::<GzipCodecConfiguration>(JSON_VALID).is_ok());
    }

    #[test]
    fn codec_gzip_configuration_invalid1() {
        const JSON_INVALID1: &str = r#"{
        "level": -1
    }"#;
        assert!(serde_json::from_str::<GzipCodecConfiguration>(JSON_INVALID1).is_err());
    }

    #[test]
    fn codec_gzip_configuration_invalid2() {
        const JSON_INVALID2: &str = r#"{
        "level": 10
    }"#;
        assert!(serde_json::from_str::<GzipCodecConfiguration>(JSON_INVALID2).is_err());
    }

    #[test]
    fn codec_gzip_round_trip1() {
        let elements: Vec<u16> = (0..32).collect();
        let bytes = crate::array::transmute_to_bytes_vec(elements);
        let bytes_representation = BytesRepresentation::FixedSize(bytes.len() as u64);

        let configuration: GzipCodecConfiguration = serde_json::from_str(JSON_VALID).unwrap();
        let codec = GzipCodec::new_with_configuration(&configuration);

        let encoded = codec
            .encode(bytes.clone(), &CodecOptions::default())
            .unwrap();
        let decoded = codec
            .decode(encoded, &bytes_representation, &CodecOptions::default())
            .unwrap();
        assert_eq!(bytes, decoded);
    }

    #[test]
    fn codec_gzip_partial_decode() {
        let elements: Vec<u16> = (0..8).collect();
        let bytes = crate::array::transmute_to_bytes_vec(elements);
        let bytes_representation = BytesRepresentation::FixedSize(bytes.len() as u64);

        let configuration: GzipCodecConfiguration = serde_json::from_str(JSON_VALID).unwrap();
        let codec = GzipCodec::new_with_configuration(&configuration);

        let encoded = codec.encode(bytes, &CodecOptions::default()).unwrap();
        let decoded_regions = [
            ByteRange::FromStart(4, Some(4)),
            ByteRange::FromStart(10, Some(2)),
        ];

        let input_handle = Box::new(std::io::Cursor::new(encoded));
        let partial_decoder = codec
            .partial_decoder(
                input_handle,
                &bytes_representation,
                &CodecOptions::default(),
            )
            .unwrap();
        let decoded_partial_chunk = partial_decoder
            .partial_decode(&decoded_regions, &CodecOptions::default())
            .unwrap()
            .unwrap();

        let decoded_partial_chunk: Vec<u16> = decoded_partial_chunk
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .chunks(std::mem::size_of::<u16>())
            .map(|b| u16::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        let answer: Vec<u16> = vec![2, 3, 5];
        assert_eq!(answer, decoded_partial_chunk);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn codec_gzip_async_partial_decode() {
        let elements: Vec<u16> = (0..8).collect();
        let bytes = crate::array::transmute_to_bytes_vec(elements);
        let bytes_representation = BytesRepresentation::FixedSize(bytes.len() as u64);

        let configuration: GzipCodecConfiguration = serde_json::from_str(JSON_VALID).unwrap();
        let codec = GzipCodec::new_with_configuration(&configuration);

        let encoded = codec.encode(bytes, &CodecOptions::default()).unwrap();
        let decoded_regions = [
            ByteRange::FromStart(4, Some(4)),
            ByteRange::FromStart(10, Some(2)),
        ];

        let input_handle = Box::new(std::io::Cursor::new(encoded));
        let partial_decoder = codec
            .async_partial_decoder(
                input_handle,
                &bytes_representation,
                &CodecOptions::default(),
            )
            .await
            .unwrap();
        let decoded_partial_chunk = partial_decoder
            .partial_decode(&decoded_regions, &CodecOptions::default())
            .await
            .unwrap()
            .unwrap();

        let decoded_partial_chunk: Vec<u16> = decoded_partial_chunk
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .chunks(std::mem::size_of::<u16>())
            .map(|b| u16::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        let answer: Vec<u16> = vec![2, 3, 5];
        assert_eq!(answer, decoded_partial_chunk);
    }
}
