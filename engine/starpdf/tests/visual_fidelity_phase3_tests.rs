use starpdf::document::PdfDocument;
use starpdf::render::{
    BlendMode, ColorRgba, DifferentialComparator, NativeRasterizer, RenderCompiler, RenderItem,
    RenderPage, RenderShading, ShadingPattern, Transform2D,
};
use std::fs;

const OUTPUT_DIR: &str = "target/visual_validation_phase3";

fn ensure_output_dir() {
    let _ = fs::create_dir_all(OUTPUT_DIR);
    let _ = fs::create_dir_all(format!("../../{OUTPUT_DIR}"));
}

fn out_path(filename: &str) -> String {
    format!("{OUTPUT_DIR}/{filename}")
}

#[test]
fn test_axial_and_radial_shading() {
    ensure_output_dir();

    // Page with both an axial (linear) gradient and a radial gradient
    let render_page = RenderPage {
        page_index: 0,
        width: 300.0,
        height: 150.0,
        media_box: [0.0, 0.0, 300.0, 150.0],
        crop_box: [0.0, 0.0, 300.0, 150.0],
        rotate: 0,
        items: vec![
            // Left half: Axial gradient from Red to Yellow
            RenderItem::Shading(RenderShading {
                pattern: ShadingPattern::Axial {
                    coords: [10.0, 10.0, 140.0, 10.0],
                    start_color: ColorRgba::from_rgb(1.0, 0.0, 0.0),
                    end_color: ColorRgba::from_rgb(1.0, 1.0, 0.0),
                    extend: [true, true],
                },
                bbox: Some([10.0, 10.0, 140.0, 140.0]),
                transform: Transform2D::identity(),
                alpha: 1.0,
                blend_mode: BlendMode::Normal,
                star_node_id: None,
                render_node_id: None,
            }),
            // Right half: Radial gradient from Cyan to Deep Blue
            RenderItem::Shading(RenderShading {
                pattern: ShadingPattern::Radial {
                    coords: [220.0, 75.0, 0.0, 220.0, 75.0, 60.0],
                    start_color: ColorRgba::from_rgb(0.0, 1.0, 1.0),
                    end_color: ColorRgba::from_rgb(0.0, 0.0, 0.5),
                    extend: [true, true],
                },
                bbox: Some([160.0, 10.0, 290.0, 140.0]),
                transform: Transform2D::identity(),
                alpha: 1.0,
                blend_mode: BlendMode::Normal,
                star_node_id: None,
                render_node_id: None,
            }),
        ],
        unsupported_operators: Vec::new(),
    };

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render shading");
    assert_eq!(rendered.width, 300);
    assert_eq!(rendered.height, 150);

    let png_path = out_path("shading_gradients_test.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save shading png");

    // Check pixel colors across the axial gradient
    // Left edge (x=15, y=75 in screen space => y=75 in PDF space)
    let idx_left = ((75 * 300 + 15) * 4) as usize;
    let r_left = rendered.rgba_pixels[idx_left];
    let g_left = rendered.rgba_pixels[idx_left + 1];
    assert!(
        r_left > 200,
        "Left side of axial gradient should be dominant Red: {r_left}"
    );
    assert!(
        g_left < 100,
        "Left side of axial gradient should have low Green: {g_left}"
    );

    // Right side of axial gradient (x=135, y=75)
    let idx_right = ((75 * 300 + 135) * 4) as usize;
    let r_right = rendered.rgba_pixels[idx_right];
    let g_right = rendered.rgba_pixels[idx_right + 1];
    assert!(
        r_right > 200,
        "Right side of axial gradient should be Yellow (high R): {r_right}"
    );
    assert!(
        g_right > 200,
        "Right side of axial gradient should be Yellow (high G): {g_right}"
    );
}

#[test]
fn test_glyph_id_propagation_and_direct_rendering() {
    let pdf_path = "../../test-assets/text-multispan-heading.pdf";
    let bytes = fs::read(pdf_path)
        .or_else(|_| fs::read("test-assets/text-multispan-heading.pdf"))
        .expect("text-multispan-heading.pdf fixture");

    let mut doc = PdfDocument::from_bytes(&bytes).expect("open doc");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");

    let text_runs: Vec<_> = render_page
        .items
        .iter()
        .filter_map(|item| match item {
            RenderItem::TextRun(r) => Some(r),
            _ => None,
        })
        .collect();

    assert!(!text_runs.is_empty(), "Must compile text runs");
    for run in &text_runs {
        assert!(
            !run.glyphs.is_empty(),
            "Text run must contain decoded glyphs"
        );
    }

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render");
    assert!(rendered.width > 0);
    assert!(rendered.height > 0);
}
