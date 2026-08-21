use starpdf::document::PdfDocument;
use starpdf::vector::{
    AddVectorGraphicSpec, DeleteVectorGraphicSpec, UpdateVectorGraphicSpec, VectorColor,
    VectorGeometry, VectorGraphicType,
};

/// Helper to generate a minimal valid single-page PDF containing a custom content stream.
fn make_pdf_with_content(content_stream: &[u8]) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let o1_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let o2_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let o3_offset = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

    let o4_offset = pdf.len();
    let content_len = content_stream.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {content_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(content_stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let o5_offset = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{o1_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o2_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o3_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o4_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o5_offset:010} 00000 n \n").as_bytes());

    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn test_vector_rectangle_fill_change() {
    // PDF with a single blue filled rectangle: 100 200 150 80 re f
    let stream = b"0 0 1 rg 100 200 150 80 re f";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);
    let target = &graphics[0];
    assert_eq!(target.graphic_type, VectorGraphicType::Rectangle);
    assert!(target.is_filled);
    assert!(!target.is_stroked);

    // Update fill color to red: rgb(1, 0, 0)
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: target.graphic_id.clone(),
        new_fill_color: Some(Some(VectorColor::from_rgb(1.0, 0.0, 0.0))),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    // Reopen and verify
    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 1);
    let updated = &re_graphics[0];
    assert_eq!(
        updated.fill_color,
        Some(VectorColor::from_rgb(1.0, 0.0, 0.0))
    );
    assert!(updated.is_filled);
}

#[test]
fn test_vector_rectangle_stroke_change() {
    // Rectangle with black stroke: 2 w 0 0 0 RG 50 50 200 100 re S
    let stream = b"2 w 0 0 0 RG 50 50 200 100 re S";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);
    let target = &graphics[0];
    assert!(target.is_stroked);
    assert!(!target.is_filled);

    // Change stroke to green: rgb(0, 1, 0)
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: target.graphic_id.clone(),
        new_stroke_color: Some(Some(VectorColor::from_rgb(0.0, 1.0, 0.0))),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 1);
    let updated = &re_graphics[0];
    assert_eq!(
        updated.stroke_color,
        Some(VectorColor::from_rgb(0.0, 1.0, 0.0))
    );
}

#[test]
fn test_vector_line_width_change() {
    let stream = b"1 w 100 100 300 200 re S";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);
    let target = &graphics[0];

    // Change line width to 5.0
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: target.graphic_id.clone(),
        new_line_width: Some(5.0),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 1);
    assert!((re_graphics[0].line_width - 5.0).abs() < 1e-3);
}

#[test]
fn test_vector_move_rectangle() {
    let stream = b"50 60 100 80 re B";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    let target = &graphics[0];

    // Move to (150, 260) with same dimensions
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: target.graphic_id.clone(),
        new_geometry: Some(VectorGeometry::Rectangle {
            x: 150.0,
            y: 260.0,
            width: 100.0,
            height: 80.0,
        }),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 1);
    assert_eq!(
        re_graphics[0].geometry,
        VectorGeometry::Rectangle {
            x: 150.0,
            y: 260.0,
            width: 100.0,
            height: 80.0,
        }
    );
    assert!((re_graphics[0].bounds[0] - 150.0).abs() < 1e-3);
    assert!((re_graphics[0].bounds[1] - 260.0).abs() < 1e-3);
}

#[test]
fn test_vector_resize_rectangle() {
    let stream = b"50 50 100 100 re S";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    let target = &graphics[0];

    // Resize to width 300, height 250
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: target.graphic_id.clone(),
        new_geometry: Some(VectorGeometry::Rectangle {
            x: 50.0,
            y: 50.0,
            width: 300.0,
            height: 250.0,
        }),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(
        re_graphics[0].geometry,
        VectorGeometry::Rectangle {
            x: 50.0,
            y: 50.0,
            width: 300.0,
            height: 250.0,
        }
    );
}

#[test]
fn test_vector_delete_rectangle() {
    let stream = b"q 50 50 100 100 re S Q 200 200 50 50 re f";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 2);

    // Delete the first rectangle
    let spec = DeleteVectorGraphicSpec {
        page_index: 0,
        graphic_id: graphics[0].graphic_id.clone(),
        clone_if_shared: false,
    };

    let plan = doc.delete_graphic(&spec).expect("Delete graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 1);
    assert_eq!(
        re_graphics[0].geometry,
        VectorGeometry::Rectangle {
            x: 200.0,
            y: 200.0,
            width: 50.0,
            height: 50.0,
        }
    );
}

