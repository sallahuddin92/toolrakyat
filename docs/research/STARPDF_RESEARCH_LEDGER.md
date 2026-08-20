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

---

## 5. Milestone Ledger Entry: v0.7

- **Starting SHA:** `efb56d36d181a6f28b0422e30b0d1ba1e6f27b7c`
- **Milestone SHA:** commit containing this ledger entry
- **Classification:** `CONVENTIONAL_ENGINEERING`, `SECURITY_HARDENING`, `PERFORMANCE_OPTIMIZATION`
- **Fuzz result:** 18,292,731 bounded executions; 0 crashes, hangs, OOMs, or unique reproducers.
- **Differential result:** StarPDF and pdf-lib reopen and agree on mutated form values; PDF.js renders the incremental result and shows a bounded-region pixel change.
- **Preservation result:** every incremental result retains the input as an exact byte prefix.
- **Refusal result:** invalid geometry, non-finite numbers, oversized annotation data, wrong-page removals, excess mutations, and excess output growth return deterministic typed errors without partial output.
- **Observed mechanism:** combining explicit appearance outcomes across an atomic mutation batch avoids treating a logical-only update as visually complete. This is classified as `CONVENTIONAL_ENGINEERING`; no patentability claim is made.

## 6. Milestone Ledger Entry: v0.8

- **Starting SHA:** `d2e847d57be9007b9a0ab486377a605e7d5e69ec`
- **Milestone SHA:** commit containing this ledger entry
- **Classification:** `STANDARD_PDF_IMPLEMENTATION`, `CONVENTIONAL_ENGINEERING`, `SECURITY_HARDENING`, `PERFORMANCE_OPTIMIZATION`
- **Production evidence:** deterministic regeneration replaces stale annotation `/AP`; compatible embedded font references are reused; rich child widgets visually roundtrip through PDF.js; every incremental generation preserves the prior bytes as an exact prefix.
- **Differential evidence:** repository and locally generated pdf-lib fixtures agree semantically with StarPDF and render visibly through PDF.js. Unsupported composite encodings and font formats are refused rather than coerced.
- **Fuzz evidence:** four new/affected bounded targets, including a final resolver rerun, completed a clean recorded 2,214,760 executions with zero crashes or hangs.
- **Performance evidence:** the second warm run measured 1,952 ns font resolution, 822 ns TrueType subset, 1,993 ns annotation AP regeneration, and 18,950 ns resource export/reopen. No reproducible unchanged-workload regression exceeded 15%.
- **Novelty boundary:** appearance dictionaries, TrueType subsetting, font resource reuse, and atomic overlays are standard or conventional engineering. No `EXPERIMENTAL_METHOD` or `POTENTIAL_NOVELTY` evidence is asserted, and no patentability claim is made.

## 7. Milestone Ledger Entry: v0.9

- **Starting SHA:** `1dd30a3d42658a9087bc4b06080d9cc5fea41c40`
- **Milestone SHA:** commit containing this ledger entry
- **Classification:** `STANDARD_PDF_IMPLEMENTATION`, `CONVENTIONAL_ENGINEERING`, `SECURITY_HARDENING`, `PERFORMANCE_OPTIMIZATION`, `EXPERIMENTAL_METHOD`
- **Compatibility evidence:** 12 locally authored fixtures from Chrome, LibreOffice, and Quartz/CUPS pass extraction, search, mutation, incremental export, and reopen. A bounded overlapping-span search recovery and widget-AP font-resource recovery were derived from observed producer structures.
- **Production evidence:** automatic TrueType subset embedding, transaction-local deterministic resource reuse, bounded composite dependency closure, exact quarter-turn widget matrices, and proved Identity-H/Identity-V output render through PDF.js.
- **Fuzz evidence:** three new and six affected 10-second campaigns completed 11,823,541 executions with zero crashes, hangs, or reproducers.
- **Novelty boundary:** all implemented PDF/font structures are standard mechanisms; the integration and hardening are conventional/security engineering. `EXPERIMENTAL_METHOD` refers only to the producer/PDF.js validation procedure. No `POTENTIAL_NOVELTY` is asserted. Patentability claims: **NONE**.

## 8. Milestone Ledger Entry: v0.10

- **Starting SHA:** `e3caf30e6bd99522a2445ef14262617fd9780c77`
- **Milestone SHA:** commit containing this ledger entry
- **Classification:** `STANDARD_PDF_IMPLEMENTATION`, `CONVENTIONAL_ENGINEERING`, `SECURITY_HARDENING`, `PERFORMANCE_OPTIMIZATION`, `EXPERIMENTAL_METHOD`
- **Compatibility evidence:** 22 meaningful fixtures across Chrome/Skia, LibreOfficeDev, Quartz/CUPS, Apple PDFKit annotation authoring, and pdf-lib cover canonical and orphan widgets, inherited fields, N/R/D appearances, rotations, requested annotation types, xref/object streams, and prior incremental revisions.
- **Production evidence:** bounded field/widget separation, proof-based NeedAppearances reconciliation, producer AP preservation, typed Link URI exposure, and collision-free prefix-preserving export/reopen pass native and PDF.js roundtrips.
- **Font evidence:** corpus CFF 0, CFF2 0, non-Identity Type0 0. Deterministic detection distinguishes TrueType, CFF, CFF2, and unknown programs; unsupported mutation refuses without reinterpretation.
- **Fuzz evidence:** twelve 10-second campaigns across ten new/affected targets completed 31,396,831 executions with zero crashes, hangs, or reproducers; form and mutation targets reran after final control-flow fixes.
- **Novelty boundary:** the implemented PDF mechanisms are standard; bounded recovery, atomic validation, and differential testing are conventional/security engineering. `EXPERIMENTAL_METHOD` refers only to the local producer/PDF.js evidence procedure. No `POTENTIAL_NOVELTY` is asserted. Patentability claims: **NONE**.
