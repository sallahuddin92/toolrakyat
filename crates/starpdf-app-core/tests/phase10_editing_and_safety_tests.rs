use std::path::PathBuf;
use starpdf::document::PdfDocument;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::selection::{Rect, SelectionState};
use starpdf_app_core::session::SessionManager;

fn resolve_fixture(rel: &str) -> PathBuf {
    let p = PathBuf::from(rel);
    if p.exists() {
        return p;
    }
    let p2 = PathBuf::from("../../").join(rel);
    if p2.exists() {
        return p2;
    }
    let p3 = PathBuf::from("../").join(rel);
    if p3.exists() {
        return p3;
    }
    p
}

/// Minimal valid 2x2 JPEG bytes for image insertion testing.
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
fn test_phase10_form_field_mutation_undo_redo_and_reopen() {
    let fixture_path = resolve_fixture("test-assets/realworld_corpus/forms-employment-application.pdf");
    let bytes = std::fs::read(&fixture_path).expect("read forms fixture");

    let mut manager = SessionManager::new();
    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(fixture_path.clone()),
            bytes,
            title: "forms.pdf".into(),
        })
        .expect("OpenDocument");

    // 1. Mutate text field 'name_field'
    manager
        .dispatch(AppCommand::SetFormField {
            field_name: "name_field".into(),
            value: "Jane Developer".into(),
        })
        .expect("SetFormField name_field");

    // 2. Mutate checkbox 'rust_cb' from true to false (uncheck)
    manager
        .dispatch(AppCommand::SetFormField {
            field_name: "rust_cb".into(),
            value: "false".into(),
        })
        .expect("SetFormField rust_cb");

    // 3. Verify in-memory document state
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let fields = doc.form_fields().unwrap();
        let name_f = fields.iter().find(|f| f.partial_name == "name_field").unwrap();
        assert_eq!(name_f.value, starpdf::forms::FieldValue::Text("Jane Developer".into()));
        let rust_f = fields.iter().find(|f| f.partial_name == "rust_cb").unwrap();
        assert_eq!(rust_f.value, starpdf::forms::FieldValue::Boolean(false));
    }

    // 4. Test Undo / Redo
    manager.dispatch(AppCommand::Undo).expect("Undo checkbox");
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let fields = doc.form_fields().unwrap();
        let rust_f = fields.iter().find(|f| f.partial_name == "rust_cb").unwrap();
        assert_eq!(rust_f.value, starpdf::forms::FieldValue::Boolean(true));
    }

    manager.dispatch(AppCommand::Redo).expect("Redo checkbox");
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let fields = doc.form_fields().unwrap();
        let rust_f = fields.iter().find(|f| f.partial_name == "rust_cb").unwrap();
        assert_eq!(rust_f.value, starpdf::forms::FieldValue::Boolean(false));
    }

    // 5. Test Atomic Save and Independent Reopen
    let temp_save_path = std::env::temp_dir().join(format!("phase10_test_form_{}.pdf", uuid::Uuid::new_v4().simple()));
    manager
        .dispatch(AppCommand::SaveDocument {
            destination_path: Some(temp_save_path.clone()),
        })
        .expect("SaveDocument");

    let saved_bytes = std::fs::read(&temp_save_path).expect("read saved file");
    let mut reopened_doc = PdfDocument::from_bytes(&saved_bytes).expect("reopen saved doc");
    let reopened_fields = reopened_doc.form_fields().expect("form fields");
    let reopened_name = reopened_fields.iter().find(|f| f.partial_name == "name_field").unwrap();
    assert_eq!(reopened_name.value, starpdf::forms::FieldValue::Text("Jane Developer".into()));
    let reopened_cb = reopened_fields.iter().find(|f| f.partial_name == "rust_cb").unwrap();
    assert_eq!(reopened_cb.value, starpdf::forms::FieldValue::Boolean(false));

    let _ = std::fs::remove_file(&temp_save_path);
}

