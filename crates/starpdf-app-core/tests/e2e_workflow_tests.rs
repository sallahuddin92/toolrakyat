use std::fs;
use std::path::PathBuf;
use std::process::Command;

use starpdf::security::{CapabilityDecision, DocumentCapabilities};
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::error::AppCoreError;
use starpdf_app_core::selection::{FontInfo, Rect, SelectionState};
use starpdf_app_core::session::SessionManager;
use tempfile::NamedTempFile;

const MULTISPAN_PDF_BYTES: &[u8] =
    include_bytes!("../../../test-assets/text-multispan-heading.pdf");

#[test]
fn test_real_pdf_e2e_workflow_edit_undo_save_reopen_and_validate() {
    let mut manager = SessionManager::new();

    // 1. Open real PDF in session manager
    let res = manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: MULTISPAN_PDF_BYTES.to_vec(),
            title: "multispan_heading.pdf".into(),
        })
        .expect("Failed to open real test asset PDF");

    assert!(res.success);
    let session_id = res.affected_session_id.expect("Expected session ID");
    assert_eq!(manager.session_count(), 1);

    let session = manager.active_session().expect("Expected active session");
    assert_eq!(session.id, session_id);
    assert_eq!(session.total_pages, 1);
    assert_eq!(session.current_page, 0);
    assert!(!session.is_dirty());
    assert_eq!(session.history.undo_depth(), 0);
    // Verified status must not be claimed without cryptographic proof or mutation verification
    assert_eq!(session.evidence_level(), "LOCAL_PARSED");

    // 2. Extract text spans from initial document and locate target
    let mut doc = session.pdf_document().expect("Valid PdfDocument");
    let pages = doc.extract_all_text().expect("Text extraction succeeded");
    assert!(!pages.is_empty());
    let page0 = &pages[0];
    assert!(page0.spans.len() >= 3, "Expected multiple text spans");

    let first_span = &page0.spans[0];
    let target_span_id = first_span.span_id.clone();
    let original_text = first_span.text.clone();
    assert_eq!(original_text, "Star");

    // Record unrelated downstream span to verify it is preserved untouched
    let downstream_span = &page0.spans[1];
    let downstream_span_id = downstream_span.span_id.clone();
    let downstream_text = downstream_span.text.clone();
    assert_eq!(downstream_text, "Orion");

    // 3. Select target span
    let star_node_id = starpdf::execution::StarNodeId::new(
        "doc",
        starpdf::syntax::object::ObjectRef::new(1, 0),
        &target_span_id,
    )
    .unwrap();
    let sel = SelectionState::Text {
        star_node_id,
        render_node_id: None,
        text: original_text.clone(),
        bounds: Rect::new(
            first_span.x,
            first_span.y,
            first_span.width,
            first_span.height,
        ),
        font_info: Some(FontInfo::new(
            first_span.font_name.clone(),
            first_span.font_size,
        )),
        is_editable: true,
    };
    manager
        .dispatch(AppCommand::SetSelection { selection: sel })
        .expect("SetSelection succeeded");

    // 4. Dispatch ReplaceText
    let new_text = "StarGalaxyNova";
    let replace_res = manager
        .dispatch(AppCommand::ReplaceText {
            target_node: target_span_id.clone(),
            new_text: new_text.into(),
        })
        .expect("ReplaceText succeeded");
    assert!(replace_res.success);

    // Verify session state updated
    let session = manager.active_session().expect("Expected active session");
    assert!(session.is_dirty(), "Session must be dirty after edit");
    assert_eq!(session.history.undo_depth(), 1);
    assert_eq!(session.evidence_level(), "MUTATION_STAGED");

    // Verify text within session
    let mut doc_mutated = session.pdf_document().expect("Valid PdfDocument");
    let pages_mutated = doc_mutated.extract_all_text().expect("Extract text");
    let mutated_first_span = pages_mutated[0]
        .spans
        .iter()
        .find(|s| s.span_id == target_span_id)
        .expect("Target span exists");
    assert_eq!(mutated_first_span.text, new_text);

    // Verify unrelated downstream span is preserved intact
    let preserved_downstream_span = pages_mutated[0]
        .spans
        .iter()
        .find(|s| s.span_id == downstream_span_id)
        .expect("Downstream span exists");
    assert_eq!(preserved_downstream_span.text, downstream_text);

    // 5. Test Undo: reverts text and dirty state
    let undo_res = manager.dispatch(AppCommand::Undo).expect("Undo succeeded");
    assert!(undo_res.success);

    let session = manager.active_session().expect("Active session");
    assert!(
        !session.is_dirty(),
        "Session must be clean after undo to baseline"
    );
    assert_eq!(session.history.undo_depth(), 0);
    assert_eq!(session.history.redo_depth(), 1);

    let mut doc_undone = session.pdf_document().expect("Valid PdfDocument");
    let pages_undone = doc_undone.extract_all_text().expect("Extract text");
    let reverted_first_span = pages_undone[0]
        .spans
        .iter()
        .find(|s| s.span_id == target_span_id)
        .expect("Target span exists");
    assert_eq!(reverted_first_span.text, original_text);

    // 6. Test Redo: re-applies text
    let redo_res = manager.dispatch(AppCommand::Redo).expect("Redo succeeded");
    assert!(redo_res.success);

    let session = manager.active_session().expect("Active session");
    assert!(session.is_dirty());
    assert_eq!(session.history.undo_depth(), 1);
    assert_eq!(session.history.redo_depth(), 0);

    // 7. Atomic Save to disk
    let temp_save_file = NamedTempFile::new().expect("Failed to create temp save file");
    let temp_path = temp_save_file.path().to_path_buf();

    let save_res = manager
        .dispatch(AppCommand::SaveDocument {
            destination_path: Some(temp_path.clone()),
        })
        .expect("SaveDocument succeeded");
    assert!(save_res.success);

    // Session must be marked clean after successful save
    let session = manager.active_session().expect("Active session");
    assert!(!session.is_dirty(), "Session must be clean after save");
    assert_eq!(session.file_path, Some(temp_path.clone()));

    // 8. Independent Validator 1: Python pypdf text extraction
    let py_output = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import pypdf\nreader = pypdf.PdfReader('{}')\nassert len(reader.pages) == 1\ntext = reader.pages[0].extract_text()\nassert 'StarGalaxyNova' in text, f'Missing edited text: {{text[:100]}}'\nassert 'Orion' in text, 'Missing preserved text'\nprint('PYPDF_VALIDATED_OK')",
            temp_path.display()
        ))
        .output();

    if let Ok(out) = py_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("PYPDF_VALIDATED_OK"),
            "Python pypdf validation output: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // 9. Independent Validator 2: Node pdf-lib structure inspection
    let node_output = Command::new("node")
        .arg("-e")
        .arg(format!(
            "const fs = require('fs');\nconst {{ PDFDocument }} = require('pdf-lib');\nasync function validate() {{\n  const bytes = fs.readFileSync('{}');\n  const doc = await PDFDocument.load(bytes);\n  if (doc.getPageCount() !== 1) process.exit(1);\n  const page = doc.getPage(0);\n  if (page.getWidth() !== 612 || page.getHeight() !== 792) process.exit(2);\n  console.log('PDFLIB_VALIDATED_OK');\n}}\nvalidate();",
            temp_path.display()
        ))
        .output();

    if let Ok(out) = node_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("PDFLIB_VALIDATED_OK"),
            "Node pdf-lib validation output: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // 10. Close session and verify count
    manager
        .dispatch(AppCommand::CloseSession { session_id })
        .expect("CloseSession succeeded");
    assert_eq!(manager.session_count(), 0);

    // 11. Reopen exported bytes in fresh session
    let saved_bytes = fs::read(&temp_path).expect("Read saved PDF bytes");
    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(temp_path),
            bytes: saved_bytes,
            title: "reopened.pdf".into(),
        })
        .expect("Reopen succeeded");

    assert_eq!(manager.session_count(), 1);
    let reopened_session = manager.active_session().expect("Active session");
    assert_eq!(reopened_session.total_pages, 1);
    let mut reopened_doc = reopened_session.pdf_document().expect("Valid reopened doc");
    let reopened_pages = reopened_doc.extract_all_text().expect("Extract text");
    assert_eq!(reopened_pages[0].spans[0].text, new_text);
    assert_eq!(reopened_pages[0].spans[1].text, downstream_text);
}

