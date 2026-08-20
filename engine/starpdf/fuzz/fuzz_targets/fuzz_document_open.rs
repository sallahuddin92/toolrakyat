#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::filter::limits::DecompressLimits;

fuzz_target!(|data: &[u8]| {
    let limits = DecompressLimits::default();
    let _ = PdfDocument::from_bytes_with_limits(data, limits);
});
