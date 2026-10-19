//! Subprocess CLI probe for rendering a single PDF page with StarPDF.
use serde_json::json;
use starpdf::document::PdfDocument;
use starpdf::render::{DifferentialComparator, NativeRasterizer, RenderCompiler};
use std::env;
use std::fs;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <input_pdf> [page_index] [scale] [output_png] [backend: cpu|gpu]",
            args[0]
        );
        std::process::exit(1);
    }

    let input_path = &args[1];
    let page_index: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let scale: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let output_png = args.get(4);
    let backend = args.get(5).map(|s| s.as_str()).unwrap_or("cpu");

    let bytes = match fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            let out = json!({
                "status": "ERROR",
                "stage": "READ",
                "error": e.to_string(),
            });
            println!("{}", out);
            return;
        }
    };

    let mut doc = match PdfDocument::from_bytes(&bytes) {
        Ok(d) => d,
        Err(e) => {
            let out = json!({
                "status": "ERROR",
                "stage": "OPEN",
                "error": format!("{:?}", e),
            });
            println!("{}", out);
            return;
        }
    };

    let page_count = match doc.page_count() {
        Ok(c) => c,
        Err(e) => {
            let out = json!({
                "status": "ERROR",
                "stage": "PAGE_COUNT",
                "error": format!("{:?}", e),
            });
            println!("{}", out);
            return;
        }
    };

    if page_index >= page_count as usize {
        let out = json!({
            "status": "ERROR",
            "stage": "PAGE_BOUNDS",
            "error": format!("Page index {} exceeds page count {}", page_index, page_count),
            "page_count": page_count,
        });
        println!("{}", out);
        return;
    }

    let t_compile = Instant::now();
    let render_page = match RenderCompiler::compile_page(&mut doc, page_index) {
        Ok(p) => p,
        Err(e) => {
            let out = json!({
                "status": "ERROR",
                "stage": "COMPILE",
                "error": format!("{:?}", e),
                "page_count": page_count,
            });
            println!("{}", out);
            return;
        }
    };
    let compile_duration_ms = t_compile.elapsed().as_secs_f64() * 1000.0;

    #[cfg(feature = "gpu")]
    if backend == "gpu" {
        match starpdf::render::WgpuRenderer::new() {
            Ok(renderer) => match renderer.render_page(&render_page, scale) {
                Ok(gpu_res) => {
                    if let Some(png_path) = output_png {
                        if let Err(e) = DifferentialComparator::save_png(
                            &gpu_res.rgba_pixels,
                            gpu_res.width,
                            gpu_res.height,
                            png_path,
                        ) {
                            let out = json!({
                                "status": "ERROR",
                                "stage": "SAVE_PNG",
                                "error": e,
                            });
                            println!("{}", out);
                            return;
                        }
                    }

                    let out = json!({
                        "status": "OK",
                        "backend": "gpu",
                        "adapter": renderer.adapter_name(),
                        "page_count": page_count,
                        "width": gpu_res.width,
                        "height": gpu_res.height,
                        "page_width": render_page.width,
                        "page_height": render_page.height,
                        "rotate": render_page.rotate,
                        "items_count": render_page.items.len(),
                        "compile_duration_ms": compile_duration_ms,
                        "render_duration_ms": gpu_res.metrics.render_duration_ms,
                        "upload_duration_ms": gpu_res.metrics.upload_duration_ms,
                        "readback_duration_ms": gpu_res.metrics.readback_duration_ms,
                        "total_frame_duration_ms": gpu_res.metrics.total_frame_duration_ms,
                        "gpu_draw_calls": gpu_res.metrics.gpu_draw_calls,
                        "gpu_paths_rendered": gpu_res.metrics.gpu_paths_rendered,
                        "gpu_images_rendered": gpu_res.metrics.gpu_images_rendered,
                        "fallback_cpu_items": gpu_res.metrics.fallback_cpu_items,
                        "fallback_reasons": gpu_res.metrics.fallback_reasons,
                    });
                    println!("{}", out);
                    return;
                }
                Err(e) => {
                    let out = json!({
                        "status": "ERROR",
                        "stage": "GPU_RENDER",
                        "error": e,
                        "page_count": page_count,
                    });
                    println!("{}", out);
                    return;
                }
            },
            Err(e) => {
                let out = json!({
                    "status": "ERROR",
                    "stage": "GPU_INIT",
                    "error": e,
                    "page_count": page_count,
                });
                println!("{}", out);
                return;
            }
        }
    }

    let rendered = match NativeRasterizer::render(&render_page, scale) {
        Some(r) => r,
        None => {
            let out = json!({
                "status": "ERROR",
                "stage": "RASTERIZE",
                "error": "Rasterizer returned None (invalid page dimensions or allocation failure)",
                "page_count": page_count,
            });
            println!("{}", out);
            return;
        }
    };

    if let Some(png_path) = output_png {
        if let Err(e) = DifferentialComparator::save_png(
            &rendered.rgba_pixels,
            rendered.width,
            rendered.height,
            png_path,
        ) {
            let out = json!({
                "status": "ERROR",
                "stage": "SAVE_PNG",
                "error": e,
            });
            println!("{}", out);
            return;
        }
    }

    let out = json!({
        "status": "OK",
        "backend": "cpu",
        "page_count": page_count,
        "width": rendered.width,
        "height": rendered.height,
        "page_width": render_page.width,
        "page_height": render_page.height,
        "rotate": render_page.rotate,
        "items_count": render_page.items.len(),
        "unsupported_operators": rendered.unsupported_operators,
        "compile_duration_ms": compile_duration_ms,
        "render_duration_ms": rendered.duration_ms,
    });
    println!("{}", out);
}