#[test]
fn test_fail_closed_security_permissions() {
    let mut manager = SessionManager::new();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: MULTISPAN_PDF_BYTES.to_vec(),
            title: "protected.pdf".into(),
        })
        .expect("Open succeeded");

    // Manually impose restricted capabilities to simulate an encrypted or signed restricted document
    {
        let session = manager.active_session_mut().unwrap();
        session.capabilities = DocumentCapabilities {
            can_view: CapabilityDecision {
                allowed: true,
                reason: "VIEW_ONLY_CREDENTIAL",
            },
            can_edit: CapabilityDecision {
                allowed: false,
                reason: "SIGNED_DOCUMENT_RESTRICTED",
            },
            can_extract: CapabilityDecision {
                allowed: true,
                reason: "EXTRACT_ALLOWED",
            },
            can_annotate: CapabilityDecision {
                allowed: false,
                reason: "ANNOTATION_FORBIDDEN",
            },
            can_modify_pages: CapabilityDecision {
                allowed: false,
                reason: "PAGE_MUTATION_RESTRICTED",
            },
        };
    }

    // 1. Text replacement must fail closed
    let err_replace = manager
        .dispatch(AppCommand::ReplaceText {
            target_node: "p0_s0_i0_o0".into(),
            new_text: "Unauthorized Edit".into(),
        })
        .expect_err("Must refuse ReplaceText");

    match err_replace {
        AppCoreError::Refused { code, reason } => {
            assert_eq!(code, "MUTATION_REFUSED");
            assert_eq!(reason, "SIGNED_DOCUMENT_RESTRICTED");
        }
        other => panic!("Expected AppCoreError::Refused, got {other:?}"),
    }

    // 2. MoveText must fail closed
    let err_move = manager
        .dispatch(AppCommand::MoveText {
            target_node: "p0_s0_i0_o0".into(),
            dx: 10.0,
            dy: 10.0,
        })
        .expect_err("Must refuse MoveText");
    assert!(matches!(err_move, AppCoreError::Refused { .. }));

    // 3. DeletePage must fail closed
    let err_page = manager
        .dispatch(AppCommand::DeletePage { page_index: 0 })
        .expect_err("Must refuse DeletePage");
    assert!(matches!(err_page, AppCoreError::Refused { .. }));

    // 4. SaveDocument must fail closed
    let err_save = manager
        .dispatch(AppCommand::SaveDocument {
            destination_path: Some(PathBuf::from("/tmp/should_never_be_written.pdf")),
        })
        .expect_err("Must refuse SaveDocument");
    assert!(matches!(err_save, AppCoreError::Refused { .. }));

    // 5. Bytes in memory must remain completely unaltered
    let session = manager.active_session().unwrap();
    assert_eq!(session.bytes, MULTISPAN_PDF_BYTES);
    assert!(!session.is_dirty());
    assert_eq!(session.history.undo_depth(), 0);
}
