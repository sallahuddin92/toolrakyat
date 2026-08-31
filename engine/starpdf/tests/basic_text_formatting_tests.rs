use std::fs;
use std::time::Instant;

use starpdf::annotation::{AnnotationSpec, AnnotationUpdateSpec};
use starpdf::font::{get_font_registry, TextStylePatch, TextWeight};
use starpdf::mutation::{PdfChange, TextEditTarget};
use starpdf::syntax::object::PdfObject;
use starpdf::{MinimalWriter, PdfDocument};

fn register_font(font_id: &str, filename: &str) {
    let candidates = [
        format!("../../public/fonts/{filename}"),
        format!("../public/fonts/{filename}"),
        format!("public/fonts/{filename}"),
    ];
    let bytes = candidates
        .iter()
        .find_map(|path| fs::read(path).ok())
        .unwrap_or_else(|| panic!("missing qualified test font {filename}"));
    get_font_registry().register_font(font_id, bytes);
}

fn register_qualified_multilingual_fonts() {
    for (font_id, filename) in [
        ("noto-sans-arabic", "NotoSansArabic-Regular.ttf"),
        ("noto-sans-hebrew", "NotoSansHebrew-Regular.ttf"),
        ("noto-sans-devanagari", "NotoSansDevanagari-Regular.ttf"),
        ("noto-sans-cjk-jp", "NotoSansJP-Regular.ttf"),
        ("noto-sans-cjk-sc", "NotoSansSC-Regular.ttf"),
        ("noto-sans-cjk-tc", "NotoSansTC-Regular.ttf"),
        ("noto-sans-cjk-kr", "NotoSansKR-Regular.ttf"),
    ] {
        register_font(font_id, filename);
    }
}

fn apply_native_style(patch: TextStylePatch) -> starpdf::TextSpan {
    let source = MinimalWriter::create_minimal_pdf("Style property").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let target = TextEditTarget::from_span(&doc.extract_page_text(0).unwrap().spans[0]);
    let mutation = doc.style_text(0, &target, &patch).unwrap();
    let exported = doc.export_incremental(&mutation).unwrap();
    PdfDocument::from_bytes(&exported)
        .unwrap()
        .extract_page_text(0)
        .unwrap()
        .spans
        .remove(0)
}

#[test]
fn native_individual_style_properties_use_real_pdf_state() {
    let family = apply_native_style(TextStylePatch {
        font_family: Some("Serif".into()),
        ..Default::default()
    });
    assert_eq!(family.font_family, "Serif");

    let size = apply_native_style(TextStylePatch {
        font_size: Some(22.0),
        ..Default::default()
    });
    assert!((size.font_size - 22.0).abs() < 0.001);

    let bold = apply_native_style(TextStylePatch {
        weight: Some(TextWeight::Bold),
        ..Default::default()
    });
    assert!(bold.is_bold);
    assert!(!bold.is_italic);

    let italic = apply_native_style(TextStylePatch {
        italic: Some(true),
        ..Default::default()
    });
    assert!(!italic.is_bold);
    assert!(italic.is_italic);

    let bold_italic = apply_native_style(TextStylePatch {
        weight: Some(TextWeight::Bold),
        italic: Some(true),
        ..Default::default()
    });
    assert!(bold_italic.is_bold);
    assert!(bold_italic.is_italic);

    let color = apply_native_style(TextStylePatch {
        fill_color: Some([0.25, 0.5, 0.75]),
        ..Default::default()
    });
    assert_eq!(color.fill_color, [0.25, 0.5, 0.75]);
}

