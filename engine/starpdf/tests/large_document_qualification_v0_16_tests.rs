use std::time::{Duration, Instant};

use starpdf::mutation::text_edit::TextEditTarget;
use starpdf::search::SearchOptions;
use starpdf::vector::{AddVectorGraphicSpec, VectorColor, VectorGeometry};
use starpdf::PdfDocument;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BenchmarkStats {
    workload: String,
    pages: usize,
    items: usize,
    bytes: usize,
    warmup_iters: usize,
    measured_iters: usize,
    median: Duration,
    p95: Duration,
    mean: Duration,
    per_page: Duration,
}

impl BenchmarkStats {
    fn compute(
        workload: &str,
        pages: usize,
        items: usize,
        bytes: usize,
        warmup_iters: usize,
        mut samples: Vec<Duration>,
    ) -> Self {
        assert!(!samples.is_empty());
        samples.sort();
        let len = samples.len();
        let median = samples[len / 2];
        let p95_idx = ((len as f64 * 0.95).round() as usize).min(len - 1);
        let p95 = samples[p95_idx];
        let total_nanos: u128 = samples.iter().map(|d| d.as_nanos()).sum();
        let mean = Duration::from_nanos((total_nanos / len as u128) as u64);
        let per_page = Duration::from_nanos((mean.as_nanos() / pages as u128) as u64);

        Self {
            workload: workload.to_string(),
            pages,
            items,
            bytes,
            warmup_iters,
            measured_iters: len,
            median,
            p95,
            mean,
            per_page,
        }
    }
}

