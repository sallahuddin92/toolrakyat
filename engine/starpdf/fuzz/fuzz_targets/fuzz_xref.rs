#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        let _ = doc.page_count();
        let _ = doc.page_dict(0);
    }
});
