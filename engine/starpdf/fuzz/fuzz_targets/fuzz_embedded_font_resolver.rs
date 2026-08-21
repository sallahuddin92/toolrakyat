#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::font::appearance::AppearanceFontResolver;
use starpdf::syntax::object::{ObjectRef, PdfObject, StreamObject};
use starpdf::writer::MinimalWriter;
use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    let Ok(pdf) = MinimalWriter::create_minimal_pdf("font fuzz") else {
        return;
    };
    let Ok(mut document) = PdfDocument::from_bytes(&pdf) else {
        return;
    };
    let font_ref = ObjectRef::new(7_001, 0);
    let descriptor_ref = ObjectRef::new(7_002, 0);
    let stream_ref = ObjectRef::new(7_003, 0);
    document.store_mut().insert_cached(
        stream_ref,
        PdfObject::Stream(StreamObject {
            dict: BTreeMap::new(),
            stream_offset: 0,
            stream_length: data.len(),
            data: data.to_vec(),
        }),
    );
    document.store_mut().insert_cached(
        descriptor_ref,
        PdfObject::Dictionary(BTreeMap::from([(
            "FontFile2".into(),
            PdfObject::Reference(stream_ref),
        )])),
    );
    document.store_mut().insert_cached(
        font_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("Subtype".into(), PdfObject::Name("TrueType".into())),
            ("BaseFont".into(), PdfObject::Name("FuzzFont".into())),
            (
                "FontDescriptor".into(),
                PdfObject::Reference(descriptor_ref),
            ),
        ])),
    );
    let field = BTreeMap::from([(
        "DR".into(),
        PdfObject::Dictionary(BTreeMap::from([(
            "Font".into(),
            PdfObject::Dictionary(BTreeMap::from([(
                "Fuzz".into(),
                PdfObject::Reference(font_ref),
            )])),
        )])),
    )]);
    let text = String::from_utf8_lossy(data);
    let _ = AppearanceFontResolver::resolve(document.store_mut(), &field, &[], "Fuzz", &text);
});
