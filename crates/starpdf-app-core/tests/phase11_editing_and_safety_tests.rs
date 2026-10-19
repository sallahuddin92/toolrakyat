use std::path::PathBuf;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::session::SessionManager;

fn resolve_fixture(rel: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().and_then(|p| p.parent()).unwrap_or(&manifest);
    root.join(rel)
}

fn make_test_jpeg(r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut data = vec![
        0xFF, 0xD8, // SOI
        0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00,
        0x01, 0x00, 0x00, // APP0
        0xFF, 0xDB, 0x00, 0x43, 0x00, // DQT
    ];
    data.extend(std::iter::repeat_n(16, 64));
    data.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x11, 0x08, // SOF0
        0x00, 0x02, // Height: 2
        0x00, 0x02, // Width: 2
        0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00, // 3 components
        0xFF, 0xC4, 0x00, 0x1F, 0x00, // DHT
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA,
        0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, // SOS
        r, g, b, 0x7F, 0xFF, 0xD9, // Scan data + EOI
    ]);
    data
}

#[test]
fn test_phase11_generate_independent_validation_artifacts() {
    let out_dir = resolve_fixture("target/phase11_validation");
    let _ = std::fs::create_dir_all(&out_dir);

    // 1. Form Mutation Artifact
    {
        let fixture_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let bytes = std::fs::read(&fixture_path).expect("read forms");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(fixture_path),
                bytes,
                title: "forms.pdf".into(),
            })
            .expect("open forms");

        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "name_field".into(),
                value: "Lead Systems Architect".into(),
            })
            .expect("set name_field");

        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "rust_cb".into(),
                value: "false".into(),
            })
            .expect("set rust_cb");

        let dest = out_dir.join("mutated_forms.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save forms");
        assert!(dest.exists());
    }

    // 2. Annotation Mutation Artifact
    {
        let fixture_path = resolve_fixture("test-assets/realworld_corpus/annotations-markup-review.pdf");
        let bytes = std::fs::read(&fixture_path).expect("read annotations");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(fixture_path),
                bytes,
                title: "annotations.pdf".into(),
            })
            .expect("open annotations");

        manager
            .dispatch(AppCommand::AddAnnotation {
                page_index: 0,
                subtype: "FreeText".into(),
                rect: [100.0, 650.0, 350.0, 700.0],
                contents: Some("Approved by Lead Architect on 2026-10-19".into()),
                color: Some(vec![0.1, 0.5, 0.9]),
            })
            .expect("add freetext");

        manager
            .dispatch(AppCommand::AddAnnotation {
                page_index: 0,
                subtype: "Highlight".into(),
                rect: [72.0, 720.0, 400.0, 740.0],
                contents: Some("Highlight key clause".into()),
                color: Some(vec![1.0, 0.9, 0.2]),
            })
            .expect("add highlight");

        let dest = out_dir.join("mutated_annotations.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save annotations");
        assert!(dest.exists());
    }

    // 3. Image Mutation Artifact
    {
        let fixture_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let bytes = std::fs::read(&fixture_path).expect("read invoice");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(fixture_path),
                bytes,
                title: "invoice.pdf".into(),
            })
            .expect("open invoice");

        let jpeg_bytes = make_test_jpeg(0x10, 0x80, 0xE0);
        manager
            .dispatch(AppCommand::AddImage {
                page_index: 0,
                image_bytes: jpeg_bytes,
                x: 120.0,
                y: 450.0,
                width: 140.0,
                height: 90.0,
            })
            .expect("add image");

        let dest = out_dir.join("mutated_image.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save image");
        assert!(dest.exists());
    }

    // 4. Page Tree Mutation Artifact
    {
        let fixture_path = resolve_fixture("test-assets/realworld_corpus/large-manual-20page.pdf");
        let bytes = std::fs::read(&fixture_path).expect("read large manual");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(fixture_path),
                bytes,
                title: "manual.pdf".into(),
            })
            .expect("open manual");

        // Duplicate page 0 -> now 21 pages
        manager
            .dispatch(AppCommand::DuplicatePage { page_index: 0 })
            .expect("duplicate page 0");

        // Insert blank page after index 1 -> now 22 pages
        manager
            .dispatch(AppCommand::InsertBlankPage {
                after_index: 1,
                width: 612.0,
                height: 792.0,
            })
            .expect("insert blank page");

        // Move page 0 to index 5
        manager
            .dispatch(AppCommand::ReorderPage {
                from_index: 0,
                to_index: 5,
            })
            .expect("reorder page 0 to 5");

        // Delete page 10 -> now 21 pages
        manager
            .dispatch(AppCommand::DeletePage { page_index: 10 })
            .expect("delete page 10");

        let dest = out_dir.join("mutated_pages.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save pages");
        assert!(dest.exists());
    }

    // 5. Text Replacement Artifact
    {
        let fixture_path = resolve_fixture("test-assets/realworld_corpus/office-technical-report.pdf");
        let bytes = std::fs::read(&fixture_path).expect("read tech report");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(fixture_path),
                bytes,
                title: "report.pdf".into(),
            })
            .expect("open report");

        let session = manager.active_session().unwrap();
        let spans = session.extract_page_text_spans(0).expect("extract text");
        if let Some(target) = spans.first() {
            manager
                .dispatch(AppCommand::ReplaceText {
                    target_node: target.span_id.clone(),
                    new_text: "CONFIDENTIAL REVIEW DRAFT".into(),
                })
                .expect("replace text");
        }

        let dest = out_dir.join("mutated_text.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save text");
        assert!(dest.exists());
    }
}
