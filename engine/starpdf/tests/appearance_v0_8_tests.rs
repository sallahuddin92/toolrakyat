use std::collections::BTreeMap;

use starpdf::annotation::{AnnotationSpec, AnnotationUpdateSpec, LineEndingStyle};
use starpdf::appearance::choice::ChoiceAppearance;
use starpdf::appearance::da_parser::DefaultAppearance;
use starpdf::appearance::text_field::{TextFieldAppearance, TextLayoutOptions};
use starpdf::appearance::AppearanceStatus;
use starpdf::document::PdfDocument;
use starpdf::font::appearance::{AppearanceFontResolver, GlyphMappingQuality};
use starpdf::font::subset::TrueTypeSubsetter;
use starpdf::font::SfntFont;
use starpdf::mutation::PdfChange;
use starpdf::syntax::object::StreamObject;
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::writer::MinimalWriter;

#[test]
fn annotation_visual_update_replaces_stale_ap_atomically() {
    let original = MinimalWriter::create_minimal_pdf("AP regeneration").unwrap();
    let mut document = PdfDocument::from_bytes(&original).unwrap();
    let added = document
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [40.0, 40.0, 240.0, 90.0],
                text: "Before".to_string(),
                font_size: Some(12.0),
                color: Some(vec![0.0]),
            },
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&added).unwrap();
    let annotation_ref = reopened.page_annotations(0).unwrap()[0].object_ref;
    let before = reopened
        .store_mut()
        .resolve(annotation_ref)
        .unwrap()
        .clone();
    let before_ap = ap_normal_reference(before.as_dict().unwrap());

    let plan = reopened
        .apply_mutation(&[PdfChange::UpdateAnnotation {
            annot_ref: annotation_ref,
            update: AnnotationUpdateSpec {
                contents: Some("After".to_string()),
                color: Some(vec![0.0, 0.4, 0.8]),
                ..AnnotationUpdateSpec::default()
            },
        }])
        .unwrap();
    assert_eq!(
        plan.appearance_status,
        AppearanceStatus::AppearanceRegenerated
    );
    let updated = plan.modified_objects.get(&annotation_ref).unwrap();
    let after_ap = ap_normal_reference(updated.as_dict().unwrap());
    assert_ne!(before_ap, after_ap);
    let stream = plan
        .modified_objects
        .get(&after_ap)
        .and_then(PdfObject::as_stream)
        .unwrap();
    assert!(String::from_utf8_lossy(&stream.data).contains("After"));

    let output = reopened.export_incremental(&plan).unwrap();
    assert!(output.starts_with(&added));
    let mut final_document = PdfDocument::from_bytes(&output).unwrap();
    assert_eq!(
        final_document.page_annotations(0).unwrap()[0]
            .contents
            .as_deref(),
        Some("After")
    );
}

#[test]
fn markup_and_line_annotations_have_deterministic_appearances() {
    let original = MinimalWriter::create_minimal_pdf("Markup AP").unwrap();
    let mut document = PdfDocument::from_bytes(&original).unwrap();
    let quad = vec![20.0, 80.0, 180.0, 80.0, 20.0, 60.0, 180.0, 60.0];
    let changes = vec![
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Highlight {
                rect: [20.0, 60.0, 180.0, 80.0],
                quad_points: quad.clone(),
                color: Some(vec![1.0, 1.0, 0.0]),
            },
        },
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Underline {
                rect: [20.0, 60.0, 180.0, 80.0],
                quad_points: quad.clone(),
                color: Some(vec![0.0, 0.0, 1.0]),
            },
        },
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::StrikeOut {
                rect: [20.0, 60.0, 180.0, 80.0],
                quad_points: quad,
                color: Some(vec![1.0, 0.0, 0.0]),
            },
        },
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Line {
                line_points: [40.0, 120.0, 220.0, 170.0],
                stroke_color: Some(vec![0.1, 0.2, 0.8]),
                fill_color: Some(vec![0.9, 0.2, 0.2]),
                stroke_width: Some(3.0),
                line_endings: [LineEndingStyle::OpenArrow, LineEndingStyle::ClosedArrow],
                contents: Some("Arrow line".to_string()),
            },
        },
    ];
    let output = document.mutate_and_export(&changes).unwrap();
    let mut reopened = PdfDocument::from_bytes(&output).unwrap();
    let annotations = reopened.page_annotations(0).unwrap();
    assert_eq!(annotations.len(), 4);
    for annotation in &annotations {
        let object = reopened
            .store_mut()
            .resolve(annotation.object_ref)
            .unwrap()
            .clone();
        assert!(object.as_dict().unwrap().get("AP").is_some());
    }
    let line = reopened
        .store_mut()
        .resolve(annotations[3].object_ref)
        .unwrap()
        .clone();
    let line = line.as_dict().unwrap();
    assert_eq!(
        line.get("L").and_then(PdfObject::as_array).unwrap().len(),
        4
    );
    assert_eq!(
        line.get("LE").and_then(PdfObject::as_array).unwrap().len(),
        2
    );
    assert!(line.get("BS").and_then(PdfObject::as_dict).is_some());
    assert!(line.get("Border").and_then(PdfObject::as_array).is_some());
    assert!(line.get("IC").and_then(PdfObject::as_array).is_some());
}

