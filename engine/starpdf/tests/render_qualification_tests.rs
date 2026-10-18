use std::fs;
use starpdf::document::PdfDocument;
use starpdf::render::{DifferentialComparator, NativeRasterizer, RenderCompiler, RenderItem};

const OUTPUT_DIR: &str = "../../target/render_qualification";

fn ensure_output_dir() {
    let _ = fs::create_dir_all(OUTPUT_DIR);
    let _ = fs::create_dir_all("target/render_qualification");
}

fn out_path(filename: &str) -> String {
    format!("target/render_qualification/{filename}")
}

#[test]
fn test_render_vector_primitives_and_clipping() {
    ensure_output_dir();
    let pdf_path = "../../test-assets/vector-primitives.pdf";
    let bytes = fs::read(pdf_path)
        .or_else(|_| fs::read("test-assets/vector-primitives.pdf"))
        .expect("vector-primitives.pdf fixture");

    let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
    let page_count = doc.page_count().expect("page count");
    assert!(page_count >= 1);

    // 1. Compile Render IR
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");
    assert!(render_page.width > 0.0);
    assert!(render_page.height > 0.0);

    // Verify Render IR items
    let has_paths = render_page
        .items
        .iter()
        .any(|item| matches!(item, RenderItem::Path(_)));
    assert!(has_paths, "Render IR must contain vector paths");

    // 2. Render at scale 1.0
    let rendered_1x = NativeRasterizer::render(&render_page, 1.0).expect("render 1.0x");
    assert_eq!(rendered_1x.width, render_page.width.round() as u32);
    assert_eq!(rendered_1x.height, render_page.height.round() as u32);
    assert_eq!(
        rendered_1x.rgba_pixels.len(),
        (rendered_1x.width * rendered_1x.height * 4) as usize
    );

    // Verify non-blank (not all white)
    let non_white_count = rendered_1x
        .rgba_pixels
        .chunks_exact(4)
        .filter(|px| px[0] != 255 || px[1] != 255 || px[2] != 255)
        .count();
    assert!(
        non_white_count > 50,
        "Rendered vector page must have visible non-white pixels (found {non_white_count})"
    );

    // Save PNG
    let png_1x_path = out_path("vector_primitives_1x.png");
    DifferentialComparator::save_png(
        &rendered_1x.rgba_pixels,
        rendered_1x.width,
        rendered_1x.height,
        &png_1x_path,
    )
    .expect("save png 1x");

    // 3. Render at zoom scale 2.0
    let rendered_2x = NativeRasterizer::render(&render_page, 2.0).expect("render 2.0x");
    assert_eq!(rendered_2x.width, (render_page.width * 2.0).round() as u32);
    assert_eq!(
        rendered_2x.height,
        (render_page.height * 2.0).round() as u32
    );

    let png_2x_path = out_path("vector_primitives_2x.png");
    DifferentialComparator::save_png(
        &rendered_2x.rgba_pixels,
        rendered_2x.width,
        rendered_2x.height,
        &png_2x_path,
    )
    .expect("save png 2x");

    println!(
        "Vector qualification passed: 1x ({}x{}) in {:.2}ms, 2x ({}x{}) in {:.2}ms",
        rendered_1x.width,
        rendered_1x.height,
        rendered_1x.duration_ms,
        rendered_2x.width,
        rendered_2x.height,
        rendered_2x.duration_ms
    );
}

#[test]
fn test_render_text_and_fonts() {
    ensure_output_dir();
    let pdf_path = "../../test-assets/text-multispan-heading.pdf";
    let bytes = fs::read(pdf_path)
        .or_else(|_| fs::read("test-assets/text-multispan-heading.pdf"))
        .expect("text-multispan-heading.pdf fixture");

    let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");

    let has_text = render_page
        .items
        .iter()
        .any(|item| matches!(item, RenderItem::TextRun(_)));
    assert!(has_text, "Render IR must contain text runs");

    let rendered = NativeRasterizer::render(&render_page, 1.5).expect("render 1.5x");
    let non_white_count = rendered
        .rgba_pixels
        .chunks_exact(4)
        .filter(|px| px[0] != 255 || px[1] != 255 || px[2] != 255)
        .count();
    assert!(
        non_white_count > 100,
        "Rendered text page must have visible glyph marks"
    );

    let png_path = out_path("text_heading_1_5x.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save text png");

    println!(
        "Text qualification passed: {}x{} in {:.2}ms, visible non-white pixels: {}",
        rendered.width, rendered.height, rendered.duration_ms, non_white_count
    );
}

