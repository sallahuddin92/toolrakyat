use std::time::Instant;

use starpdf::mutation::text_edit::TextEditTarget;
use starpdf::search::SearchOptions;
use starpdf::vector::{AddVectorGraphicSpec, VectorColor, VectorGeometry};
use starpdf::PdfDocument;

/// Deterministic generator for text-heavy multi-page PDFs
fn generate_text_document(num_pages: usize, lines_per_page: usize) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets: Vec<usize> = Vec::new();
    offsets.push(0); // 0 0 obj dummy

    // Object 1: Catalog
    let o1 = pdf.len();
    offsets.push(o1);
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Obj 1: Catalog
    // Obj 2: Pages root
    // For each page i in 0..num_pages:
    //   Page obj: 3 + i*2
    //   Content stream obj: 4 + i*2
    // Obj (3 + num_pages*2): Font F1
    let font_obj_num = 3 + num_pages * 2;

    // Object 2: Pages root
    let o2 = pdf.len();
    offsets.push(o2);
    let mut kids = String::new();
    for i in 0..num_pages {
        let p_num = 3 + i * 2;
        kids.push_str(&format!("{p_num} 0 R "));
    }
    pdf.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {num_pages} >>\nendobj\n")
            .as_bytes(),
    );

    // Page objects and content streams
    for i in 0..num_pages {
        let page_obj_num = 3 + i * 2;
        let content_obj_num = 4 + i * 2;

        // Page Object
        let p_offset = pdf.len();
        offsets.push(p_offset);
        pdf.extend_from_slice(
            format!(
                "{page_obj_num} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content_obj_num} 0 R /Resources << /Font << /F1 {font_obj_num} 0 R >> >> >>\nendobj\n"
            )
            .as_bytes(),
        );

        // Content stream
        let mut content = String::new();
        content.push_str("BT\n/F1 11 Tf\n");
        for line in 0..lines_per_page {
            let y = 740 - (line * 16);
            content.push_str(&format!(
                "50 {y} Td (Page {p} Line {line}: StarPDF large-document performance qualification text stream record #{rec:04}.) Tj\n",
                p = i + 1,
                line = line + 1,
                rec = (i * lines_per_page + line) % 10000
            ));
        }
        content.push_str("ET\n");

        let c_offset = pdf.len();
        offsets.push(c_offset);
        pdf.extend_from_slice(
            format!(
                "{content_obj_num} 0 obj\n<< /Length {len} >>\nstream\n{content}\nendstream\nendobj\n",
                len = content.len()
            )
            .as_bytes(),
        );
    }

    // Font object F1 (Standard Type1 Helvetica)
    let font_offset = pdf.len();
    offsets.push(font_offset);
    pdf.extend_from_slice(
        format!("{font_obj_num} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n")
            .as_bytes(),
    );

    let total_objs = offsets.len();
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {total_objs}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }

    pdf.extend_from_slice(
        format!("trailer\n<< /Size {total_objs} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );

    pdf
}

/// Deterministic generator for vector-heavy multi-page PDFs
fn generate_vector_document(num_pages: usize, shapes_per_page: usize) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets: Vec<usize> = Vec::new();
    offsets.push(0);

    let o1 = pdf.len();
    offsets.push(o1);
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let o2 = pdf.len();
    offsets.push(o2);
    let mut kids = String::new();
    for i in 0..num_pages {
        let p_num = 3 + i * 2;
        kids.push_str(&format!("{p_num} 0 R "));
    }
    pdf.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {num_pages} >>\nendobj\n")
            .as_bytes(),
    );

    for i in 0..num_pages {
        let page_obj_num = 3 + i * 2;
        let content_obj_num = 4 + i * 2;

        let p_offset = pdf.len();
        offsets.push(p_offset);
        pdf.extend_from_slice(
            format!(
                "{page_obj_num} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content_obj_num} 0 R >>\nendobj\n"
            )
            .as_bytes(),
        );

        let mut content = String::new();
        for s in 0..shapes_per_page {
            let x = 50 + (s % 5) * 100;
            let y = 100 + (s / 5) * 60;
            let r = ((s * 47) % 255) as f64 / 255.0;
            let g = ((s * 93) % 255) as f64 / 255.0;
            let b = ((s * 137) % 255) as f64 / 255.0;
            if s % 2 == 0 {
                // Rectangle
                content.push_str(&format!(
                    "q\n{r:.2} {g:.2} {b:.2} rg\n1.5 w\n0 0 0 RG\n{x} {y} 80 40 re\nB\nQ\n"
                ));
            } else {
                // Line
                content.push_str(&format!(
                    "q\n2.0 w\n{r:.2} {g:.2} {b:.2} RG\n{x} {y} m {x2} {y2} l\nS\nQ\n",
                    x2 = x + 80,
                    y2 = y + 40
                ));
            }
        }

        let c_offset = pdf.len();
        offsets.push(c_offset);
        pdf.extend_from_slice(
            format!(
                "{content_obj_num} 0 obj\n<< /Length {len} >>\nstream\n{content}\nendstream\nendobj\n",
                len = content.len()
            )
            .as_bytes(),
        );
    }

    let total_objs = offsets.len();
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {total_objs}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }

    pdf.extend_from_slice(
        format!("trailer\n<< /Size {total_objs} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );

    pdf
}

#[test]
fn test_qualification_scaling_10_100_500_pages() {
    // 1. Generate 10, 100, 500 page documents
    let t0 = Instant::now();
    let doc_10_bytes = generate_text_document(10, 20);
    let doc_100_bytes = generate_text_document(100, 20);
    let doc_500_bytes = generate_text_document(500, 20);
    let gen_time = t0.elapsed();
    println!("Generated 10, 100, 500-page test PDFs in {:.2?}", gen_time);

    // 2. Measure Open & Page Count
    let start = Instant::now();
    let mut doc_10 = PdfDocument::from_bytes(&doc_10_bytes).expect("Open 10-page doc");
    let open_10 = start.elapsed();
    assert_eq!(doc_10.page_count().unwrap(), 10);

    let start = Instant::now();
    let mut doc_100 = PdfDocument::from_bytes(&doc_100_bytes).expect("Open 100-page doc");
    let open_100 = start.elapsed();
    assert_eq!(doc_100.page_count().unwrap(), 100);

    let start = Instant::now();
    let mut doc_500 = PdfDocument::from_bytes(&doc_500_bytes).expect("Open 500-page doc");
    let open_500 = start.elapsed();
    assert_eq!(doc_500.page_count().unwrap(), 500);

    println!(
        "Open & Page Tree Resolution: 10p={:.2?}, 100p={:.2?}, 500p={:.2?}",
        open_10, open_100, open_500
    );

    // 3. Measure Text Extraction across all pages
    let start = Instant::now();
    for p in 0..10 {
        let _ = doc_10.extract_page_text(p).unwrap();
    }
    let extract_10 = start.elapsed();

    let start = Instant::now();
    for p in 0..100 {
        let _ = doc_100.extract_page_text(p).unwrap();
    }
    let extract_100 = start.elapsed();

    let start = Instant::now();
    for p in 0..500 {
        let _ = doc_500.extract_page_text(p).unwrap();
    }
    let extract_500 = start.elapsed();

    println!(
        "Text Extraction (all pages): 10p={:.2?}, 100p={:.2?}, 500p={:.2?}",
        extract_10, extract_100, extract_500
    );

    // 4. Measure Full-Document Search Indexing & Querying
    let start = Instant::now();
    let hits_10 = doc_10
        .search("qualification", &SearchOptions::default())
        .unwrap();
    let search_10 = start.elapsed();
    assert_eq!(hits_10.len(), 200);

    let start = Instant::now();
    let hits_100 = doc_100
        .search("qualification", &SearchOptions::default())
        .unwrap();
    let search_100 = start.elapsed();
    assert_eq!(hits_100.len(), 2000);

    let start = Instant::now();
    let hits_500 = doc_500
        .search("qualification", &SearchOptions::default())
        .unwrap();
    let search_500 = start.elapsed();
    assert_eq!(hits_500.len(), 10000);

    println!(
        "Search Across All Pages: 10p={:.2?} ({} hits), 100p={:.2?} ({} hits), 500p={:.2?} ({} hits)",
        search_10,
        hits_10.len(),
        search_100,
        hits_100.len(),
        search_500,
        hits_500.len()
    );

    // Scaling verification: 500p search scales predictably
    assert!(search_500.as_millis() <= (search_100.as_millis() * 8 + 50));
}

#[test]
fn test_qualification_vector_scaling_10_100_500_pages() {
    let vec_10_bytes = generate_vector_document(10, 10);
    let vec_100_bytes = generate_vector_document(100, 10);
    let vec_500_bytes = generate_vector_document(500, 10);

    let mut doc_10 = PdfDocument::from_bytes(&vec_10_bytes).expect("Open vec 10");
    let mut doc_100 = PdfDocument::from_bytes(&vec_100_bytes).expect("Open vec 100");
    let mut doc_500 = PdfDocument::from_bytes(&vec_500_bytes).expect("Open vec 500");

    let start = Instant::now();
    let g_10 = doc_10.enumerate_all_graphics().unwrap();
    let enum_10 = start.elapsed();
    assert_eq!(g_10.len(), 100);

    let start = Instant::now();
    let g_100 = doc_100.enumerate_all_graphics().unwrap();
    let enum_100 = start.elapsed();
    assert_eq!(g_100.len(), 1000);

    let start = Instant::now();
    let g_500 = doc_500.enumerate_all_graphics().unwrap();
    let enum_500 = start.elapsed();
    assert_eq!(g_500.len(), 5000);

    println!(
        "Vector Enumeration (all pages): 10p={:.2?} (100 shapes), 100p={:.2?} (1000 shapes), 500p={:.2?} (5000 shapes)",
        enum_10, enum_100, enum_500
    );
}

#[test]
fn test_qualification_editing_on_large_document() {
    // 100-page document
    let doc_bytes = generate_text_document(100, 10);
    let mut doc = PdfDocument::from_bytes(&doc_bytes).expect("Open doc");

    // Extract text on page 50
    let text_page_50 = doc.extract_page_text(50).unwrap();
    assert!(!text_page_50.spans.is_empty());
    let span_to_edit = &text_page_50.spans[0];
    let target = TextEditTarget::from_span(span_to_edit);

    // Native text mutation on page 50
    let start = Instant::now();
    let plan = doc
        .replace_text(50, &target, "MUTATED_TEXT_QUALIFICATION_50")
        .expect("Replace text");
    let text_edit_time = start.elapsed();

    // Incremental export
    let start = Instant::now();
    let exported = doc.export_incremental(&plan).expect("Export");
    let export_time = start.elapsed();

    // Reopen and verify page 50 has the new text and page 49 is unchanged
    let mut reopened = PdfDocument::from_bytes(&exported).expect("Reopen");
    let p50_reopened = reopened.extract_page_text(50).unwrap();
    assert!(p50_reopened
        .plain_text()
        .contains("MUTATED_TEXT_QUALIFICATION_50"));

    let p49_reopened = reopened.extract_page_text(49).unwrap();
    assert!(!p49_reopened
        .plain_text()
        .contains("MUTATED_TEXT_QUALIFICATION_50"));

    println!(
        "Large-Doc Text Edit on Page 50 of 100: Edit={:.2?}, Export={:.2?}",
        text_edit_time, export_time
    );
}

#[test]
fn test_qualification_page_operations_on_large_document() {
    let doc_bytes = generate_text_document(100, 5);
    let mut doc = PdfDocument::from_bytes(&doc_bytes).expect("Open doc");

    // 1. Move page 99 to position 0
    let start = Instant::now();
    let reordered_bytes = doc.move_page(99, 0).expect("Move page 99 to 0");
    let reorder_time = start.elapsed();

    let mut reopened = PdfDocument::from_bytes(&reordered_bytes).expect("Reopen reordered");
    assert_eq!(reopened.page_count().unwrap(), 100);

    // The first page should now contain "Page 100"
    let p0_text = reopened.extract_page_text(0).unwrap();
    assert!(p0_text.plain_text().contains("Page 100"));

    // 2. Extract 10 pages from the 100-page document into a standalone document
    let start = Instant::now();
    let extracted_bytes = doc
        .extract_pages(&[0, 10, 20, 30, 40, 50, 60, 70, 80, 90])
        .expect("Extract 10 pages");
    let extract_time = start.elapsed();

    let mut extracted_doc = PdfDocument::from_bytes(&extracted_bytes).expect("Open extracted doc");
    assert_eq!(extracted_doc.page_count().unwrap(), 10);

    println!(
        "Large-Doc Page Operations: 100-page Move={:.2?}, 10-page Extract={:.2?}",
        reorder_time, extract_time
    );
}

#[test]
fn test_qualification_merge_large_documents() {
    let doc1 = generate_text_document(50, 10);
    let doc2 = generate_vector_document(50, 5);

    let start = Instant::now();
    let merged_bytes = PdfDocument::merge_documents(&[&doc1, &doc2]).expect("Merge 50+50 pages");
    let merge_time = start.elapsed();

    let mut merged_doc = PdfDocument::from_bytes(&merged_bytes).expect("Open merged doc");
    assert_eq!(merged_doc.page_count().unwrap(), 100);

    // Page 0 has text
    let p0_text = merged_doc.extract_page_text(0).unwrap();
    assert!(p0_text.plain_text().contains("Page 1"));

    // Page 50 has vector graphics
    let p50_graphics = merged_doc.enumerate_graphics(50).unwrap();
    assert_eq!(p50_graphics.len(), 5);

    println!("Merged 50p + 50p documents in {:.2?}", merge_time);
}

#[test]
fn test_qualification_20_cycle_memory_retention() {
    let doc_bytes = generate_text_document(50, 10);

    // Run 20 cycles of: open -> edit text -> add vector -> export -> close
    let start_all = Instant::now();
    let mut current_bytes = doc_bytes.clone();

    for cycle in 1..=20 {
        let t0 = Instant::now();
        // 1. Text edit
        let next_bytes = {
            let mut doc = PdfDocument::from_bytes(&current_bytes).expect("Open cycle");
            let page_text = doc.extract_page_text(0).unwrap();
            let span = &page_text.spans[0];
            let edit_text = format!("CYCLE_{cycle}_MUTATION");
            let target = TextEditTarget::from_span(span);
            let plan = doc.replace_text(0, &target, &edit_text).unwrap();
            doc.export_incremental(&plan).unwrap()
        };
        current_bytes = next_bytes;

        // 2. Add vector shape
        let next_bytes2 = {
            let mut doc2 = PdfDocument::from_bytes(&current_bytes).unwrap();
            let add_rect = AddVectorGraphicSpec {
                page_index: 0,
                geometry: VectorGeometry::Rectangle {
                    x: 10.0 * (cycle as f64),
                    y: 10.0 * (cycle as f64),
                    width: 50.0,
                    height: 30.0,
                },
                stroke_color: Some(VectorColor::from_rgb(0.0, 0.5, 1.0)),
                fill_color: None,
                line_width: 1.0,
                is_stroked: true,
                is_filled: false,
            };
            let plan2 = doc2.add_graphic(&add_rect).unwrap();
            doc2.export_incremental(&plan2).unwrap()
        };
        current_bytes = next_bytes2;

        let cycle_time = t0.elapsed();
        if cycle % 5 == 0 {
            println!("Completed Cycle {cycle}/20 in {:.2?}", cycle_time);
        }
    }

    let total_time = start_all.elapsed();
    println!(
        "20 Repeated Open/Edit/Save/Close cycles finished in {:.2?} (avg {:.2?}/cycle)",
        total_time,
        total_time / 20
    );

    // Reopen final result and verify all 20 added vector rectangles are present
    let mut final_doc = PdfDocument::from_bytes(&current_bytes).expect("Open final cycle result");
    let graphics = final_doc.enumerate_graphics(0).unwrap();
    assert_eq!(graphics.len(), 20);
}

#[test]
fn test_qualification_incremental_save_growth_10_cycles() {
    let initial_bytes = generate_text_document(20, 10);
    let initial_size = initial_bytes.len();
    let mut current_bytes = initial_bytes;
    let mut size_history = vec![initial_size];

    for cycle in 1..=10 {
        let next_bytes = {
            let mut doc = PdfDocument::from_bytes(&current_bytes).expect("Open doc for size test");
            let page_text = doc.extract_page_text(0).unwrap();
            let span = &page_text.spans[0];
            let target = TextEditTarget::from_span(span);
            let new_text = format!("EDIT_SAVE_CYCLE_{cycle}");
            let plan = doc.replace_text(0, &target, &new_text).unwrap();
            doc.export_incremental(&plan).unwrap()
        };
        current_bytes = next_bytes;
        size_history.push(current_bytes.len());
    }

    println!("Incremental Save Growth over 10 cycles:");
    for (i, sz) in size_history.iter().enumerate() {
        let delta = if i == 0 {
            0
        } else {
            *sz as isize - size_history[i - 1] as isize
        };
        println!("  Cycle {i:2}: {sz} bytes (delta: +{delta} bytes)");
    }

    let total_growth = current_bytes.len() - initial_size;
    let avg_growth = total_growth / 10;
    println!("Total growth over 10 saves: {total_growth} bytes (avg {avg_growth} bytes/save)");
    assert!(
        avg_growth < 3000,
        "Incremental save growth must remain strictly bounded"
    );
}
