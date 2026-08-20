#![allow(clippy::all, clippy::pedantic, clippy::let_unit_value)]
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::collections::BTreeMap;
use std::time::Instant;

use starpdf::content::ContentParser;
use starpdf::document::{ObjectStreamReader, PdfDocument};
use starpdf::filter::{DecompressLimits, FlateDecoder};
use starpdf::font::{Font, PageResources, UnicodeCMap};
use starpdf::syntax::object::{PdfObject, StreamObject};
use starpdf::syntax::{Lexer, Parser};
use starpdf::text::TextExtractor;
use starpdf::writer::MinimalWriter;
use starpdf::xref::table::XrefTable;
use starpdf::xref::XrefStreamParser;

fn main() {
    println!("================================================================");
    println!("          StarPDF Engine v0.3 Micro-Benchmark Suite             ");
    println!("================================================================");

    let sample_pdf = MinimalWriter::create_minimal_pdf("StarPDF Performance Benchmark Document")
        .expect("Failed to create fixture");

    // 1. Lexer Throughput
    {
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
            "1. Lexer Throughput:         {:>8.2} MB/s  ({} tokens in {:.2?})",
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
            "2. Object Parser:            {:>8.2} MB/s  ({} objects parsed in {:.2?})",
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
            "3. FlateDecode Throughput:   {:>8.2} MB/s  ({:.2?} for {} decompressions)",
            mb_per_sec, elapsed, iterations
        );
    }

    // 4. ToUnicode CMap Parser
    {
        let cmap_data = b"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
1 begincodespacerange <0001> <FFFF> endcodespacerange
10 beginbfchar
<0001> <0041> <0002> <0042> <0003> <0043> <0004> <0044> <0005> <0045>
<0006> <0046> <0007> <0047> <0008> <0048> <0009> <0049> <000A> <004A>
endbfchar
endcmap
end
";
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = UnicodeCMap::parse(cmap_data).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "4. CMap Stream Parsing:      {:>8} ns/op  ({:.2?} for {} parses)",
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
            "5. Text Extractor:           {:>8.2} MB/s  ({} spans extracted in {:.2?})",
            mb_per_sec, span_count, elapsed
        );
    }

    // 6. XRef Stream Parser Throughput
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
            "6. XRef Stream Parsing:      {:>8} ns/op  ({:.2?} for {} 100-entry streams)",
            ns_per_op, elapsed, iterations
        );
    }

    // 7. Compressed Object Resolution (ObjStm)
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
            "7. ObjStm Object Extraction: {:>8} ns/op  ({:.2?} for {} extractions)",
            ns_per_op,
            elapsed,
            iterations * 2
        );
    }

    // 8. Document Open & XRef Resolution
    {
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = PdfDocument::from_bytes(&sample_pdf).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "8. Document Open & XRef:     {:>8} ns/op  ({:.2?} for {} opens)",
            ns_per_op, elapsed, iterations
        );
    }

    // 9. Page Tree Traversal
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
            "9. Page Tree Resolution:     {:>8} ns/op  ({:.2?} for {} resolutions)",
            ns_per_op, elapsed, iterations
        );
    }

    // 10. Content Stream Parser
    {
        let content_stream =
            b"q 1 0 0 1 50 700 cm BT /F1 12 Tf (Hello World) Tj T* (Second Line) Tj ET Q";
        let iterations = 10_000;
        let start = Instant::now();
        let mut op_count = 0;
        for _ in 0..iterations {
            let mut parser = ContentParser::from_bytes(content_stream);
            let instrs = parser.parse_instructions().unwrap();
            op_count += instrs.len();
        }
        let elapsed = start.elapsed();
        let total_bytes = content_stream.len() * iterations;
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
        println!(
            "10. Content Stream Parser:   {:>8.2} MB/s  ({} instructions in {:.2?})",
            mb_per_sec, op_count, elapsed
        );
    }
    println!("================================================================");
}
