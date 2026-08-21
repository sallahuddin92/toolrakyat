#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::vector::{
    AddVectorGraphicSpec, DeleteVectorGraphicSpec, UpdateVectorGraphicSpec, VectorColor,
    VectorGeometry,
};
use starpdf::PdfDocument;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    // Branch 1: Parse and mutate on arbitrary PDF inputs
    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        if let Ok(graphics) = doc.enumerate_graphics(0) {
            for g in graphics.iter().take(3) {
                // Try update graphic with fuzzed geometry/color/line width
                let x = f64::from(i16::from_le_bytes([data[0], data[1]]));
                let y = f64::from(i16::from_le_bytes([data[2], data[3]]));
                let w = f64::from(u16::from_le_bytes([data[4], data[5]]));
                let h = f64::from(u16::from_le_bytes([data[6], data[7]]));
                let lw = (data[8] as f64) / 10.0;

                let r = (data[9] as f64) / 255.0;
                let gr = (data[10] as f64) / 255.0;
                let b = (data[11] as f64) / 255.0;

                let update_spec = UpdateVectorGraphicSpec {
                    page_index: 0,
                    graphic_id: g.graphic_id.clone(),
                    new_geometry: Some(VectorGeometry::Rectangle {
                        x,
                        y,
                        width: w,
                        height: h,
                    }),
                    new_stroke_color: Some(Some(VectorColor::from_rgb(r, gr, b))),
                    new_fill_color: Some(Some(VectorColor::from_rgb(b, gr, r))),
                    new_line_width: Some(lw),
                    new_is_stroked: Some(data[12] % 2 == 0),
                    new_is_filled: Some(data[13] % 2 == 0),
                    clone_if_shared: data[14] % 2 == 0,
                };

                if let Ok(plan) = doc.update_graphic(&update_spec) {
                    if let Ok(exported) = doc.export_incremental(&plan) {
                        let _ = PdfDocument::from_bytes(&exported);
                    }
                }

                // Try delete graphic
                let del_spec = DeleteVectorGraphicSpec {
                    page_index: 0,
                    graphic_id: g.graphic_id.clone(),
                    clone_if_shared: data[15] % 2 == 0,
                };
                if let Ok(plan) = doc.delete_graphic(&del_spec) {
                    if let Ok(exported) = doc.export_incremental(&plan) {
                        let _ = PdfDocument::from_bytes(&exported);
                    }
                }
            }
        }

        // Try add rectangle
        let add_rect = AddVectorGraphicSpec {
            page_index: 0,
            geometry: VectorGeometry::Rectangle {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 100.0,
            },
            stroke_color: Some(VectorColor::from_rgb(0.0, 0.0, 0.0)),
            fill_color: Some(VectorColor::from_rgb(1.0, 0.0, 0.0)),
            line_width: 1.0,
            is_stroked: true,
            is_filled: true,
        };
        if let Ok(plan) = doc.add_graphic(&add_rect) {
            if let Ok(exported) = doc.export_incremental(&plan) {
                let _ = PdfDocument::from_bytes(&exported);
            }
        }

        // Try add line
        let add_line = AddVectorGraphicSpec {
            page_index: 0,
            geometry: VectorGeometry::Line {
                x1: 10.0,
                y1: 10.0,
                x2: 200.0,
                y2: 200.0,
            },
            stroke_color: Some(VectorColor::from_rgb(0.0, 1.0, 0.0)),
            fill_color: None,
            line_width: 2.0,
            is_stroked: true,
            is_filled: false,
        };
        if let Ok(plan) = doc.add_graphic(&add_line) {
            if let Ok(exported) = doc.export_incremental(&plan) {
                let _ = PdfDocument::from_bytes(&exported);
            }
        }
    }
});