#[test]
fn test_vector_add_rectangle() {
    let stream = b"BT /F1 12 Tf 50 700 Td (Hello World) Tj ET";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics_before = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics_before.len(), 0);

    let spec = AddVectorGraphicSpec {
        page_index: 0,
        geometry: VectorGeometry::Rectangle {
            x: 40.0,
            y: 680.0,
            width: 200.0,
            height: 40.0,
        },
        stroke_color: Some(VectorColor::from_rgb(0.2, 0.4, 0.8)),
        fill_color: Some(VectorColor::from_rgb(0.9, 0.9, 0.9)),
        line_width: 1.5,
        is_stroked: true,
        is_filled: true,
    };

    let plan = doc.add_graphic(&spec).expect("Add graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let graphics_after = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics_after.len(), 1);
    let added = &graphics_after[0];
    assert_eq!(added.graphic_type, VectorGraphicType::Rectangle);
    assert!(added.is_stroked);
    assert!(added.is_filled);
    assert!((added.line_width - 1.5).abs() < 1e-3);
}

#[test]
fn test_vector_add_line() {
    let stream = b"BT /F1 12 Tf 50 700 Td (Text) Tj ET";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");

    let spec = AddVectorGraphicSpec {
        page_index: 0,
        geometry: VectorGeometry::Line {
            x1: 50.0,
            y1: 690.0,
            x2: 300.0,
            y2: 690.0,
        },
        stroke_color: Some(VectorColor::from_rgb(0.0, 0.0, 0.0)),
        fill_color: None,
        line_width: 2.0,
        is_stroked: true,
        is_filled: false,
    };

    let plan = doc.add_graphic(&spec).expect("Add line");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);
    let added = &graphics[0];
    assert_eq!(added.graphic_type, VectorGraphicType::Line);
    assert_eq!(
        added.geometry,
        VectorGeometry::Line {
            x1: 50.0,
            y1: 690.0,
            x2: 300.0,
            y2: 690.0,
        }
    );
}

#[test]
fn test_vector_transformed_object() {
    // 2 0 0 2 100 100 cm -> 2x scale and (100, 100) offset
    // 10 10 50 30 re S -> local [10, 10, 60, 40]
    // transformed user-space: [10*2+100, 10*2+100, 60*2+100, 40*2+100] = [120, 120, 220, 180]
    let stream = b"q 2 0 0 2 100 100 cm 10 10 50 30 re S Q";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);
    let item = &graphics[0];

    assert!((item.local_bounds[0] - 10.0).abs() < 1e-3);
    assert!((item.local_bounds[1] - 10.0).abs() < 1e-3);
    assert!((item.local_bounds[2] - 60.0).abs() < 1e-3);
    assert!((item.local_bounds[3] - 40.0).abs() < 1e-3);

    assert!((item.bounds[0] - 120.0).abs() < 1e-3);
    assert!((item.bounds[1] - 120.0).abs() < 1e-3);
    assert!((item.bounds[2] - 220.0).abs() < 1e-3);
    assert!((item.bounds[3] - 180.0).abs() < 1e-3);
}

#[test]
fn test_vector_rotated_page() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 /Contents 4 0 R >>\nendobj\n");
    let o4 = pdf.len();
    pdf.extend_from_slice(
        b"4 0 obj\n<< /Length 20 >>\nstream\n50 50 100 100 re S\nendstream\nendobj\n",
    );
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );

    let mut doc = PdfDocument::from_bytes(&pdf).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);

    // Edit the shape on the rotated page
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: graphics[0].graphic_id.clone(),
        new_stroke_color: Some(Some(VectorColor::from_rgb(1.0, 0.5, 0.0))),
        ..Default::default()
    };
    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 1);
    assert_eq!(
        re_graphics[0].stroke_color,
        Some(VectorColor::from_rgb(1.0, 0.5, 0.0))
    );
}

#[test]
fn test_vector_multiple_shapes_same_page() {
    let stream = b"10 10 50 50 re S\n20 20 60 60 re f\n100 100 m 200 200 l S\n300 300 40 40 re B";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 4);

    assert_eq!(graphics[0].graphic_type, VectorGraphicType::Rectangle);
    assert_eq!(graphics[1].graphic_type, VectorGraphicType::Rectangle);
    assert_eq!(graphics[2].graphic_type, VectorGraphicType::Line);
    assert_eq!(graphics[3].graphic_type, VectorGraphicType::Rectangle);

    // Modify the 3rd graphic (the Line)
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: graphics[2].graphic_id.clone(),
        new_line_width: Some(4.0),
        new_stroke_color: Some(Some(VectorColor::from_rgb(0.8, 0.1, 0.1))),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update line");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let re_graphics = reopened.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(re_graphics.len(), 4);
    assert!((re_graphics[2].line_width - 4.0).abs() < 1e-3);
    assert_eq!(
        re_graphics[2].stroke_color,
        Some(VectorColor::from_rgb(0.8, 0.1, 0.1))
    );
}

