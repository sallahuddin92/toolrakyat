use starpdf::content::source::scan_instruction_sources;
use starpdf::document::PdfDocument;
use starpdf::image::{
    AddImageSpec, ImageFormat, RemoveImageSpec, ReplaceImageSpec, UpdateImageSpec,
};
use starpdf::syntax::object::{ObjectRef, PdfObject};

/// Generates a minimal 2x2 RGB JPEG JFIF binary.
fn make_minimal_jpeg(r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut data = vec![
        0xFF, 0xD8, // SOI
        0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00,
        0x01, 0x00, 0x00, // APP0
        0xFF, 0xDB, 0x00, 0x43, 0x00, // DQT (Table 0)
    ];
    data.extend(std::iter::repeat_n(16, 64)); // 64 quant values
    data.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x11, 0x08, // SOF0: 8-bit precision
        0x00, 0x02, // Height: 2
        0x00, 0x02, // Width: 2
        0x03, // 3 components (YCbCr / RGB)
        0x01, 0x11, 0x00, // Comp 1
        0x02, 0x11, 0x00, // Comp 2
        0x03, 0x11, 0x00, // Comp 3
        0xFF, 0xC4, 0x00, 0x1F, 0x00, // DHT DC
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA,
        0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, // SOS
        r, g, b, 0x7F, 0xFF, 0xD9, // Scan data + EOI
    ]);
    data
}

