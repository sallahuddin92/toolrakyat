#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::writer::IncrementalWriter;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }
    let mut modified = std::collections::BTreeMap::new();
    let num = (data[0] as u64) % 100;
    let obj_ref = ObjectRef::new(num, 0);
    modified.insert(
        obj_ref,
        PdfObject::String(data[1..16].to_vec()),
    );

    let trailer = std::collections::BTreeMap::from([(
        "Size".to_string(),
        PdfObject::Integer(10),
    )]);

    if let Ok(updated) = IncrementalWriter::write_update(data, &modified, 0, &trailer) {
        let _ = PdfDocument::from_bytes(&updated);
    }
});
