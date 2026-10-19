//! SmartPDF Desktop Application (egui Acrobat Pro Edition)
//! Powered by StarPDF App-Core and WGPU / Metal acceleration.

mod app;
mod canvas;
mod chrome;
mod icons;
mod panels;
mod theme;

use std::path::PathBuf;
use app::SmartPdfEguiApp;

const DEFAULT_SAMPLE_PDF: &[u8] = include_bytes!("../../../test-assets/text-multispan-heading.pdf");

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("SmartPDF 0.1.0-rc.1 (Acrobat Pro egui Edition)");
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("SmartPDF 0.1.0 - Native Acrobat Pro-class PDF Editor");
        println!("Usage: smartpdf-desktop-egui [OPTIONS] [PDF_FILE]");
        println!("\nOptions:");
        println!("  -h, --help       Print help information");
        println!("  -V, --version    Print version information");
        return Ok(());
    }

    let initial_path = args.iter().skip(1).find(|a| !a.starts_with('-')).map(PathBuf::from);

    let (bytes, title, path_opt) = if let Some(path) = initial_path {
        if let Ok(b) = std::fs::read(&path) {
            let t = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "document.pdf".to_string());
            (b, t, Some(path))
        } else {
            eprintln!("Warning: could not read path {:?}, loading sample PDF", path);
            (DEFAULT_SAMPLE_PDF.to_vec(), "sample_contract.pdf".to_string(), None)
        }
    } else {
        (DEFAULT_SAMPLE_PDF.to_vec(), "sample_contract.pdf".to_string(), None)
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SmartPDF")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 500.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "SmartPDF",
        native_options,
        Box::new(move |cc| {
            // Install image loaders for SVG icons and images
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let mut app = SmartPdfEguiApp::new(cc);
            app.load_document_bytes(path_opt, bytes, title);
            Ok(Box::new(app))
        }),
    )
}