#[test]
fn native_combined_style_apply_is_isolated_and_roundtrips() {
    let bytes = MinimalWriter::create_minimal_pdf("Style me").unwrap();
    let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
    let original = doc.extract_page_text(0).unwrap();
    let target = TextEditTarget::from_span(&original.spans[0]);
    let patch = TextStylePatch {
        font_family: Some("Serif".into()),
        font_size: Some(18.0),
        weight: Some(TextWeight::Bold),
        italic: Some(true),
        fill_color: Some([0.1, 0.3, 0.7]),
        replacement_text: Some("Styled text".into()),
    };
    let inspected = doc.inspect_text_style(0, &target).unwrap();
    assert_eq!(inspected.font_size, original.spans[0].font_size);
    let style_plan = doc.plan_text_style_change(0, &target, &patch).unwrap();
    assert!(style_plan.replacement.is_executable());
    let mutation = doc.apply_text_style_plan(&style_plan).unwrap();
    let exported = doc.export_incremental(&mutation).unwrap();
    assert!(
        exported
            .windows(b" q\n".len())
            .any(|window| window == b" q\n")
            || exported
                .windows(b"\nq\n".len())
                .any(|window| window == b"\nq\n")
    );
    assert!(exported
        .windows(b"0.1000 0.3000 0.7000 rg".len())
        .any(|window| { window == b"0.1000 0.3000 0.7000 rg" }));

    let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
    let text = reopened.extract_page_text(0).unwrap();
    assert_eq!(text.plain_text(), "Styled text");
    assert!((text.spans[0].font_size - 18.0).abs() < 0.001);
    assert_eq!(text.spans[0].font_family, "Serif");
    assert!(text.spans[0].is_bold);
    assert!(text.spans[0].is_italic);
    assert_eq!(text.spans[0].fill_color, [0.1, 0.3, 0.7]);
}

