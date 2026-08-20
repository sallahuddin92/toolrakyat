#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut document) = PdfDocument::from_bytes(data) {
        if let Ok(fields) = document.form_fields() {
            if let Some(field) = fields.iter().find(|field| field.is_radio()) {
                let _ = document.apply_mutation(&[PdfChange::SetRadio {
                    parent_ref: field.object_ref,
                    selected_widget_ref: field.object_ref,
                    on_state: "Fuzz".into(),
                }]);
            }
        }
    }
});
