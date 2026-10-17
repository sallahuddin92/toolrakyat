use starpdf::writer::MinimalWriter;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::editing::TextEditingSession;
use starpdf_app_core::selection::{FontInfo, Rect};
use starpdf_app_core::session::SessionManager;

#[test]
fn test_text_editing_session_draft_lifecycle() {
    let mut session = TextEditingSession::new(
        "p0_s0_i0_o0",
        0,
        "Original Text",
        Some(FontInfo::new("Helvetica", 14.0)),
        Some(Rect::new(10.0, 20.0, 100.0, 14.0)),
    );

    assert_eq!(session.original_text(), "Original Text");
    assert_eq!(session.draft_text(), "Original Text");
    assert!(!session.is_dirty());
    assert!(session.is_active);

    // Update draft
    session.update_draft("Modified Text");
    assert_eq!(session.draft_text(), "Modified Text");
    assert!(session.is_dirty());

    // Cancel draft
    session.cancel();
    assert_eq!(session.draft_text(), "Original Text");
    assert!(!session.is_dirty());
    assert!(!session.is_active);

    // Reactivate and commit
    let mut session2 = TextEditingSession::new(
        "p0_s0_i0_o0",
        0,
        "Hello World",
        Some(FontInfo::new("Helvetica", 12.0)),
        None,
    );
    session2.update_draft("Hello StarPDF");
    let (target, committed_text) = session2.commit();
    assert_eq!(target, "p0_s0_i0_o0");
    assert_eq!(committed_text, "Hello StarPDF");
    assert!(!session2.is_active);
}

#[test]
fn test_text_replacement_command_and_undo_redo() {
    let mut manager = SessionManager::new();
    let initial_pdf = MinimalWriter::create_minimal_pdf("Alpha Beta Gamma").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: initial_pdf,
            title: "text_test.pdf".to_string(),
        })
        .expect("Open document");

    // Extract initial text to find the target span ID
    let initial_spans = {
        let active = manager.active_session().unwrap();
        let mut doc = active.pdf_document().unwrap();
        doc.extract_all_text().unwrap()
    };
    assert!(!initial_spans.is_empty());
    assert!(!initial_spans[0].spans.is_empty());

    let target_span_id = initial_spans[0].spans[0].span_id.clone();
    assert_eq!(initial_spans[0].spans[0].text, "Alpha Beta Gamma");

    // 1. Execute ReplaceText
    let replace_res = manager
        .dispatch(AppCommand::ReplaceText {
            target_node: target_span_id.clone(),
            new_text: "Omega Epsilon".to_string(),
        })
        .expect("ReplaceText command");
    assert!(replace_res.success);

    // Verify text mutated in document
    {
        let active = manager.active_session().unwrap();
        assert!(active.is_dirty());
        assert!(active.history.can_undo());
        let mut doc = active.pdf_document().unwrap();
        let updated_text = doc.extract_all_text().unwrap();
        assert_eq!(updated_text[0].spans[0].text, "Omega Epsilon");
    }

    // 2. Undo
    let undo_res = manager.dispatch(AppCommand::Undo).expect("Undo command");
    assert!(undo_res.success);

    // Verify text restored to original
    {
        let active = manager.active_session().unwrap();
        assert!(!active.is_dirty()); // Back to clean baseline!
        assert!(active.history.can_redo());
        let mut doc = active.pdf_document().unwrap();
        let restored_text = doc.extract_all_text().unwrap();
        assert_eq!(restored_text[0].spans[0].text, "Alpha Beta Gamma");
    }

    // 3. Redo
    let redo_res = manager.dispatch(AppCommand::Redo).expect("Redo command");
    assert!(redo_res.success);

    // Verify text re-mutated
    {
        let active = manager.active_session().unwrap();
        assert!(active.is_dirty());
        let mut doc = active.pdf_document().unwrap();
        let redone_text = doc.extract_all_text().unwrap();
        assert_eq!(redone_text[0].spans[0].text, "Omega Epsilon");
    }
}

#[test]
fn test_move_text_command_and_undo() {
    let mut manager = SessionManager::new();
    let initial_pdf = MinimalWriter::create_minimal_pdf("Movable Text").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: initial_pdf,
            title: "move_test.pdf".to_string(),
        })
        .expect("Open document");

    let span_id = {
        let active = manager.active_session().unwrap();
        let mut doc = active.pdf_document().unwrap();
        doc.extract_all_text().unwrap()[0].spans[0].span_id.clone()
    };

    // Move text by (15.0, -20.0)
    let move_res = manager
        .dispatch(AppCommand::MoveText {
            target_node: span_id,
            dx: 15.0,
            dy: -20.0,
        })
        .expect("MoveText command");
    assert!(move_res.success);

    assert!(manager.active_session().unwrap().is_dirty());

    // Undo move
    let undo_res = manager.dispatch(AppCommand::Undo).expect("Undo move");
    assert!(undo_res.success);
    assert!(!manager.active_session().unwrap().is_dirty());
}