#[test]
fn comb_multiline_and_list_layouts_are_bounded_and_visible() {
    let da = DefaultAppearance::parse("/Helv 10 Tf 0 g").unwrap();
    let comb = TextFieldAppearance::generate_stream_with_options(
        [0.0, 0.0, 200.0, 30.0],
        "AB12",
        &da,
        0,
        TextLayoutOptions {
            multiline: false,
            comb_max_len: Some(6),
        },
    )
    .unwrap();
    let comb_data = String::from_utf8_lossy(&comb.data);
    assert_eq!(comb_data.matches(" Tj").count(), 4);
    assert!(comb_data.contains("33.33 0 m"));
    assert!(TextFieldAppearance::generate_stream_with_options(
        [0.0, 0.0, 200.0, 30.0],
        "TOO-LONG",
        &da,
        0,
        TextLayoutOptions {
            multiline: false,
            comb_max_len: Some(4),
        },
    )
    .is_err());

    let multiline = TextFieldAppearance::generate_stream_with_options(
        [0.0, 0.0, 75.0, 80.0],
        "explicit line\nwrapped words continue",
        &da,
        2,
        TextLayoutOptions {
            multiline: true,
            comb_max_len: None,
        },
    )
    .unwrap();
    let multiline_data = String::from_utf8_lossy(&multiline.data);
    assert!(multiline_data.matches(" Tj").count() >= 3);
    assert!(multiline_data.contains("W\nn"));

    let list = ChoiceAppearance::generate_list_stream(
        [0.0, 0.0, 180.0, 60.0],
        &["First".into(), "Second".into(), "Third".into()],
        &[0, 2],
        0,
        &da,
    )
    .unwrap();
    let list_data = String::from_utf8_lossy(&list.data);
    assert_eq!(list_data.matches("0.153 0.400 0.820 rg").count(), 2);
    assert!(list_data.contains("(First) Tj"));
    assert!(list_data.contains("(Third) Tj"));
}

#[test]
fn fallback_glyph_coverage_and_atomic_failure_are_explicit() {
    let bytes = MinimalWriter::create_minimal_pdf("Glyph coverage").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let fallback = AppearanceFontResolver::resolve(
        document.store_mut(),
        &BTreeMap::new(),
        &[],
        "Helv",
        "ASCII",
    )
    .unwrap();
    assert_eq!(fallback.quality, GlyphMappingQuality::Fallback);
    assert_eq!(
        fallback.verify_text("ASCII").unwrap(),
        GlyphMappingQuality::Fallback
    );
    assert!(fallback.verify_text("snowman ☃").is_err());

    let field_ref = ObjectRef::new(9_000, 0);
    document.store_mut().insert_cached(
        field_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("FT".to_string(), PdfObject::Name("Tx".to_string())),
            (
                "DA".to_string(),
                PdfObject::String(b"/Helv 12 Tf 0 g".to_vec()),
            ),
            (
                "Rect".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Real(20.0),
                    PdfObject::Real(20.0),
                    PdfObject::Real(200.0),
                    PdfObject::Real(50.0),
                ]),
            ),
        ])),
    );
    let result = document.apply_mutation(&[
        PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [10.0, 10.0, 30.0, 30.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        },
        PdfChange::SetTextField {
            field_ref,
            value: "☃".to_string(),
        },
    ]);
    assert!(result.unwrap_err().to_string().contains("UNREPRESENTABLE"));
    assert!(document.page_annotations(0).unwrap().is_empty());
}

