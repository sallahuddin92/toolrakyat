#![allow(clippy::all, clippy::pedantic, clippy::let_unit_value, unused_imports)]
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::collections::BTreeMap;
use std::time::Instant;

use starpdf::annotation::{AnnotationGenerator, AnnotationParser, AnnotationSpec};
use starpdf::appearance::checkbox::CheckboxAppearance;
use starpdf::appearance::choice::ChoiceAppearance;
use starpdf::appearance::da_parser::DefaultAppearance;
use starpdf::appearance::text_field::{TextFieldAppearance, TextLayoutOptions};
use starpdf::content::ContentParser;
use starpdf::document::{ObjectStreamReader, PdfDocument};
use starpdf::filter::{DecompressLimits, FlateDecoder};
use starpdf::font::appearance::AppearanceFontResolver;
use starpdf::font::sfnt::{HeadTable, HheaTable, HmtxTable, MaxpTable, SfntCmapTable, SfntFont};
use starpdf::font::subset::TrueTypeSubsetter;
use starpdf::font::{Font, PageResources, UnicodeCMap};
use starpdf::forms::AcroFormParser;
use starpdf::mutation::{MutationEngine, PdfChange};
use starpdf::search::{DocumentSearchIndex, SearchOptions};
use starpdf::syntax::object::{ObjectRef, PdfObject, StreamObject};
use starpdf::syntax::{Lexer, Parser};
use starpdf::text::TextExtractor;
use starpdf::writer::{IncrementalWriter, MinimalWriter};
use starpdf::xref::table::XrefTable;
use starpdf::xref::XrefStreamParser;

