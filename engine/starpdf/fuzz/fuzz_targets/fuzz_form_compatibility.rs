#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::forms::FieldType;
use starpdf::mutation::PdfChange;

fuzz_target!(|data: &[u8]| {
    let Ok(mut document) = PdfDocument::from_bytes(data) else {
        return;
    };
    let Ok(fields) = document.form_fields() else {
        return;
    };
    let Some(field) = fields.first() else {
        return;
    };
    let change = match &field.field_type {
        FieldType::Text { .. } => PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: "Fuzz compatibility".into(),
        },
        FieldType::Checkbox => PdfChange::SetCheckbox {
            field_ref: field.object_ref,
            widget_refs: field.widgets.iter().map(|widget| widget.object_ref).collect(),
            checked: true,
        },
        FieldType::Choice { .. } => {
            let Some(option) = field.options.first() else {
                return;
            };
            PdfChange::SetChoice {
                field_ref: field.object_ref,
                value: option.export_value.clone(),
            }
        }
        _ => return,
    };
    if let Ok(plan) = document.apply_mutation(&[change]) {
        let _ = document.export_incremental(&plan);
    }
});
