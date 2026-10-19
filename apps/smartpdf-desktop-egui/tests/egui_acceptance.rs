//! StarPDF App-Core Session Integration Tests (Historical Acceptance Suite).
//!
//! AUDIT OF TEST COVERAGE AND LAYER DECOUPLING:
//! -----------------------------------------------------------------------------------------
//! - What these 4 tests EXERCISE:
//!   * Core `SessionManager` document ingestion, tab state tracking, and tab metadata vectors.
//!   * Headless CPU and GPU rasterization pipelines (`session.render_page` / `render_page_gpu`).
//!   * High-level mutation commands (`AddAnnotation`, `Undo`, `Redo`, `DuplicatePage`, `ExportDocument`).
//!   * Text extraction and parser stability on complex enterprise and private homestay documents.
//!
//! - What these 4 tests BYPASS:
//!   * The real `SmartPdfEguiApp` eframe/egui frontend state and lifecycle.
//!   * Native window event polling, drag-and-drop input handling, and keyboard shortcuts.
//!   * Top chrome `+ Open` and `CloseTab` UI interaction buttons.
//!   * Asynchronous native file dialog channels (`rfd`).
//!   * Egui texture generation (`ctx.load_texture`) and session-keyed texture cache invalidation.
//!   * Interactive canvas pan/zoom, selection handles, and inline text editing buffers.
//!
//! NOTE: For genuine end-to-end GUI acceptance testing that exercises the complete desktop UI,
//! see `tests/real_gui_e2e_acceptance.rs`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use starpdf_app_core::command::AppCommand;
use starpdf_app_core::export::ExportProfile;
use starpdf_app_core::session::SessionManager;

const SAMPLE_PDF: &[u8] = include_bytes!("../../../test-assets/text-multispan-heading.pdf");
const PRIVATE_PDF_PATH: &str = "/Users/sallahuddin/Downloads/Booking confirmation - 31.12.2021.pdf";

#[test]
fn test_01_document_loading_and_tab_lifecycle() {
    let mut manager = SessionManager::new();

    // 1. Load initial document
    let res = manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: SAMPLE_PDF.to_vec(),
            title: "sample.pdf".into(),
        })
        .expect("open document");

    assert!(res.success);
    assert_eq!(manager.session_count(), 1);
    assert_eq!(manager.tabs().len(), 1);
    let first_id = manager.tabs()[0].session_id.clone();

    // 2. Load second document
    let invoice_path = PathBuf::from("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
    if invoice_path.exists() {
        let bytes = std::fs::read(&invoice_path).unwrap();
        manager
            .dispatch(AppCommand::OpenDocument {
                path: Some(invoice_path),
                bytes,
                title: "invoice.pdf".into(),
            })
            .expect("open second document");

        assert_eq!(manager.session_count(), 2);
        assert_eq!(manager.tabs().len(), 2);

        // Switch back to first tab
        manager
            .dispatch(AppCommand::SwitchTab {
                session_id: first_id.clone(),
            })
            .unwrap();
        assert_eq!(manager.active_session().unwrap().id, first_id);

        // Close first tab
        manager
            .dispatch(AppCommand::CloseSession {
                session_id: first_id,
            })
            .unwrap();
        assert_eq!(manager.session_count(), 1);
    }
}

