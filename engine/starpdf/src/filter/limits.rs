#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecompressLimits {
    pub max_decoded_bytes: usize,
    pub max_expansion_ratio: usize,
    pub max_object_stream_objects: usize,
    pub max_xref_entries: usize,
    pub max_xref_chain_depth: usize,
    pub max_xref_recovery_scan_bytes: usize,
}

impl Default for DecompressLimits {
    fn default() -> Self {
        Self {
            max_decoded_bytes: 64 * 1024 * 1024, // 64 MB
            max_expansion_ratio: 100,            // 100x max expansion
            max_object_stream_objects: 10_000,
            max_xref_entries: 1_000_000,
            max_xref_chain_depth: 64,
            max_xref_recovery_scan_bytes: 64 * 1024,
        }
    }
}

impl DecompressLimits {
    pub const fn strict() -> Self {
        Self {
            max_decoded_bytes: 16 * 1024 * 1024, // 16 MB
            max_expansion_ratio: 50,
            max_object_stream_objects: 2_000,
            max_xref_entries: 100_000,
            max_xref_chain_depth: 32,
            max_xref_recovery_scan_bytes: 16 * 1024,
        }
    }
}
