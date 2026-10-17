use starpdf::writer::MinimalWriter;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::error::AppCoreError;
use starpdf_app_core::session::{SessionManager, ZoomMode};

#[test]
fn test_multi_document_tab_lifecycle() {
    let mut manager = SessionManager::new();
    assert_eq!(manager.session_count(), 0);
    assert_eq!(manager.active_tab_index(), None);

    let doc1_bytes = MinimalWriter::create_minimal_pdf("Document One Content").unwrap();
    let doc2_bytes = MinimalWriter::create_minimal_pdf("Document Two Content").unwrap();
    let doc3_bytes = MinimalWriter::create_minimal_pdf("Document Three Content").unwrap();

    // 1. Open document 1
    let id1 = manager
        .create_session(None, doc1_bytes, "doc1.pdf".to_string())
        .expect("Open doc1");
    assert_eq!(manager.session_count(), 1);
    assert_eq!(manager.active_tab_index(), Some(0));
    assert_eq!(manager.active_session().unwrap().id, id1);

    // 2. Open document 2
    let id2 = manager
        .create_session(None, doc2_bytes, "doc2.pdf".to_string())
        .expect("Open doc2");
    assert_eq!(manager.session_count(), 2);
    assert_eq!(manager.active_tab_index(), Some(1));
    assert_eq!(manager.active_session().unwrap().id, id2);

    // 3. Open document 3
    let id3 = manager
        .create_session(None, doc3_bytes, "doc3.pdf".to_string())
        .expect("Open doc3");
    assert_eq!(manager.session_count(), 3);
    assert_eq!(manager.active_tab_index(), Some(2));
    assert_eq!(manager.active_session().unwrap().id, id3);

    // Verify TabInfo DTOs
    let tabs = manager.tabs();
    assert_eq!(tabs.len(), 3);
    assert_eq!(tabs[0].title, "doc1.pdf");
    assert!(!tabs[0].is_active);
    assert_eq!(tabs[1].title, "doc2.pdf");
    assert!(!tabs[1].is_active);
    assert_eq!(tabs[2].title, "doc3.pdf");
    assert!(tabs[2].is_active);

    // 4. Switch tabs by ID
    manager.switch_tab(&id1).expect("Switch to tab 1");
    assert_eq!(manager.active_tab_index(), Some(0));
    assert_eq!(manager.active_session().unwrap().id, id1);
    assert!(manager.tabs()[0].is_active);
    assert!(!manager.tabs()[2].is_active);

    // 5. Close non-active tab (tab 2)
    manager.close_session(&id2).expect("Close tab 2");
    assert_eq!(manager.session_count(), 2);
    assert_eq!(manager.active_tab_index(), Some(0)); // Still tab 1

    // 6. Close active tab (tab 1) -> focus moves to remaining tab
    manager.close_session(&id1).expect("Close tab 1");
    assert_eq!(manager.session_count(), 1);
    assert_eq!(manager.active_tab_index(), Some(0));
    assert_eq!(manager.active_session().unwrap().id, id3);

    // 7. Close last tab
    manager.close_session(&id3).expect("Close tab 3");
    assert_eq!(manager.session_count(), 0);
    assert_eq!(manager.active_tab_index(), None);
    assert!(manager.active_session().is_none());
}

#[test]
fn test_tab_commands_and_navigation() {
    let mut manager = SessionManager::new();
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Page Navigation Test").unwrap();

    let res = manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: pdf_bytes,
            title: "nav.pdf".to_string(),
        })
        .expect("Open document command");

    assert!(res.success);
    let session_id = res.affected_session_id.unwrap();

    // Set page out of bounds
    let err = manager
        .dispatch(AppCommand::SetPage { page_index: 5 })
        .unwrap_err();
    assert!(matches!(err, AppCoreError::InvalidPageIndex { .. }));

    // Set zoom
    let zoom_res = manager
        .dispatch(AppCommand::SetZoom { percent: 150.0 })
        .expect("Set zoom");
    assert!(zoom_res.success);
    let session = manager.active_session().unwrap();
    assert_eq!(session.zoom.percent, 150.0);
    assert_eq!(session.zoom.mode, ZoomMode::Custom(150.0));

    // Close session via command
    let close_res = manager
        .dispatch(AppCommand::CloseSession { session_id })
        .expect("Close command");
    assert!(close_res.success);
    assert_eq!(manager.session_count(), 0);
}
