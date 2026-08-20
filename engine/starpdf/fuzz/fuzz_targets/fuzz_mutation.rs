#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;
use starpdf::syntax::object::ObjectRef;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    let obj_num = u64::from_le_bytes([data[0], data[1], data[2], data[3], 0, 0, 0, 0]);
    let obj_gen = u16::from_le_bytes([data[4], data[5]]);
    let field_ref = ObjectRef::new(obj_num, obj_gen);

    let val_str = String::from_utf8_lossy(&data[6..]);

    let changes = vec![
        PdfChange::SetTextField {
            field_ref,
            value: val_str.to_string(),
        },
        PdfChange::SetCheckbox {
            field_ref,
            widget_refs: vec![field_ref],
            checked: data[0] % 2 == 0,
        },
        PdfChange::SetChoice {
            field_ref,
            value: val_str.to_string(),
        },
    ];

    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        let _ = doc.apply_mutation(&changes);
        let _ = doc.mutate_and_export(&changes);
    }
});