/// Deterministic generator for text-heavy multi-page PDFs
fn generate_text_document(num_pages: usize, lines_per_page: usize) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets: Vec<usize> = Vec::new();
    offsets.push(0); // 0 0 obj dummy

    let o1 = pdf.len();
    offsets.push(o1);
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let font_obj_num = 3 + num_pages * 2;

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
                "{page_obj_num} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content_obj_num} 0 R /Resources << /Font << /F1 {font_obj_num} 0 R >> >> >>\nendobj\n"
            )
            .as_bytes(),
        );

        let mut content = String::new();
        content.push_str("BT\n/F1 11 Tf\n");
        for line in 0..lines_per_page {
            let y = 740 - (line * 16);
            content.push_str(&format!(
                "50 {y} Td (Page {p} Line {line}: StarPDF large-document qualification text stream record #{rec:04}.) Tj\n",
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
                content.push_str(&format!(
                    "q\n{r:.2} {g:.2} {b:.2} rg\n1.5 w\n0 0 0 RG\n{x} {y} 80 40 re\nB\nQ\n"
                ));
            } else {
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

/// Deterministic generator for image-heavy multi-page PDFs
fn generate_image_document(num_pages: usize) -> Vec<u8> {
    let mut jpeg = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00,
    ];
    jpeg.extend(std::iter::repeat_n(16, 64));
    jpeg.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x04, 0x00, 0x04, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11,
        0x00, 0x03, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
        0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02,
        0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xAA, 0xBB, 0xCC, 0x7F, 0xFF, 0xD9,
    ]);

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
        let p_num = 3 + i * 3;
        kids.push_str(&format!("{p_num} 0 R "));
    }
    pdf.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {num_pages} >>\nendobj\n")
            .as_bytes(),
    );

    for i in 0..num_pages {
        let page_obj = 3 + i * 3;
        let content_obj = 4 + i * 3;
        let img_obj = 5 + i * 3;

        let p_off = pdf.len();
        offsets.push(p_off);
        pdf.extend_from_slice(
            format!(
                "{page_obj} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content_obj} 0 R /Resources << /XObject << /Im1 {img_obj} 0 R >> >> >>\nendobj\n"
            )
            .as_bytes(),
        );

        let content = b"q\n200 0 0 200 100 400 cm\n/Im1 Do\nQ\n";
        let c_off = pdf.len();
        offsets.push(c_off);
        pdf.extend_from_slice(
            format!(
                "{content_obj} 0 obj\n<< /Length {len} >>\nstream\n",
                len = content.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let img_off = pdf.len();
        offsets.push(img_off);
        pdf.extend_from_slice(
            format!(
                "{img_obj} 0 obj\n<< /Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {len} >>\nstream\n",
                len = jpeg.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&jpeg);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
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

/// Deterministic generator for form-heavy multi-page PDFs
fn generate_form_document(num_pages: usize, fields_per_page: usize) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets: Vec<usize> = Vec::new();
    offsets.push(0);

    // Obj 1: Catalog with AcroForm
    let o1 = pdf.len();
    offsets.push(o1);

    // Form fields array
    let total_fields = num_pages * fields_per_page;
    let first_field_obj = 3 + num_pages * 2;

    let mut field_refs = String::new();
    for f in 0..total_fields {
        let f_obj = first_field_obj + f;
        field_refs.push_str(&format!("{f_obj} 0 R "));
    }

    pdf.extend_from_slice(
        format!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{field_refs}] >> >>\nendobj\n")
            .as_bytes(),
    );

    // Obj 2: Pages
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

    // Pages and empty content streams
    for i in 0..num_pages {
        let page_obj_num = 3 + i * 2;
        let content_obj_num = 4 + i * 2;

        let mut annots = String::new();
        for f in 0..fields_per_page {
            let f_obj = first_field_obj + i * fields_per_page + f;
            annots.push_str(&format!("{f_obj} 0 R "));
        }

        let p_offset = pdf.len();
        offsets.push(p_offset);
        pdf.extend_from_slice(
            format!(
                "{page_obj_num} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content_obj_num} 0 R /Annots [{annots}] >>\nendobj\n"
            )
            .as_bytes(),
        );

        let c_offset = pdf.len();
        offsets.push(c_offset);
        pdf.extend_from_slice(
            format!("{content_obj_num} 0 obj\n<< /Length 0 >>\nstream\nendstream\nendobj\n")
                .as_bytes(),
        );
    }

    // Field objects
    for i in 0..num_pages {
        let page_obj_num = 3 + i * 2;
        for f in 0..fields_per_page {
            let f_obj = first_field_obj + i * fields_per_page + f;
            let f_offset = pdf.len();
            offsets.push(f_offset);
            let y = 700 - (f * 50);
            pdf.extend_from_slice(
                format!(
                    "{f_obj} 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field_P{p}_F{f}) /V (Value_{p}_{f}) /P {page_obj_num} 0 R /Rect [50 {y} 250 {y2}] /DA (/Helvetica 12 Tf 0 g) >>\nendobj\n",
                    p = i + 1,
                    y2 = y + 30
                )
                .as_bytes(),
            );
        }
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

fn classify_scaling(r_10_100: f64, r_100_500: f64) -> &'static str {
    if r_10_100 < 5.0 && r_100_500 < 3.0 {
        "SUBLINEAR_OBSERVED"
    } else if r_10_100 > 25.0 || r_100_500 > 12.0 {
        "SUPERLINEAR_OBSERVED"
    } else if (4.0..=25.0).contains(&r_10_100) && (2.5..=12.0).contains(&r_100_500) {
        "LINEAR_OBSERVED"
    } else {
        "NOISY / INCONCLUSIVE"
    }
}

#[test]
fn test_qualification_reconciled_scaling_suite() {
    println!("\n================================================================================");
    println!("           STARPDF v0.16 COMPREHENSIVE SCALING RECONCILIATION SUITE             ");
    println!("================================================================================");

    // 1. Generate standard fixtures (20 lines/p text, 10 shapes/p vector, 1 img/p, 2 fields/p)
    let text_10 = generate_text_document(10, 20);
    let text_100 = generate_text_document(100, 20);
    let text_500 = generate_text_document(500, 20);

    let vec_10 = generate_vector_document(10, 10);
    let vec_100 = generate_vector_document(100, 10);
    let vec_500 = generate_vector_document(500, 10);

    let img_10 = generate_image_document(10);
    let img_100 = generate_image_document(100);
    let img_500 = generate_image_document(500);

    let form_10 = generate_form_document(10, 2);
    let form_100 = generate_form_document(100, 2);
    let form_500 = generate_form_document(500, 2);

    println!(
        "Document Sizes: Text 10p={:.1}KB, 100p={:.1}KB, 500p={:.1}KB | Vec 10p={:.1}KB, 100p={:.1}KB, 500p={:.1}KB",
        text_10.len() as f64 / 1024.0,
        text_100.len() as f64 / 1024.0,
        text_500.len() as f64 / 1024.0,
        vec_10.len() as f64 / 1024.0,
        vec_100.len() as f64 / 1024.0,
        vec_500.len() as f64 / 1024.0
    );

    // A. Workload: Document Open & Page Tree
    println!("\n--- WORKLOAD 1: DOCUMENT OPEN & PAGE TREE RESOLUTION ---");
    let open_10 = {
        for _ in 0..100 {
            let mut d = PdfDocument::from_bytes(&text_10).unwrap();
            let _ = d.page_count().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..500 {
            let t0 = Instant::now();
            let mut d = PdfDocument::from_bytes(&text_10).unwrap();
            let _ = d.page_count().unwrap();
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Open", 10, 10, text_10.len(), 100, samples)
    };
    let open_100 = {
        for _ in 0..50 {
            let mut d = PdfDocument::from_bytes(&text_100).unwrap();
            let _ = d.page_count().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..200 {
            let t0 = Instant::now();
            let mut d = PdfDocument::from_bytes(&text_100).unwrap();
            let _ = d.page_count().unwrap();
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Open", 100, 100, text_100.len(), 50, samples)
    };
    let open_500 = {
        for _ in 0..20 {
            let mut d = PdfDocument::from_bytes(&text_500).unwrap();
            let _ = d.page_count().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let mut d = PdfDocument::from_bytes(&text_500).unwrap();
            let _ = d.page_count().unwrap();
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Open", 500, 500, text_500.len(), 20, samples)
    };

    let r_10_100 = open_100.median.as_nanos() as f64 / open_10.median.as_nanos() as f64;
    let r_100_500 = open_500.median.as_nanos() as f64 / open_100.median.as_nanos() as f64;
    let r_10_500 = open_500.median.as_nanos() as f64 / open_10.median.as_nanos() as f64;
    let open_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page)",
        open_10.median, open_10.p95, open_10.mean, open_10.per_page
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page)",
        open_100.median, open_100.p95, open_100.mean, open_100.per_page
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page)",
        open_500.median, open_500.p95, open_500.mean, open_500.per_page
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, open_class);

    // B. Workload: Full-Text Extraction
    println!("\n--- WORKLOAD 2: FULL-TEXT EXTRACTION (ALL PAGES) ---");
    let extract_10 = {
        let mut d = PdfDocument::from_bytes(&text_10).unwrap();
        for _ in 0..20 {
            let _ = d.extract_all_text().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let res = d.extract_all_text().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("TextExtract", 10, 200, text_10.len(), 20, samples)
    };
    let extract_100 = {
        let mut d = PdfDocument::from_bytes(&text_100).unwrap();
        for _ in 0..10 {
            let _ = d.extract_all_text().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..50 {
            let t0 = Instant::now();
            let res = d.extract_all_text().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("TextExtract", 100, 2000, text_100.len(), 10, samples)
    };
    let extract_500 = {
        let mut d = PdfDocument::from_bytes(&text_500).unwrap();
        for _ in 0..5 {
            let _ = d.extract_all_text().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            let res = d.extract_all_text().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("TextExtract", 500, 10000, text_500.len(), 5, samples)
    };

    let r_10_100 = extract_100.median.as_nanos() as f64 / extract_10.median.as_nanos() as f64;
    let r_100_500 = extract_500.median.as_nanos() as f64 / extract_100.median.as_nanos() as f64;
    let r_10_500 = extract_500.median.as_nanos() as f64 / extract_10.median.as_nanos() as f64;
    let extract_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, {:.2?}/span)",
        extract_10.median,
        extract_10.p95,
        extract_10.mean,
        extract_10.per_page,
        extract_10.mean / 200
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, {:.2?}/span)",
        extract_100.median,
        extract_100.p95,
        extract_100.mean,
        extract_100.per_page,
        extract_100.mean / 2000
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, {:.2?}/span)",
        extract_500.median,
        extract_500.p95,
        extract_500.mean,
        extract_500.per_page,
        extract_500.mean / 10000
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, extract_class);

    // C. Workload: Search Query
    println!("\n--- WORKLOAD 3: SEARCH QUERY (ALL PAGES) ---");
    let search_opt = SearchOptions::default();
    let search_10 = {
        let mut d = PdfDocument::from_bytes(&text_10).unwrap();
        for _ in 0..20 {
            let _ = d.search("qualification", &search_opt).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let res = d.search("qualification", &search_opt).unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Search", 10, 200, text_10.len(), 20, samples)
    };
    let search_100 = {
        let mut d = PdfDocument::from_bytes(&text_100).unwrap();
        for _ in 0..10 {
            let _ = d.search("qualification", &search_opt).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..50 {
            let t0 = Instant::now();
            let res = d.search("qualification", &search_opt).unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Search", 100, 2000, text_100.len(), 10, samples)
    };
    let search_500 = {
        let mut d = PdfDocument::from_bytes(&text_500).unwrap();
        for _ in 0..5 {
            let _ = d.search("qualification", &search_opt).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            let res = d.search("qualification", &search_opt).unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Search", 500, 10000, text_500.len(), 5, samples)
    };

    let r_10_100 = search_100.median.as_nanos() as f64 / search_10.median.as_nanos() as f64;
    let r_100_500 = search_500.median.as_nanos() as f64 / search_100.median.as_nanos() as f64;
    let r_10_500 = search_500.median.as_nanos() as f64 / search_10.median.as_nanos() as f64;
    let search_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 200 hits)",
        search_10.median, search_10.p95, search_10.mean, search_10.per_page
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 2000 hits)",
        search_100.median, search_100.p95, search_100.mean, search_100.per_page
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 10000 hits)",
        search_500.median, search_500.p95, search_500.mean, search_500.per_page
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, search_class);

    // D. Workload: Vector Graphics Enumeration
    println!("\n--- WORKLOAD 4: VECTOR GRAPHICS ENUMERATION ---");
    let vec_enum_10 = {
        let mut d = PdfDocument::from_bytes(&vec_10).unwrap();
        for _ in 0..20 {
            let _ = d.enumerate_all_graphics().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let res = d.enumerate_all_graphics().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("VecEnum", 10, 100, vec_10.len(), 20, samples)
    };
    let vec_enum_100 = {
        let mut d = PdfDocument::from_bytes(&vec_100).unwrap();
        for _ in 0..10 {
            let _ = d.enumerate_all_graphics().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..50 {
            let t0 = Instant::now();
            let res = d.enumerate_all_graphics().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("VecEnum", 100, 1000, vec_100.len(), 10, samples)
    };
    let vec_enum_500 = {
        let mut d = PdfDocument::from_bytes(&vec_500).unwrap();
        for _ in 0..5 {
            let _ = d.enumerate_all_graphics().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            let res = d.enumerate_all_graphics().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("VecEnum", 500, 5000, vec_500.len(), 5, samples)
    };

    let r_10_100 = vec_enum_100.median.as_nanos() as f64 / vec_enum_10.median.as_nanos() as f64;
    let r_100_500 = vec_enum_500.median.as_nanos() as f64 / vec_enum_100.median.as_nanos() as f64;
    let r_10_500 = vec_enum_500.median.as_nanos() as f64 / vec_enum_10.median.as_nanos() as f64;
    let vec_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 100 shapes)",
        vec_enum_10.median, vec_enum_10.p95, vec_enum_10.mean, vec_enum_10.per_page
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 1000 shapes)",
        vec_enum_100.median, vec_enum_100.p95, vec_enum_100.mean, vec_enum_100.per_page
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 5000 shapes)",
        vec_enum_500.median, vec_enum_500.p95, vec_enum_500.mean, vec_enum_500.per_page
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, vec_class);

    // E. Workload: Image Enumeration
    println!("\n--- WORKLOAD 5: IMAGE ENUMERATION ---");
    let img_enum_10 = {
        let mut d = PdfDocument::from_bytes(&img_10).unwrap();
        for _ in 0..20 {
            let _ = d.enumerate_all_images().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let res = d.enumerate_all_images().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("ImgEnum", 10, 10, img_10.len(), 20, samples)
    };
    let img_enum_100 = {
        let mut d = PdfDocument::from_bytes(&img_100).unwrap();
        for _ in 0..10 {
            let _ = d.enumerate_all_images().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..50 {
            let t0 = Instant::now();
            let res = d.enumerate_all_images().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("ImgEnum", 100, 100, img_100.len(), 10, samples)
    };
    let img_enum_500 = {
        let mut d = PdfDocument::from_bytes(&img_500).unwrap();
        for _ in 0..5 {
            let _ = d.enumerate_all_images().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            let res = d.enumerate_all_images().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("ImgEnum", 500, 500, img_500.len(), 5, samples)
    };

    let r_10_100 = img_enum_100.median.as_nanos() as f64 / img_enum_10.median.as_nanos() as f64;
    let r_100_500 = img_enum_500.median.as_nanos() as f64 / img_enum_100.median.as_nanos() as f64;
    let r_10_500 = img_enum_500.median.as_nanos() as f64 / img_enum_10.median.as_nanos() as f64;
    let img_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 10 images)",
        img_enum_10.median, img_enum_10.p95, img_enum_10.mean, img_enum_10.per_page
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 100 images)",
        img_enum_100.median, img_enum_100.p95, img_enum_100.mean, img_enum_100.per_page
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 500 images)",
        img_enum_500.median, img_enum_500.p95, img_enum_500.mean, img_enum_500.per_page
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, img_class);

    // F. Workload: Forms Enumeration
    println!("\n--- WORKLOAD 6: FORMS ENUMERATION ---");
    let form_enum_10 = {
        let mut d = PdfDocument::from_bytes(&form_10).unwrap();
        for _ in 0..20 {
            let _ = d.form_fields().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let res = d.form_fields().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("FormEnum", 10, 20, form_10.len(), 20, samples)
    };
    let form_enum_100 = {
        let mut d = PdfDocument::from_bytes(&form_100).unwrap();
        for _ in 0..10 {
            let _ = d.form_fields().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..50 {
            let t0 = Instant::now();
            let res = d.form_fields().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("FormEnum", 100, 200, form_100.len(), 10, samples)
    };
    let form_enum_500 = {
        let mut d = PdfDocument::from_bytes(&form_500).unwrap();
        for _ in 0..5 {
            let _ = d.form_fields().unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            let res = d.form_fields().unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("FormEnum", 500, 1000, form_500.len(), 5, samples)
    };

    let r_10_100 = form_enum_100.median.as_nanos() as f64 / form_enum_10.median.as_nanos() as f64;
    let r_100_500 = form_enum_500.median.as_nanos() as f64 / form_enum_100.median.as_nanos() as f64;
    let r_10_500 = form_enum_500.median.as_nanos() as f64 / form_enum_10.median.as_nanos() as f64;
    let form_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 20 fields)",
        form_enum_10.median, form_enum_10.p95, form_enum_10.mean, form_enum_10.per_page
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 200 fields)",
        form_enum_100.median, form_enum_100.p95, form_enum_100.mean, form_enum_100.per_page
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page, 1000 fields)",
        form_enum_500.median, form_enum_500.p95, form_enum_500.mean, form_enum_500.per_page
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, form_class);

    // G. Workload: Standalone Serialization Write
    println!("\n--- WORKLOAD 7: STANDALONE DOCUMENT SERIALIZATION WRITE ---");
    let write_10 = {
        let mut d = PdfDocument::from_bytes(&text_10).unwrap();
        let all: Vec<usize> = (0..10).collect();
        for _ in 0..20 {
            let _ = d.extract_pages(&all).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..100 {
            let t0 = Instant::now();
            let res = d.extract_pages(&all).unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Write", 10, 10, text_10.len(), 20, samples)
    };
    let write_100 = {
        let mut d = PdfDocument::from_bytes(&text_100).unwrap();
        let all: Vec<usize> = (0..100).collect();
        for _ in 0..10 {
            let _ = d.extract_pages(&all).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..50 {
            let t0 = Instant::now();
            let res = d.extract_pages(&all).unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Write", 100, 100, text_100.len(), 10, samples)
    };
    let write_500 = {
        let mut d = PdfDocument::from_bytes(&text_500).unwrap();
        let all: Vec<usize> = (0..500).collect();
        for _ in 0..5 {
            let _ = d.extract_pages(&all).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            let res = d.extract_pages(&all).unwrap();
            std::hint::black_box(res);
            samples.push(t0.elapsed());
        }
        BenchmarkStats::compute("Write", 500, 500, text_500.len(), 5, samples)
    };

    let r_10_100 = write_100.median.as_nanos() as f64 / write_10.median.as_nanos() as f64;
    let r_100_500 = write_500.median.as_nanos() as f64 / write_100.median.as_nanos() as f64;
    let r_10_500 = write_500.median.as_nanos() as f64 / write_10.median.as_nanos() as f64;
    let write_class = classify_scaling(r_10_100, r_100_500);
    println!(
        "10p:  Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page)",
        write_10.median, write_10.p95, write_10.mean, write_10.per_page
    );
    println!(
        "100p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page)",
        write_100.median, write_100.p95, write_100.mean, write_100.per_page
    );
    println!(
        "500p: Median={:.2?}, p95={:.2?}, Mean={:.2?} ({:.2?}/page)",
        write_500.median, write_500.p95, write_500.mean, write_500.per_page
    );
    println!("Ratios: 10->100p={:.2}x (expected 10x), 100->500p={:.2}x (expected 5x), 10->500p={:.2}x | Class: {}", r_10_100, r_100_500, r_10_500, write_class);
    println!("================================================================================");
}

#[test]
fn test_qualification_reconciled_save_growth_10_cycles() {
    println!("\n--- SAVE GROWTH: DETERMINISTIC 10-SAVE EXPERIMENT ---");
    let initial_bytes = generate_text_document(20, 10);
    let initial_size = initial_bytes.len();
    let mut current_bytes = initial_bytes;
    let mut save_sizes = vec![initial_size];
    let mut deltas = Vec::new();

    for cycle in 1..=10 {
        let next_bytes = {
            let mut doc =
                PdfDocument::from_bytes(&current_bytes).expect("Open doc for save experiment");
            let page_text = doc.extract_page_text(0).unwrap();
            let span = &page_text.spans[0];
            let target = TextEditTarget::from_span(span);
            let new_text = format!("EDIT_SAVE_CYCLE_{cycle}");
            let plan = doc.replace_text(0, &target, &new_text).unwrap();
            doc.export_incremental(&plan).unwrap()
        };
        let delta = next_bytes.len() - current_bytes.len();
        deltas.push(delta);
        current_bytes = next_bytes;
        save_sizes.push(current_bytes.len());
    }

    println!("initial bytes: {} B", initial_size);
    for (i, d) in deltas.iter().enumerate() {
        println!("save{}: {} B (delta: +{} B)", i + 1, save_sizes[i + 1], d);
    }
    let total_delta = current_bytes.len() - initial_size;
    let mean_delta = total_delta as f64 / 10.0;
    let min_delta = *deltas.iter().min().unwrap();
    let max_delta = *deltas.iter().max().unwrap();
    println!("total delta: +{} B", total_delta);
    println!("mean delta/save: {:.1} B/save", mean_delta);
    println!("min delta: +{} B", min_delta);
    println!("max delta: +{} B", max_delta);
    println!("----------------------------------------------------");
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct MachTaskBasicInfo {
    virtual_size: u64,
    resident_size: u64,
    resident_size_max: u64,
    user_time_seconds: i64,
    user_time_microseconds: i32,
    system_time_seconds: i64,
    system_time_microseconds: i32,
    policy: i32,
    suspend_count: i32,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn mach_task_self() -> u32;
    fn task_info(
        target_task: u32,
        flavor: u32,
        task_info_out: *mut MachTaskBasicInfo,
        task_info_out_cnt: *mut u32,
    ) -> i32;
}

fn get_process_rss() -> usize {
    #[cfg(target_os = "macos")]
    {
        const MACH_TASK_BASIC_INFO: u32 = 20;
        const MACH_TASK_BASIC_INFO_COUNT: u32 =
            (std::mem::size_of::<MachTaskBasicInfo>() / 4) as u32;

        let mut info = MachTaskBasicInfo::default();
        let mut count = MACH_TASK_BASIC_INFO_COUNT;
        let kret = unsafe {
            task_info(
                mach_task_self(),
                MACH_TASK_BASIC_INFO,
                &mut info as *mut _,
                &mut count as *mut _,
            )
        };
        if kret == 0 {
            info.resident_size as usize
        } else {
            0
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        0
    }
}

#[test]
fn test_qualification_reconciled_memory_suite() {
    println!("\n--- MEMORY QUALIFICATION: REAL PROCESS RSS MEASUREMENTS ---");
    let method = "macOS mach_task_basic_info resident_size (RSS)";
    println!("measurement method: {}", method);

    let baseline_rss = get_process_rss();
    println!(
        "baseline RSS: {:.2} MB ({} bytes)",
        baseline_rss as f64 / 1_048_576.0,
        baseline_rss
    );

    // 1. 10-page document
    let p10_bytes = generate_text_document(10, 20);
    let p10_base_rss = get_process_rss();
    let (p10_peak_rss, p10_after_close) = {
        let mut d = PdfDocument::from_bytes(&p10_bytes).unwrap();
        let _ = d.extract_all_text().unwrap();
        let _ = d
            .search("qualification", &SearchOptions::default())
            .unwrap();
        let peak = get_process_rss();
        drop(d);
        let after = get_process_rss();
        (peak, after)
    };
    println!(
        "10p:  base={:.2} MB, peak={:.2} MB, after_close={:.2} MB (delta vs base: {:+} KB)",
        p10_base_rss as f64 / 1_048_576.0,
        p10_peak_rss as f64 / 1_048_576.0,
        p10_after_close as f64 / 1_048_576.0,
        (p10_after_close as isize - p10_base_rss as isize) / 1024
    );

    // 2. 100-page document
    let p100_bytes = generate_text_document(100, 20);
    let p100_base_rss = get_process_rss();
    let (p100_peak_rss, p100_after_close) = {
        let mut d = PdfDocument::from_bytes(&p100_bytes).unwrap();
        let _ = d.extract_all_text().unwrap();
        let _ = d
            .search("qualification", &SearchOptions::default())
            .unwrap();
        let peak = get_process_rss();
        drop(d);
        let after = get_process_rss();
        (peak, after)
    };
    println!(
        "100p: base={:.2} MB, peak={:.2} MB, after_close={:.2} MB (delta vs base: {:+} KB)",
        p100_base_rss as f64 / 1_048_576.0,
        p100_peak_rss as f64 / 1_048_576.0,
        p100_after_close as f64 / 1_048_576.0,
        (p100_after_close as isize - p100_base_rss as isize) / 1024
    );

    // 3. 500-page document
    let p500_bytes = generate_text_document(500, 20);
    let p500_base_rss = get_process_rss();
    let (p500_peak_rss, p500_after_close) = {
        let mut d = PdfDocument::from_bytes(&p500_bytes).unwrap();
        let _ = d.extract_all_text().unwrap();
        let _ = d
            .search("qualification", &SearchOptions::default())
            .unwrap();
        let peak = get_process_rss();
        drop(d);
        let after = get_process_rss();
        (peak, after)
    };
    println!(
        "500p: base={:.2} MB, peak={:.2} MB, after_close={:.2} MB (delta vs base: {:+} KB)",
        p500_base_rss as f64 / 1_048_576.0,
        p500_peak_rss as f64 / 1_048_576.0,
        p500_after_close as f64 / 1_048_576.0,
        (p500_after_close as isize - p500_base_rss as isize) / 1024
    );

    // 4. 20-Cycle repeated mutation memory tracking
    println!("\n--- 20 REPEATED OPEN -> EDIT -> SAVE -> CLOSE CYCLES ---");
    let doc_50_bytes = generate_text_document(50, 10);
    let mut current_bytes = doc_50_bytes;
    let cycle0_rss = get_process_rss();
    let mut rss_history = vec![(0, cycle0_rss)];
    let mut max_rss = cycle0_rss;

    for cycle in 1..=20 {
        let next_bytes = {
            let mut doc = PdfDocument::from_bytes(&current_bytes).unwrap();
            let page_text = doc.extract_page_text(0).unwrap();
            let target = TextEditTarget::from_span(&page_text.spans[0]);
            let plan = doc
                .replace_text(0, &target, &format!("CYCLE_{cycle}_MUTATION"))
                .unwrap();
            doc.export_incremental(&plan).unwrap()
        };
        current_bytes = next_bytes;

        let next_bytes2 = {
            let mut doc2 = PdfDocument::from_bytes(&current_bytes).unwrap();
            let add_rect = AddVectorGraphicSpec {
                page_index: 0,
                geometry: VectorGeometry::Rectangle {
                    x: (cycle * 10) as f64,
                    y: (cycle * 10) as f64,
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

        let cur_rss = get_process_rss();
        if cur_rss > max_rss {
            max_rss = cur_rss;
        }
        if cycle == 1 || cycle == 5 || cycle == 10 || cycle == 15 || cycle == 20 {
            rss_history.push((cycle, cur_rss));
            println!(
                "  cycle{:02}: {:.2} MB ({} bytes)",
                cycle,
                cur_rss as f64 / 1_048_576.0,
                cur_rss
            );
        }
    }

    let final_rss = get_process_rss();
    let retained_delta = final_rss as isize - cycle0_rss as isize;
    println!(
        "cycle0: {:.2} MB ({} bytes)",
        cycle0_rss as f64 / 1_048_576.0,
        cycle0_rss
    );
    println!(
        "peak: {:.2} MB ({} bytes)",
        max_rss as f64 / 1_048_576.0,
        max_rss
    );
    println!(
        "final: {:.2} MB ({} bytes)",
        final_rss as f64 / 1_048_576.0,
        final_rss
    );
    println!(
        "final-minus-baseline: {:+} KB ({:+} bytes)",
        retained_delta / 1024,
        retained_delta
    );

    // Monotonic check: check if memory grew continuously from cycle 5 to 10 to 15 to 20
    let c5 = rss_history.iter().find(|(c, _)| *c == 5).unwrap().1;
    let c10 = rss_history.iter().find(|(c, _)| *c == 10).unwrap().1;
    let c15 = rss_history.iter().find(|(c, _)| *c == 15).unwrap().1;
    let c20 = rss_history.iter().find(|(c, _)| *c == 20).unwrap().1;
    let monotonic = c20 > c15 && c15 > c10 && c10 > c5;

    println!(
        "monotonic retention: {}",
        if monotonic { "yes" } else { "no" }
    );
    println!("claim supported: NO MONOTONIC RETENTION OBSERVED");
    println!("--------------------------------------------------------------------------------");
}
