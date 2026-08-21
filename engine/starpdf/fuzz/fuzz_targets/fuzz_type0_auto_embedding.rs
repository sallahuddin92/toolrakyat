#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;
use starpdf::syntax::object::{ObjectRef, PdfObject, StreamObject};
use starpdf::writer::MinimalWriter;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let Ok(pdf) = MinimalWriter::create_minimal_pdf("Type0 fuzz") else {
        return;
    };
    let Ok(mut document) = PdfDocument::from_bytes(&pdf) else {
        return;
    };
    let Ok(page_ref) = document.page_ref(0) else {
        return;
    };
    let font_ref = ObjectRef::new(8_001, 0);
    let descendant_ref = ObjectRef::new(8_002, 0);
    let descriptor_ref = ObjectRef::new(8_003, 0);
    let stream_ref = ObjectRef::new(8_004, 0);
    let field_ref = ObjectRef::new(8_005, 0);
    let first_widget = ObjectRef::new(8_006, 0);
    let second_widget = ObjectRef::new(8_007, 0);
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
        descendant_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("Subtype".into(), PdfObject::Name("CIDFontType2".into())),
            ("BaseFont".into(), PdfObject::Name("FuzzCID".into())),
            (
                "FontDescriptor".into(),
                PdfObject::Reference(descriptor_ref),
            ),
            ("CIDToGIDMap".into(), PdfObject::Name("Identity".into())),
        ])),
    );
    document.store_mut().insert_cached(
        font_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("Subtype".into(), PdfObject::Name("Type0".into())),
            ("BaseFont".into(), PdfObject::Name("FuzzCID".into())),
            ("Encoding".into(), PdfObject::Name("Identity-H".into())),
            (
                "DescendantFonts".into(),
                PdfObject::Array(vec![PdfObject::Reference(descendant_ref)]),
            ),
        ])),
    );
    document.store_mut().insert_cached(
        field_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("FT".into(), PdfObject::Name("Tx".into())),
            ("DA".into(), PdfObject::String(b"/CID 12 Tf 0 g".to_vec())),
            (
                "Kids".into(),
                PdfObject::Array(vec![
                    PdfObject::Reference(first_widget),
                    PdfObject::Reference(second_widget),
                ]),
            ),
        ])),
    );
    for (widget, rotation) in [(first_widget, 90), (second_widget, 270)] {
        document.store_mut().insert_cached(
            widget,
            PdfObject::Dictionary(BTreeMap::from([
                ("Subtype".into(), PdfObject::Name("Widget".into())),
                ("Parent".into(), PdfObject::Reference(field_ref)),
                ("P".into(), PdfObject::Reference(page_ref)),
                (
                    "Rect".into(),
                    PdfObject::Array(vec![
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                        PdfObject::Integer(160),
                        PdfObject::Integer(30),
                    ]),
                ),
                (
                    "MK".into(),
                    PdfObject::Dictionary(BTreeMap::from([(
                        "R".into(),
                        PdfObject::Integer(rotation),
                    )])),
                ),
                (
                    "AP".into(),
                    PdfObject::Dictionary(BTreeMap::from([(
                        "N".into(),
                        PdfObject::Stream(StreamObject {
                            dict: BTreeMap::from([(
                                "Resources".into(),
                                PdfObject::Dictionary(BTreeMap::from([(
                                    "Font".into(),
                                    PdfObject::Dictionary(BTreeMap::from([(
                                        "CID".into(),
                                        PdfObject::Reference(font_ref),
                                    )])),
                                )])),
                            )]),
                            stream_offset: 0,
                            stream_length: 0,
                            data: Vec::new(),
                        }),
                    )])),
                ),
            ])),
        );
    }
    let change = PdfChange::SetTextField {
        field_ref,
        value: "A".into(),
    };
    if let Ok(plan) = document.apply_mutation(std::slice::from_ref(&change)) {
        let _ = document.export_incremental(&plan);
    }
});
