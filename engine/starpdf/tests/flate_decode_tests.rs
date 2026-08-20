use miniz_oxide::deflate::compress_to_vec_zlib;
use starpdf::filter::{DecompressLimits, FlateDecoder};

#[test]
fn test_flate_decode_simple_roundtrip() {
    let original = b"Hello, StarPDF Modern Container Compatibility! 1234567890";
    let compressed = compress_to_vec_zlib(original, 6);
    let limits = DecompressLimits::default();

    let decoded = FlateDecoder::decode(&compressed, &limits).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_flate_decode_empty_input() {
    let limits = DecompressLimits::default();
    let decoded = FlateDecoder::decode(b"", &limits).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn test_flate_decode_expansion_limit_protection() {
    // Generate a repetitive payload that compresses massively (1000x)
    let payload = vec![b'A'; 100_000];
    let compressed = compress_to_vec_zlib(&payload, 9);

    // Limit max_expansion_ratio to 5x
    let strict_limits = DecompressLimits {
        max_decoded_bytes: 1_000_000,
        max_expansion_ratio: 5,
        max_object_stream_objects: 1000,
        max_xref_entries: 1000,
        max_xref_chain_depth: 10,
    };

    let result = FlateDecoder::decode(&compressed, &strict_limits);
    assert!(result.is_err());
}

#[test]
fn test_flate_decode_corrupted_data_no_panic() {
    let corrupted = b"\x78\x9C\xFF\xFF\xFF\x00\x12\x34";
    let limits = DecompressLimits::default();
    let result = FlateDecoder::decode(corrupted, &limits);
    assert!(result.is_err());
}