#[test]
fn embedded_true_type_resolves_from_field_resources_and_reuses_reference() {
    let bytes = MinimalWriter::create_minimal_pdf("Embedded font resolver").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let font_ref = ObjectRef::new(7_001, 0);
    let descriptor_ref = ObjectRef::new(7_002, 0);
    let stream_ref = ObjectRef::new(7_003, 0);
    let font_bytes = synthetic_true_type();
    document.store_mut().insert_cached(
        stream_ref,
        PdfObject::Stream(StreamObject {
            dict: BTreeMap::from([(
                "Length".to_string(),
                PdfObject::Integer(font_bytes.len() as i64),
            )]),
            stream_offset: 0,
            stream_length: font_bytes.len(),
            data: font_bytes,
        }),
    );
    document.store_mut().insert_cached(
        descriptor_ref,
        PdfObject::Dictionary(BTreeMap::from([(
            "FontFile2".to_string(),
            PdfObject::Reference(stream_ref),
        )])),
    );
    document.store_mut().insert_cached(
        font_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("Type".to_string(), PdfObject::Name("Font".to_string())),
            (
                "Subtype".to_string(),
                PdfObject::Name("TrueType".to_string()),
            ),
            (
                "BaseFont".to_string(),
                PdfObject::Name("Synthetic".to_string()),
            ),
            (
                "Encoding".to_string(),
                PdfObject::Name("WinAnsiEncoding".to_string()),
            ),
            (
                "FontDescriptor".to_string(),
                PdfObject::Reference(descriptor_ref),
            ),
        ])),
    );
    let field = BTreeMap::from([(
        "DR".to_string(),
        PdfObject::Dictionary(BTreeMap::from([(
            "Font".to_string(),
            PdfObject::Dictionary(BTreeMap::from([(
                "Embed".to_string(),
                PdfObject::Reference(font_ref),
            )])),
        )])),
    )]);
    let resolved =
        AppearanceFontResolver::resolve(document.store_mut(), &field, &[], "Embed", "AB").unwrap();
    assert_eq!(resolved.quality, GlyphMappingQuality::Exact);
    assert_eq!(resolved.resource_object, PdfObject::Reference(font_ref));
    assert_eq!(resolved.encode_text("AB").unwrap(), b"AB");
    assert!(resolved.verify_text("C").is_err());
    let mut non_identity = resolved.clone();
    non_identity.font.is_composite = true;
    non_identity.font.composite_identity_mapping = false;
    assert!(non_identity
        .encode_text("A")
        .unwrap_err()
        .to_string()
        .contains("Identity-H/Identity-V"));
    let appearance = TextFieldAppearance::generate_stream_with_font(
        [0.0, 0.0, 100.0, 20.0],
        "AB",
        &DefaultAppearance::parse("/Embed 10 Tf 0 g").unwrap(),
        0,
        TextLayoutOptions::default(),
        Some(&resolved),
    )
    .unwrap();
    let fonts = appearance
        .dict
        .get("Resources")
        .and_then(PdfObject::as_dict)
        .and_then(|resources| resources.get("Font"))
        .and_then(PdfObject::as_dict)
        .unwrap();
    assert_eq!(fonts.get("Embed"), Some(&PdfObject::Reference(font_ref)));
    assert!(String::from_utf8_lossy(&appearance.data).contains("<4142> Tj"));

    let field_ref = ObjectRef::new(7_004, 0);
    let first_widget_ref = ObjectRef::new(7_005, 0);
    let second_widget_ref = ObjectRef::new(7_006, 0);
    let page_ref = document.page_ref(0).unwrap();
    let mut page = document.page_dict(0).unwrap();
    page.insert("Rotate".to_string(), PdfObject::Integer(90));
    document
        .store_mut()
        .insert_cached(page_ref, PdfObject::Dictionary(page));
    let mut mutation_field = field;
    mutation_field.extend([
        ("FT".to_string(), PdfObject::Name("Tx".to_string())),
        (
            "DA".to_string(),
            PdfObject::String(b"/Embed 12 Tf 0 g".to_vec()),
        ),
        (
            "Rect".to_string(),
            PdfObject::Array(vec![
                PdfObject::Real(20.0),
                PdfObject::Real(20.0),
                PdfObject::Real(200.0),
                PdfObject::Real(50.0),
            ]),
        ),
        (
            "Kids".to_string(),
            PdfObject::Array(vec![
                PdfObject::Reference(first_widget_ref),
                PdfObject::Reference(second_widget_ref),
            ]),
        ),
    ]);
    document
        .store_mut()
        .insert_cached(field_ref, PdfObject::Dictionary(mutation_field));
    for (widget_ref, rotation) in [(first_widget_ref, 90), (second_widget_ref, 270)] {
        document.store_mut().insert_cached(
            widget_ref,
            PdfObject::Dictionary(BTreeMap::from([
                ("Subtype".to_string(), PdfObject::Name("Widget".to_string())),
                ("Parent".to_string(), PdfObject::Reference(field_ref)),
                ("P".to_string(), PdfObject::Reference(page_ref)),
                (
                    "Rect".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Real(20.0),
                        PdfObject::Real(20.0),
                        PdfObject::Real(200.0),
                        PdfObject::Real(50.0),
                    ]),
                ),
                (
                    "MK".to_string(),
                    PdfObject::Dictionary(BTreeMap::from([(
                        "R".to_string(),
                        PdfObject::Integer(rotation),
                    )])),
                ),
            ])),
        );
    }
    let plan = document
        .apply_mutation(&[PdfChange::SetTextField {
            field_ref,
            value: "AB".to_string(),
        }])
        .unwrap();
    assert_eq!(plan.glyph_mapping_quality, Some(GlyphMappingQuality::Exact));
    let mut subset_font_refs = Vec::new();
    for widget_ref in [first_widget_ref, second_widget_ref] {
        let appearance = plan
            .modified_objects
            .get(&widget_ref)
            .and_then(PdfObject::as_dict)
            .and_then(|dict| dict.get("AP"))
            .and_then(PdfObject::as_dict)
            .and_then(|dict| dict.get("N"))
            .and_then(PdfObject::as_stream)
            .unwrap();
        assert_eq!(
            appearance
                .dict
                .get("BBox")
                .and_then(PdfObject::as_array)
                .unwrap(),
            &[
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(30.0),
                PdfObject::Real(180.0),
            ]
        );
        let subset_fonts = appearance
            .dict
            .get("Resources")
            .and_then(PdfObject::as_dict)
            .and_then(|dict| dict.get("Font"))
            .and_then(PdfObject::as_dict)
            .unwrap();
        assert_eq!(subset_fonts.len(), 1);
        let (resource_name, resource) = subset_fonts.iter().next().unwrap();
        assert!(resource_name.starts_with("SPF"));
        subset_font_refs.push(resource.as_reference().unwrap());
    }
    assert_eq!(subset_font_refs[0], subset_font_refs[1]);
    let subset_font_ref = subset_font_refs[0];
    let subset_font = plan
        .modified_objects
        .get(&subset_font_ref)
        .and_then(PdfObject::as_dict)
        .unwrap();
    assert!(subset_font
        .get("BaseFont")
        .and_then(PdfObject::as_name)
        .is_some_and(|name| name.contains("+Synthetic")));
}

