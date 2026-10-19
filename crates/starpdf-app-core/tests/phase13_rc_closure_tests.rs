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
fn test_phase13_generate_rc_closure_artifacts() {
    let out_dir = resolve_fixture("target/phase13_validation");
    let _ = std::fs::create_dir_all(&out_dir);

    // 1. Multiple Revisions Qualification (Chained /Prev Incremental Revisions)
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
        let bytes = std::fs::read(&orig_path).expect("read forms");
        std::fs::copy(&orig_path, out_dir.join("orig_forms_multi_rev.pdf")).expect("copy orig");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "forms.pdf".into(),
            })
            .expect("open forms");

        // Revision 1: Set text field
        manager
            .dispatch(AppCommand::SetFormField {
                field_name: "name_field".into(),
                value: "Phase 13 Release Lead".into(),
            })
            .expect("rev 1");
        let rev1_dest = out_dir.join("multi_rev_step1.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(rev1_dest.clone()),
            })
            .expect("save rev 1");

        // Revision 2: Reopen saved step 1 and toggle checkbox
        let rev1_bytes = std::fs::read(&rev1_dest).expect("read rev 1");
        let mut mgr2 = SessionManager::new();
        mgr2.dispatch(AppCommand::OpenDocument {
            path: Some(rev1_dest),
            bytes: rev1_bytes,
            title: "forms_rev1.pdf".into(),
        }).expect("open rev 1");

        mgr2.dispatch(AppCommand::SetFormField {
            field_name: "rust_cb".into(),
            value: "false".into(),
        }).expect("rev 2");
        let rev2_dest = out_dir.join("multi_rev_step2.pdf");
        mgr2.dispatch(AppCommand::SaveDocument {
            destination_path: Some(rev2_dest.clone()),
        }).expect("save rev 2");

        // Revision 3: Reopen saved step 2 and add FreeText annotation
        let rev2_bytes = std::fs::read(&rev2_dest).expect("read rev 2");
        let mut mgr3 = SessionManager::new();
        mgr3.dispatch(AppCommand::OpenDocument {
            path: Some(rev2_dest),
            bytes: rev2_bytes,
            title: "forms_rev2.pdf".into(),
        }).expect("open rev 2");

        mgr3.dispatch(AppCommand::AddAnnotation {
            page_index: 0,
            subtype: "FreeText".into(),
            rect: [120.0, 680.0, 320.0, 720.0],
            contents: Some("Revision 3 Qualified".into()),
            color: Some(vec![0.1, 0.6, 0.2]),
        }).expect("rev 3 add annot");

        let rev3_dest = out_dir.join("mutated_multi_revision_chained.pdf");
        mgr3.dispatch(AppCommand::SaveDocument {
            destination_path: Some(rev3_dest.clone()),
        }).expect("save rev 3");
        assert!(rev3_dest.exists());
    }

    // 2. Overlapping Objects & Transparency Qualification
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/complex-infographic-graphics.pdf");
        let bytes = std::fs::read(&orig_path).expect("read infographic");
        std::fs::copy(&orig_path, out_dir.join("orig_infographic.pdf")).expect("copy orig infographic");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "infographic.pdf".into(),
            })
            .expect("open infographic");

        // Add annotation overlapping with existing complex graphics
        manager
            .dispatch(AppCommand::AddAnnotation {
                page_index: 0,
                subtype: "Highlight".into(),
                rect: [150.0, 450.0, 350.0, 520.0],
                contents: Some("Overlapping Markup".into()),
                color: Some(vec![1.0, 0.8, 0.0]),
            })
            .expect("add highlight");

        // Add semi-transparent image stamp
        let stamp_bytes = make_test_jpeg(220, 50, 50);
        manager
            .dispatch(AppCommand::AddImage {
                page_index: 0,
                image_bytes: stamp_bytes,
                x: 100.0,
                y: 400.0,
                width: 80.0,
                height: 80.0,
            })
            .expect("add stamp image");

        let dest = out_dir.join("mutated_overlapping_transparency.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save overlapping doc");
        assert!(dest.exists());
    }

    // 3. Shared XObjects Isolation Qualification
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
        let bytes = std::fs::read(&orig_path).expect("read invoice");
        std::fs::copy(&orig_path, out_dir.join("orig_invoice.pdf")).expect("copy orig invoice");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "invoice.pdf".into(),
            })
            .expect("open invoice");

        // Insert image on page 0
        let img_bytes = make_test_jpeg(30, 140, 240);
        manager
            .dispatch(AppCommand::AddImage {
                page_index: 0,
                image_bytes: img_bytes,
                x: 80.0,
                y: 600.0,
                width: 120.0,
                height: 60.0,
            })
            .expect("add image");

        // Move image with clone_if_shared: true
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
                    bounds: Rect::new(80.0, 132.0, 120.0, 60.0),
                    is_shared: true,
                },
            })
            .expect("select image");

        manager
            .dispatch(AppCommand::MoveElement { dx: 20.0, dy: 15.0 })
            .expect("move image");

        let dest = out_dir.join("mutated_shared_xobject_isolated.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save shared xobject doc");
        assert!(dest.exists());
    }

    // 4. Multilingual Document Conformance Artifact
    {
        let orig_path = resolve_fixture("test-assets/realworld_corpus/multilingual-arabic-jawi.pdf");
        let bytes = std::fs::read(&orig_path).expect("read multilingual");
        std::fs::copy(&orig_path, out_dir.join("orig_multilingual.pdf")).expect("copy orig multilingual");

        let mut manager = SessionManager::new();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(orig_path),
                bytes,
                title: "multilingual.pdf".into(),
            })
            .expect("open multilingual");

        // Add note in Arabic
        manager
            .dispatch(AppCommand::AddAnnotation {
                page_index: 0,
                subtype: "FreeText".into(),
                rect: [100.0, 500.0, 350.0, 560.0],
                contents: Some("مرحبا بكم في سمارت بي دي إف".into()),
                color: Some(vec![0.2, 0.5, 0.8]),
            })
            .expect("add arabic annotation");

        let dest = out_dir.join("mutated_multilingual_arabic_annotation.pdf");
        manager
            .dispatch(AppCommand::SaveDocument {
                destination_path: Some(dest.clone()),
            })
            .expect("save multilingual doc");
        assert!(dest.exists());
    }

    // 5. Unsafe Mutation Refusal Policy Verification
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

        // Lock editing capabilities
        {
            let session = manager.active_session_mut().unwrap();
            session.capabilities.can_edit.allowed = false;
            session.capabilities.can_edit.reason = "Cryptographic signature lock".into();
        }

        // Attempt mutation -> MUST fail closed
        let res = manager.dispatch(AppCommand::SetFormField {
            field_name: "name_field".into(),
            value: "Unauthorized Hack".into(),
        });
        assert!(res.is_err(), "Must refuse mutation when capabilities forbid editing");
    }
}
