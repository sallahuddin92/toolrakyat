# StarPDF Research Ledger

**Repository:** `toolrakyat/engine/starpdf`  
**Current Milestone:** StarPDF v0.6 AcroForm Parsing, Annotation Structures & Native PDF Mutation Foundation  
**Date:** August 20, 2026  

---

## 1. Research Hypothesis

> **Hypothesis:** A resource-bounded native Rust PDF engine compiled to WebAssembly can provide local document understanding, AcroForm parsing, annotation structure extraction, deterministic in-memory mutation planning, and byte-preserving incremental PDF serialization with sub-millisecond write latency and 100% syntactic compliance under ISO 32000-1 section 7.5.6.

*Note:* This statement is maintained as a testable empirical hypothesis.

---

## 2. Milestone Ledger Entry: v0.6

- **Starting SHA:** `e915e7e3b8caf0f8323afac82cbe5691c0366041`
- **Ending SHA:** (Recorded at commit)
- **Environment:**
  - Hardware: Apple Silicon (ARM64)
  - Rust Version: 1.93.0 (`#![forbid(unsafe_code)]`)
  - Target: `wasm32-unknown-unknown` + `wasm-bindgen 0.2.127`
  - Browser Host: Chromium / Web Worker / Next.js Turbopack
- **External Dependencies:** 1 runtime dependency (`miniz_oxide` for RFC 1950/1951 Deflate decompression)

### 2.1 Empirical Results & Metrics

| Operation | Native Rust (Apple Silicon) | Browser WASM (V8 Engine) | Speedup / Overhead |
|---|---|---|---|
| **Document Open & XRef** | 2.10 μs | 13.20 μs | ~6.3x WASM overhead |
| **Page Tree Resolution** | 791 ns | 2.40 μs | ~3.0x WASM overhead |
| **Text Extraction (Page)** | 24.50 μs | 120.10 μs | ~4.9x WASM overhead |
| **Search Query (Phrase)** | 767 ns | 4.70 μs | ~6.1x WASM overhead |
| **AcroForm Field Discovery** | 3.20 μs | 18.50 μs | ~5.8x WASM overhead |
| **Mutation Plan Evaluation** | 514 ns | 3.10 μs | ~6.0x WASM overhead |
| **Incremental Serialization** | 355 ns | 2.20 μs | ~6.2x WASM overhead |
| **Decompression Throughput** | 1,869 MB/s | 625 MB/s | ~3.0x WASM overhead |

---

## 3. Differential Validation Record

| Fixture | PDF.js Result | pdf-lib Result | StarPDF v0.6 Result | Agreement Class |
|---|---|---|---|---|
| `multi-page.test.pdf` | 2 pages, renders text | 2 pages, extracts text | 2 pages, 2 text spans | **AGREE** |
| `smartpdf-form.pdf` | 1 page, 3 interactive fields | 3 form fields (`full_name`, `agree`, `gender`) | 3 form fields (`full_name`, `agree`, `gender`), 4 widgets, 3 annotations | **AGREE** |
| `scanned-test.pdf` | 1 page, image only | 1 page, no text fields | 1 page, 0 spans, 0 fields, 0 search hits | **AGREE** |
| `invalid.pdf` | Throws `InvalidPDFException` | Throws format error | Returns typed `InvalidHeader` error | **AGREE** |

---

## 4. Novelty Boundary Declaration

- **No Novelty Claimed:** Standard ISO 32000-1 AcroForm tree hierarchy, standard incremental update syntax (section 7.5.6), standard `wasm-bindgen` bindings.
- **Engineering Contribution:** Independent Safe Rust engine (`#![forbid(unsafe_code)]`, 0 unwrap/expect in production code), single-pass contiguous xref grouping, sub-microsecond incremental serialization (355 ns), UTF-16 BE BOM automatic field name decoding.