#[test]
fn true_type_subsetter_produces_reopenable_bounded_font() {
    let font = synthetic_true_type();
    let subset = TrueTypeSubsetter::subset(&font, &[1]).unwrap();
    assert_eq!(subset.glyph_ids, vec![0, 1]);
    assert!(subset.bytes.len() < font.len());
    let parsed = SfntFont::parse(&subset.bytes).unwrap();
    assert_eq!(
        parsed.cmap.as_ref().unwrap().map_char_to_glyph('A' as u32),
        Some(1)
    );
    assert!(TrueTypeSubsetter::subset(&font, &[999]).is_err());
    assert!(TrueTypeSubsetter::subset(&font[..20], &[1]).is_err());
}

#[test]
fn subset_rotation_and_annotation_survive_three_incremental_exports() {
    let original = MinimalWriter::create_minimal_pdf("Sequential v0.9").unwrap();
    let mut document = PdfDocument::from_bytes(&original).unwrap();
    let page_ref = document.page_ref(0).unwrap();
    let font_ref = ObjectRef::new(7_101, 0);
    let descriptor_ref = ObjectRef::new(7_102, 0);
    let stream_ref = ObjectRef::new(7_103, 0);
    let field_ref = ObjectRef::new(7_104, 0);
    let widget_ref = ObjectRef::new(7_105, 0);
    let font_bytes = synthetic_true_type();
    let initial = starpdf::mutation::MutationPlan {
        modified_objects: BTreeMap::from([
            (
                stream_ref,
                PdfObject::Stream(StreamObject {
                    dict: BTreeMap::from([(
                        "Length".to_string(),
                        PdfObject::Integer(font_bytes.len() as i64),
                    )]),
                    stream_offset: 0,
                    stream_length: font_bytes.len(),
                    data: font_bytes,
                }),
            ),
            (
                descriptor_ref,
                PdfObject::Dictionary(BTreeMap::from([(
                    "FontFile2".to_string(),
                    PdfObject::Reference(stream_ref),
                )])),
            ),
            (
                font_ref,
                PdfObject::Dictionary(BTreeMap::from([
                    ("Type".to_string(), PdfObject::Name("Font".to_string())),
                    (
                        "Subtype".to_string(),
                        PdfObject::Name("TrueType".to_string()),
                    ),
                    (
                        "BaseFont".to_string(),
                        PdfObject::Name("Synthetic".to_string()),
                    ),
                    (
                        "Encoding".to_string(),
                        PdfObject::Name("WinAnsiEncoding".to_string()),
                    ),
                    (
                        "FontDescriptor".to_string(),
                        PdfObject::Reference(descriptor_ref),
                    ),
                ])),
            ),
            (
                field_ref,
                PdfObject::Dictionary(BTreeMap::from([
                    ("FT".to_string(), PdfObject::Name("Tx".to_string())),
                    (
                        "DA".to_string(),
                        PdfObject::String(b"/Embed 12 Tf 0 g".to_vec()),
                    ),
                    (
                        "Kids".to_string(),
                        PdfObject::Array(vec![PdfObject::Reference(widget_ref)]),
                    ),
                    (
                        "DR".to_string(),
                        PdfObject::Dictionary(BTreeMap::from([(
                            "Font".to_string(),
                            PdfObject::Dictionary(BTreeMap::from([(
                                "Embed".to_string(),
                                PdfObject::Reference(font_ref),
                            )])),
                        )])),
                    ),
                ])),
            ),
            (
                widget_ref,
                PdfObject::Dictionary(BTreeMap::from([
                    ("Subtype".to_string(), PdfObject::Name("Widget".to_string())),
                    ("Parent".to_string(), PdfObject::Reference(field_ref)),
                    ("P".to_string(), PdfObject::Reference(page_ref)),
                    (
                        "Rect".to_string(),
                        PdfObject::Array(vec![
                            PdfObject::Real(20.0),
                            PdfObject::Real(20.0),
                            PdfObject::Real(200.0),
                            PdfObject::Real(50.0),
                        ]),
                    ),
                    (
                        "MK".to_string(),
                        PdfObject::Dictionary(BTreeMap::from([(
                            "R".to_string(),
                            PdfObject::Integer(90),
                        )])),
                    ),
                ])),
            ),
        ]),
        appearance_status: AppearanceStatus::AppearancePreserved,
        glyph_mapping_quality: None,
        layout_policy_result: None,
    };
    let seeded = document.export_incremental(&initial).unwrap();

    let mut generation = seeded;
    for (index, value) in ["A", "AB", "B"].into_iter().enumerate() {
        let mut reopened = PdfDocument::from_bytes(&generation).unwrap();
        let mut changes = vec![PdfChange::SetTextField {
            field_ref,
            value: value.to_string(),
        }];
        if index == 2 {
            changes.push(PdfChange::AddAnnotation {
                page_index: 0,
                spec: AnnotationSpec::Square {
                    rect: [10.0, 10.0, 30.0, 30.0],
                    stroke_color: None,
                    fill_color: None,
                    border_width: Some(1.0),
                },
            });
        }
        let next = reopened.mutate_and_export(&changes).unwrap();
        assert!(next.starts_with(&generation));
        let mut verified = PdfDocument::from_bytes(&next).unwrap();
        assert_eq!(verified.page_count().unwrap(), 1);
        generation = next;
    }
    let mut final_document = PdfDocument::from_bytes(&generation).unwrap();
    assert_eq!(final_document.page_annotations(0).unwrap().len(), 1);
    let widget = final_document.store_mut().resolve(widget_ref).unwrap();
    let appearance = widget
        .as_dict()
        .and_then(|dict| dict.get("AP"))
        .and_then(PdfObject::as_dict)
        .and_then(|dict| dict.get("N"));
    assert!(appearance.is_some());
}

