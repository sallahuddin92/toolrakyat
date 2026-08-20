#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::security::parse_byte_range;
use starpdf::syntax::PdfObject;

fuzz_target!(|data: &[u8]| {
    let values = data
        .chunks(8)
        .take(16)
        .map(|chunk| {
            let mut bytes = [0u8; 8];
            bytes[..chunk.len()].copy_from_slice(chunk);
            PdfObject::Integer(i64::from_le_bytes(bytes))
        })
        .collect();
    let _ = parse_byte_range(&PdfObject::Array(values), data.len());
    if let Ok(mut document) = PdfDocument::from_bytes(data) {
        let _ = document.security_info();
    }
});