#[test]
fn test_phase10_annotation_addition_and_deletion_lifecycle() {
    let fixture_path = resolve_fixture("test-assets/realworld_corpus/annotations-markup-review.pdf");
    let bytes = std::fs::read(&fixture_path).expect("read annotations fixture");

    let mut manager = SessionManager::new();
    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(fixture_path.clone()),
            bytes,
            title: "annot.pdf".into(),
        })
        .expect("OpenDocument");

    let initial_annot_count = {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        doc.page_annotations(0).unwrap().len()
    };

    // 1. Add Highlight Annotation
    manager
        .dispatch(AppCommand::AddAnnotation {
            page_index: 0,
            subtype: "Highlight".into(),
            rect: [50.0, 700.0, 200.0, 720.0],
            contents: Some("Review this section".into()),
            color: Some(vec![1.0, 1.0, 0.0]),
        })
        .expect("AddAnnotation Highlight");

    let after_add_count = {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), initial_annot_count + 1);
        annots.len()
    };

    // 2. Add FreeText Annotation
    manager
        .dispatch(AppCommand::AddAnnotation {
            page_index: 0,
            subtype: "FreeText".into(),
            rect: [100.0, 500.0, 300.0, 540.0],
            contents: Some("Approved by Engineering".into()),
            color: Some(vec![0.0, 0.5, 0.0]),
        })
        .expect("AddAnnotation FreeText");

    let latest_annot = {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), after_add_count + 1);
        annots.last().unwrap().clone()
    };

    // 3. Select the newly added annotation and delete it via DeleteSelected
    manager
        .dispatch(AppCommand::SetSelection {
            selection: SelectionState::Annotation {
                annot_ref: format!("{} {} R", latest_annot.object_ref.number, latest_annot.object_ref.generation),
                subtype: latest_annot.subtype.as_name().to_string(),
                contents: latest_annot.contents.clone(),
                bounds: Rect::new(latest_annot.rect[0], latest_annot.rect[1], latest_annot.rect[2] - latest_annot.rect[0], latest_annot.rect[3] - latest_annot.rect[1]),
            },
        })
        .expect("SetSelection");

    manager.dispatch(AppCommand::DeleteSelected).expect("DeleteSelected");

    // 4. Verify annotation was removed from the PDF document
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), after_add_count);
    }

    // 5. Undo deletion and verify restored
    manager.dispatch(AppCommand::Undo).expect("Undo");
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), after_add_count + 1);
    }
}

#[test]
fn test_phase10_image_addition_and_deletion_lifecycle() {
    let fixture_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
    let bytes = std::fs::read(&fixture_path).expect("read invoice fixture");

    let mut manager = SessionManager::new();
    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(fixture_path),
            bytes,
            title: "invoice.pdf".into(),
        })
        .expect("OpenDocument");

    let initial_img_count = {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        doc.enumerate_images(0).unwrap().len()
    };

    // 1. Add Image
    let test_jpeg = make_test_jpeg(255, 0, 0);
    manager
        .dispatch(AppCommand::AddImage {
            page_index: 0,
            image_bytes: test_jpeg,
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        })
        .expect("AddImage");

    let added_img = {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let imgs = doc.enumerate_images(0).unwrap();
        assert_eq!(imgs.len(), initial_img_count + 1);
        imgs.last().unwrap().clone()
    };

    // 2. Select the added image and delete via DeleteSelected
    manager
        .dispatch(AppCommand::SetSelection {
            selection: SelectionState::Image {
                star_node_id: starpdf::execution::StarNodeId::new("doc", added_img.object_ref, &added_img.image_id).unwrap(),
                render_node_id: None,
                image_ref: added_img.image_id.clone(),
                bounds: Rect::new(added_img.rect[0], added_img.rect[1], added_img.rect[2] - added_img.rect[0], added_img.rect[3] - added_img.rect[1]),
                is_shared: added_img.is_shared,
            },
        })
        .expect("SetSelection");

    manager.dispatch(AppCommand::DeleteSelected).expect("DeleteSelected");

    // 3. Verify image removed from page content stream
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let imgs = doc.enumerate_images(0).unwrap();
        assert_eq!(imgs.len(), initial_img_count);
    }

    // 4. Undo and verify restored
    manager.dispatch(AppCommand::Undo).expect("Undo");
    {
        let session = manager.active_session().unwrap();
        let mut doc = session.pdf_document().unwrap();
        let imgs = doc.enumerate_images(0).unwrap();
        assert_eq!(imgs.len(), initial_img_count + 1);
    }
}