#[test]
fn rotated_checkbox_and_choice_emit_supported_matrices() {
    let bytes = MinimalWriter::create_minimal_pdf("Rotated controls").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let checkbox_ref = ObjectRef::new(8_801, 0);
    let choice_ref = ObjectRef::new(8_802, 0);
    for (reference, field_type, rotation) in [(checkbox_ref, "Btn", 180), (choice_ref, "Ch", 270)] {
        let mut dict = BTreeMap::from([
            ("FT".to_string(), PdfObject::Name(field_type.to_string())),
            (
                "Rect".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Real(10.0),
                    PdfObject::Real(10.0),
                    PdfObject::Real(130.0),
                    PdfObject::Real(40.0),
                ]),
            ),
            (
                "MK".to_string(),
                PdfObject::Dictionary(BTreeMap::from([(
                    "R".to_string(),
                    PdfObject::Integer(rotation),
                )])),
            ),
        ]);
        if reference == choice_ref {
            dict.insert(
                "DA".to_string(),
                PdfObject::String(b"/Helv 12 Tf 0 g".to_vec()),
            );
            dict.insert(
                "Opt".to_string(),
                PdfObject::Array(vec![PdfObject::String(b"Alpha".to_vec())]),
            );
        }
        document
            .store_mut()
            .insert_cached(reference, PdfObject::Dictionary(dict));
    }
    let plan = document
        .apply_mutation(&[
            PdfChange::SetCheckbox {
                field_ref: checkbox_ref,
                widget_refs: vec![checkbox_ref],
                checked: true,
            },
            PdfChange::SetChoice {
                field_ref: choice_ref,
                value: "Alpha".to_string(),
            },
        ])
        .unwrap();
    let checkbox = plan
        .modified_objects
        .get(&checkbox_ref)
        .and_then(PdfObject::as_dict)
        .unwrap();
    let states = checkbox
        .get("AP")
        .and_then(PdfObject::as_dict)
        .and_then(|dict| dict.get("N"))
        .and_then(PdfObject::as_dict)
        .unwrap();
    for state in states.values() {
        assert!(state
            .as_stream()
            .and_then(|stream| stream.dict.get("Matrix"))
            .is_some());
    }
    let choice_matrix = plan
        .modified_objects
        .get(&choice_ref)
        .and_then(PdfObject::as_dict)
        .and_then(|dict| dict.get("AP"))
        .and_then(PdfObject::as_dict)
        .and_then(|dict| dict.get("N"))
        .and_then(PdfObject::as_stream)
        .and_then(|stream| stream.dict.get("Matrix"));
    assert!(choice_matrix.is_some());
}