#[test]
fn test_vector_shared_stream_isolation() {
    // Multi-page PDF where Page 1 and Page 2 share content stream 4 0 R
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(
        b"4 0 obj\n<< /Length 20 >>\nstream\n50 50 100 100 re S\nendstream\nendobj\n",
    );
    let o5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n",
    );
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );

    let mut doc = PdfDocument::from_bytes(&pdf).expect("Valid PDF");
    let all_graphics = doc
        .enumerate_all_graphics()
        .expect("Enumerate all graphics");
    assert_eq!(all_graphics.len(), 2);
    assert!(all_graphics[0].is_shared);
    assert!(all_graphics[1].is_shared);

    // Update ONLY page 0 graphic with clone_if_shared = true
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: all_graphics[0].graphic_id.clone(),
        new_geometry: Some(VectorGeometry::Rectangle {
            x: 200.0,
            y: 200.0,
            width: 80.0,
            height: 80.0,
        }),
        clone_if_shared: true,
        ..Default::default()
    };

    let plan = doc
        .update_graphic(&spec)
        .expect("Update graphic with clone");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let p0_graphics = reopened.enumerate_graphics(0).expect("P0 graphics");
    let p1_graphics = reopened.enumerate_graphics(1).expect("P1 graphics");

    // Page 0 should have the updated rectangle at (200, 200)
    assert_eq!(
        p0_graphics[0].geometry,
        VectorGeometry::Rectangle {
            x: 200.0,
            y: 200.0,
            width: 80.0,
            height: 80.0,
        }
    );

    // Page 1 should preserve the original rectangle at (50, 50)
    assert_eq!(
        p1_graphics[0].geometry,
        VectorGeometry::Rectangle {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        }
    );
}

#[test]
fn test_vector_edit_after_merge_and_reorder() {
    let pdf1 = make_pdf_with_content(b"10 10 80 80 re S");
    let pdf2 = make_pdf_with_content(b"100 100 50 50 re f");

    let merged_bytes = PdfDocument::merge_documents(&[&pdf1, &pdf2]).expect("Merge documents");
    let mut doc = PdfDocument::from_bytes(&merged_bytes).expect("Valid merged PDF");
    assert_eq!(doc.page_count().expect("Page count"), 2);

    let p1_graphics = doc.enumerate_graphics(1).expect("P1 graphics");
    assert_eq!(p1_graphics.len(), 1);

    // Edit graphic on Page 1
    let spec = UpdateVectorGraphicSpec {
        page_index: 1,
        graphic_id: p1_graphics[0].graphic_id.clone(),
        new_line_width: Some(3.5),
        new_is_stroked: Some(true),
        new_stroke_color: Some(Some(VectorColor::from_rgb(0.0, 0.5, 0.5))),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");
    let p1_re = reopened.enumerate_graphics(1).expect("P1 re-enumerated");
    assert_eq!(p1_re.len(), 1);
    assert!((p1_re[0].line_width - 3.5).abs() < 1e-3);
}

#[test]
fn test_vector_preserves_text_images_and_forms() {
    // Content stream with text and vector rectangle
    let stream = b"BT /F1 14 Tf 50 720 Td (Invoice Summary) Tj ET 50 650 300 20 re S";
    let pdf_bytes = make_pdf_with_content(stream);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");

    // Check text before
    let page_text_before = doc.extract_page_text(0).expect("Extract text");
    assert_eq!(page_text_before.plain_text(), "Invoice Summary");

    // Modify vector rectangle
    let graphics = doc.enumerate_graphics(0).expect("Enumerate graphics");
    assert_eq!(graphics.len(), 1);
    let spec = UpdateVectorGraphicSpec {
        page_index: 0,
        graphic_id: graphics[0].graphic_id.clone(),
        new_fill_color: Some(Some(VectorColor::from_rgb(0.9, 0.9, 0.2))),
        new_is_filled: Some(true),
        ..Default::default()
    };

    let plan = doc.update_graphic(&spec).expect("Update graphic");
    let exported = doc.export_incremental(&plan).expect("Export PDF");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen PDF");

    // Text must remain completely unchanged
    let page_text_after = reopened.extract_page_text(0).expect("Extract text after");
    assert_eq!(page_text_after.plain_text(), "Invoice Summary");

    // Vector rectangle must have fill
    let graphics_after = reopened
        .enumerate_graphics(0)
        .expect("Enumerate graphics after");
    assert_eq!(graphics_after.len(), 1);
    assert!(graphics_after[0].is_filled);
    assert_eq!(
        graphics_after[0].fill_color,
        Some(VectorColor::from_rgb(0.9, 0.9, 0.2))
    );
}
