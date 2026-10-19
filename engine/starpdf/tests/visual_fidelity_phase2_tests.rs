use starpdf::document::PdfDocument;
use starpdf::render::{
    BlendMode, ColorRgba, DifferentialComparator, FillRule, NativeRasterizer, PathVerb,
    RenderCompiler, RenderGlyph, RenderImage, RenderItem, RenderPage, RenderPath, RenderTextRun,
    Transform2D,
};
use std::fs;

const OUTPUT_DIR: &str = "target/visual_validation_phase2";

fn ensure_output_dir() {
    let _ = fs::create_dir_all(OUTPUT_DIR);
    let _ = fs::create_dir_all(format!("../../{OUTPUT_DIR}"));
}

fn out_path(filename: &str) -> String {
    format!("{OUTPUT_DIR}/{filename}")
}

#[test]
fn test_extgstate_transparency_and_blend_modes() {
    ensure_output_dir();

    // Verify RenderPath with transparency and blend modes compiles and renders
    let render_page = RenderPage {
        page_index: 0,
        width: 200.0,
        height: 200.0,
        media_box: [0.0, 0.0, 200.0, 200.0],
        crop_box: [0.0, 0.0, 200.0, 200.0],
        rotate: 0,
        items: vec![
            // Base opaque red rectangle
            RenderItem::Path(RenderPath {
                verbs: vec![
                    PathVerb::MoveTo(20.0, 20.0),
                    PathVerb::LineTo(120.0, 20.0),
                    PathVerb::LineTo(120.0, 120.0),
                    PathVerb::LineTo(20.0, 120.0),
                    PathVerb::Close,
                ],
                fill: Some(starpdf::render::FillStyle {
                    color: ColorRgba::from_rgb(1.0, 0.0, 0.0),
                    rule: FillRule::NonZero,
                }),
                stroke: None,
                transform: Transform2D::identity(),
                stroke_alpha: 1.0,
                fill_alpha: 1.0,
                blend_mode: BlendMode::Normal,
                star_node_id: None,
                render_node_id: None,
            }),
            // Overlapping 50% semi-transparent blue rectangle with Multiply blend mode
            RenderItem::Path(RenderPath {
                verbs: vec![
                    PathVerb::MoveTo(60.0, 60.0),
                    PathVerb::LineTo(160.0, 60.0),
                    PathVerb::LineTo(160.0, 160.0),
                    PathVerb::LineTo(60.0, 160.0),
                    PathVerb::Close,
                ],
                fill: Some(starpdf::render::FillStyle {
                    color: ColorRgba::from_rgb(0.0, 0.0, 1.0),
                    rule: FillRule::NonZero,
                }),
                stroke: None,
                transform: Transform2D::identity(),
                stroke_alpha: 1.0,
                fill_alpha: 0.5,
                blend_mode: BlendMode::Multiply,
                star_node_id: None,
                render_node_id: None,
            }),
        ],
        unsupported_operators: Vec::new(),
    };

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render transparency");
    assert_eq!(rendered.width, 200);
    assert_eq!(rendered.height, 200);

    let png_path = out_path("transparency_blend_test.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save png");

    // Check overlap pixel at (90, 100):
    // In Multiply blend mode: Red (1,0,0) multiplied by Blue (0,0,1) produces Black (0,0,0).
    // Modulated by 50% alpha: (1 - 0.5) * (255, 0, 0) + 0.5 * (0, 0, 0) = (127, 0, 0).
    let idx = ((100 * 200 + 90) * 4) as usize;
    let r = rendered.rgba_pixels[idx];
    let g = rendered.rgba_pixels[idx + 1];
    let b = rendered.rgba_pixels[idx + 2];
    assert!(
        r > 100 && r < 150,
        "Blended R must be attenuated by alpha multiply: got {r}"
    );
    assert_eq!(g, 0, "Multiply of Red and Blue has zero G");
    assert_eq!(b, 0, "Multiply of Red and Blue has zero B");
}

#[test]
fn test_image_mask_rendering() {
    ensure_output_dir();

    // 8x8 checkerboard mask: 8 bytes, each byte has alternating bits 0xAA / 0x55
    let mask_bytes = vec![0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55];

    let mut rgba_data = Vec::with_capacity(8 * 8 * 4);
    for row in 0..8 {
        for col in 0..8 {
            let bit = (mask_bytes[row] >> (7 - col)) & 1;
            if bit == 1 {
                rgba_data.push(0);
                rgba_data.push(180);
                rgba_data.push(0);
                rgba_data.push(255);
            } else {
                rgba_data.push(0);
                rgba_data.push(0);
                rgba_data.push(0);
                rgba_data.push(0);
            }
        }
    }

    let render_page = RenderPage {
        page_index: 0,
        width: 100.0,
        height: 100.0,
        media_box: [0.0, 0.0, 100.0, 100.0],
        crop_box: [0.0, 0.0, 100.0, 100.0],
        rotate: 0,
        items: vec![RenderItem::Image(RenderImage {
            width: 8,
            height: 8,
            rgba_data,
            transform: Transform2D::new(80.0, 0.0, 0.0, 80.0, 10.0, 10.0),
            opacity: 1.0,
            is_mask: true,
            blend_mode: BlendMode::Normal,
            star_node_id: None,
            render_node_id: None,
        })],
        unsupported_operators: Vec::new(),
    };

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render image mask");
    assert_eq!(rendered.width, 100);
    assert_eq!(rendered.height, 100);

    let png_path = out_path("image_mask_test.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save mask png");

    let green_pixels = rendered
        .rgba_pixels
        .chunks_exact(4)
        .filter(|px| px[1] > 100 && px[0] < 50)
        .count();
    assert!(green_pixels > 0, "Image mask must draw green pixels");
}

