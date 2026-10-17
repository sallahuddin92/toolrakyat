use starpdf::writer::MinimalWriter;
use starpdf_app_core::command::AppCommand;
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::session::SessionManager;

#[test]
fn test_export_profiles() {
    let mut manager = SessionManager::new();
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Exportable Content 12345").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: pdf_bytes,
            title: "export_test.pdf".to_string(),
        })
        .expect("Open document");

    // 1. Export Standard PDF
    let std_res = manager
        .dispatch(AppCommand::ExportDocument {
            profile: ExportProfile::StandardPdf,
        })
        .expect("Export StandardPdf");
    assert!(std_res.success);
    let std_bytes = std_res.exported_bytes.unwrap();
    assert!(std_bytes.starts_with(b"%PDF-"));

    // 2. Export Incremental PDF
    let inc_res = manager
        .dispatch(AppCommand::ExportDocument {
            profile: ExportProfile::IncrementalPdf,
        })
        .expect("Export IncrementalPdf");
    assert!(inc_res.success);
    let inc_bytes = inc_res.exported_bytes.unwrap();
    assert!(inc_bytes.starts_with(b"%PDF-"));

    // 3. Export Canonical PDF
    let can_res = manager
        .dispatch(AppCommand::ExportDocument {
            profile: ExportProfile::CanonicalPdf,
        })
        .expect("Export CanonicalPdf");
    assert!(can_res.success);
    let can_bytes = can_res.exported_bytes.unwrap();
    assert!(can_bytes.starts_with(b"%PDF-"));

    // 4. Export Text
    let txt_res = manager
        .dispatch(AppCommand::ExportDocument {
            profile: ExportProfile::Text,
        })
        .expect("Export Text");
    assert!(txt_res.success);
    let txt_bytes = txt_res.exported_bytes.unwrap();
    let txt_str = String::from_utf8(txt_bytes).unwrap();
    assert!(txt_str.contains("Exportable Content 12345"));

    // 5. Export JSON
    let json_res = manager
        .dispatch(AppCommand::ExportDocument {
            profile: ExportProfile::Json,
        })
        .expect("Export Json");
    assert!(json_res.success);
    let json_bytes = json_res.exported_bytes.unwrap();
    let json_str = String::from_utf8(json_bytes).unwrap();
    assert!(json_str.contains("\"format\":\"starpdf_document\""));
    assert!(json_str.contains("Exportable Content 12345"));
}

#[test]
fn test_dirty_state_lifecycle_and_baseline() {
    let mut manager = SessionManager::new();
    let pdf_bytes = MinimalWriter::create_minimal_pdf("Baseline Text").unwrap();

    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: pdf_bytes,
            title: "baseline.pdf".to_string(),
        })
        .expect("Open document");

    // Initially clean
    assert!(!manager.active_session().unwrap().is_dirty());
    assert!(!manager.tabs()[0].is_dirty);

    // Mutate via insert blank page
    manager
        .dispatch(AppCommand::InsertBlankPage {
            after_index: 0,
            width: 612.0,
            height: 792.0,
        })
        .expect("Insert page");

    // Now dirty
    assert!(manager.active_session().unwrap().is_dirty());
    assert!(manager.tabs()[0].is_dirty);

    // Undo -> returns to clean baseline!
    manager.dispatch(AppCommand::Undo).expect("Undo");
    assert!(!manager.active_session().unwrap().is_dirty());
    assert!(!manager.tabs()[0].is_dirty);

    // Redo -> dirty again
    manager.dispatch(AppCommand::Redo).expect("Redo");
    assert!(manager.active_session().unwrap().is_dirty());
    assert!(manager.tabs()[0].is_dirty);

    // Mark clean (e.g. user saved)
    manager.active_session_mut().unwrap().history.mark_clean();
    assert!(!manager.active_session().unwrap().is_dirty());
    assert!(!manager.tabs()[0].is_dirty);
}
