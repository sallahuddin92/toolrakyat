#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::annotation::AnnotationUpdateSpec;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;

fuzz_target!(|data: &[u8]| {
    let Ok(mut document) = PdfDocument::from_bytes(data) else {
        return;
    };
    let Ok(page_count) = document.page_count() else {
        return;
    };
    for page_index in 0..page_count.min(4) {
        let Ok(annotations) = document.page_annotations(page_index) else {
            continue;
        };
        let Some(annotation) = annotations.first() else {
            continue;
        };
        let change = PdfChange::UpdateAnnotation {
            annot_ref: annotation.object_ref,
            update: AnnotationUpdateSpec {
                contents: Some("Fuzz semantic preservation".into()),
                ..Default::default()
            },
        };
        if let Ok(plan) = document.apply_mutation(&[change]) {
            let _ = document.export_incremental(&plan);
        }
    }
});