#[test]
fn test_render_images() {
    ensure_output_dir();
    let pdf_path = "../../test-assets/image-shared-xobject.pdf";
    let bytes = fs::read(pdf_path)
        .or_else(|_| fs::read("test-assets/image-shared-xobject.pdf"))
        .expect("image-shared-xobject.pdf fixture");

    let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");

    let has_image = render_page
        .items
        .iter()
        .any(|item| matches!(item, RenderItem::Image(_)));
    assert!(has_image, "Render IR must contain Image items");

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render 1.0x");
    let png_path = out_path("image_shared_1x.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save image png");

    println!(
        "Image qualification passed: {}x{} in {:.2}ms",
        rendered.width, rendered.height, rendered.duration_ms
    );
}

#[test]
fn test_render_multi_orientation_landscape() {
    ensure_output_dir();
    let pdf_path = "../../engine/starpdf/tests/fixtures/v0_9_compat/chrome-landscape.pdf";
    let bytes = fs::read(pdf_path)
        .or_else(|_| fs::read("tests/fixtures/v0_9_compat/chrome-landscape.pdf"))
        .expect("chrome-landscape.pdf fixture");

    let mut doc = PdfDocument::from_bytes(&bytes).expect("open document");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");
    assert!(
        render_page.width > render_page.height,
        "Landscape page must have width > height"
    );

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render landscape");
    assert!(rendered.width > rendered.height);

    let png_path = out_path("landscape_1x.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save landscape png");

    println!(
        "Landscape orientation qualification passed: {}x{}",
        rendered.width, rendered.height
    );
}

#[test]
fn test_differential_comparator_metrics() {
    ensure_output_dir();
    let width = 100u32;
    let height = 100u32;

    // Buffer A: Solid White
    let buf_a = vec![255u8; (width * height * 4) as usize];

    // Buffer B: Identical Copy
    let buf_b = buf_a.clone();

    // 1. Identical comparison
    let metrics_identical =
        DifferentialComparator::compare(&buf_a, &buf_b, width, height, 0).expect("compare");
    assert_eq!(metrics_identical.differing_pixels, 0);
    assert_eq!(metrics_identical.diff_percentage, 0.0);
    assert_eq!(metrics_identical.rmse, 0.0);
    assert_eq!(metrics_identical.psnr, 100.0);
    assert_eq!(metrics_identical.max_channel_delta, 0);

    // 2. Perturbed comparison (add a black square in center)
    let mut buf_c = buf_a.clone();
    for y in 40..60 {
        for x in 40..60 {
            let idx = ((y * width + x) * 4) as usize;
            buf_c[idx] = 0; // R
            buf_c[idx + 1] = 0; // G
            buf_c[idx + 2] = 0; // B
        }
    }

    let metrics_diff =
        DifferentialComparator::compare(&buf_a, &buf_c, width, height, 2).expect("compare diff");
    assert_eq!(metrics_diff.differing_pixels, 20 * 20);
    assert_eq!(metrics_diff.diff_percentage, 4.0);
    assert!(metrics_diff.rmse > 0.0);
    assert!(metrics_diff.psnr < 100.0);
    assert_eq!(metrics_diff.max_channel_delta, 255);

    // Generate diff highlight image
    let diff_img = DifferentialComparator::generate_diff_image(&buf_a, &buf_c, width, height, 2)
        .expect("diff image");
    let diff_path = out_path("test_differential_heatmap.png");
    DifferentialComparator::save_png(&diff_img, width, height, &diff_path).expect("save diff png");

    println!(
        "Differential comparator passed: identical PSNR={:.1}dB, perturbed RMSE={:.2}, diff={:.1}%",
        metrics_identical.psnr, metrics_diff.rmse, metrics_diff.diff_percentage
    );
}

#[test]
fn test_unsupported_operators_policy_fails_closed() {
    let dummy_pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 25 >>\nstream\n/Shading1 sh\n10 10 m\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000216 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n290\n%%EOF\n";

    let mut doc = PdfDocument::from_bytes(dummy_pdf).expect("open dummy pdf");
    let page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");

    assert!(page.has_unsupported_operators());
    let op_names: Vec<&str> = page
        .unsupported_operators
        .iter()
        .map(|o| o.operator.as_str())
        .collect();
    assert!(
        op_names.contains(&"sh"),
        "Shading operator 'sh' must be reported as unsupported"
    );

    let rendered = NativeRasterizer::render(&page, 1.0).expect("render");
    assert!(rendered.has_unsupported_operators());
    assert!(rendered.unsupported_operators.contains(&"sh".to_string()));
}