#[test]
fn subset_resource_limit_failure_is_atomic() {
    let bytes = MinimalWriter::create_minimal_pdf("Resource limit").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let descriptor_ref = ObjectRef::new(9_800, 0);
    let stream_ref = ObjectRef::new(9_801, 0);
    let font_bytes = synthetic_true_type();
    document.store_mut().insert_cached(
        stream_ref,
        PdfObject::Stream(StreamObject {
            dict: BTreeMap::new(),
            stream_offset: 0,
            stream_length: font_bytes.len(),
            data: font_bytes,
        }),
    );
    document.store_mut().insert_cached(
        descriptor_ref,
        PdfObject::Dictionary(BTreeMap::from([(
            "FontFile2".to_string(),
            PdfObject::Reference(stream_ref),
        )])),
    );
    let mut changes = vec![PdfChange::AddAnnotation {
        page_index: 0,
        spec: AnnotationSpec::Square {
            rect: [10.0, 10.0, 30.0, 30.0],
            stroke_color: None,
            fill_color: None,
            border_width: None,
        },
    }];
    for index in 0..65u64 {
        let font_ref = ObjectRef::new(10_000 + index, 0);
        let field_ref = ObjectRef::new(11_000 + index, 0);
        document.store_mut().insert_cached(
            font_ref,
            PdfObject::Dictionary(BTreeMap::from([
                (
                    "Subtype".to_string(),
                    PdfObject::Name("TrueType".to_string()),
                ),
                (
                    "BaseFont".to_string(),
                    PdfObject::Name("Synthetic".to_string()),
                ),
                (
                    "Encoding".to_string(),
                    PdfObject::Name("WinAnsiEncoding".to_string()),
                ),
                (
                    "FontDescriptor".to_string(),
                    PdfObject::Reference(descriptor_ref),
                ),
            ])),
        );
        document.store_mut().insert_cached(
            field_ref,
            PdfObject::Dictionary(BTreeMap::from([
                ("FT".to_string(), PdfObject::Name("Tx".to_string())),
                (
                    "DA".to_string(),
                    PdfObject::String(b"/Embed 12 Tf 0 g".to_vec()),
                ),
                (
                    "Rect".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Integer(0),
                        PdfObject::Integer(0),
                        PdfObject::Integer(100),
                        PdfObject::Integer(20),
                    ]),
                ),
                (
                    "DR".to_string(),
                    PdfObject::Dictionary(BTreeMap::from([(
                        "Font".to_string(),
                        PdfObject::Dictionary(BTreeMap::from([(
                            "Embed".to_string(),
                            PdfObject::Reference(font_ref),
                        )])),
                    )])),
                ),
            ])),
        );
        changes.push(PdfChange::SetTextField {
            field_ref,
            value: "A".to_string(),
        });
    }
    let error = document.apply_mutation(&changes).unwrap_err();
    assert!(error.to_string().contains("Font resources per mutation"));
    assert!(document.page_annotations(0).unwrap().is_empty());
}

