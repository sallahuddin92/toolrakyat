#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::annotation::AnnotationSpec;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let rect = [50.0, 50.0, 200.0, 100.0];
    let contents = String::from_utf8_lossy(&data[16..]).to_string();

    let changes = vec![
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect,
                text: contents,
                font_size: Some(12.0),
                color: Some(vec![0.0, 0.0, 0.0]),
            },
        },
    ];

    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        let _ = doc.apply_mutation(&changes);
        let _ = doc.mutate_and_export(&changes);
    }
});
