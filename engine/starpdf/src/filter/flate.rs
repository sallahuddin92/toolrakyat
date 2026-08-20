use miniz_oxide::inflate::{decompress_to_vec_with_limit, decompress_to_vec_zlib_with_limit};

use crate::error::{PdfError, PdfResult};
use crate::filter::limits::DecompressLimits;

pub struct FlateDecoder;

impl FlateDecoder {
    /// Decompresses zlib/deflate encoded bytes with strict security and size limits.
    pub fn decode(input: &[u8], limits: &DecompressLimits) -> PdfResult<Vec<u8>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let max_allowed = if input.is_empty() {
            limits.max_decoded_bytes
        } else {
            let max_by_ratio = input.len().saturating_mul(limits.max_expansion_ratio);
            max_by_ratio.min(limits.max_decoded_bytes)
        };

        // 1. Try standard zlib wrapper (RFC 1950)
        match decompress_to_vec_zlib_with_limit(input, max_allowed) {
            Ok(data) => Ok(data),
            Err(_) => {
                // 2. Fallback to raw Deflate (RFC 1951) for non-standard PDF producers
                decompress_to_vec_with_limit(input, max_allowed).map_err(|e| {
                    PdfError::MalformedInput(format!("FlateDecode decompression failed: {e:?}"))
                })
            }
        }
    }
}