#[test]
fn multi_select_synchronizes_values_indexes_and_appearance() {
    let bytes = MinimalWriter::create_minimal_pdf("List box").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let field_ref = ObjectRef::new(8_500, 0);
    document.store_mut().insert_cached(
        field_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("FT".to_string(), PdfObject::Name("Ch".to_string())),
            ("Ff".to_string(), PdfObject::Integer(1 << 21)),
            (
                "DA".to_string(),
                PdfObject::String(b"/Helv 10 Tf 0 g".to_vec()),
            ),
            (
                "Rect".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Real(0.0),
                    PdfObject::Real(0.0),
                    PdfObject::Real(160.0),
                    PdfObject::Real(60.0),
                ]),
            ),
            (
                "Opt".to_string(),
                PdfObject::Array(vec![
                    PdfObject::String(b"A".to_vec()),
                    PdfObject::String(b"B".to_vec()),
                    PdfObject::String(b"C".to_vec()),
                ]),
            ),
        ])),
    );
    let plan = document
        .apply_mutation(&[PdfChange::SetChoiceValues {
            field_ref,
            values: vec!["A".into(), "C".into()],
        }])
        .unwrap();
    let field = plan
        .modified_objects
        .get(&field_ref)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(
        field.get("V").and_then(PdfObject::as_array).unwrap().len(),
        2
    );
    let indexes = field.get("I").and_then(PdfObject::as_array).unwrap();
    assert_eq!(indexes[0].as_integer(), Some(0));
    assert_eq!(indexes[1].as_integer(), Some(2));
    assert!(field.get("AP").is_some());
}

