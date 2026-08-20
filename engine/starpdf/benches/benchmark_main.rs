use starpdf::content::ContentParser;
use starpdf::document::PdfDocument;
use starpdf::syntax::{Lexer, Parser};
use starpdf::writer::MinimalWriter;
use std::time::Instant;

fn main() {
    println!("================================================================");
    println!("          StarPDF Engine v0.1 Micro-Benchmark Suite             ");
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
            "1. Lexer Throughput:      {:>8.2} MB/s  ({} tokens in {:.2?})",
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
            "2. Object Parser:         {:>8.2} MB/s  ({} objects parsed in {:.2?})",
            mb_per_sec, iterations, elapsed
        );
    }

    // 3. Document Open & XRef Resolution
    {
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = PdfDocument::from_bytes(&sample_pdf).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / iterations as u128;
        println!(
            "3. Document Open & XRef:  {:>8} ns/op  ({:.2?} for {} opens)",
            ns_per_op, elapsed, iterations
        );
    }

    // 4. Page Tree Traversal
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
            "4. Page Tree Resolution:  {:>8} ns/op  ({:.2?} for {} resolutions)",
            ns_per_op, elapsed, iterations
        );
    }

    // 5. Content Stream Parser
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
            "5. Content Stream Parser: {:>8.2} MB/s  ({} instructions in {:.2?})",
            mb_per_sec, op_count, elapsed
        );
    }
    println!("================================================================");
}