#[test]
fn test_page_rotation_90_180_270() {
    let mut render_page = RenderPage {
        page_index: 0,
        width: 400.0,
        height: 200.0,
        media_box: [0.0, 0.0, 400.0, 200.0],
        crop_box: [0.0, 0.0, 400.0, 200.0],
        rotate: 0,
        items: vec![],
        unsupported_operators: Vec::new(),
    };

    // 0 degrees: 400x200
    let r0 = NativeRasterizer::render(&render_page, 1.0).unwrap();
    assert_eq!(r0.width, 400);
    assert_eq!(r0.height, 200);

    // 90 degrees: width and height swapped -> 200x400
    render_page.rotate = 90;
    let r90 = NativeRasterizer::render(&render_page, 1.0).unwrap();
    assert_eq!(r90.width, 200);
    assert_eq!(r90.height, 400);

    // 180 degrees: 400x200
    render_page.rotate = 180;
    let r180 = NativeRasterizer::render(&render_page, 1.0).unwrap();
    assert_eq!(r180.width, 400);
    assert_eq!(r180.height, 200);

    // 270 degrees: 200x400
    render_page.rotate = 270;
    let r270 = NativeRasterizer::render(&render_page, 1.0).unwrap();
    assert_eq!(r270.width, 200);
    assert_eq!(r270.height, 400);
}

#[test]
fn test_multilingual_font_fallback_rendering() {
    ensure_output_dir();

    // Arabic, Hebrew, CJK, Devanagari text runs
    let arabic_text = "مرحبا بالعالم";
    let cjk_text = "世界你好";

    let mut arabic_glyphs = Vec::new();
    let mut x = 0.0f32;
    for ch in arabic_text.chars() {
        arabic_glyphs.push(RenderGlyph {
            codepoint: ch as u32,
            glyph_id: None,
            x,
            y: 0.0,
            advance: 14.0,
            text: ch.to_string(),
        });
        x += 14.0;
    }

    let mut cjk_glyphs = Vec::new();
    let mut cx = 0.0f32;
    for ch in cjk_text.chars() {
        cjk_glyphs.push(RenderGlyph {
            codepoint: ch as u32,
            glyph_id: None,
            x: cx,
            y: 0.0,
            advance: 18.0,
            text: ch.to_string(),
        });
        cx += 18.0;
    }

    let render_page = RenderPage {
        page_index: 0,
        width: 300.0,
        height: 150.0,
        media_box: [0.0, 0.0, 300.0, 150.0],
        crop_box: [0.0, 0.0, 300.0, 150.0],
        rotate: 0,
        items: vec![
            RenderItem::TextRun(RenderTextRun {
                font_name: "ArabicFallback".to_string(),
                font_size: 16.0,
                glyphs: arabic_glyphs,
                fill_color: Some(ColorRgba::BLACK),
                stroke_color: None,
                transform: Transform2D::translation(20.0, 100.0),
                fill_alpha: 1.0,
                stroke_alpha: 1.0,
                render_mode: 0,
                font_data: None,
                is_vertical: false,
                star_node_id: None,
                render_node_id: None,
            }),
            RenderItem::TextRun(RenderTextRun {
                font_name: "CJKFallback".to_string(),
                font_size: 18.0,
                glyphs: cjk_glyphs,
                fill_color: Some(ColorRgba::from_rgb(0.2, 0.2, 0.8)),
                stroke_color: None,
                transform: Transform2D::translation(20.0, 50.0),
                fill_alpha: 1.0,
                stroke_alpha: 1.0,
                render_mode: 0,
                font_data: None,
                is_vertical: false,
                star_node_id: None,
                render_node_id: None,
            }),
        ],
        unsupported_operators: Vec::new(),
    };

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render multilingual");
    assert_eq!(rendered.width, 300);
    assert_eq!(rendered.height, 150);

    let png_path = out_path("multilingual_test.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save multilingual png");

    let non_white_count = rendered
        .rgba_pixels
        .chunks_exact(4)
        .filter(|px| px[0] != 255 || px[1] != 255 || px[2] != 255)
        .count();
    assert!(
        non_white_count > 50,
        "Multilingual text rendering must produce non-white pixels via fallback fonts"
    );
}

#[test]
fn test_form_xobject_hierarchical_rendering() {
    ensure_output_dir();

    let pdf_path = "../../test-assets/flat-form-generated.pdf";
    let bytes = fs::read(pdf_path)
        .or_else(|_| fs::read("test-assets/flat-form-generated.pdf"))
        .expect("flat-form-generated.pdf fixture");

    let mut doc = PdfDocument::from_bytes(&bytes).expect("open doc");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page");
    assert!(render_page.width > 0.0);
    assert!(render_page.height > 0.0);

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("render flat form");
    let png_path = out_path("flat_form_1x.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save flat form png");

    assert!(rendered.width > 0);
    assert!(rendered.height > 0);
}
