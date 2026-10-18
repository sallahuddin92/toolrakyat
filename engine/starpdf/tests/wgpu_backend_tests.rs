#[cfg(feature = "gpu")]
mod gpu_tests {
    use std::fs;
    use starpdf::document::PdfDocument;
    use starpdf::render::{
        DifferentialComparator, NativeRasterizer, RenderCompiler, WgpuRenderer,
    };

    #[test]
    fn test_wgpu_isolated_vertical_slice_and_performance_metrics() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping GPU test (no compatible GPU adapter): {e}");
                return;
            }
        };

        println!("Initialized WGPU backend adapter: {}", renderer.adapter_name());
        assert!(!renderer.adapter_name().is_empty());

        let pdf_path = "../../test-assets/vector-primitives.pdf";
        let bytes = fs::read(pdf_path)
            .or_else(|_| fs::read("test-assets/vector-primitives.pdf"))
            .expect("vector-primitives.pdf fixture");

        let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
        let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

        // 1. Render on GPU
        let t0 = std::time::Instant::now();
        let gpu_result = renderer.render_page(&page, 1.0).expect("gpu render page");
        let total_elapsed = t0.elapsed().as_secs_f64() * 1000.0;

        assert_eq!(gpu_result.width, page.width.round() as u32);
        assert_eq!(gpu_result.height, page.height.round() as u32);
        assert_eq!(gpu_result.rgba_pixels.len(), (gpu_result.width * gpu_result.height * 4) as usize);

        println!("GPU Performance Breakdown for {}x{} page on {}:", gpu_result.width, gpu_result.height, renderer.adapter_name());
        println!("  Upload Duration:   {:.3} ms", gpu_result.metrics.upload_duration_ms);
        println!("  Render Duration:   {:.3} ms", gpu_result.metrics.render_duration_ms);
        println!("  Readback Duration: {:.3} ms", gpu_result.metrics.readback_duration_ms);
        println!("  Total Frame Time:  {:.3} ms (measured client: {:.3} ms)", gpu_result.metrics.total_frame_duration_ms, total_elapsed);
        println!("  GPU Draw Calls:    {}", gpu_result.metrics.gpu_draw_calls);

        assert!(gpu_result.metrics.total_frame_duration_ms > 0.0);
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

        println!("GPU vs CPU differential: diff={:.4}%, rmse={:.4}, psnr={:.2} dB",
            diff_metrics.diff_percentage, diff_metrics.rmse, diff_metrics.psnr
        );

        assert!(
            diff_metrics.diff_percentage < 1.0,
            "GPU render must match CPU render fidelity within 1.0% (got {:.2}%)",
            diff_metrics.diff_percentage
        );

        let _ = fs::create_dir_all("target/visual_validation_phase5");
        let _ = fs::create_dir_all("../../target/visual_validation_phase5");
        let png_path = "target/visual_validation_phase5/wgpu_vector_primitives.png";
        let _ = DifferentialComparator::save_png(
            &gpu_result.rgba_pixels,
            gpu_result.width,
            gpu_result.height,
            png_path,
        );
    }
}
