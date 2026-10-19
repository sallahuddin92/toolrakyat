//! SmartPDF Desktop Application (egui Acrobat Pro Edition)
//! Powered by StarPDF App-Core and WGPU / Metal acceleration.

use std::path::PathBuf;
use smartpdf_desktop_egui::app::SmartPdfEguiApp;
use smartpdf_desktop_egui::panels;
use smartpdf_desktop_egui::theme;

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

    let mut screenshot_out: Option<PathBuf> = None;
    let mut initial_mode = panels::AcrobatMode::AllTools;
    let mut initial_panel = panels::LeftPanelMode::AllTools;
    let mut initial_theme = theme::ThemeKind::Light;
    let mut window_width = 1440.0;
    let mut window_height = 900.0;
    let mut file_args: Vec<PathBuf> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" if i + 1 < args.len() => {
                i += 1;
                screenshot_out = Some(PathBuf::from(&args[i]));
            }
            "--mode" if i + 1 < args.len() => {
                i += 1;
                match args[i].as_str() {
                    "edit" => {
                        initial_mode = panels::AcrobatMode::Edit;
                        initial_panel = panels::LeftPanelMode::Tool("edit");
                    }
                    "thumbnails" => {
                        initial_mode = panels::AcrobatMode::AllTools;
                        initial_panel = panels::LeftPanelMode::Thumbnails;
                    }
                    "organize" => {
                        initial_mode = panels::AcrobatMode::Organize;
                        initial_panel = panels::LeftPanelMode::Thumbnails;
                    }
                    _ => {
                        initial_mode = panels::AcrobatMode::AllTools;
                        initial_panel = panels::LeftPanelMode::AllTools;
                    }
                }
            }
            "--theme" if i + 1 < args.len() => {
                i += 1;
                if args[i] == "dark" {
                    initial_theme = theme::ThemeKind::Dark;
                } else {
                    initial_theme = theme::ThemeKind::Light;
                }
            }
            "--width" if i + 1 < args.len() => {
                i += 1;
                if let Ok(w) = args[i].parse::<f32>() {
                    window_width = w;
                }
            }
            "--height" if i + 1 < args.len() => {
                i += 1;
                if let Ok(h) = args[i].parse::<f32>() {
                    window_height = h;
                }
            }
            arg if !arg.starts_with('-') => {
                file_args.push(PathBuf::from(arg));
            }
            _ => {}
        }
        i += 1;
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SmartPDF")
            .with_inner_size([window_width, window_height])
            .with_min_inner_size([600.0, 400.0])
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
            app.active_mode = initial_mode;
            app.left_panel = initial_panel;
            app.theme_kind = initial_theme;
            app.screenshot_target = screenshot_out;

            if file_args.is_empty() {
                app.load_document_bytes(None, DEFAULT_SAMPLE_PDF.to_vec(), "sample_contract.pdf".to_string());
            } else {
                for path in file_args {
                    if let Err(e) = app.open_file_path(&path) {
                        eprintln!("[ERROR] Failed to open provided file {:?}: {}", path, e);
                    }
                }
            }

            Ok(Box::new(app))
        }),
    )
}
