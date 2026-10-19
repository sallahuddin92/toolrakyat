#[cfg(feature = "gpu")]
mod gpu_tests {
    use starpdf::document::PdfDocument;
    use starpdf::render::{DifferentialComparator, NativeRasterizer, RenderCompiler, WgpuRenderer};
    use std::fs;

    fn load_fixture(rel_path: &str) -> Vec<u8> {
        let candidates = [
            format!("../../test-assets/{rel_path}"),
            format!("test-assets/{rel_path}"),
            format!("../../{rel_path}"),
            rel_path.to_string(),
        ];
        for c in &candidates {
            if let Ok(bytes) = fs::read(c) {
                return bytes;
            }
        }
        panic!("Fixture not found: {rel_path}");
    }

    #[test]
    fn test_wgpu_vector_primitives_gpu_rasterization() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping GPU test (no compatible GPU adapter): {e}");
                return;
            }
        };

        println!(
            "Initialized WGPU backend adapter: {}",
            renderer.adapter_name()
        );
        assert!(!renderer.adapter_name().is_empty());

        let bytes = load_fixture("vector-primitives.pdf");
        let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
        let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

        // 1. Render on GPU
        let t0 = std::time::Instant::now();
        let gpu_result = renderer.render_page(&page, 1.0).expect("gpu render page");
        let total_elapsed = t0.elapsed().as_secs_f64() * 1000.0;

        assert_eq!(gpu_result.width, page.width.round() as u32);
        assert_eq!(gpu_result.height, page.height.round() as u32);
        assert_eq!(
            gpu_result.rgba_pixels.len(),
            (gpu_result.width * gpu_result.height * 4) as usize
        );

        println!(
            "GPU Performance Breakdown for {}x{} page on {}:",
            gpu_result.width,
            gpu_result.height,
            renderer.adapter_name()
        );
        println!(
            "  Upload Duration:       {:.3} ms",
            gpu_result.metrics.upload_duration_ms
        );
        println!(
            "  Render Duration:       {:.3} ms",
            gpu_result.metrics.render_duration_ms
        );
        println!(
            "  Readback Duration:     {:.3} ms",
            gpu_result.metrics.readback_duration_ms
        );
        println!(
            "  Total Frame Time:      {:.3} ms (client: {:.3} ms)",
            gpu_result.metrics.total_frame_duration_ms, total_elapsed
        );
        println!(
            "  GPU Draw Calls:        {}",
            gpu_result.metrics.gpu_draw_calls
        );
        println!(
            "  GPU Paths Rendered:    {}",
            gpu_result.metrics.gpu_paths_rendered
        );
        println!(
            "  GPU Images Rendered:   {}",
            gpu_result.metrics.gpu_images_rendered
        );
        println!(
            "  Fallback CPU Items:    {}",
            gpu_result.metrics.fallback_cpu_items
        );

        // Assert genuine GPU rasterization
        assert!(
            gpu_result.metrics.gpu_paths_rendered > 0,
            "GPU must genuinely rasterize vector paths"
        );
        assert!(gpu_result.metrics.gpu_draw_calls > 0);

        // 2. Render on CPU with NativeRasterizer
        let cpu_result = NativeRasterizer::render(&page, 1.0).expect("cpu render page");

        // 3. Compare GPU vs CPU outputs
        let diff_metrics = DifferentialComparator::compare(
            &gpu_result.rgba_pixels,
            &cpu_result.rgba_pixels,
            gpu_result.width,
            gpu_result.height,
            5,
        )
        .expect("compare gpu vs cpu");

        println!(
            "GPU vs CPU differential: diff={:.4}%, rmse={:.4}, psnr={:.2} dB",
            diff_metrics.diff_percentage, diff_metrics.rmse, diff_metrics.psnr
        );

        assert!(
            diff_metrics.diff_percentage < 1.0,
            "GPU render must match CPU render fidelity within 1.0% (got {:.2}%)",
            diff_metrics.diff_percentage
        );

        let _ = fs::create_dir_all("target/visual_validation_phase6");
        let png_path = "target/visual_validation_phase6/wgpu_vector_primitives.png";
        let _ = DifferentialComparator::save_png(
            &gpu_result.rgba_pixels,
            gpu_result.width,
            gpu_result.height,
            png_path,
        );
    }

    #[test]
    fn test_wgpu_multiresolution_and_zoom() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(_) => return,
        };

        let bytes = load_fixture("vector-primitives.pdf");
        let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
        let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

        for scale in [0.5f32, 1.0f32, 1.5f32, 2.0f32] {
            let gpu_res = renderer.render_page(&page, scale).expect("render scale");
            let cpu_res = NativeRasterizer::render(&page, scale).expect("cpu render scale");

            assert_eq!(gpu_res.width, cpu_res.width);
            assert_eq!(gpu_res.height, cpu_res.height);

            let diff = DifferentialComparator::compare(
                &gpu_res.rgba_pixels,
                &cpu_res.rgba_pixels,
                gpu_res.width,
                gpu_res.height,
                5,
            )
            .expect("diff compare");

            println!(
                "Scale {:.1}x ({}x{}): diff={:.3}%, psnr={:.2} dB",
                scale, gpu_res.width, gpu_res.height, diff.diff_percentage, diff.psnr
            );
            assert!(diff.diff_percentage < 1.5);
        }
    }

    #[test]
    fn test_wgpu_image_rasterization() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(_) => return,
        };

        let bytes = load_fixture("image-shared-xobject.pdf");
        let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
        let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

        let gpu_res = renderer.render_page(&page, 1.0).expect("gpu render image");
        let cpu_res = NativeRasterizer::render(&page, 1.0).expect("cpu render image");

        println!(
            "Image fixture metrics: GPU images rendered: {}, draw calls: {}",
            gpu_res.metrics.gpu_images_rendered, gpu_res.metrics.gpu_draw_calls
        );
        assert!(
            gpu_res.metrics.gpu_images_rendered > 0,
            "GPU must rasterize image XObjects"
        );

        let diff = DifferentialComparator::compare(
            &gpu_res.rgba_pixels,
            &cpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            5,
        )
        .expect("diff compare");

        println!(
            "Image fixture diff: diff={:.4}%, psnr={:.2} dB",
            diff.diff_percentage, diff.psnr
        );
        assert!(diff.diff_percentage < 1.5);

        let _ = fs::create_dir_all("target/visual_validation_phase6");
        let _ = DifferentialComparator::save_png(
            &gpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            "target/visual_validation_phase6/wgpu_image_shared.png",
        );
    }

    #[test]
    fn test_wgpu_hybrid_rendering_and_honest_fallback_classification() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(_) => return,
        };

        let bytes = load_fixture("text-multispan-heading.pdf");
        let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
        let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

        let gpu_res = renderer.render_page(&page, 1.0).expect("gpu render text");
        let cpu_res = NativeRasterizer::render(&page, 1.0).expect("cpu render text");

        println!("Hybrid text fixture metrics:");
        println!("  GPU draw calls:      {}", gpu_res.metrics.gpu_draw_calls);
        println!(
            "  GPU paths rendered:  {}",
            gpu_res.metrics.gpu_paths_rendered
        );
        println!(
            "  Fallback CPU items:  {}",
            gpu_res.metrics.fallback_cpu_items
        );
        println!(
            "  Fallback reasons:    {:?}",
            gpu_res.metrics.fallback_reasons
        );

        assert!(gpu_res.metrics.fallback_cpu_items > 0);
        assert!(gpu_res
            .metrics
            .fallback_reasons
            .contains(&"TextRun".to_string()));

        let diff = DifferentialComparator::compare(
            &gpu_res.rgba_pixels,
            &cpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            5,
        )
        .expect("diff compare");

        println!(
            "Hybrid text diff: diff={:.4}%, psnr={:.2} dB",
            diff.diff_percentage, diff.psnr
        );
        assert!(diff.diff_percentage < 2.0);

        let _ = fs::create_dir_all("target/visual_validation_phase6");
        let _ = DifferentialComparator::save_png(
            &gpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            "target/visual_validation_phase6/wgpu_hybrid_text.png",
        );
    }

    #[test]
    fn test_wgpu_form_primitives() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(_) => return,
        };

        let bytes = load_fixture("smartpdf-adobe-like-form.pdf");
        let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
        let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

        let gpu_res = renderer.render_page(&page, 1.0).expect("gpu render form");
        let cpu_res = NativeRasterizer::render(&page, 1.0).expect("cpu render form");

        println!(
            "Form fixture: GPU paths: {}, Fallback CPU: {}",
            gpu_res.metrics.gpu_paths_rendered, gpu_res.metrics.fallback_cpu_items
        );

        let diff = DifferentialComparator::compare(
            &gpu_res.rgba_pixels,
            &cpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            5,
        )
        .expect("diff compare");

        println!(
            "Form diff: diff={:.4}%, psnr={:.2} dB",
            diff.diff_percentage, diff.psnr
        );
        assert!(diff.diff_percentage < 2.0);
    }
}
