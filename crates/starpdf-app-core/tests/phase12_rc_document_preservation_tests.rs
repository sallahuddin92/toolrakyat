use std::path::PathBuf;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::SelectionState;
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
fn test_phase12_generate_rc_preservation_artifacts() {
    let out_dir = resolve_fixture("target/phase12_validation");
    let _ = std::fs::create_dir_all(&out_dir);

    // 1. Form Field Mutation Artifact
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let bytes = std::fs::read(&orig_path).expect("read forms");
        std::fs::copy(&orig_path, out_dir.join("orig_form.pdf")).expect("copy orig form");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
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

        let dest = out_dir.join("mutated_form_toggled.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save forms");
        assert!(dest.exists());
    }

    // 2. Annotation Addition, Move, and Resize Artifact
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/annotations-markup-review.pdf");
        let bytes = std::fs::read(&orig_path).expect("read annotations");
        std::fs::copy(&orig_path, out_dir.join("orig_annotation.pdf")).expect("copy orig annotation");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "annotations.pdf".into(),
            })
            .expect("open annotations");

        // Add annotation
        manager
            .dispatch(AppCommand::AddAnnotation {
                page_index: 0,
                subtype: "FreeText".into(),
                rect: [100.0, 650.0, 300.0, 700.0],
                contents: Some("RC Hardening Note".into()),
                color: Some(vec![0.1, 0.4, 0.8]),
            })
            .expect("add freetext");

        // Select and Move annotation
        let annot_ref = {
            let session = manager.active_session().unwrap();
            let mut doc = session.pdf_document().unwrap();
            let annots = doc.page_annotations(0).unwrap();
            let last = annots.last().unwrap();
            format!("{} 0 R", last.object_ref.number)
        };

        manager
            .dispatch(AppCommand::SetSelection {
                selection: SelectionState::Annotation {
                    annot_ref: annot_ref.clone(),
                    subtype: "FreeText".into(),
                    contents: Some("RC Hardening Note".into()),
                    bounds: starpdf_app_core::selection::Rect::new(100.0, 92.0, 200.0, 50.0),
                },
            })
            .expect("set selection");

        manager
            .dispatch(AppCommand::MoveElement { dx: 15.0, dy: 10.0 })
            .expect("move element");

        manager
            .dispatch(AppCommand::ResizeElement { dw: 25.0, dh: 15.0 })
            .expect("resize element");

        let dest = out_dir.join("mutated_annotation_moved_resized.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save annotations");
        assert!(dest.exists());
    }

    // 3. Image Insertion and Replacement Artifact
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let bytes = std::fs::read(&orig_path).expect("read invoice");
        std::fs::copy(&orig_path, out_dir.join("orig_image.pdf")).expect("copy orig image");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "invoice.pdf".into(),
            })
            .expect("open invoice");

        // Add initial image
        let initial_img = make_test_jpeg(255, 0, 0);
        manager
            .dispatch(AppCommand::AddImage {
                page_index: 0,
                image_bytes: initial_img,
                x: 100.0,
                y: 500.0,
                width: 120.0,
                height: 80.0,
            })
            .expect("add image");

        // Replace the image with distinct blue JPEG
        let rep_img = make_test_jpeg(0, 50, 220);
        let img_id = {
            let session = manager.active_session().unwrap();
            let mut doc = session.pdf_document().unwrap();
            let images = doc.enumerate_images(0).unwrap();
            images.last().unwrap().image_id.clone()
        };

        manager
            .dispatch(AppCommand::ReplaceImage {
                page_index: 0,
                image_ref: img_id.clone(),
                image_bytes: rep_img,
            })
            .expect("replace image");

        let dest = out_dir.join("mutated_image_replaced.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save image");
        assert!(dest.exists());
    }

    // 4. Page Duplication Artifact
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/office-technical-report.pdf");
        let bytes = std::fs::read(&orig_path).expect("read technical report");
        std::fs::copy(&orig_path, out_dir.join("orig_page.pdf")).expect("copy orig page");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "report.pdf".into(),
            })
            .expect("open report");

        let initial_pages = manager.active_session().unwrap().total_pages;

        manager
            .dispatch(AppCommand::DuplicatePage { page_index: 0 })
            .expect("duplicate page");

        assert_eq!(manager.active_session().unwrap().total_pages, initial_pages + 1);

        let dest = out_dir.join("mutated_page_duplicated.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save duplicated page");
        assert!(dest.exists());
    }

    println!("  [Phase 12 Preservation Artifacts Generated Successfully in {:?}]", out_dir);
}
