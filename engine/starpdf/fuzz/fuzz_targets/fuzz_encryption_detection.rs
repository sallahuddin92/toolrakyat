#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::annotation::AnnotationSpec;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut document) = PdfDocument::from_bytes(data) {
        let _ = document.security_info();
        let _ = document.apply_mutation(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [1.0, 1.0, 2.0, 2.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        }]);
    }
});
