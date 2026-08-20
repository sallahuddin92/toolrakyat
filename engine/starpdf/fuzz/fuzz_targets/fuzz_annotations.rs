#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        if let Ok(count) = doc.page_count() {
            for i in 0..count.min(10) {
                let _ = doc.page_annotations(i);
            }
        }
    }
});
