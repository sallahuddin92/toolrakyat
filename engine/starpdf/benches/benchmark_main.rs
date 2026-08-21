#![allow(clippy::all, clippy::pedantic, clippy::let_unit_value, unused_imports)]
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::collections::BTreeMap;
use std::time::Instant;

use starpdf::annotation::{AnnotationGenerator, AnnotationParser, AnnotationSpec};
use starpdf::appearance::checkbox::CheckboxAppearance;
use starpdf::appearance::choice::ChoiceAppearance;
use starpdf::appearance::da_parser::DefaultAppearance;
use starpdf::appearance::rotation::WidgetRotation;
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
use starpdf::page_ops::PageSource;
use starpdf::search::{DocumentSearchIndex, SearchOptions};
use starpdf::syntax::object::{ObjectRef, PdfObject, StreamObject};
use starpdf::syntax::{Lexer, Parser};
use starpdf::text::TextExtractor;
use starpdf::writer::{IncrementalWriter, MinimalWriter};
use starpdf::xref::table::XrefTable;
use starpdf::xref::XrefStreamParser;

fn main() {
    println!("================================================================");
    println!("         StarPDF Engine v0.12B Micro-Benchmark Suite            ");
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
        let field_ref = ObjectRef::new(20, 0);
        let field_dict = BTreeMap::from([
            ("FT".to_string(), PdfObject::Name("Tx".to_string())),
            ("T".to_string(), PdfObject::String(b"field".to_vec())),
            ("V".to_string(), PdfObject::String(b"initial".to_vec())),
        ]);
        let mut document = PdfDocument::from_bytes(&sample_pdf).unwrap();
        document
            .store_mut()
            .insert_cached(field_ref, PdfObject::Dictionary(field_dict));

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = document
                .apply_mutation(&[PdfChange::SetTextField {
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

    // 26. Rotated Widget Matrix Construction
    {
        let iterations = 100_000;
        let mut stream = StreamObject {
            dict: BTreeMap::new(),
            data: Vec::new(),
            stream_offset: 0,
            stream_length: 0,
        };
        let start = Instant::now();
        for index in 0..iterations {
            let rotation = match index % 4 {
                0 => WidgetRotation::Degrees0,
                1 => WidgetRotation::Degrees90,
                2 => WidgetRotation::Degrees180,
                _ => WidgetRotation::Degrees270,
            };
            rotation
                .apply_to_stream([10.0, 20.0, 210.0, 50.0], &mut stream)
                .unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "26. Rotated Widget Matrix:   {:>8} ns/op  ({:.2?} for {} matrices)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 27. Composite Glyph Dependency Closure + Subset
    {
        let font = benchmark_composite_true_type();
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let subset = TrueTypeSubsetter::subset(&font, &[1]).unwrap();
            assert!(subset.glyph_ids.contains(&2));
        }
        let elapsed = start.elapsed();
        println!(
            "27. Composite Glyph Closure: {:>8} ns/op  ({:.2?} for {} subsets)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 28. Automatic Type0 Font Subset Appearance
    {
        let fixture = include_bytes!("../tests/fixtures/v0_9_compat/chrome-unicode.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let field_ref = configure_embedded_benchmark_field(&mut doc, false, 0);
        let change = PdfChange::SetTextField {
            field_ref,
            value: "Benchmark".to_string(),
        };
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            let plan = doc.apply_mutation(std::slice::from_ref(&change)).unwrap();
            assert!(plan.glyph_mapping_quality.is_some());
        }
        let elapsed = start.elapsed();
        println!(
            "28. Auto Type0 Font Embed:   {:>8} ns/op  ({:.2?} for {} plans)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 29. Repeated-Widget Subset Resource Deduplication
    {
        let fixture = include_bytes!("../tests/fixtures/v0_9_compat/chrome-unicode.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let field_ref = configure_embedded_benchmark_field(&mut doc, true, 90);
        let change = PdfChange::SetTextField {
            field_ref,
            value: "Benchmark".to_string(),
        };
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = doc.apply_mutation(std::slice::from_ref(&change)).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "29. Font Resource Dedup:     {:>8} ns/op  ({:.2?} for {} two-widget plans)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 30. Incremental Export + Reopen With Embedded Subset
    {
        let fixture = include_bytes!("../tests/fixtures/v0_9_compat/chrome-unicode.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let field_ref = configure_embedded_benchmark_field(&mut doc, false, 270);
        let plan = doc
            .apply_mutation(&[PdfChange::SetTextField {
                field_ref,
                value: "Benchmark".to_string(),
            }])
            .unwrap();
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            let output = doc.export_incremental(&plan).unwrap();
            let _ = PdfDocument::from_bytes(&output).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "30. Subset Export+Reopen:    {:>8} ns/op  ({:.2?} for {} roundtrips)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 31. Producer AcroForm Field Tree + Inheritance Traversal
    {
        let fixture = include_bytes!("../tests/fixtures/v0_10_compat/pdflib-complete-form.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            assert_eq!(doc.form_fields().unwrap().len(), 6);
        }
        let elapsed = start.elapsed();
        println!(
            "31. Producer Field Traversal: {:>8} ns/op  ({:.2?} for {} traversals)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 32. Orphan PDFKit Widget Resolution
    {
        let fixture = include_bytes!("../tests/fixtures/v0_10_compat/pdfkit-text-checkbox.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            assert_eq!(doc.form_fields().unwrap().len(), 4);
        }
        let elapsed = start.elapsed();
        println!(
            "32. Orphan Widget Resolution: {:>8} ns/op  ({:.2?} for {} resolutions)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 33. Producer Annotation Graph + Existing AP Parsing
    {
        let fixture = include_bytes!("../tests/fixtures/v0_10_compat/pdfkit-shapes-ink-link.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let iterations = 5_000;
        let start = Instant::now();
        for _ in 0..iterations {
            assert_eq!(doc.page_annotations(0).unwrap().len(), 5);
        }
        let elapsed = start.elapsed();
        println!(
            "33. Annotation/AP Traversal:  {:>8} ns/op  ({:.2?} for {} traversals)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 34. CFF/CFF2/TrueType Program Detection
    {
        let font = benchmark_true_type();
        let iterations = 100_000;
        let start = Instant::now();
        for index in 0..iterations {
            let (key, subtype) = if index % 2 == 0 {
                ("FontFile2", None)
            } else {
                ("FontFile3", Some("Type1C"))
            };
            let _ = Font::detect_font_program(key, subtype, &font).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "34. Font Program Detection:   {:>8} ns/op  ({:.2?} for {} detections)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 35. Inherited Field Mutation + AP Preservation Planning
    {
        let fixture = include_bytes!("../tests/fixtures/v0_10_compat/pdflib-inherited-field.pdf");
        let mut doc = PdfDocument::from_bytes(fixture).unwrap();
        let field_ref = doc.form_fields().unwrap()[0].object_ref;
        let change = PdfChange::SetTextField {
            field_ref,
            value: "Inherited benchmark".into(),
        };
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let plan = doc.apply_mutation(std::slice::from_ref(&change)).unwrap();
            assert!(!plan.modified_objects.is_empty());
        }
        let elapsed = start.elapsed();
        println!(
            "35. Inherited Mutation Plan:  {:>8} ns/op  ({:.2?} for {} plans)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 36. Multi-revision chain traversal
    {
        let fixture =
            include_bytes!("../tests/fixtures/v0_11_complex/synthetic-hybrid-multi-revision.pdf");
        let iterations = 5_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let table = starpdf::xref::XrefResolver::load_xref_and_trailer(
                starpdf::io::source::ByteSource::new(fixture),
            )
            .unwrap();
            assert_eq!(table.revisions.len(), 3);
        }
        let elapsed = start.elapsed();
        println!(
            "36. Revision Chain Traversal: {:>8} ns/op  ({:.2?} for {} traversals)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 37. Hybrid xref and latest-definition resolution
    {
        let fixture =
            include_bytes!("../tests/fixtures/v0_11_complex/synthetic-hybrid-multi-revision.pdf");
        let iterations = 5_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(fixture).unwrap();
            assert!(doc.page_dict(0).unwrap().contains_key("StarPDFRevision"));
        }
        let elapsed = start.elapsed();
        println!(
            "37. Hybrid/Conflict Resolve:  {:>8} ns/op  ({:.2?} for {} resolutions)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 38. ByteRange validation
    {
        let byte_range = PdfObject::Array(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(64),
            PdfObject::Integer(128),
            PdfObject::Integer(64),
        ]);
        let iterations = 100_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let ranges = starpdf::security::parse_byte_range(&byte_range, 1024).unwrap();
            assert_eq!(ranges.len(), 2);
        }
        let elapsed = start.elapsed();
        println!(
            "38. ByteRange Parsing:        {:>8} ns/op  ({:.2?} for {} parses)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 39. Signature structure detection
    {
        let fixture = include_bytes!("../tests/fixtures/v0_11_complex/synthetic-signed-valid.pdf");
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(fixture).unwrap();
            assert_eq!(doc.security_info().unwrap().signature_count, 1);
        }
        let elapsed = start.elapsed();
        println!(
            "39. Signature Detection:      {:>8} ns/op  ({:.2?} for {} inspections)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 40. Encryption structure and permission detection
    {
        let fixture =
            include_bytes!("../tests/fixtures/v0_11_complex/synthetic-encrypted-standard.pdf");
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(fixture).unwrap();
            assert!(!doc.security_info().unwrap().mutation_allowed);
        }
        let elapsed = start.elapsed();
        println!(
            "40. Encryption Detection:     {:>8} ns/op  ({:.2?} for {} inspections)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 41. Orphan/ambiguous field graph classification
    {
        let fixture =
            include_bytes!("../tests/fixtures/v0_11_complex/synthetic-ambiguous-orphan-radio.pdf");
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(fixture).unwrap();
            assert_eq!(doc.form_fields().unwrap().len(), 2);
        }
        let elapsed = start.elapsed();
        println!(
            "41. Field Graph Classification:{:>8} ns/op  ({:.2?} for {} classifications)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 42. Metadata-preserving mutation planning
    {
        let fixture = include_bytes!("../tests/fixtures/v0_11_complex/synthetic-metadata-rich.pdf");
        let iterations = 2_000;
        let change = PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [20.0, 20.0, 40.0, 40.0],
                stroke_color: None,
                fill_color: None,
                border_width: None,
            },
        };
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(fixture).unwrap();
            assert!(!doc
                .apply_mutation(std::slice::from_ref(&change))
                .unwrap()
                .modified_objects
                .is_empty());
        }
        let elapsed = start.elapsed();
        println!(
            "42. Metadata Preserve Plan:   {:>8} ns/op  ({:.2?} for {} plans)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 43. Effective trailer lookup after mixed revision resolution
    {
        let fixture =
            include_bytes!("../tests/fixtures/v0_11_complex/synthetic-hybrid-multi-revision.pdf");
        let doc = PdfDocument::from_bytes(fixture).unwrap();
        let iterations = 1_000_000;
        let start = Instant::now();
        for _ in 0..iterations {
            assert!(doc.trailer().get("Root").is_some());
            assert!(doc.trailer().get("ID").is_some());
        }
        let elapsed = start.elapsed();
        println!(
            "43. Effective Trailer Lookup:{:>8} ns/op  ({:.2?} for {} lookups)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    let page_fixture = include_bytes!("../../../test-assets/multi-page.test.pdf");

    // 44. Incremental page deletion
    {
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            std::hint::black_box(doc.delete_page(0).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "44. Delete Page:             {:>8} ns/op  ({:.2?} for {} operations)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 45. Incremental page reorder
    {
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            std::hint::black_box(doc.move_page(0, 1).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "45. Move Page:               {:>8} ns/op  ({:.2?} for {} operations)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 46. Complete-build page duplication
    {
        let iterations = 200;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            std::hint::black_box(doc.duplicate_page(0, 1).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "46. Duplicate Page:          {:>8} ns/op  ({:.2?} for {} operations)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 47. Incremental blank-page insertion
    {
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            std::hint::black_box(doc.insert_blank_page(1, 612.0, 792.0, 0).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "47. Insert Blank Page:       {:>8} ns/op  ({:.2?} for {} operations)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 48. Single-page extraction and standalone serialization
    let extracted_one = {
        let iterations = 200;
        let start = Instant::now();
        let mut last = Vec::new();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            last = doc.extract_pages(&[0]).unwrap();
            std::hint::black_box(&last);
        }
        let elapsed = start.elapsed();
        println!(
            "48. Extract 1 + Write:       {:>8} ns/op  ({:.2?} for {} operations)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
        last
    };

    // 49. Ten-page repeated-selection extraction
    {
        let selection = [0usize, 1, 0, 1, 0, 1, 0, 1, 0, 1];
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            std::hint::black_box(doc.extract_pages(&selection).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "49. Extract 10 + Write:      {:>8} ns/op  ({:.2?} for {} operations)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 50. Standalone writer path (graph remap plus complete serialization)
    {
        let iterations = 200;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(page_fixture).unwrap();
            std::hint::black_box(doc.extract_pages(&[1]).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "50. Standalone PDF Write:    {:>8} ns/op  ({:.2?} for {} writes)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 51. Generated-output reopen
    {
        let iterations = 1_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc = PdfDocument::from_bytes(&extracted_one).unwrap();
            std::hint::black_box(doc.page_count().unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "51. Page Output Reopen:      {:>8} ns/op  ({:.2?} for {} reopens)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 52. One-page cross-document import and remap
    {
        let iterations = 200;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(
                PdfDocument::merge_selected(
                    &[page_fixture, page_fixture],
                    &[PageSource::new(1, 0)],
                )
                .unwrap(),
            );
        }
        let elapsed = start.elapsed();
        println!(
            "52. Cross-doc Import 1:      {:>8} ns/op  ({:.2?} for {} imports)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 53. Ten-page cross-document selection and shared dependency reuse
    {
        let selection = (0..10)
            .map(|index| PageSource::new(index % 2, index % 2))
            .collect::<Vec<_>>();
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(
                PdfDocument::merge_selected(&[page_fixture, page_fixture], &selection).unwrap(),
            );
        }
        let elapsed = start.elapsed();
        println!(
            "53. Cross-doc Import 10:     {:>8} ns/op  ({:.2?} for {} imports)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    let merged_two = {
        // 54. Merge two multi-page documents
        let iterations = 100;
        let start = Instant::now();
        let mut last = Vec::new();
        for _ in 0..iterations {
            last = PdfDocument::merge_documents(&[page_fixture, page_fixture]).unwrap();
            std::hint::black_box(&last);
        }
        let elapsed = start.elapsed();
        println!(
            "54. Merge 2 Documents:       {:>8} ns/op  ({:.2?} for {} merges)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
        last
    };

    // 55. Merge three documents
    {
        let iterations = 75;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(
                PdfDocument::merge_documents(&[page_fixture, page_fixture, page_fixture]).unwrap(),
            );
        }
        let elapsed = start.elapsed();
        println!(
            "55. Merge 3 Documents:       {:>8} ns/op  ({:.2?} for {} merges)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 56. Resource-heavy graph traversal and remap
    {
        let image = include_bytes!("../../../test-assets/scanned-test.pdf");
        let annotations =
            include_bytes!("../tests/fixtures/v0_10_compat/pdfkit-shapes-ink-link.pdf");
        let iterations = 50;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(PdfDocument::merge_documents(&[image, annotations]).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "56. Resource Graph Remap:    {:>8} ns/op  ({:.2?} for {} remaps)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 57. AcroForm collision isolation and deterministic renaming
    {
        let form = include_bytes!("../tests/fixtures/v0_10_compat/pdflib-complete-form.pdf");
        let iterations = 50;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(PdfDocument::merge_documents(&[form, form]).unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "57. Form Collision Remap:    {:>8} ns/op  ({:.2?} for {} remaps)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 58. Complete merge writer path
    {
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(
                PdfDocument::merge_selected(
                    &[page_fixture, page_fixture],
                    &[PageSource::new(0, 0), PageSource::new(1, 0)],
                )
                .unwrap(),
            );
        }
        let elapsed = start.elapsed();
        println!(
            "58. Merge Standalone Write:  {:>8} ns/op  ({:.2?} for {} writes)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 59. Merge output reopen and page enumeration
    {
        let iterations = 1_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut document = PdfDocument::from_bytes(&merged_two).unwrap();
            std::hint::black_box(document.page_count().unwrap());
        }
        let elapsed = start.elapsed();
        println!(
            "59. Merge Output Reopen:     {:>8} ns/op  ({:.2?} for {} reopens)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 60. Single text span editability check
    {
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let page_text = doc.extract_page_text(0).unwrap();
        let span = &page_text.spans[0];
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(span.is_editable);
            std::hint::black_box(&span.editability_status);
        }
        let elapsed = start.elapsed();
        println!(
            "60. Span Editability Check:  {:>8} ns/op  ({:.2?} for {} checks)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 61. Content stream parsing, operand mutation and re-serialization
    {
        let raw_stream =
            b"q\n1 0 0 1 50 700 cm\nBT\n/F1 12 Tf\n(Benchmark Original Text) Tj\nET\nQ\n";
        let target = starpdf::mutation::text_edit::TextEditTarget::new(0, 0, 4, 0);
        let new_bytes = b"Benchmark Replaced Text";
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let modified = starpdf::mutation::text_edit::ContentStreamEditor::replace_in_stream(
                raw_stream, &target, new_bytes,
            )
            .unwrap();
            std::hint::black_box(modified);
        }
        let elapsed = start.elapsed();
        println!(
            "61. Stream Parse/Mutate/Ser: {:>8} ns/op  ({:.2?} for {} replacements)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 62. Full Native Text Replacement & Layout Policy Check
    {
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let page_text = doc.extract_page_text(0).unwrap();
        let target = starpdf::mutation::text_edit::TextEditTarget::from_span(&page_text.spans[0]);
        let iterations = 2_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut doc_run = PdfDocument::from_bytes(&sample_pdf).unwrap();
            let plan = doc_run
                .replace_text(0, &target, "StarPDF Performance Suite Document")
                .unwrap();
            std::hint::black_box(plan);
        }
        let elapsed = start.elapsed();
        println!(
            "62. Text Replace & Layout:   {:>8} ns/op  ({:.2?} for {} replacements)",
            elapsed.as_nanos() / iterations as u128,
            elapsed,
            iterations
        );
    }

    // 63. Incremental Export with Modified Content Stream
    {
        let mut doc = PdfDocument::from_bytes(&sample_pdf).unwrap();
        let page_text = doc.extract_page_text(0).unwrap();
        let target = starpdf::mutation::text_edit::TextEditTarget::from_span(&page_text.spans[0]);
        let plan = doc
            .replace_text(0, &target, "Export Benchmark Suite")
            .unwrap();
        let iterations = 5_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let exported = doc.export_incremental(&plan).unwrap();
            std::hint::black_box(exported);
        }
        let elapsed = start.elapsed();
        println!(
            "63. Incremental Text Export: {:>8} ns/op  ({:.2?} for {} exports)",
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

fn benchmark_composite_true_type() -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&3u16.to_be_bytes());
    let mut loca = Vec::new();
    for offset in [0u32, 12, 28, 40] {
        loca.extend_from_slice(&offset.to_be_bytes());
    }
    let mut glyf = vec![0u8; 40];
    glyf[12..14].copy_from_slice(&(-1i16).to_be_bytes());
    glyf[22..24].copy_from_slice(&0u16.to_be_bytes());
    glyf[24..26].copy_from_slice(&2u16.to_be_bytes());
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

fn configure_embedded_benchmark_field(
    doc: &mut PdfDocument<'_>,
    repeated: bool,
    rotation: i64,
) -> ObjectRef {
    let field_ref = ObjectRef::new(9_000, 0);
    let first_widget = ObjectRef::new(9_001, 0);
    let second_widget = ObjectRef::new(9_002, 0);
    let page_ref = doc.page_ref(0).unwrap();
    let mut field = BTreeMap::from([
        ("FT".to_string(), PdfObject::Name("Tx".to_string())),
        (
            "DA".to_string(),
            PdfObject::String(b"/F5 12 Tf 0 g".to_vec()),
        ),
        (
            "Rect".to_string(),
            PdfObject::Array(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(180),
                PdfObject::Integer(30),
            ]),
        ),
    ]);
    if repeated {
        field.insert(
            "Kids".to_string(),
            PdfObject::Array(vec![
                PdfObject::Reference(first_widget),
                PdfObject::Reference(second_widget),
            ]),
        );
        for widget_ref in [first_widget, second_widget] {
            doc.store_mut().insert_cached(
                widget_ref,
                PdfObject::Dictionary(BTreeMap::from([
                    ("Subtype".to_string(), PdfObject::Name("Widget".to_string())),
                    ("Parent".to_string(), PdfObject::Reference(field_ref)),
                    ("P".to_string(), PdfObject::Reference(page_ref)),
                    (
                        "Rect".to_string(),
                        PdfObject::Array(vec![
                            PdfObject::Integer(0),
                            PdfObject::Integer(0),
                            PdfObject::Integer(180),
                            PdfObject::Integer(30),
                        ]),
                    ),
                    (
                        "MK".to_string(),
                        PdfObject::Dictionary(BTreeMap::from([(
                            "R".to_string(),
                            PdfObject::Integer(rotation),
                        )])),
                    ),
                ])),
            );
        }
    } else if rotation != 0 {
        field.insert("P".to_string(), PdfObject::Reference(page_ref));
        field.insert(
            "MK".to_string(),
            PdfObject::Dictionary(BTreeMap::from([(
                "R".to_string(),
                PdfObject::Integer(rotation),
            )])),
        );
    }
    doc.store_mut()
        .insert_cached(field_ref, PdfObject::Dictionary(field));
    field_ref
}