#[test]
fn native_second_style_edit_after_reopen_targets_only_selected_text() {
    let source = MinimalWriter::create_minimal_pdf("First target").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let target = TextEditTarget::from_span(&doc.extract_page_text(0).unwrap().spans[0]);
    let first = doc
        .style_text(
            0,
            &target,
            &TextStylePatch {
                font_family: Some("Serif".into()),
                weight: Some(TextWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    let first_export = doc.export_incremental(&first).unwrap();

    let mut reopened = PdfDocument::from_bytes(&first_export).unwrap();
    let target = TextEditTarget::from_span(&reopened.extract_page_text(0).unwrap().spans[0]);
    let second = reopened
        .style_text(
            0,
            &target,
            &TextStylePatch {
                font_family: Some("Monospace".into()),
                weight: Some(TextWeight::Normal),
                italic: Some(true),
                font_size: Some(15.0),
                fill_color: Some([0.0, 0.4, 0.2]),
                ..Default::default()
            },
        )
        .unwrap();
    let second_export = reopened.export_incremental(&second).unwrap();
    let mut verified = PdfDocument::from_bytes(&second_export).unwrap();
    let span = verified.extract_page_text(0).unwrap().spans.remove(0);
    assert_eq!(span.text, "First target");
    assert_eq!(span.font_family, "Monospace");
    assert!(!span.is_bold);
    assert!(span.is_italic);
    assert!((span.font_size - 15.0).abs() < 0.001);
    assert_eq!(span.fill_color, [0.0, 0.4, 0.2]);
}

#[test]
fn native_style_refuses_shared_tj_array() {
    let source = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let page = doc.page_dict(0).unwrap();
    let stream_ref = page
        .get("Contents")
        .and_then(PdfObject::as_reference)
        .unwrap();
    let mut stream = doc
        .store_mut()
        .resolve(stream_ref)
        .unwrap()
        .as_stream()
        .unwrap()
        .clone();
    stream.data = b"BT /F1 12 Tf 50 700 Td [(First) 20 (Second)] TJ ET\n".to_vec();
    stream.stream_length = stream.data.len();
    stream.dict.remove("Filter");
    stream.dict.insert(
        "Length".into(),
        PdfObject::Integer(stream.data.len() as i64),
    );
    let seeded = doc
        .export_incremental(&starpdf::MutationPlan {
            modified_objects: [(stream_ref, PdfObject::Stream(stream))].into(),
            appearance_status: starpdf::AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
        .unwrap();
    let mut seeded_doc = PdfDocument::from_bytes(&seeded).unwrap();
    let spans = seeded_doc.extract_page_text(0).unwrap().spans;
    let target = TextEditTarget::from_span(&spans[0]);
    let plan = seeded_doc
        .plan_text_style_change(
            0,
            &target,
            &TextStylePatch {
                font_size: Some(16.0),
                ..Default::default()
            },
        )
        .unwrap();
    let error = seeded_doc.apply_text_style_plan(&plan).unwrap_err();
    assert!(error.to_string().contains("TEXT_STYLE_SHARED_TJ_REFUSAL"));
}

#[test]
fn native_style_does_not_change_unrelated_text_style_or_position() {
    let source = MinimalWriter::create_minimal_pdf("placeholder").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let page = doc.page_dict(0).unwrap();
    let stream_ref = page
        .get("Contents")
        .and_then(PdfObject::as_reference)
        .unwrap();
    let mut stream = doc
        .store_mut()
        .resolve(stream_ref)
        .unwrap()
        .as_stream()
        .unwrap()
        .clone();
    stream.data = b"BT /F1 12 Tf 50 700 Td (First) Tj 0 -30 Td (Unrelated) Tj ET\n".to_vec();
    stream.stream_length = stream.data.len();
    stream.dict.remove("Filter");
    stream.dict.insert(
        "Length".into(),
        PdfObject::Integer(stream.data.len() as i64),
    );
    let seeded = doc
        .export_incremental(&starpdf::MutationPlan {
            modified_objects: [(stream_ref, PdfObject::Stream(stream))].into(),
            appearance_status: starpdf::AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
        .unwrap();
    let mut seeded_doc = PdfDocument::from_bytes(&seeded).unwrap();
    let before = seeded_doc.extract_page_text(0).unwrap();
    let unrelated_before = before.spans[1].clone();
    let target = TextEditTarget::from_span(&before.spans[0]);
    let mutation = seeded_doc
        .style_text(
            0,
            &target,
            &TextStylePatch {
                font_family: Some("Serif".into()),
                font_size: Some(18.0),
                fill_color: Some([0.7, 0.1, 0.2]),
                ..Default::default()
            },
        )
        .unwrap();
    let exported = seeded_doc.export_incremental(&mutation).unwrap();
    let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
    let after = reopened.extract_page_text(0).unwrap();
    let unrelated_after = &after.spans[1];
    assert_eq!(unrelated_after.text, unrelated_before.text);
    assert_eq!(unrelated_after.font_family, unrelated_before.font_family);
    assert_eq!(unrelated_after.font_size, unrelated_before.font_size);
    assert_eq!(unrelated_after.fill_color, unrelated_before.fill_color);
    assert!((unrelated_after.x - unrelated_before.x).abs() < 0.001);
    assert!((unrelated_after.y - unrelated_before.y).abs() < 0.001);
}

#[test]
fn freetext_style_updates_da_ap_and_preserves_rect() {
    let source = MinimalWriter::create_minimal_pdf("FreeText style").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let added = doc
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [40.0, 50.0, 240.0, 90.0],
                text: "Formatted note".into(),
                font_size: Some(12.0),
                color: Some(vec![0.0, 0.0, 0.0]),
            },
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&added).unwrap();
    let annotation = reopened.page_annotations(0).unwrap().remove(0);
    let styled = reopened
        .mutate_and_export(&[PdfChange::UpdateAnnotation {
            annot_ref: annotation.object_ref,
            update: AnnotationUpdateSpec {
                font_family: Some("Serif".into()),
                font_size: Some(20.0),
                bold: Some(true),
                italic: Some(true),
                text_color: Some([0.8, 0.1, 0.2]),
                ..Default::default()
            },
        }])
        .unwrap();
    let mut verified = PdfDocument::from_bytes(&styled).unwrap();
    let after = verified.page_annotations(0).unwrap().remove(0);
    assert_eq!(after.object_ref, annotation.object_ref);
    assert_eq!(after.rect, annotation.rect);
    let dict = verified
        .store_mut()
        .resolve(after.object_ref)
        .unwrap()
        .as_dict()
        .unwrap();
    let da = dict.get("DA").and_then(PdfObject::as_string_lossy).unwrap();
    assert!(da.contains("/TimesBoldItalic 20.00 Tf"));
    assert!(da.contains("0.800 0.100 0.200 rg"));
    assert!(dict.contains_key("AP"));
}

#[test]
fn freetext_multilingual_second_edit_roundtrips_with_stable_identity() {
    register_qualified_multilingual_fonts();
    let source = MinimalWriter::create_minimal_pdf("FreeText lifecycle").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let added = doc
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [40.0, 50.0, 300.0, 100.0],
                text: "ڤ ڠ ڽ چ 日本".into(),
                font_size: Some(14.0),
                color: Some(vec![0.0, 0.0, 0.0]),
            },
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&added).unwrap();
    let annotation = reopened.page_annotations(0).unwrap().remove(0);
    let rect = annotation.rect;
    let first_update = reopened
        .mutate_and_export(&[PdfChange::UpdateAnnotation {
            annot_ref: annotation.object_ref,
            update: AnnotationUpdateSpec {
                contents: Some("Report تقرير 日本 2026".into()),
                font_size: Some(16.0),
                text_color: Some([0.1, 0.2, 0.6]),
                ..Default::default()
            },
        }])
        .unwrap();
    let mut second_open = PdfDocument::from_bytes(&first_update).unwrap();
    let after_first = second_open.page_annotations(0).unwrap().remove(0);
    assert_eq!(after_first.object_ref, annotation.object_ref);
    assert_eq!(after_first.rect, rect);
    let second_update = second_open
        .mutate_and_export(&[PdfChange::UpdateAnnotation {
            annot_ref: annotation.object_ref,
            update: AnnotationUpdateSpec {
                contents: Some("Laporan تقرير 中文 2027".into()),
                font_size: Some(18.0),
                text_color: Some([0.6, 0.1, 0.2]),
                ..Default::default()
            },
        }])
        .unwrap();
    let mut verified = PdfDocument::from_bytes(&second_update).unwrap();
    let after_second = verified.page_annotations(0).unwrap().remove(0);
    assert_eq!(after_second.object_ref, annotation.object_ref);
    assert_eq!(after_second.rect, rect);
    assert_eq!(
        after_second.contents.as_deref(),
        Some("Laporan تقرير 中文 2027")
    );
    let dict = verified
        .store_mut()
        .resolve(after_second.object_ref)
        .unwrap()
        .as_dict()
        .unwrap();
    assert!(dict.contains_key("AP"));
}

#[test]
fn freetext_multilingual_style_only_after_reopen_uses_shared_adaptive_runtime() {
    register_qualified_multilingual_fonts();
    for (label, contents) in [
        ("Jawi", "ساي جاوي ڤ ڠ ڽ چ"),
        ("Arabic", "تقرير عربي"),
        ("Japanese", "日本語の報告"),
        ("Chinese", "中文測試報告"),
        ("Korean", "한국어 보고서"),
        ("mixed", "Report تقرير 日本 2026"),
    ] {
        let source = MinimalWriter::create_minimal_pdf(label).unwrap();
        let mut doc = PdfDocument::from_bytes(&source).unwrap();
        let created = doc
            .mutate_and_export(&[PdfChange::AddAnnotation {
                page_index: 0,
                spec: AnnotationSpec::FreeText {
                    rect: [40.0, 50.0, 360.0, 105.0],
                    text: contents.into(),
                    font_size: Some(14.0),
                    color: Some(vec![0.0, 0.0, 0.0]),
                },
            }])
            .unwrap();

        let mut reopened = PdfDocument::from_bytes(&created).unwrap();
        let original = reopened.page_annotations(0).unwrap().remove(0);
        let styled = reopened
            .mutate_and_export(&[PdfChange::UpdateAnnotation {
                annot_ref: original.object_ref,
                update: AnnotationUpdateSpec {
                    font_size: Some(23.0),
                    text_color: Some([0.12, 0.34, 0.68]),
                    ..Default::default()
                },
            }])
            .unwrap();
        assert!(styled
            .windows(b"/Subtype /Type0".len())
            .any(|w| w == b"/Subtype /Type0"));
        assert!(styled
            .windows(b"/ToUnicode".len())
            .any(|w| w == b"/ToUnicode"));

        let mut second_open = PdfDocument::from_bytes(&styled).unwrap();
        let after_first = second_open.page_annotations(0).unwrap().remove(0);
        assert_eq!(after_first.object_ref, original.object_ref, "{label}");
        assert_eq!(after_first.contents.as_deref(), Some(contents), "{label}");
        assert_eq!(after_first.rect, original.rect, "{label}");
        let second_style = second_open
            .mutate_and_export(&[PdfChange::UpdateAnnotation {
                annot_ref: original.object_ref,
                update: AnnotationUpdateSpec {
                    font_size: Some(19.0),
                    text_color: Some([0.55, 0.16, 0.22]),
                    ..Default::default()
                },
            }])
            .unwrap();
        let mut verified = PdfDocument::from_bytes(&second_style).unwrap();
        let final_annotation = verified.page_annotations(0).unwrap().remove(0);
        assert_eq!(final_annotation.object_ref, original.object_ref, "{label}");
        assert_eq!(
            final_annotation.contents.as_deref(),
            Some(contents),
            "{label}"
        );
        assert_eq!(final_annotation.rect, original.rect, "{label}");
    }
}

#[test]
fn multilingual_style_apply_preserves_unicode_and_reports_timings() {
    register_qualified_multilingual_fonts();
    let cases = [
        ("Latin", "Teks Melayu: é ñ 2026"),
        ("Jawi", "ڤ ڠ ڽ چ"),
        ("CJK", "日本語 測試"),
        ("mixed", "Report تقرير 日本 2026"),
    ];
    for (label, replacement) in cases {
        let source = MinimalWriter::create_minimal_pdf("Original").unwrap();
        let mut doc = PdfDocument::from_bytes(&source).unwrap();
        let span = doc.extract_page_text(0).unwrap().spans.remove(0);
        let target = TextEditTarget::from_span(&span);
        let started = Instant::now();
        let plan = doc
            .style_text(
                0,
                &target,
                &TextStylePatch {
                    font_size: Some(14.0),
                    fill_color: Some([0.2, 0.2, 0.6]),
                    replacement_text: Some(replacement.into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let exported = doc.export_incremental(&plan).unwrap();
        eprintln!("{label} style apply: {:?}", started.elapsed());
        let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
        assert_eq!(
            reopened.extract_page_text(0).unwrap().plain_text(),
            replacement
        );
    }
}

#[test]
fn qualified_scripts_preserve_exact_unicode_and_unavailable_variants_refuse() {
    register_qualified_multilingual_fonts();
    for replacement in [
        "Teks Melayu é ñ",
        "العربية ڤ ڠ ڽ چ",
        "עברית",
        "देवनागरी",
        "日本語",
        "简体中文",
        "繁體中文",
        "한국어",
        "Report تقرير 日本 2026",
    ] {
        let source = MinimalWriter::create_minimal_pdf("Original").unwrap();
        let mut doc = PdfDocument::from_bytes(&source).unwrap();
        let target = TextEditTarget::from_span(&doc.extract_page_text(0).unwrap().spans[0]);
        let mutation = doc
            .style_text(
                0,
                &target,
                &TextStylePatch {
                    font_size: Some(13.5),
                    fill_color: Some([0.15, 0.25, 0.35]),
                    replacement_text: Some(replacement.into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let exported = doc.export_incremental(&mutation).unwrap();
        let mut reopened = PdfDocument::from_bytes(&exported).unwrap();
        assert_eq!(
            reopened.extract_page_text(0).unwrap().plain_text(),
            replacement
        );
    }

    let source = MinimalWriter::create_minimal_pdf("Original").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let target = TextEditTarget::from_span(&doc.extract_page_text(0).unwrap().spans[0]);
    let plan = doc
        .plan_text_style_change(
            0,
            &target,
            &TextStylePatch {
                weight: Some(TextWeight::Bold),
                replacement_text: Some("ڤ ڠ ڽ چ".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(!plan.replacement.is_executable());
    assert!(plan
        .replacement
        .refusal_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("TEXT_STYLE_VARIANT_UNAVAILABLE")));
}

#[test]
fn invalid_style_values_refuse_before_mutation() {
    let patch = TextStylePatch {
        font_size: Some(145.0),
        ..Default::default()
    };
    assert!(patch
        .validate()
        .unwrap_err()
        .to_string()
        .contains("TEXT_STYLE_SIZE_OUT_OF_RANGE"));

    let source = MinimalWriter::create_minimal_pdf("FreeText size validation").unwrap();
    let mut doc = PdfDocument::from_bytes(&source).unwrap();
    let error = doc
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [40.0, 50.0, 260.0, 110.0],
                text: "No silent clamp".into(),
                font_size: Some(145.0),
                color: Some(vec![0.0, 0.0, 0.0]),
            },
        }])
        .unwrap_err();
    assert!(error.to_string().contains("TEXT_STYLE_SIZE_OUT_OF_RANGE"));

    let accepted = doc
        .mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [40.0, 50.0, 360.0, 230.0],
                text: "Exact 144 pt".into(),
                font_size: Some(144.0),
                color: Some(vec![0.0, 0.0, 0.0]),
            },
        }])
        .unwrap();
    let mut reopened = PdfDocument::from_bytes(&accepted).unwrap();
    let annotation = reopened.page_annotations(0).unwrap().remove(0);
    let dict = reopened
        .store_mut()
        .resolve(annotation.object_ref)
        .unwrap()
        .as_dict()
        .unwrap();
    let da = dict.get("DA").and_then(PdfObject::as_string_lossy).unwrap();
    assert!(da.contains("144.00 Tf"));
}
