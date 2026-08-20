# StarPDF Prior Art & Ecosystem Analysis

**Document Status:** Research Reference  
**Last Updated:** August 2026  

---

## 1. Context & Objective

PDF (Portable Document Format, ISO 32000-1 / ISO 32000-2) is a rich, complex binary format with deep legacy compatibility requirements. Modern browser document editing typically relies on either:
1. **Full rasterization/rendering engines** (e.g., PDF.js, MuPDF WASM, Chromium PDFium).
2. **Server-side post-processing** (e.g., Ghostscript, QPDF, Poppler, iText).
3. **Pure JavaScript manipulation libraries** (e.g., `pdf-lib`).

StarPDF represents an independent, resource-bounded native Rust engine compiled to WebAssembly designed specifically for fast client-side document inspection, coordinate-aware text extraction, deterministic search geometry, AcroForm parsing, and byte-preserving incremental PDF mutations without server dependencies.

---

## 2. Comparison Matrix

| Dimension | PDF.js (Mozilla) | pdf-lib (JS) | MuPDF / PDFium (WASM) | StarPDF v0.6 |
|---|---|---|---|---|
| **Primary Focus** | Visual canvas rendering & display | PDF modification & full re-serialization | Full rendering & C++ engine port | Structural inspection, text search, AcroForms & byte-preserving incremental mutation |
| **Runtime Language** | JavaScript / Web Workers | JavaScript / TypeScript | C / C++ (compiled via Emscripten) | Safe Rust (`#![forbid(unsafe_code)]`) + WebAssembly |
| **Binary Size** | ~2.5 MB - 4.5 MB | ~350 KB | ~8 MB - 15 MB | **~180 KB** (gzipped WASM) |
| **Mutation Paradigm** | N/A (Read-only viewer) | Full document re-serialization (~4.5 ms) | Native mutation | **Incremental update append (355 ns / 2.2 μs WASM)** |
| **Byte Preservation**| N/A | Rewrites original document bytes | Re-serializes output | **Preserves original byte prefix 100% verbatim** |
| **Memory Isolation** | Standard JS Heap | Standard JS Heap | Emscripten Linear Memory | Bounded WASM Linear Memory + Handle Registry |
| **Hostile Input Defense**| JS Exception model | JS Exception model | C/C++ memory safety mitigation | Safe Rust memory guarantees + caps on depth, fields, and memory |
| **External Runtime Deps**| Multiple NPM packages | Zero external dependencies | Complex C toolchains | **1 runtime crate** (`miniz_oxide` for Deflate) |

---

## 3. Prior Art & Non-Invention Boundaries

1. **Standard Technology Use:** WebAssembly (`wasm-bindgen`) and Web Worker message passing are standard web engineering practices and constitute no novel claim.
2. **PDF Specification Compliance:** ISO 32000-1 parsing algorithms (XRef streams, Object streams, `/ToUnicode` CMaps, SFNT format 4/12 tables, AcroForm hierarchies, incremental updates under §7.5.6) follow standard open specifications.
3. **Differential Findings:** Disagreements across engines in malformed edge cases reflect differences in leniency vs strict validation rather than proprietary inventions.

## 4. v0.7 Boundary Update

Form XObject appearance streams, AcroForm `/DA` parsing, annotation dictionaries, and incremental updates are standard PDF mechanisms (`STANDARD_PDF_IMPLEMENTATION`). Safe Rust validation, atomic mutation overlays, explicit appearance status propagation, typed worker errors, and visual differential testing are classified as `CONVENTIONAL_ENGINEERING` or `SECURITY_HARDENING`. No patentability claim is made.
