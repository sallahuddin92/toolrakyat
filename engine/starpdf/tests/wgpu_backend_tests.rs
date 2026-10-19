#[cfg(feature = "gpu")]
mod gpu_tests {
    use starpdf::document::PdfDocument;
    use starpdf::render::ir::{RenderItem, RenderPage};
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
            "  GPU glyphs rendered: {}",
            gpu_res.metrics.gpu_glyphs_rendered
        );
        println!(
            "  Fallback CPU items:  {}",
            gpu_res.metrics.fallback_cpu_items
        );
        println!(
            "  Fallback reasons:    {:?}",
            gpu_res.metrics.fallback_reasons
        );

        // In Phase 7, embedded TrueType glyph outlines are rendered directly on the GPU
        assert!(gpu_res.metrics.gpu_glyphs_rendered > 0);
        assert!(gpu_res.metrics.gpu_draw_calls > 0);

        // Verify that unsupported primitives are honestly classified as fallback
        let mut fallback_page = page.clone();
        fallback_page.items.push(RenderItem::Shading(starpdf::render::ir::RenderShading {
            pattern: starpdf::render::ir::ShadingPattern::Axial {
                coords: [0.0, 0.0, 100.0, 100.0],
                start_color: starpdf::render::ir::ColorRgba::BLACK,
                end_color: starpdf::render::ir::ColorRgba::WHITE,
                extend: [false, false],
            },
            bbox: None,
            transform: starpdf::render::ir::Transform2D::identity(),
            alpha: 1.0,
            blend_mode: starpdf::render::ir::BlendMode::Normal,
            star_node_id: None,
            render_node_id: None,
        }));
        let fb_res = renderer.render_page(&fallback_page, 1.0).expect("gpu render with fallback");
        assert!(fb_res.metrics.fallback_cpu_items > 0);
        assert!(fb_res.metrics.fallback_reasons.contains(&"Shading".to_string()));

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
        assert!(diff.diff_percentage < 5.0);

        let _ = fs::create_dir_all("target/visual_validation_phase7");
        let _ = DifferentialComparator::save_png(
            &gpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            "target/visual_validation_phase7/wgpu_hybrid_text.png",
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

    #[test]
    fn test_wgpu_paint_order_and_z_index_layering() {
        let renderer = match WgpuRenderer::new() {
            Ok(r) => r,
            Err(_) => return,
        };

        use starpdf::render::ir::{BlendMode, ColorRgba, FillRule, FillStyle, PathVerb, RenderGlyph, RenderPath, RenderTextRun};

        // Construct synthetic page:
        // Item 0: Blue background rect [0, 0, 200, 200]
        // Item 1: Text run in yellow at (20, 50)
        // Item 2: Red foreground rect [0, 0, 100, 100] covering the left half
        let mut page = RenderPage {
            page_index: 0,
            width: 200.0,
            height: 200.0,
            media_box: [0.0, 0.0, 200.0, 200.0],
            crop_box: [0.0, 0.0, 200.0, 200.0],
            rotate: 0,
            items: Vec::new(),
            unsupported_operators: Vec::new(),
        };

        let blue_fill = RenderPath {
            verbs: vec![
                PathVerb::MoveTo(0.0, 0.0),
                PathVerb::LineTo(200.0, 0.0),
                PathVerb::LineTo(200.0, 200.0),
                PathVerb::LineTo(0.0, 200.0),
                PathVerb::Close,
            ],
            fill: Some(FillStyle {
                color: ColorRgba::new(0.0, 0.0, 1.0, 1.0),
                rule: FillRule::NonZero,
            }),
            stroke: None,
            transform: starpdf::render::ir::Transform2D::identity(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend_mode: BlendMode::Normal,
            star_node_id: None,
            render_node_id: None,
        };

        let yellow_text = RenderTextRun {
            font_name: "Helvetica".to_string(),
            font_size: 32.0,
            glyphs: vec![
                RenderGlyph {
                    codepoint: 'H' as u32,
                    glyph_id: None,
                    x: 20.0,
                    y: 60.0,
                    advance: 20.0,
                    text: "H".to_string(),
                },
                RenderGlyph {
                    codepoint: 'i' as u32,
                    glyph_id: None,
                    x: 40.0,
                    y: 60.0,
                    advance: 15.0,
                    text: "i".to_string(),
                },
            ],
            fill_color: Some(ColorRgba::new(1.0, 1.0, 0.0, 1.0)),
            stroke_color: None,
            transform: starpdf::render::ir::Transform2D::identity(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            render_mode: 0,
            font_data: None, // Will use system fallback
            is_vertical: false,
            star_node_id: None,
            render_node_id: None,
        };

        let red_overlay = RenderPath {
            verbs: vec![
                PathVerb::MoveTo(0.0, 0.0),
                PathVerb::LineTo(100.0, 0.0),
                PathVerb::LineTo(100.0, 100.0),
                PathVerb::LineTo(0.0, 100.0),
                PathVerb::Close,
            ],
            fill: Some(FillStyle {
                color: ColorRgba::new(1.0, 0.0, 0.0, 1.0),
                rule: FillRule::NonZero,
            }),
            stroke: None,
            transform: starpdf::render::ir::Transform2D::identity(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend_mode: BlendMode::Normal,
            star_node_id: None,
            render_node_id: None,
        };

        page.items = vec![
            RenderItem::Path(blue_fill),
            RenderItem::TextRun(yellow_text),
            RenderItem::Path(red_overlay),
        ];

        let gpu_res = renderer.render_page(&page, 1.0).expect("render layered page");
        let cpu_res = NativeRasterizer::render(&page, 1.0).expect("cpu render layered page");

        let diff = DifferentialComparator::compare(
            &gpu_res.rgba_pixels,
            &cpu_res.rgba_pixels,
            gpu_res.width,
            gpu_res.height,
            5,
        )
        .expect("diff compare");

        println!(
            "Layering diff: diff={:.4}%, psnr={:.2} dB",
            diff.diff_percentage, diff.psnr
        );

        // Check a pixel well inside the red overlay [50, 150] (screen coords, PDF Y in [0, 100]):
        // Must be purely RED (i.e. not yellow text showing through from a broken global overlay!)
        let p_idx = ((150 * gpu_res.width + 50) * 4) as usize;
        let r = gpu_res.rgba_pixels[p_idx];
        let g = gpu_res.rgba_pixels[p_idx + 1];
        let b = gpu_res.rgba_pixels[p_idx + 2];
        assert_eq!((r, g, b), (255, 0, 0), "Pixel at screen (50, 150) must be pure red covering text");

        // Check a pixel in the blue background area [50, 50] (screen coords, PDF Y in [100, 200]):
        let b_idx = ((50 * gpu_res.width + 50) * 4) as usize;
        let br = gpu_res.rgba_pixels[b_idx];
        let bg = gpu_res.rgba_pixels[b_idx + 1];
        let bb = gpu_res.rgba_pixels[b_idx + 2];
        assert_eq!((br, bg, bb), (0, 0, 255), "Pixel at screen (50, 50) must be pure blue");

        assert!(diff.diff_percentage < 3.0);
    }
}