#[test]
fn test_phase10_page_rearrangement_and_structure_validation() {
    let fixture_path = resolve_fixture("test-assets/realworld_corpus/large-manual-20page.pdf");
    let bytes = std::fs::read(&fixture_path).expect("read manual fixture");

    let mut manager = SessionManager::new();
    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(fixture_path),
            bytes,
            title: "manual20.pdf".into(),
        })
        .expect("OpenDocument");

    assert_eq!(manager.active_session().unwrap().total_pages, 20);

    // 1. Reorder page 0 to 5
    manager
        .dispatch(AppCommand::ReorderPage {
            from_index: 0,
            to_index: 5,
        })
        .expect("ReorderPage");
    assert_eq!(manager.active_session().unwrap().total_pages, 20);

    // 2. Duplicate page 2
    manager
        .dispatch(AppCommand::DuplicatePage { page_index: 2 })
        .expect("DuplicatePage");
    assert_eq!(manager.active_session().unwrap().total_pages, 21);

    // 3. Insert blank page
    manager
        .dispatch(AppCommand::InsertBlankPage {
            after_index: 3,
            width: 612.0,
            height: 792.0,
        })
        .expect("InsertBlankPage");
    assert_eq!(manager.active_session().unwrap().total_pages, 22);

    // 4. Delete page 1
    manager
        .dispatch(AppCommand::DeletePage { page_index: 1 })
        .expect("DeletePage");
    assert_eq!(manager.active_session().unwrap().total_pages, 21);

    // 5. Save and Reopen - Validate Page Tree
    let temp_save = std::env::temp_dir().join(format!("phase10_pages_{}.pdf", uuid::Uuid::new_v4().simple()));
    manager
        .dispatch(AppCommand::SaveDocument {
            destination_path: Some(temp_save.clone()),
        })
        .expect("SaveDocument");

    let saved_bytes = std::fs::read(&temp_save).expect("read saved file");
    let mut reopened = PdfDocument::from_bytes(&saved_bytes).expect("reopen");
    assert_eq!(reopened.page_count().unwrap(), 21);

    let _ = std::fs::remove_file(&temp_save);
}

#[test]
fn test_phase10_document_safety_fail_closed_policy() {
    let fixture_path = resolve_fixture("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
    let bytes = std::fs::read(&fixture_path).expect("read fixture");

    let mut manager = SessionManager::new();
    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(fixture_path),
            bytes,
            title: "invoice.pdf".into(),
        })
        .expect("OpenDocument");

    // 1. Manually simulate restricted permissions: editing forbidden
    {
        let session = manager.active_session_mut().unwrap();
        session.capabilities.can_edit.allowed = false;
        session.capabilities.can_edit.reason = "Signed or policy-locked document".into();
    }

    // 2. Any mutating command must fail closed with typed refusal
    let res = manager.dispatch(AppCommand::ReplaceText {
        target_node: "p0_s0_i0_o0".into(),
        new_text: "Malicious Tampering".into(),
    });
    assert!(res.is_err(), "Must refuse editing restricted document");
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("Editing not permitted") || err_msg.contains("policy-locked"));

    let res_ann = manager.dispatch(AppCommand::AddAnnotation {
        page_index: 0,
        subtype: "Highlight".into(),
        rect: [0.0, 0.0, 100.0, 100.0],
        contents: None,
        color: None,
    });
    assert!(res_ann.is_err(), "Must refuse annotation mutation on restricted document");

    let res_del = manager.dispatch(AppCommand::DeleteSelected);
    assert!(res_del.is_err(), "Must refuse deletion on restricted document");
}
