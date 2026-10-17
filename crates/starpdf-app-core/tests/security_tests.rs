use starpdf::security::CapabilityDecision;
use starpdf::writer::MinimalWriter;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::error::AppCoreError;
use starpdf_app_core::session::SessionManager;

#[test]
fn test_capability_refusal_when_editing_forbidden() {
    let mut manager = SessionManager::new();
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Restricted Document").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: pdf_bytes,
            title: "restricted.pdf".to_string(),
        })
        .expect("Open document");

    // Manually configure capabilities to simulate permission restrictions
    // (e.g. signed document or restricted permissions)
    {
        let session = manager.active_session_mut().unwrap();
        session.capabilities.can_edit = CapabilityDecision {
            allowed: false,
            reason: "MODIFICATION_PERMISSION_DENIED",
        };
        session.capabilities.can_modify_pages = CapabilityDecision {
            allowed: false,
            reason: "SIGNED_PAGE_MUTATION_UNSUPPORTED",
        };
    }

    let initial_bytes = manager.active_session().unwrap().bytes.clone();

    // 1. Attempt ReplaceText -> Must reject with Refused
    let err_replace = manager
        .dispatch(AppCommand::ReplaceText {
            target_node: "p0_s0_i0_o0".to_string(),
            new_text: "Prohibited Replacement".to_string(),
        })
        .unwrap_err();

    match &err_replace {
        AppCoreError::Refused { code, reason } => {
            assert_eq!(code, "MUTATION_REFUSED");
            assert_eq!(reason, "MODIFICATION_PERMISSION_DENIED");
        }
        other => panic!("Expected AppCoreError::Refused, got {other:?}"),
    }

    // Verify document bytes 100% unaltered
    assert_eq!(manager.active_session().unwrap().bytes, initial_bytes);
    assert!(!manager.active_session().unwrap().is_dirty());

    // 2. Attempt MoveText -> Must reject with Refused
    let err_move = manager
        .dispatch(AppCommand::MoveText {
            target_node: "p0_s0_i0_o0".to_string(),
            dx: 10.0,
            dy: 10.0,
        })
        .unwrap_err();

    assert!(matches!(err_move, AppCoreError::Refused { .. }));
    assert_eq!(manager.active_session().unwrap().bytes, initial_bytes);

    // 3. Attempt DeletePage -> Must reject with Refused
    let err_del_page = manager
        .dispatch(AppCommand::DeletePage { page_index: 0 })
        .unwrap_err();

    match &err_del_page {
        AppCoreError::Refused { code, reason } => {
            assert_eq!(code, "PAGE_MUTATION_REFUSED");
            assert_eq!(reason, "SIGNED_PAGE_MUTATION_UNSUPPORTED");
        }
        other => panic!("Expected AppCoreError::Refused, got {other:?}"),
    }
    assert_eq!(manager.active_session().unwrap().bytes, initial_bytes);

    // 4. Attempt DuplicatePage -> Must reject with Refused
    let err_dup = manager
        .dispatch(AppCommand::DuplicatePage { page_index: 0 })
        .unwrap_err();

    assert!(matches!(err_dup, AppCoreError::Refused { .. }));
    assert_eq!(manager.active_session().unwrap().bytes, initial_bytes);

    // 5. Attempt InsertBlankPage -> Must reject with Refused
    let err_insert = manager
        .dispatch(AppCommand::InsertBlankPage {
            after_index: 0,
            width: 612.0,
            height: 792.0,
        })
        .unwrap_err();

    assert!(matches!(err_insert, AppCoreError::Refused { .. }));
    assert_eq!(manager.active_session().unwrap().bytes, initial_bytes);
}
