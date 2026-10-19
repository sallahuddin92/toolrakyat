use std::path::PathBuf;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::{Rect, SelectionState};
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
fn test_rc1_independent_document_safety_and_preservation() {
    let out_dir = resolve_fixture("target/rc1_acceptance");
    let _ = std::fs::create_dir_all(&out_dir);

    // 1. Independent Document A: 14-page Real PDF (Producer: pdf-lib)
    {
        let orig_path = resolve_fixture("test-assets/14-page-real.pdf");
        let bytes = std::fs::read(&orig_path).expect("read 14-page-real");
        std::fs::copy(&orig_path, out_dir.join("orig_14page_real.pdf")).expect("copy orig");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "14-page-real.pdf".into(),
            })
            .expect("open 14-page");

        assert_eq!(manager.active_session().unwrap().total_pages, 14);

        // Add annotation on page 0
        manager
            .dispatch(AppCommand::AddAnnotation {
                page_index: 0,
                subtype: "FreeText".into(),
                rect: [80.0, 600.0, 300.0, 650.0],
                contents: Some("RC1 Independent Qualification Note".into()),
                color: Some(vec![0.1, 0.5, 0.9]),
            })
            .expect("add freetext");

        // Move annotation
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
                    annot_ref,
                    subtype: "FreeText".into(),
                    contents: Some("RC1 Independent Qualification Note".into()),
                    bounds: Rect::new(80.0, 142.0, 220.0, 50.0),
                },
            })
            .expect("select annot");

        manager
            .dispatch(AppCommand::MoveElement { dx: 15.0, dy: 10.0 })
            .expect("move annot");

        // Add Image
        let img_bytes = make_test_jpeg(40, 120, 220);
        manager
            .dispatch(AppCommand::AddImage {
                page_index: 0,
                image_bytes: img_bytes,
                x: 80.0,
                y: 500.0,
                width: 100.0,
                height: 50.0,
            })
            .expect("add image");

        // Page operation: delete page 13
        manager
            .dispatch(AppCommand::DeletePage { page_index: 13 })
            .expect("delete page 13");
        assert_eq!(manager.active_session().unwrap().total_pages, 13);

        // Undo deletion
        manager.dispatch(AppCommand::Undo).expect("undo delete");
        assert_eq!(manager.active_session().unwrap().total_pages, 14);

        // Save mutated document
        let dest = out_dir.join("mutated_14page_real.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save 14page");
        assert!(dest.exists());
    }

    // 2. Independent Document B: macOS Quartz PDFContext
    {
        let quartz_path = PathBuf::from("/tmp/apple_quartz_doc.pdf");
        if !quartz_path.exists() {
            // Generate if missing
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg("echo 'Independent Document from macOS CUPS Printer Engine' > /tmp/cups_test.txt && cupsfilter /tmp/cups_test.txt > /tmp/apple_quartz_doc.pdf 2>/dev/null")
                .status();
        }

        if quartz_path.exists() {
            let bytes = std::fs::read(&quartz_path).expect("read quartz");
            std::fs::copy(&quartz_path, out_dir.join("orig_apple_quartz.pdf")).expect("copy quartz");

            let mut manager = SessionManager::new();
            manager
                .dispatch(AppCommand::OpenDocument {
                    path: Some(quartz_path),
                    bytes,
                    title: "apple_quartz_doc.pdf".into(),
                })
                .expect("open quartz");

            // Add annotation
            manager
                .dispatch(AppCommand::AddAnnotation {
                    page_index: 0,
                    subtype: "FreeText".into(),
                    rect: [100.0, 600.0, 350.0, 660.0],
                    contents: Some("Quartz Document Verified in StarPDF".into()),
                    color: Some(vec![0.2, 0.7, 0.3]),
                })
                .expect("add annot to quartz");

            // Insert blank page
            manager
                .dispatch(AppCommand::InsertBlankPage {
                    after_index: 0,
                    width: 612.0,
                    height: 792.0,
                })
                .expect("insert blank page");
            assert_eq!(manager.active_session().unwrap().total_pages, 2);

            let dest = out_dir.join("mutated_apple_quartz.pdf");
            manager
                .dispatch(AppCommand::SaveDocument {
                    destination_path: Some(dest.clone()),
                })
                .expect("save quartz doc");
            assert!(dest.exists());
        }
    }

    // 3. Independent Document C: Forms Application (AcroForm Controls)
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let bytes = std::fs::read(&orig_path).expect("read forms");
        std::fs::copy(&orig_path, out_dir.join("orig_forms_rc1.pdf")).expect("copy forms");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "forms.pdf".into(),
            })
            .expect("open forms");

        // Toggle checkbox with "toggle"
        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "rust_cb".into(),
                value: "toggle".into(),
            })
            .expect("toggle rust_cb");

        // Commit text field
        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "name_field".into(),
                value: "RC1 Lead Acceptance Engineer".into(),
            })
            .expect("set name_field");

        let dest = out_dir.join("mutated_forms_rc1.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save forms rc1");
        assert!(dest.exists());
    }

    // 4. Independent Document D: Enterprise Invoice (Type1 Helvetica Layout)
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let bytes = std::fs::read(&orig_path).expect("read invoice");
        std::fs::copy(&orig_path, out_dir.join("orig_invoice_rc1.pdf")).expect("copy invoice");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "invoice.pdf".into(),
            })
            .expect("open invoice");

        // Add image stamp
        let stamp_bytes = make_test_jpeg(10, 160, 80);
        manager
            .dispatch(AppCommand::AddImage {
                page_index: 0,
                image_bytes: stamp_bytes,
                x: 100.0,
                y: 550.0,
                width: 90.0,
                height: 45.0,
            })
            .expect("add stamp image");

        // Move image
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let images = doc.enumerate_images(0).unwrap();
        let last_img = images.last().unwrap();
        let image_id = last_img.image_id.clone();
        drop(doc);

        manager
            .dispatch(AppCommand::SetSelection {
                selection: SelectionState::Image {
                    star_node_id: starpdf_app_core::StarNodeId::new("doc", last_img.object_ref, &image_id).unwrap(),
                    render_node_id: None,
                    image_ref: image_id.clone(),
                    bounds: Rect::new(100.0, 197.0, 90.0, 45.0),
                    is_shared: true,
                },
            })
            .expect("select image");

        manager
            .dispatch(AppCommand::MoveElement { dx: 20.0, dy: 15.0 })
            .expect("move image");

        manager
            .dispatch(AppCommand::ResizeElement { dw: 30.0, dh: 15.0 })
            .expect("resize image");

        let dest = out_dir.join("mutated_invoice_rc1.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save invoice");
        assert!(dest.exists());
    }

    // 5. Corrupted & Invalid Document Refusal
    {
        let invalid_path = resolve_fixture("test-assets/invalid.pdf");
        let bytes = std::fs::read(&invalid_path).unwrap_or_else(|_| b"not a valid pdf".to_vec());
        let mut manager = SessionManager::new();
        let res = manager.dispatch(AppCommand::OpenDocument {
            path: Some(invalid_path),
            bytes,
            title: "invalid.pdf".into(),
        });
        assert!(res.is_err(), "Must fail closed on malformed/invalid PDF");
    }

    // 6. Security Refusal on Restricted Document
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let bytes = std::fs::read(&orig_path).expect("read forms");
        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "forms.pdf".into(),
            })
            .expect("open forms");

        {
            let session = manager.active_session_mut().unwrap();
            session.capabilities.can_edit.allowed = false;
            session.capabilities.can_edit.reason = "Cryptographic signature lock".into();
        }

        let res = manager.dispatch(AppCommand::SetFormField {
            field_name: "name_field".into(),
            value: "Attacking restricted document".into(),
        });
        assert!(res.is_err(), "Must fail closed when editing capability is restricted");
    }
}
