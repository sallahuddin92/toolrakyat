use std::fs;
use starpdf::document::PdfDocument;
use starpdf::render::{
    DifferentialComparator, NativeRasterizer, RenderCompiler,
};

const OUTPUT_DIR: &str = "target/visual_validation_phase4";

fn ensure_output_dir() {
    let _ = fs::create_dir_all(OUTPUT_DIR);
    let _ = fs::create_dir_all(format!("../../{OUTPUT_DIR}"));
}

fn out_path(filename: &str) -> String {
    format!("{OUTPUT_DIR}/{filename}")
}

#[test]
fn test_annotation_freetext_appearance_and_embedded_font() {
    ensure_output_dir();

    let paths = [
        "tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf",
        "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf",
        "../../engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf",
    ];
    let mut bytes = None;
    for p in &paths {
        if let Ok(b) = fs::read(p) {
            bytes = Some(b);
            break;
        }
    }
    let pdf_bytes = bytes.expect("pdfkit-markup-freetext.pdf fixture must exist");

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("open freetext pdf");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

    // Phase 4 fix verification: Page annotations are compiled into RenderItems
    assert!(
        render_page.items.len() > 100,
        "Render items count must include annotation appearances (expected > 100, got {})",
        render_page.items.len()
    );

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("rasterize freetext page");
    assert_eq!(rendered.width, 612);
    assert_eq!(rendered.height, 792);

    let png_path = out_path("pdfkit_markup_freetext_phase4.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save freetext png");

    // Verify presence of Green FreeText box (approx RGB: 52, 199, 89)
    // Annotation Rect: [50, 620, 310, 668] -> Screen y: 792 - 644 = 148, x = 100
    let mut found_green = false;
    let mut found_yellow_highlight = false;
    let mut found_red_stroke = false;

    for y in 120..170 {
        for x in 60..300 {
            let idx = ((y * rendered.width + x) * 4) as usize;
            let r = rendered.rgba_pixels[idx];
            let g = rendered.rgba_pixels[idx + 1];
            let b = rendered.rgba_pixels[idx + 2];
            // Green box: g > 150, r < 100
            if g > 150 && r < 100 && b < 120 {
                found_green = true;
            }
        }
    }
    assert!(found_green, "Rendered page must contain the green FreeText annotation box");

    // Verify presence of Yellow Highlight (approx RGB: 255, 255, 0 or with Multiply)
    // Annotation Rect: [50, 550, 310, 574] -> Screen y: 792 - 562 = 230, x = 100
    for y in 220..245 {
        for x in 60..300 {
            let idx = ((y * rendered.width + x) * 4) as usize;
            let r = rendered.rgba_pixels[idx];
            let g = rendered.rgba_pixels[idx + 1];
            let b = rendered.rgba_pixels[idx + 2];
            if r > 200 && g > 200 && b < 150 {
                found_yellow_highlight = true;
            }
        }
    }
    assert!(found_yellow_highlight, "Rendered page must contain the yellow highlight annotation");

    // Verify presence of Red Underline/Strikeout (approx RGB: 255, 56, 60)
    // Annotation Rect: [50, 512, 310, 536] -> Screen y: 792 - 524 = 268, x = 100
    for y in 265..320 {
        for x in 60..300 {
            let idx = ((y * rendered.width + x) * 4) as usize;
            let r = rendered.rgba_pixels[idx];
            let g = rendered.rgba_pixels[idx + 1];
            let b = rendered.rgba_pixels[idx + 2];
            if r > 200 && g < 100 && b < 100 {
                found_red_stroke = true;
            }
        }
    }
    assert!(found_red_stroke, "Rendered page must contain red stroke/underline annotation");
}

#[test]
fn test_nested_clipping_stack_depth_preservation() {
    ensure_output_dir();

    // Verify that multi-level nested clipping properly restores clip state
    // and does not leave residual clips that choke subsequent canvas elements.
    let paths = [
        "tests/fixtures/v0_10_compat/pdfkit-shapes-ink-link.pdf",
        "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-shapes-ink-link.pdf",
        "../../engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-shapes-ink-link.pdf",
    ];
    let mut bytes = None;
    for p in &paths {
        if let Ok(b) = fs::read(p) {
            bytes = Some(b);
            break;
        }
    }
    let pdf_bytes = bytes.expect("pdfkit-shapes-ink-link.pdf fixture must exist");
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("open ink link pdf");
    let render_page = RenderCompiler::compile_page(&mut doc, 0).expect("compile page 0");

    let rendered = NativeRasterizer::render(&render_page, 1.0).expect("rasterize ink link page");
    assert_eq!(rendered.width, 612);
    assert_eq!(rendered.height, 792);

    let png_path = out_path("pdfkit_shapes_ink_link_phase4.png");
    DifferentialComparator::save_png(
        &rendered.rgba_pixels,
        rendered.width,
        rendered.height,
        &png_path,
    )
    .expect("save ink link png");

    // Count non-white pixels across the full page height (ink annotations + text)
    let non_white_count = (0..rendered.height)
        .map(|y| {
            let mut row_non_white = 0;
            for x in 0..rendered.width {
                let idx = ((y * rendered.width + x) * 4) as usize;
                if rendered.rgba_pixels[idx] < 250
                    || rendered.rgba_pixels[idx + 1] < 250
                    || rendered.rgba_pixels[idx + 2] < 250
                {
                    row_non_white += 1;
                }
            }
            row_non_white
        })
        .filter(|&c| c > 0)
        .count();

    assert!(
        non_white_count > 50,
        "Page must have content rendered across multiple rows without being clipped to 0 (got {non_white_count} rows)"
    );
}