fn main() {
    println!("================================================================");
    println!("          StarPDF Engine v0.8 Micro-Benchmark Suite             ");
    println!("================================================================");

    let sample_pdf = MinimalWriter::create_minimal_pdf("StarPDF Performance Benchmark Document")
        .expect("Failed to create fixture");

    // 1. Lexer Throughput
    {
        for _ in 0..500 {
            let mut lexer = Lexer::from_bytes(&sample_pdf);
            while let Ok(Some(_)) = lexer.next_token() {}
        }

        let iterations = 10_000;
        let start = Instant::now();
        let mut token_count = 0;
        for _ in 0..iterations {
            let mut lexer = Lexer::from_bytes(&sample_pdf);
            while let Ok(Some(_)) = lexer.next_token() {
                token_count += 1;
            }
        }
        let elapsed = start.elapsed();
        let total_bytes = sample_pdf.len() * iterations;
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
        println!(
            "1.  Lexer Throughput:        {:>8.2} MB/s  ({} tokens in {:.2?})",
            mb_per_sec, token_count, elapsed
        );
    }

    // 2. Object Parser Throughput
    {
        let dict_data = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents [6 0 R 7 0 R] >>";
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut parser = Parser::from_bytes(dict_data);
            let _ = parser.parse_object().unwrap();
        }
        let elapsed = start.elapsed();
        let total_bytes = dict_data.len() * iterations;
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
        println!(
            "2.  Object Parser:           {:>8.2} MB/s  ({} objects parsed in {:.2?})",
            mb_per_sec, iterations, elapsed
        );
    }

    // 3. FlateDecode Throughput
    {
        let mut large_stream = Vec::new();
        for i in 0..500 {
            let line = format!(
                "BT /F1 12 Tf 50 {y} Td (Invoice #{i:06} Customer {i:04} Item description and pricing line) Tj ET\n",
                y = 800 - (i % 50) * 15
            );
            large_stream.extend_from_slice(line.as_bytes());
        }
        let compressed = compress_to_vec_zlib(&large_stream, 6);
        let limits = DecompressLimits::default();

        let iterations = 1_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = FlateDecoder::decode(&compressed, &limits).unwrap();
        }
        let elapsed = start.elapsed();
        let total_bytes = large_stream.len() * iterations;
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
        println!(
            "3.  FlateDecode Throughput:  {:>8.2} MB/s  ({:.2?} for {} decompressions)",
            mb_per_sec, elapsed, iterations
        );
    }

    // 4. SFNT Table & Cmap Parser
    {
        let mut cmap_data = Vec::new();
        cmap_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x03, 0x00, 0x01]);
        cmap_data.extend_from_slice(&12u32.to_be_bytes());
        let mut subtable = Vec::new();
        subtable.extend_from_slice(&4u16.to_be_bytes());
        subtable.extend_from_slice(&32u16.to_be_bytes());
        subtable.extend_from_slice(&0u16.to_be_bytes());
        subtable.extend_from_slice(&4u16.to_be_bytes());
        subtable.extend_from_slice(&[0x00, 0x04, 0x00, 0x01, 0x00, 0x00]);
        subtable.extend_from_slice(&65u16.to_be_bytes());
        subtable.extend_from_slice(&0xFFFFu16.to_be_bytes());
        subtable.extend_from_slice(&0u16.to_be_bytes());
        subtable.extend_from_slice(&65u16.to_be_bytes());
        subtable.extend_from_slice(&0xFFFFu16.to_be_bytes());
        subtable.extend_from_slice(&(-64i16).to_be_bytes());
        subtable.extend_from_slice(&1i16.to_be_bytes());
        subtable.extend_from_slice(&0u16.to_be_bytes());
        subtable.extend_from_slice(&0u16.to_be_bytes());
        cmap_data.extend_from_slice(&subtable);

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = SfntCmapTable::parse(&cmap_data).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "4.  SFNT Cmap Parsing:       {:>8} ns/op  ({:.2?} for {} parses)",
            ns_per_op, elapsed, iterations
        );
    }

    // 5. Text Extraction Throughput
    {
        let content_stream = b"
BT
/F1 12 Tf
50 700 Td
[(StarPDF) 120 (Text) 120 (Extraction) 120 (Pipeline)] TJ
T*
(Second Line of Extracted Body Text) Tj
ET
";
        let mut resources = PageResources {
            fonts: BTreeMap::new(),
        };
        resources
            .fonts
            .insert("/F1".into(), Font::standard_fallback("/F1"));

        let iterations = 10_000;
        let start = Instant::now();
        let mut span_count = 0;
        for _ in 0..iterations {
            let page_text =
                TextExtractor::extract_from_content(0, content_stream, &resources).unwrap();
            span_count += page_text.spans.len();
        }
        let elapsed = start.elapsed();
        let total_bytes = content_stream.len() * iterations;
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
        println!(
            "5.  Text Extractor:          {:>8.2} MB/s  ({} spans extracted in {:.2?})",
            mb_per_sec, span_count, elapsed
        );
    }

    // 6. Text Search Index & Phrase Matching
    {
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let search_index = doc.build_search_index().unwrap();
        let options = SearchOptions::default();

        let iterations = 20_000;
        let start = Instant::now();
        let mut hit_count = 0;
        for _ in 0..iterations {
            let hits = search_index.search("Benchmark", &options);
            hit_count += hits.len();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "6.  Search Index Query:      {:>8} ns/op  ({:.2?} for {} queries, {} hits)",
            ns_per_op, elapsed, iterations, hit_count
        );
    }

    // 7. XRef Stream Parser Throughput
    {
        let mut dict = BTreeMap::new();
        dict.insert("Type".into(), PdfObject::Name("XRef".into()));
        dict.insert("Size".into(), PdfObject::Integer(100));
        dict.insert(
            "W".into(),
            PdfObject::Array(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(2),
                PdfObject::Integer(1),
            ]),
        );

        let mut raw_entries = Vec::new();
        for i in 0..100 {
            raw_entries.extend_from_slice(&[1, (i >> 8) as u8, (i & 0xFF) as u8, 0]);
        }
        let stream = StreamObject {
            dict,
            data: raw_entries,
            stream_offset: 0,
            stream_length: 400,
        };

        let limits = DecompressLimits::default();
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut table = XrefTable::new();
            let _ = XrefStreamParser::parse_into_table(&stream, &mut table, &limits).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "7.  XRef Stream Parsing:     {:>8} ns/op  ({:.2?} for {} 100-entry streams)",
            ns_per_op, elapsed, iterations
        );
    }

    // 8. Compressed Object Resolution (ObjStm)
    {
        let mut dict = BTreeMap::new();
        dict.insert("Type".into(), PdfObject::Name("ObjStm".into()));
        dict.insert("N".into(), PdfObject::Integer(2));
        let header = b"10 0 11 15 ";
        let first = header.len();
        dict.insert("First".into(), PdfObject::Integer(first as i64));
        let body = b"<< /A 1 >>     [ 1 2 3 ] ";
        let mut raw = Vec::new();
        raw.extend_from_slice(header);
        raw.extend_from_slice(body);

        let stream = StreamObject {
            dict,
            data: raw,
            stream_offset: 0,
            stream_length: first + body.len(),
        };

        let limits = DecompressLimits::default();
        let decoded = ObjectStreamReader::decode_stream(&stream, &limits).unwrap();

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = ObjectStreamReader::extract_object(&decoded, 0).unwrap();
            let _ = ObjectStreamReader::extract_object(&decoded, 1).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / (iterations * 2) as u128;
        println!(
            "8.  ObjStm Object Extraction:{:>8} ns/op  ({:.2?} for {} extractions)",
            ns_per_op,
            elapsed,
            iterations * 2
        );
    }

    // 9. Document Open & XRef Resolution
    {
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = PdfDocument::from_bytes(&sample_pdf).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "9.  Document Open & XRef:    {:>8} ns/op  ({:.2?} for {} opens)",
            ns_per_op, elapsed, iterations
        );
    }

    // 10. Page Tree Traversal
    {
        let iterations = 10_000;
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = doc.page_count().unwrap();
            let _ = doc.page_dict(0).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "10. Page Tree Resolution:    {:>8} ns/op  ({:.2?} for {} resolutions)",
            ns_per_op, elapsed, iterations
        );
    }

    // 11. Incremental Writer Serialization
    {
        let modified = BTreeMap::from([(
            ObjectRef::new(3, 0),
            PdfObject::Dictionary(BTreeMap::from([(
                "Type".to_string(),
                PdfObject::Name("Page".to_string()),
            )])),
        )]);
        let trailer = BTreeMap::from([("Size".to_string(), PdfObject::Integer(5))]);

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = IncrementalWriter::write_update(&sample_pdf, &modified, 300, &trailer).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "11. Incremental Serialization:{:>7} ns/op  ({:.2?} for {} updates)",
            ns_per_op, elapsed, iterations
        );
    }

    // 12. Mutation Plan Evaluation
    {
        let dummy_bytes = b"%PDF-1.7\n";
        let source = starpdf::io::source::ByteSource::new(dummy_bytes);
        let field_ref = ObjectRef::new(20, 0);
        let field_dict = BTreeMap::from([
            ("FT".to_string(), PdfObject::Name("Tx".to_string())),
            ("T".to_string(), PdfObject::String(b"field".to_vec())),
            ("V".to_string(), PdfObject::String(b"initial".to_vec())),
        ]);
        let objects = BTreeMap::from([(field_ref, PdfObject::Dictionary(field_dict))]);
        let mut xref = starpdf::xref::table::XrefTable::new();
        xref.insert_in_use(20, 10, 0);

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut store = starpdf::document::object_store::ObjectStore::new(source, xref.clone());
            for (r, obj) in &objects {
                store.insert_cached(*r, obj.clone());
            }
            let mut engine = MutationEngine::new(&mut store, &[]);
            let _ = engine
                .prepare_plan(&[PdfChange::SetTextField {
                    field_ref,
                    value: "mutated_value".to_string(),
                }])
                .unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "12. Mutation Plan Eval:      {:>8} ns/op  ({:.2?} for {} plans)",
            ns_per_op, elapsed, iterations
        );
    }

    // 13. Text Appearance Generation
    {
        let rect = [50.0, 50.0, 250.0, 80.0];
        let da = DefaultAppearance::default();
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = TextFieldAppearance::generate_stream(
                rect,
                "Sample Appearance Text Value",
                &da,
                0,
                false,
            )
            .unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "13. Text Appearance Gen:     {:>8} ns/op  ({:.2?} for {} streams)",
            ns_per_op, elapsed, iterations
        );
    }

    // 14. Checkbox Appearance Generation
    {
        let rect = [10.0, 10.0, 30.0, 30.0];
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = CheckboxAppearance::generate_on_stream(rect).unwrap();
            let _ = CheckboxAppearance::generate_off_stream(rect).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / (iterations * 2) as u128;
        println!(
            "14. Checkbox Appearance Gen: {:>8} ns/op  ({:.2?} for {} on/off streams)",
            ns_per_op,
            elapsed,
            iterations * 2
        );
    }

    // 15. Annotation Object Generation
    {
        let spec = AnnotationSpec::Square {
            rect: [100.0, 100.0, 250.0, 200.0],
            stroke_color: Some(vec![1.0, 0.0, 0.0]),
            fill_color: Some(vec![0.9, 0.9, 0.9]),
            border_width: Some(2.0),
        };
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = AnnotationGenerator::generate_annotation_objects(&spec).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "15. Annotation Gen:          {:>8} ns/op  ({:.2?} for {} objects)",
            ns_per_op, elapsed, iterations
        );
    }

    // 16. Annotation Mutation Transaction Planning
    {
        let change = PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [100.0, 100.0, 250.0, 200.0],
                stroke_color: Some(vec![1.0, 0.0, 0.0]),
                fill_color: None,
                border_width: Some(2.0),
            },
        };
        let iterations = 10_000;
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = doc.apply_mutation(std::slice::from_ref(&change)).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "16. Annotation Mutation:     {:>8} ns/op  ({:.2?} for {} plans)",
            ns_per_op, elapsed, iterations
        );
    }

    // 17. Incremental Export + Reopen
    {
        let change = PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::FreeText {
                rect: [100.0, 100.0, 250.0, 140.0],
                text: "Benchmark note".to_string(),
                font_size: Some(12.0),
                color: Some(vec![0.0]),
            },
        };
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let plan = doc.apply_mutation(&[change]).unwrap();
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let output = doc.export_incremental(&plan).unwrap();
            let _ = PdfDocument::from_bytes(&output).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "17. Export + Reopen:         {:>8} ns/op  ({:.2?} for {} roundtrips)",
            ns_per_op, elapsed, iterations
        );
    }

    // 18. Embedded/Document Font Resolution
    {
        let fixture = include_bytes!("../../../test-assets/smartpdf-form.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let page_refs = doc.page_refs().unwrap();
        let form = doc.acroform().unwrap().unwrap();
        let field_ref = form.fields[0].object_ref;
        let da = DefaultAppearance::parse(
            form.fields[0]
                .default_appearance
                .as_deref()
                .unwrap_or("/Helv 12 Tf 0 g"),
        )
        .unwrap();
        let field = doc.store_mut().resolve(field_ref).unwrap().clone();
        let field = field.as_dict().unwrap().clone();
        let iterations = 5_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = AppearanceFontResolver::resolve(
                doc.store_mut(),
                &field,
                &page_refs,
                &da.font_name,
                "Benchmark",
            )
            .unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "18. Font Resolution:         {:>8} ns/op  ({:.2?} for {} resolutions)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 19. Glyph Coverage Lookup
    {
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let font = AppearanceFontResolver::resolve(
            doc.store_mut(),
            &BTreeMap::new(),
            &[],
            "Helv",
            "Coverage",
        )
        .unwrap();
        let iterations = 100_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = font.verify_text("Coverage").unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "19. Glyph Coverage:          {:>8} ns/op  ({:.2?} for {} lookups)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 20. TrueType Font Subsetting
    {
        let font = benchmark_true_type();
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = TrueTypeSubsetter::subset(&font, &[1]).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "20. TrueType Subset:         {:>8} ns/op  ({:.2?} for {} subsets)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 21. Comb Appearance
    {
        let da = DefaultAppearance::default();
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = TextFieldAppearance::generate_stream_with_options(
                [0.0, 0.0, 240.0, 32.0],
                "AB1234",
                &da,
                0,
                TextLayoutOptions {
                    multiline: false,
                    comb_max_len: Some(8),
                },
            )
            .unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "21. Comb Appearance:         {:>8} ns/op  ({:.2?} for {} streams)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 22. Multiline Layout
    {
        let da = DefaultAppearance::default();
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = TextFieldAppearance::generate_stream_with_options(
                [0.0, 0.0, 180.0, 100.0],
                "A bounded paragraph that wraps across multiple lines.\nExplicit line.",
                &da,
                0,
                TextLayoutOptions {
                    multiline: true,
                    comb_max_len: None,
                },
            )
            .unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "22. Multiline Layout:        {:>8} ns/op  ({:.2?} for {} streams)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 23. List-Box Appearance
    {
        let da = DefaultAppearance::default();
        let options: Vec<String> = (0..20).map(|index| format!("Option {index}")).collect();
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = ChoiceAppearance::generate_list_stream(
                [0.0, 0.0, 180.0, 100.0],
                &options,
                &[1, 3],
                0,
                &da,
            )
            .unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "23. List-Box Appearance:     {:>8} ns/op  ({:.2?} for {} streams)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 24. Annotation Appearance Regeneration
    {
        let spec = AnnotationSpec::Square {
            rect: [10.0, 10.0, 120.0, 80.0],
            stroke_color: Some(vec![1.0, 0.0, 0.0]),
            fill_color: Some(vec![0.9, 0.9, 0.9]),
            border_width: Some(2.0),
        };
        let (dictionary, _) = AnnotationGenerator::generate_annotation_objects(&spec).unwrap();
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = AnnotationGenerator::regenerate_from_dictionary(&dictionary).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "24. Annotation AP Regen:     {:>8} ns/op  ({:.2?} for {} streams)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 25. Incremental Export With Font Resources + Reopen
    {
        let fixture = include_bytes!("../../../test-assets/smartpdf-form.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let field_ref = doc.acroform().unwrap().unwrap().fields[0].object_ref;
        let plan = doc
            .apply_mutation(&[PdfChange::SetTextField {
                field_ref,
                value: "Embedded resource benchmark".to_string(),
            }])
            .unwrap();
        let iterations = 1_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let output = doc.export_incremental(&plan).unwrap();
            let _ = PdfDocument::from_bytes(&output).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "25. Resource Export+Reopen:  {:>8} ns/op  ({:.2?} for {} roundtrips)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    println!("================================================================");
}

fn benchmark_true_type() -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&3u16.to_be_bytes());
    let mut loca = Vec::new();
    for offset in [0u32, 12, 24, 36] {
        loca.extend_from_slice(&offset.to_be_bytes());
    }
    let glyf = vec![0u8; 36];
    let tables = vec![
        (*b"glyf", glyf),
        (*b"head", head),
        (*b"loca", loca),
        (*b"maxp", maxp),
    ];
    let mut output = vec![0u8; 12 + tables.len() * 16];
    output[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    output[4..6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
    for (index, (tag, bytes)) in tables.iter().enumerate() {
        while output.len() % 4 != 0 {
            output.push(0);
        }
        let offset = output.len();
        let record = 12 + index * 16;
        output[record..record + 4].copy_from_slice(tag);
        output[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        output[record + 12..record + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
        output.extend_from_slice(bytes);
    }
    output
}