#[test]
fn test_02_gpu_and_cpu_rasterization_latency_benchmark() {
    let mut manager = SessionManager::new();
    let doc_path = PathBuf::from("test-assets/realworld_corpus/office-invoice-enterprise.pdf");
    let (bytes, title) = if doc_path.exists() {
        (std::fs::read(&doc_path).unwrap(), "invoice.pdf".into())
    } else {
        (SAMPLE_PDF.to_vec(), "sample.pdf".into())
    };

    manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(doc_path),
            bytes,
            title,
        })
        .unwrap();

    let session = manager.active_session().unwrap();

    // 1. CPU Rasterization Benchmark
    let t_cpu_start = Instant::now();
    let cpu_rendered = session.render_page(0, 1.0).expect("cpu render");
    let cpu_elapsed = t_cpu_start.elapsed();

    assert!(cpu_rendered.width > 0);
    assert!(cpu_rendered.height > 0);
    assert_eq!(
        cpu_rendered.rgba_pixels.len(),
        (cpu_rendered.width * cpu_rendered.height * 4) as usize
    );
    println!(
        "\n  [BENCHMARK] StarPDF CPU Rasterization Latency: {:.2} ms",
        cpu_elapsed.as_secs_f64() * 1000.0
    );

    // 2. GPU Rasterization Benchmark (if GPU available)
    if let Ok(renderer) = starpdf::render::WgpuRenderer::new() {
        let t_gpu_start = Instant::now();
        let gpu_rendered = session
            .render_page_gpu(0, 1.0, &renderer)
            .expect("gpu render");
        let gpu_elapsed = t_gpu_start.elapsed();

        assert_eq!(gpu_rendered.width, cpu_rendered.width);
        assert_eq!(gpu_rendered.height, cpu_rendered.height);
        println!(
            "  [BENCHMARK] StarPDF GPU ({}) Latency: {:.2} ms",
            renderer.adapter_name(),
            gpu_elapsed.as_secs_f64() * 1000.0
        );
    }
}

#[test]
fn test_03_interactive_mutations_and_undo_redo() {
    let mut manager = SessionManager::new();
    manager
        .dispatch(AppCommand::OpenDocument {
            path: None,
            bytes: SAMPLE_PDF.to_vec(),
            title: "test.pdf".into(),
        })
        .unwrap();

    let initial_rev = manager.active_session().unwrap().revision;

    // 1. Add Annotation
    manager
        .dispatch(AppCommand::AddAnnotation {
            page_index: 0,
            subtype: "Highlight".into(),
            rect: [50.0, 700.0, 200.0, 720.0],
            contents: Some("Acceptance Highlight".into()),
            color: Some(vec![1.0, 0.9, 0.0]),
        })
        .expect("add annotation");

    assert!(manager.active_session().unwrap().revision > initial_rev);
    assert!(manager.active_session().unwrap().is_dirty());

    // 2. Undo Mutation
    let undo_res = manager.dispatch(AppCommand::Undo).expect("undo");
    assert!(undo_res.success);

    // 3. Redo Mutation
    let redo_res = manager.dispatch(AppCommand::Redo).expect("redo");
    assert!(redo_res.success);

    // 4. Duplicate Page
    let pages_before = manager.active_session().unwrap().total_pages;
    manager
        .dispatch(AppCommand::DuplicatePage { page_index: 0 })
        .expect("duplicate page");
    assert_eq!(
        manager.active_session().unwrap().total_pages,
        pages_before + 1
    );

    // 5. Export Document
    let export_res = manager
        .dispatch(AppCommand::ExportDocument {
            profile: ExportProfile::StandardPdf,
        })
        .expect("export document");
    assert!(export_res.exported_bytes.is_some());
    assert!(!export_res.exported_bytes.unwrap().is_empty());
}

#[test]
fn test_04_private_homestay_booking_confirmation_qualification() {
    let path = Path::new(PRIVATE_PDF_PATH);
    if !path.exists() {
        println!("Private homestay booking confirmation PDF not present at {:?}; skipping private verification", path);
        return;
    }

    let bytes = std::fs::read(path).expect("read private booking confirmation");
    let mut manager = SessionManager::new();

    let open_res = manager
        .dispatch(AppCommand::OpenDocument {
            path: Some(path.to_path_buf()),
            bytes,
            title: "Booking confirmation.pdf".into(),
        })
        .expect("open private homestay document");

    assert!(open_res.success);
    let session = manager.active_session().unwrap();
    assert_eq!(session.total_pages, 1);

    // Verify text extraction
    let spans = session
        .extract_page_text_spans(0)
        .expect("extract text spans");
    assert!(!spans.is_empty(), "Page text spans must not be empty");

    // Verify render integrity
    let rendered = session.render_page(0, 1.0).expect("render private page");
    assert!(rendered.width > 0);
    assert!(rendered.height > 0);

    println!(
        "  [PRIVATE DOCUMENT QUALIFICATION]: Successfully parsed and rendered {} spans at {}x{}",
        spans.len(),
        rendered.width,
        rendered.height
    );
}
