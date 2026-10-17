use starpdf::writer::MinimalWriter;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::error::AppCoreError;
use starpdf_app_core::session::SessionManager;

#[test]
fn test_page_operations_and_undo_redo() {
    let mut manager = SessionManager::new();
    let initial_pdf = MinimalWriter::create_minimal_pdf("Initial Page 1").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: initial_pdf,
            title: "page_ops.pdf".to_string(),
        })
        .expect("Open document");

    assert_eq!(manager.active_session().unwrap().total_pages, 1);

    // 1. Insert blank page after page 0
    let insert_res = manager
        .dispatch(AppCommand::InsertBlankPage {
            after_index: 0,
            width: 612.0,
            height: 792.0,
        })
        .expect("InsertBlankPage");
    assert!(insert_res.success);

    {
        let active = manager.active_session().unwrap();
        assert_eq!(active.total_pages, 2);
        assert!(active.is_dirty());
    }

    // 2. Duplicate page 0
    let dup_res = manager
        .dispatch(AppCommand::DuplicatePage { page_index: 0 })
        .expect("DuplicatePage");
    assert!(dup_res.success);

    {
        let active = manager.active_session().unwrap();
        assert_eq!(active.total_pages, 3);
    }

    // 3. Reorder page 0 to index 2
    let reorder_res = manager
        .dispatch(AppCommand::ReorderPage {
            from_index: 0,
            to_index: 2,
        })
        .expect("ReorderPage");
    assert!(reorder_res.success);

    {
        let active = manager.active_session().unwrap();
        assert_eq!(active.total_pages, 3);
        assert_eq!(active.current_page, 2);
    }

    // 4. Delete page at index 1
    let del_res = manager
        .dispatch(AppCommand::DeletePage { page_index: 1 })
        .expect("DeletePage");
    assert!(del_res.success);

    {
        let active = manager.active_session().unwrap();
        assert_eq!(active.total_pages, 2);
    }

    // 5. Undo page deletion
    manager.dispatch(AppCommand::Undo).expect("Undo deletion");
    assert_eq!(manager.active_session().unwrap().total_pages, 3);

    // 6. Undo reorder
    manager.dispatch(AppCommand::Undo).expect("Undo reorder");
    assert_eq!(manager.active_session().unwrap().total_pages, 3);

    // 7. Undo duplicate
    manager.dispatch(AppCommand::Undo).expect("Undo duplicate");
    assert_eq!(manager.active_session().unwrap().total_pages, 2);

    // 8. Undo insert blank page
    manager.dispatch(AppCommand::Undo).expect("Undo insert");
    assert_eq!(manager.active_session().unwrap().total_pages, 1);
    assert!(!manager.active_session().unwrap().is_dirty()); // Back to clean!

    // 9. Redo insert blank page
    manager.dispatch(AppCommand::Redo).expect("Redo insert");
    assert_eq!(manager.active_session().unwrap().total_pages, 2);
    assert!(manager.active_session().unwrap().is_dirty());
}

#[test]
fn test_refuse_delete_only_page() {
    let mut manager = SessionManager::new();
    let initial_pdf = MinimalWriter::create_minimal_pdf("Single Page Document").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: initial_pdf,
            title: "single_page.pdf".to_string(),
        })
        .expect("Open document");

    assert_eq!(manager.active_session().unwrap().total_pages, 1);

    // Attempt to delete page 0 when only 1 page exists
    let err = manager
        .dispatch(AppCommand::DeletePage { page_index: 0 })
        .unwrap_err();

    assert!(matches!(err, AppCoreError::InvalidOperation(_)));
    assert_eq!(manager.active_session().unwrap().total_pages, 1);
}