#[test]
fn parent_fields_regenerate_separate_child_widget_appearances() {
    let bytes = MinimalWriter::create_minimal_pdf("Child widgets").unwrap();
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    let field_ref = ObjectRef::new(8_600, 0);
    let first_widget_ref = ObjectRef::new(8_601, 0);
    let second_widget_ref = ObjectRef::new(8_602, 0);
    document.store_mut().insert_cached(
        field_ref,
        PdfObject::Dictionary(BTreeMap::from([
            ("FT".to_string(), PdfObject::Name("Tx".to_string())),
            (
                "DA".to_string(),
                PdfObject::String(b"/Helv 10 Tf 0 g".to_vec()),
            ),
            (
                "Kids".to_string(),
                PdfObject::Array(vec![
                    PdfObject::Reference(first_widget_ref),
                    PdfObject::Reference(second_widget_ref),
                ]),
            ),
        ])),
    );
    for (widget_ref, y) in [(first_widget_ref, 10.0), (second_widget_ref, 50.0)] {
        document.store_mut().insert_cached(
            widget_ref,
            PdfObject::Dictionary(BTreeMap::from([
                ("Subtype".to_string(), PdfObject::Name("Widget".to_string())),
                (
                    "Rect".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Real(10.0),
                        PdfObject::Real(y),
                        PdfObject::Real(180.0),
                        PdfObject::Real(y + 24.0),
                    ]),
                ),
            ])),
        );
    }

    let plan = document
        .apply_mutation(&[PdfChange::SetTextField {
            field_ref,
            value: "Mirrored value".into(),
        }])
        .unwrap();
    let field = plan
        .modified_objects
        .get(&field_ref)
        .and_then(PdfObject::as_dict)
        .unwrap();
    assert_eq!(
        field
            .get("V")
            .and_then(PdfObject::as_string_lossy)
            .as_deref(),
        Some("Mirrored value")
    );
    assert!(field.get("AP").is_none());
    for widget_ref in [first_widget_ref, second_widget_ref] {
        assert!(plan
            .modified_objects
            .get(&widget_ref)
            .and_then(PdfObject::as_dict)
            .and_then(|widget| widget.get("AP"))
            .is_some());
    }
}

fn ap_normal_reference(dict: &BTreeMap<String, PdfObject>) -> ObjectRef {
    dict.get("AP")
        .and_then(PdfObject::as_dict)
        .and_then(|ap| ap.get("N"))
        .and_then(PdfObject::as_reference)
        .unwrap()
}

fn synthetic_true_type() -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&3u16.to_be_bytes());
    let mut hhea = vec![0u8; 36];
    hhea[34..36].copy_from_slice(&3u16.to_be_bytes());
    let mut hmtx = Vec::new();
    for _ in 0..3 {
        hmtx.extend_from_slice(&600u16.to_be_bytes());
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }
    let mut cmap = vec![0, 0, 0, 1, 0, 3, 0, 1];
    cmap.extend_from_slice(&12u32.to_be_bytes());
    let mut format4 = Vec::new();
    format4.extend_from_slice(&4u16.to_be_bytes());
    format4.extend_from_slice(&40u16.to_be_bytes());
    format4.extend_from_slice(&0u16.to_be_bytes());
    format4.extend_from_slice(&6u16.to_be_bytes());
    format4.extend_from_slice(&4u16.to_be_bytes());
    format4.extend_from_slice(&1u16.to_be_bytes());
    format4.extend_from_slice(&2u16.to_be_bytes());
    for value in [65u16, 66, 0xFFFF] {
        format4.extend_from_slice(&value.to_be_bytes());
    }
    format4.extend_from_slice(&0u16.to_be_bytes());
    for value in [65u16, 66, 0xFFFF] {
        format4.extend_from_slice(&value.to_be_bytes());
    }
    for value in [-64i16, -64, 1] {
        format4.extend_from_slice(&value.to_be_bytes());
    }
    for _ in 0..3 {
        format4.extend_from_slice(&0u16.to_be_bytes());
    }
    cmap.extend_from_slice(&format4);
    let mut loca = Vec::new();
    for offset in [0u32, 12, 24, 36] {
        loca.extend_from_slice(&offset.to_be_bytes());
    }
    let mut glyf = Vec::new();
    for marker in [0u8, 1, 2] {
        let mut glyph = vec![0u8; 12];
        glyph[10] = marker;
        glyf.extend_from_slice(&glyph);
    }
    build_sfnt(vec![
        (*b"cmap", cmap),
        (*b"glyf", glyf),
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"hmtx", hmtx),
        (*b"loca", loca),
        (*b"maxp", maxp),
    ])
}

fn build_sfnt(tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    let mut output = vec![0u8; 12 + tables.len() * 16];
    output[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    output[4..6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
    for (index, (tag, bytes)) in tables.iter().enumerate() {
        while !output.len().is_multiple_of(4) {
            output.push(0);
        }
        let offset = output.len();
        let record = 12 + index * 16;
        output[record..record + 4].copy_from_slice(tag);
        output[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        output[record + 12..record + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
        output.extend_from_slice(bytes);
    }
    output
}