/// Generates a valid minimal PDF with a single page containing an image XObject.
fn make_pdf_with_image(jpeg_bytes: &[u8]) -> Vec<u8> {
    let img_len = jpeg_bytes.len();
    let content_stream = b"q\n100 0 0 100 50 600 cm\n/Im1 Do\nQ\nBT\n/F1 12 Tf\n50 500 Td\n(Hello Image World) Tj\nET\n";
    let content_len = content_stream.len();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let o1_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let o2_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let o3_offset = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 6 0 R >> /XObject << /Im1 5 0 R >> >> >>\nendobj\n");

    let o4_offset = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {content_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(content_stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let o5_offset = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {img_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(jpeg_bytes);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let o6_offset = pdf.len();
    pdf.extend_from_slice(
        b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{o1_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o2_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o3_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o4_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o5_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o6_offset:010} 00000 n \n").as_bytes());

    pdf.extend_from_slice(
        format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn make_adversarial_shipping_label_pdf(jpeg_bytes: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let stream1 = b"% stream one stays byte-identical\nq 36 0 0 20 20 730 cm /Im1 Do Q\n".to_vec();
    let mut stream2 = b"% shipping label complex stream\n/Artifact << /Type /Pagination /Attached [/Top] /Flag true /Nil null >> BDC\nq\n80 0 0 24 40 650 cm\n/Im1 Do\nQ\n".to_vec();
    stream2.extend_from_slice(b"BI /W 1 /H 1 /BPC 8 /CS /RGB ID \x00EI\xff\nEI\n");
    stream2.extend_from_slice(b"q\n60 0 0 60 180 620 cm\n/Im1 Do\nQ\n");
    stream2.extend_from_slice(b"q 1 0 0 1 30 560 cm 0 0 250 40 re W n 0.5 w 0 0 m 250 0 l S Q\n");
    stream2.extend_from_slice(b"q\n120 0 0 30 40 500 cm\n/Im1 Do\nQ\nEMC\n");
    let stream3 =
        b"BT /F1 14 Tf 40 440 Td (SHIP TO: RAKYAT) Tj ET\n0 0 0 RG 1 w 35 420 260 100 re S\n"
            .to_vec();
    let streams = vec![stream1, stream2, stream3];

    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    {
        let mut object = |number: usize, body: &[u8]| {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        };
        object(1, b"<< /Type /Catalog /Pages 2 0 R /AcroForm 10 0 R >>");
        object(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
        object(3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 320 780] /Contents [4 0 R 5 0 R 6 0 R] /Resources << /Font << /F1 8 0 R >> /XObject << /Im1 7 0 R >> >> /Annots [9 0 R] >>");
        for (index, stream) in streams.iter().enumerate() {
            let mut body = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
            body.extend_from_slice(stream);
            body.extend_from_slice(b"\nendstream");
            object(index + 4, &body);
        }
        let mut image_body = format!("<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg_bytes.len()).into_bytes();
        image_body.extend_from_slice(jpeg_bytes);
        image_body.extend_from_slice(b"\nendstream");
        object(7, &image_body);
        object(8, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
        object(9, b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (Tracking) /Rect [40 380 200 405] /P 3 0 R >>");
        object(10, b"<< /Fields [9 0 R] >>");
    }

    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 11\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 11 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    (pdf, streams)
}

fn changed_stream(plan: &starpdf::mutation::MutationPlan, reference: ObjectRef) -> Vec<u8> {
    plan.modified_objects
        .get(&reference)
        .and_then(PdfObject::as_stream)
        .expect("target stream mutation")
        .data
        .clone()
}

#[test]
fn surgical_remove_preserves_every_unrelated_decoded_byte_in_complex_multi_stream_page() {
    let jpeg = make_minimal_jpeg(0x10, 0x20, 0x30);
    let (pdf, streams) = make_adversarial_shipping_label_pdf(&jpeg);
    let mut doc = PdfDocument::from_bytes(&pdf).expect("adversarial PDF");
    let images = doc.enumerate_images(0).expect("images");
    assert_eq!(images.len(), 4);
    let target = images
        .iter()
        .filter(|image| image.stream_index == 1)
        .nth(1)
        .expect("middle repeated image")
        .clone();
    let sources = scan_instruction_sources(&streams[1]).expect("source scan");
    let source = &sources[target.instruction_index];
    assert_eq!(&streams[1][source.byte_start..source.byte_end], b"/Im1 Do");
    let mut expected = streams[1][..source.byte_start].to_vec();
    expected.extend_from_slice(&streams[1][source.byte_end..]);

    let plan = doc
        .remove_image(&RemoveImageSpec {
            page_index: 0,
            image_id: target.image_id,
        })
        .expect("surgical remove");
    assert_eq!(changed_stream(&plan, ObjectRef::new(5, 0)), expected);
    assert!(!plan.modified_objects.contains_key(&ObjectRef::new(4, 0)));
    assert!(!plan.modified_objects.contains_key(&ObjectRef::new(6, 0)));
    assert!(!plan.modified_objects.contains_key(&ObjectRef::new(7, 0)));
    assert!(expected.windows(3).any(|window| window == b"\0EI"));
    assert!(expected.windows(3).any(|window| window == b"BDC"));

    let exported = doc.export_incremental(&plan).expect("export");
    let mut reopened = PdfDocument::from_bytes(&exported).expect("reopen");
    assert_eq!(reopened.enumerate_images(0).expect("images").len(), 3);
    assert_eq!(reopened.form_fields().expect("form").len(), 1);
    let reopened_text_stream = reopened
        .store_mut()
        .resolve(ObjectRef::new(6, 0))
        .expect("text stream")
        .as_stream()
        .expect("stream")
        .data
        .clone();
    assert_eq!(reopened_text_stream, streams[2]);
}

#[test]
fn surgical_update_changes_only_isolated_cm_source_range() {
    let jpeg = make_minimal_jpeg(0x10, 0x20, 0x30);
    let (pdf, streams) = make_adversarial_shipping_label_pdf(&jpeg);
    let mut doc = PdfDocument::from_bytes(&pdf).expect("adversarial PDF");
    let target = doc
        .enumerate_images(0)
        .expect("images")
        .into_iter()
        .find(|image| image.stream_index == 1)
        .expect("image");
    let sources = scan_instruction_sources(&streams[1]).expect("scan");
    let cm = &sources[target.instruction_index - 1];
    let replacement = b"90.0000 0 0 25.0000 45.0000 645.0000 cm";
    let mut expected = streams[1][..cm.byte_start].to_vec();
    expected.extend_from_slice(replacement);
    expected.extend_from_slice(&streams[1][cm.byte_end..]);
    let plan = doc
        .update_image(&UpdateImageSpec {
            page_index: 0,
            image_id: target.image_id,
            new_x: 45.0,
            new_y: 645.0,
            new_width: 90.0,
            new_height: 25.0,
            clone_if_shared: true,
        })
        .expect("surgical update");
    assert_eq!(changed_stream(&plan, ObjectRef::new(5, 0)), expected);
}

#[test]
fn shared_same_resource_replacement_changes_only_selected_occurrence() {
    let jpeg = make_minimal_jpeg(0x10, 0x20, 0x30);
    let replacement = make_minimal_jpeg(0x90, 0x80, 0x70);
    let (pdf, _) = make_adversarial_shipping_label_pdf(&jpeg);
    let mut doc = PdfDocument::from_bytes(&pdf).expect("adversarial PDF");
    let before = doc.enumerate_images(0).expect("images");
    let target = before
        .iter()
        .filter(|image| image.stream_index == 1)
        .nth(1)
        .expect("middle image");
    let plan = doc
        .replace_image(&ReplaceImageSpec {
            page_index: 0,
            image_id: target.image_id.clone(),
            new_image_bytes: replacement.clone(),
            format: ImageFormat::Jpeg,
            clone_if_shared: true,
        })
        .expect("isolated replace");
    let exported = doc.export_incremental(&plan).expect("export");
    let mut reopened = PdfDocument::from_bytes(&exported).expect("reopen");
    let after = reopened.enumerate_images(0).expect("images");
    assert_eq!(after.len(), 4);
    let stream_two: Vec<_> = after
        .iter()
        .filter(|image| image.stream_index == 1)
        .collect();
    assert_eq!(stream_two.len(), 3);
    assert_eq!(stream_two[0].object_ref, before[1].object_ref);
    assert_ne!(stream_two[1].object_ref, before[2].object_ref);
    assert_eq!(stream_two[2].object_ref, before[3].object_ref);
    let selected_data = reopened
        .store_mut()
        .resolve(stream_two[1].object_ref)
        .expect("replacement")
        .as_stream()
        .expect("stream")
        .data
        .clone();
    assert_eq!(selected_data, replacement);
}

#[test]
fn removal_clones_a_content_stream_shared_by_two_pages() {
    let jpeg = make_minimal_jpeg(0x10, 0x20, 0x30);
    let pdf = make_pdf_with_image(&jpeg);
    let mut original = PdfDocument::from_bytes(&pdf).expect("PDF");
    let duplicated = original
        .apply_page_operations(
            &starpdf::page_ops::PageOperationPlan {
                edits: vec![starpdf::page_ops::PageEdit::DuplicatePage {
                    index: 0,
                    insert_at: 1,
                }],
            },
            &starpdf::page_ops::PageOperationLimits::default(),
        )
        .expect("duplicate");
    let mut doc = PdfDocument::from_bytes(&duplicated).expect("reopen");
    let page_zero = doc.enumerate_images(0).expect("page zero");
    let plan = doc
        .remove_image(&RemoveImageSpec {
            page_index: 0,
            image_id: page_zero[0].image_id.clone(),
        })
        .expect("remove one occurrence");
    let exported = doc.export_incremental(&plan).expect("export");
    let mut reopened = PdfDocument::from_bytes(&exported).expect("reopen");
    assert!(reopened.enumerate_images(0).expect("page zero").is_empty());
    assert_eq!(reopened.enumerate_images(1).expect("page one").len(), 1);
}

#[test]
fn test_image_enumeration_and_identity() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate images");

    assert_eq!(images.len(), 1);
    let img = &images[0];
    assert_eq!(img.resource_name, "Im1");
    assert_eq!(img.width, 2);
    assert_eq!(img.height, 2);
    assert_eq!(img.color_space, "DeviceRGB");
    assert_eq!(img.bits_per_component, 8);
    assert_eq!(img.filter.as_deref(), Some("DCTDecode"));
    assert!(!img.is_nested_form);
    assert!(!img.is_shared);

    // Verify geometry: 100 0 0 100 50 600 cm -> rect [50, 600, 150, 700]
    assert!((img.rect[0] - 50.0).abs() < 1e-3);
    assert!((img.rect[1] - 600.0).abs() < 1e-3);
    assert!((img.rect[2] - 150.0).abs() < 1e-3);
    assert!((img.rect[3] - 700.0).abs() < 1e-3);
}

#[test]
fn test_replace_jpeg_image() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let img2 = make_minimal_jpeg(0xFF, 0x00, 0xAA);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate images");
    let target_id = images[0].image_id.clone();

    let spec = ReplaceImageSpec {
        page_index: 0,
        image_id: target_id,
        new_image_bytes: img2.clone(),
        format: ImageFormat::Jpeg,
        clone_if_shared: false,
    };

    let plan = doc.replace_image(&spec).expect("Replace image");
    let exported = doc.export_incremental(&plan).expect("Export");

    // Reopen and verify
    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let reopened_images = reopened.enumerate_images(0).expect("Enumerate");
    assert_eq!(reopened_images.len(), 1);
    assert_eq!(reopened_images[0].width, 2);
    assert_eq!(reopened_images[0].height, 2);

    // Verify stream object contains new JPEG bytes
    let obj = reopened
        .store_mut()
        .resolve(reopened_images[0].object_ref)
        .expect("Resolve image");
    let stream = obj.as_stream().expect("Is stream");
    assert_eq!(stream.data, img2);
}

#[test]
fn test_replace_flate_image() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate images");
    let target_id = images[0].image_id.clone();

    // Raw 4x4 RGB pixels = 48 bytes
    let raw_pixels = vec![200u8; 48];
    let spec = ReplaceImageSpec {
        page_index: 0,
        image_id: target_id,
        new_image_bytes: raw_pixels,
        format: ImageFormat::Flate {
            color_space: "DeviceRGB".to_string(),
            width: 4,
            height: 4,
            bits_per_component: 8,
        },
        clone_if_shared: false,
    };

    let plan = doc.replace_image(&spec).expect("Replace image with flate");
    let exported = doc.export_incremental(&plan).expect("Export");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let reopened_images = reopened.enumerate_images(0).expect("Enumerate");
    assert_eq!(reopened_images.len(), 1);
    assert_eq!(reopened_images[0].width, 4);
    assert_eq!(reopened_images[0].height, 4);
    assert_eq!(reopened_images[0].filter.as_deref(), Some("FlateDecode"));
}

#[test]
fn test_add_image_to_page() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let img2 = make_minimal_jpeg(0x40, 0x50, 0x60);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let add_spec = AddImageSpec {
        page_index: 0,
        image_bytes: img2,
        format: ImageFormat::Jpeg,
        x: 200.0,
        y: 300.0,
        width: 150.0,
        height: 120.0,
    };

    let plan = doc.add_image(&add_spec).expect("Add image");
    let exported = doc.export_incremental(&plan).expect("Export");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let reopened_images = reopened.enumerate_images(0).expect("Enumerate");
    assert_eq!(reopened_images.len(), 2);

    let added_img = &reopened_images[1];
    assert!((added_img.rect[0] - 200.0).abs() < 1e-3);
    assert!((added_img.rect[1] - 300.0).abs() < 1e-3);
    assert!((added_img.rect[2] - 350.0).abs() < 1e-3);
    assert!((added_img.rect[3] - 420.0).abs() < 1e-3);
}

#[test]
fn test_remove_image_from_page() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate images");
    let target_id = images[0].image_id.clone();

    let remove_spec = RemoveImageSpec {
        page_index: 0,
        image_id: target_id,
    };

    let plan = doc.remove_image(&remove_spec).expect("Remove image");
    let exported = doc.export_incremental(&plan).expect("Export");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let reopened_images = reopened.enumerate_images(0).expect("Enumerate");
    assert_eq!(reopened_images.len(), 0);

    // Verify text is still present!
    let hits = reopened
        .search(
            "Hello Image World",
            &starpdf::search::SearchOptions {
                case_sensitive: true,
            },
        )
        .expect("Search");
    assert_eq!(hits.len(), 1);
}

#[test]
fn test_shared_xobject_clone_isolation() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let img_replacement = make_minimal_jpeg(0x99, 0x88, 0x77);

    let pdf_bytes = make_pdf_with_image(&img1);
    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");

    // Duplicate page 0
    let dup_plan = starpdf::page_ops::PageOperationPlan {
        edits: vec![starpdf::page_ops::PageEdit::DuplicatePage {
            index: 0,
            insert_at: 1,
        }],
    };
    let multi_page_bytes = doc
        .apply_page_operations(
            &dup_plan,
            &starpdf::page_ops::PageOperationLimits::default(),
        )
        .expect("Duplicate page");

    let mut doc2 = PdfDocument::from_bytes(&multi_page_bytes).expect("Reopen 2-page doc");
    assert_eq!(doc2.page_count().expect("Count"), 2);

    let all_images_before = doc2.enumerate_all_images().expect("Enumerate all");
    assert_eq!(all_images_before.len(), 2);
    assert!(all_images_before[0].is_shared);
    assert!(all_images_before[1].is_shared);
    assert_eq!(
        all_images_before[0].object_ref,
        all_images_before[1].object_ref
    );

    // Replace image on page 0 with clone_if_shared = true
    let spec = ReplaceImageSpec {
        page_index: 0,
        image_id: all_images_before[0].image_id.clone(),
        new_image_bytes: img_replacement.clone(),
        format: ImageFormat::Jpeg,
        clone_if_shared: true,
    };

    let plan = doc2.replace_image(&spec).expect("Replace shared image");
    let exported = doc2.export_incremental(&plan).expect("Export");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let all_images_after = reopened.enumerate_all_images().expect("Enumerate all");
    assert_eq!(all_images_after.len(), 2);

    // Page 0 should now point to a DIFFERENT object reference than Page 1!
    assert_ne!(
        all_images_after[0].object_ref,
        all_images_after[1].object_ref
    );

    let page0_stream = reopened
        .store_mut()
        .resolve(all_images_after[0].object_ref)
        .expect("Resolve")
        .as_stream()
        .expect("Is stream")
        .data
        .clone();

    let page1_stream = reopened
        .store_mut()
        .resolve(all_images_after[1].object_ref)
        .expect("Resolve")
        .as_stream()
        .expect("Is stream")
        .data
        .clone();

    assert_eq!(page0_stream, img_replacement);
    assert_eq!(page1_stream, img1);
}

#[test]
fn test_sequential_image_edits() {
    let img1 = make_minimal_jpeg(0x10, 0x10, 0x10);
    let img2 = make_minimal_jpeg(0x20, 0x20, 0x20);
    let img3 = make_minimal_jpeg(0x30, 0x30, 0x30);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");

    // 1. Add second image
    let add_plan = doc
        .add_image(&AddImageSpec {
            page_index: 0,
            image_bytes: img2.clone(),
            format: ImageFormat::Jpeg,
            x: 250.0,
            y: 250.0,
            width: 80.0,
            height: 80.0,
        })
        .expect("Add");
    let pdf_v2 = doc.export_incremental(&add_plan).expect("Export v2");

    // 2. Reopen and replace first image
    let mut doc_v2 = PdfDocument::from_bytes(&pdf_v2).expect("Reopen v2");
    let images_v2 = doc_v2.enumerate_images(0).expect("Enumerate v2");
    assert_eq!(images_v2.len(), 2);

    let replace_plan = doc_v2
        .replace_image(&ReplaceImageSpec {
            page_index: 0,
            image_id: images_v2[0].image_id.clone(),
            new_image_bytes: img3.clone(),
            format: ImageFormat::Jpeg,
            clone_if_shared: false,
        })
        .expect("Replace");
    let pdf_v3 = doc_v2.export_incremental(&replace_plan).expect("Export v3");

    // 3. Reopen and remove the added image
    let mut doc_v3 = PdfDocument::from_bytes(&pdf_v3).expect("Reopen v3");
    let images_v3 = doc_v3.enumerate_images(0).expect("Enumerate v3");
    assert_eq!(images_v3.len(), 2);

    let remove_plan = doc_v3
        .remove_image(&RemoveImageSpec {
            page_index: 0,
            image_id: images_v3[1].image_id.clone(),
        })
        .expect("Remove");
    let pdf_v4 = doc_v3.export_incremental(&remove_plan).expect("Export v4");

    // Final verification
    let mut doc_final = PdfDocument::from_bytes(&pdf_v4).expect("Reopen final");
    let images_final = doc_final.enumerate_images(0).expect("Enumerate final");
    assert_eq!(images_final.len(), 1);

    let final_obj = doc_final
        .store_mut()
        .resolve(images_final[0].object_ref)
        .expect("Resolve")
        .as_stream()
        .expect("Stream");
    assert_eq!(final_obj.data, img3);
}

#[test]
fn test_image_edit_after_merge_and_reorder() {
    let img1 = make_minimal_jpeg(0x11, 0x22, 0x33);
    let doc1 = make_pdf_with_image(&img1);
    let doc2 = make_pdf_with_image(&img1);

    let merged_bytes = PdfDocument::merge_documents(&[&doc1, &doc2]).expect("Merge");

    let mut doc_merged = PdfDocument::from_bytes(&merged_bytes).expect("Reopen merged");
    assert_eq!(doc_merged.page_count().expect("Count"), 2);

    let img_new = make_minimal_jpeg(0xAA, 0xBB, 0xCC);
    let page1_images = doc_merged.enumerate_images(1).expect("Enumerate page 1");
    assert_eq!(page1_images.len(), 1);

    let plan = doc_merged
        .replace_image(&ReplaceImageSpec {
            page_index: 1,
            image_id: page1_images[0].image_id.clone(),
            new_image_bytes: img_new.clone(),
            format: ImageFormat::Jpeg,
            clone_if_shared: true,
        })
        .expect("Replace on merged page 1");

    let exported = doc_merged.export_incremental(&plan).expect("Export");
    let mut doc_final = PdfDocument::from_bytes(&exported).expect("Reopen");
    let final_p0 = doc_final.enumerate_images(0).expect("P0");
    let final_p1 = doc_final.enumerate_images(1).expect("P1");

    assert_eq!(final_p0.len(), 1);
    assert_eq!(final_p1.len(), 1);

    let p0_data = doc_final
        .store_mut()
        .resolve(final_p0[0].object_ref)
        .expect("P0 obj")
        .as_stream()
        .expect("Stream")
        .data
        .clone();
    let p1_data = doc_final
        .store_mut()
        .resolve(final_p1[0].object_ref)
        .expect("P1 obj")
        .as_stream()
        .expect("Stream")
        .data
        .clone();

    assert_eq!(p0_data, img1);
    assert_eq!(p1_data, img_new);
}

#[test]
fn test_multiple_images_same_page() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let img2 = make_minimal_jpeg(0x40, 0x50, 0x60);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");

    // Add 2 more images to same page
    let plan1 = doc
        .add_image(&AddImageSpec {
            page_index: 0,
            image_bytes: img2.clone(),
            format: ImageFormat::Jpeg,
            x: 50.0,
            y: 200.0,
            width: 100.0,
            height: 100.0,
        })
        .expect("Add 1");
    let pdf_v2 = doc.export_incremental(&plan1).expect("Export 1");

    let mut doc_v2 = PdfDocument::from_bytes(&pdf_v2).expect("Reopen 1");
    let plan2 = doc_v2
        .add_image(&AddImageSpec {
            page_index: 0,
            image_bytes: img2.clone(),
            format: ImageFormat::Jpeg,
            x: 200.0,
            y: 200.0,
            width: 120.0,
            height: 80.0,
        })
        .expect("Add 2");
    let pdf_v3 = doc_v2.export_incremental(&plan2).expect("Export 2");

    let mut doc_v3 = PdfDocument::from_bytes(&pdf_v3).expect("Reopen 2");
    let images = doc_v3.enumerate_images(0).expect("Enumerate");
    assert_eq!(images.len(), 3);

    // Replace the middle image
    let replace_img = make_minimal_jpeg(0xAA, 0xBB, 0xCC);
    let plan_rep = doc_v3
        .replace_image(&ReplaceImageSpec {
            page_index: 0,
            image_id: images[1].image_id.clone(),
            new_image_bytes: replace_img.clone(),
            format: ImageFormat::Jpeg,
            clone_if_shared: false,
        })
        .expect("Replace middle");
    let pdf_v4 = doc_v3.export_incremental(&plan_rep).expect("Export rep");

    let mut doc_v4 = PdfDocument::from_bytes(&pdf_v4).expect("Reopen rep");
    let final_images = doc_v4.enumerate_images(0).expect("Enumerate final");
    assert_eq!(final_images.len(), 3);

    let middle_data = doc_v4
        .store_mut()
        .resolve(final_images[1].object_ref)
        .expect("Resolve")
        .as_stream()
        .expect("Stream")
        .data
        .clone();
    assert_eq!(middle_data, replace_img);
}

#[test]
fn test_transformed_and_scaled_image() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    // PDF with scale and translation matrix
    let content_stream = b"q\n200 0 0 150 100 400 cm\n/Im1 Do\nQ\n";
    let content_len = content_stream.len();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n");
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {content_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(content_stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", img1.len()).as_bytes());
    pdf.extend_from_slice(&img1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
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
    let images = doc.enumerate_images(0).expect("Enumerate");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].transform, [200.0, 0.0, 0.0, 150.0, 100.0, 400.0]);
    assert_eq!(images[0].rect, [100.0, 400.0, 300.0, 550.0]);
}

#[test]
fn test_image_on_rotated_page() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let content_stream = b"q\n100 0 0 100 50 50 cm\n/Im1 Do\nQ\n";
    let content_len = content_stream.len();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Rotate 90 /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n");
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {content_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(content_stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", img1.len()).as_bytes());
    pdf.extend_from_slice(&img1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
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
    let images = doc.enumerate_images(0).expect("Enumerate");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].resource_name, "Im1");

    // Replace on rotated page
    let img2 = make_minimal_jpeg(0x99, 0x88, 0x77);
    let plan = doc
        .replace_image(&ReplaceImageSpec {
            page_index: 0,
            image_id: images[0].image_id.clone(),
            new_image_bytes: img2.clone(),
            format: ImageFormat::Jpeg,
            clone_if_shared: false,
        })
        .expect("Replace");
    let exported = doc.export_incremental(&plan).expect("Export");

    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let reopened_images = reopened.enumerate_images(0).expect("Enumerate");
    assert_eq!(reopened_images.len(), 1);
}

#[test]
fn test_nested_form_xobject_inspection_and_refusal() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    // Form XObject stream drawing /ImNested
    let form_content = b"q\n50 0 0 50 10 10 cm\n/ImNested Do\nQ\n";
    let form_len = form_content.len();
    let page_content = b"/Fm1 Do\n";
    let page_len = page_content.len();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /Fm1 5 0 R >> >> >>\nendobj\n");
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {page_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(page_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /XObject << /ImNested 6 0 R >> >> /Length {form_len} >>\nstream\n").as_bytes());
    pdf.extend_from_slice(form_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o6 = pdf.len();
    pdf.extend_from_slice(format!("6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", img1.len()).as_bytes());
    pdf.extend_from_slice(&img1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{o6:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );

    let mut doc = PdfDocument::from_bytes(&pdf).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate");
    assert_eq!(images.len(), 1);
    assert!(images[0].is_nested_form);
    assert_eq!(images[0].resource_name, "ImNested");

    // Mutation of nested Form XObject should be refused with typed NestedFormXObjectRefusal
    let res = doc.replace_image(&ReplaceImageSpec {
        page_index: 0,
        image_id: images[0].image_id.clone(),
        new_image_bytes: img1.clone(),
        format: ImageFormat::Jpeg,
        clone_if_shared: false,
    });
    match res {
        Err(starpdf::error::PdfError::NestedFormXObjectRefusal(_)) => {}
        other => panic!("Expected NestedFormXObjectRefusal, got {:?}", other),
    }
}

#[test]
fn test_invalid_image_format_refusal() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate");

    let invalid_bytes = b"NOT_A_VALID_JPEG_HEADER";
    let res = doc.replace_image(&ReplaceImageSpec {
        page_index: 0,
        image_id: images[0].image_id.clone(),
        new_image_bytes: invalid_bytes.to_vec(),
        format: ImageFormat::Jpeg,
        clone_if_shared: false,
    });

    match res {
        Err(starpdf::error::PdfError::UnsupportedImageFormat(_)) => {}
        other => panic!("Expected UnsupportedImageFormat, got {:?}", other),
    }
}

#[test]
fn test_image_move_and_resize_roundtrip() {
    let img1 = make_minimal_jpeg(0x10, 0x20, 0x30);
    let pdf_bytes = make_pdf_with_image(&img1);

    let mut doc = PdfDocument::from_bytes(&pdf_bytes).expect("Valid PDF");
    let images = doc.enumerate_images(0).expect("Enumerate");
    assert_eq!(images.len(), 1);
    let orig_id = images[0].image_id.clone();

    // Move to (200, 400) and resize to (150, 180)
    let plan = doc
        .update_image(&starpdf::image::UpdateImageSpec {
            page_index: 0,
            image_id: orig_id,
            new_x: 200.0,
            new_y: 400.0,
            new_width: 150.0,
            new_height: 180.0,
            clone_if_shared: true,
        })
        .expect("Update image position/size");

    let exported = doc.export_incremental(&plan).expect("Export");
    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let reopened_images = reopened.enumerate_images(0).expect("Enumerate reopened");

    assert_eq!(reopened_images.len(), 1);
    let updated_img = &reopened_images[0];
    assert!((updated_img.rect[0] - 200.0).abs() < 1e-2);
    assert!((updated_img.rect[1] - 400.0).abs() < 1e-2);
    assert!((updated_img.rect[2] - 350.0).abs() < 1e-2);
    assert!((updated_img.rect[3] - 580.0).abs() < 1e-2);
}
