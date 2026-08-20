# StarPDF Engine v0.1 Specification & Architecture Report

**Status:** v0.1 Foundation Complete  
**Language:** Rust 1.93 (Safe Rust, `#![forbid(unsafe_code)]`)  
**External Runtime Dependencies:** 0  
**Baseline Commit:** `7ff5a848034011187634e3e1b0bca77cdd12e25c`  

---

## 1. Executive Summary

StarPDF is an in-house, low-level, zero-dependency, high-performance native PDF engine designed from first principles. It operates directly on raw PDF byte streams without intermediate wrappers (e.g. `pdf-lib`, `PDF.js`, `qpdf`, `MuPDF`, or `Ghostscript`).

In v0.1, StarPDF provides:
- Direct byte-level lexical tokenization.
- Complete raw PDF 1.7 / 2.0 object model (`Null`, `Bool`, `Integer`, `Real`, `Name`, `String`, `Array`, `Dictionary`, `Stream`, `Reference`).
- Classic cross-reference (`xref`) table parsing and `startxref` resolution.
- On-demand lazy indirect object store with resolution caching and access telemetry.
- Catalog (`/Root`) and hierarchical Page tree (`/Pages`) traversal.
- Page content stream tokenizer and graphics/text operator parser (27+ operators).
- Canonical PDF object serializer and standard-compliant single-page PDF generator/writer.

---

## 2. Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────────┐
│                               PdfDocument                                       │
│  (Holds ByteSource, Version, Root Catalog Ref, Root Pages Ref, ObjectStore)     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌───────────────────────┐   ┌────────────────────────┐   ┌──────────────────┐  │
│  │      PageTree         │   │      ObjectStore       │   │  ContentParser   │  │
│  │ (Hierarchical page    │   │ (Lazy object resolver, │   │ (Graphics/text   │  │
│  │  count & lookup)      │   │  cache, metrics)       │   │  operator AST)   │  │
│  └───────────────────────┘   └────────────────────────┘   └──────────────────┘  │
│                                                                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                Syntax Layer                                     │
│                                                                                 │
│   Lexer (Byte-level tokens)                       Parser (Object AST)           │
│   • Whitespace & Comments                         • Primitives & Containers     │
│   • Names (/Name & #xx escapes)                   • Indirect Objects (n g obj)  │
│   • Strings (Literal & Hex)                       • Stream Bodies               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                   I/O Layer                                     │
│   ByteSource (&[u8] borrowed memory slice)        ByteCursor (Bounded indexer)  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Invariants

1. **Panic Freedom on Hostile Input:** The parser treats all input as untrusted. Truncated buffers, circular references, invalid tokens, and corrupted tables return structured `Result<T, PdfError>` instead of panicking.
2. **Zero External Runtime Dependencies:** The engine compiles using the standard library alone (`std::collections::BTreeMap`, `std::vec::Vec`, `std::io::Write`).
3. **Parse Once, Resolve Lazily:** The document open path parses only the header, trailer, and xref table. Indirect objects and streams are parsed only when explicitly accessed.
4. **Preserve Original Byte Offsets:** Indirect object offsets and stream offsets are retained to enable future incremental updates.

---

## 4. Supported PDF Syntax

- **Lexical Primitives:**
  - Whitespace: `0x00` (NUL), `0x09` (HT), `0x0A` (LF), `0x0C` (FF), `0x0D` (CR), `0x20` (SP).
  - Comments: `%...` terminated by LF or CR/LF.
  - Numbers: Signed/unsigned integers (`123`, `+45`, `-67`) and real numbers (`3.14`, `-0.001`, `0.0`).
  - Names: `/Name` with `#xx` hex character decoding (ISO 32000 §7.3.5).
  - Literal Strings: `(...)` with nested balanced parentheses and escape sequences (`\n`, `\r`, `\t`, `\b`, `\f`, `\(`, `\)`, `\\`, `\ddd` octal, `\<EOL>` line continuation).
  - Hexadecimal Strings: `<...>` with whitespace tolerance and odd nibble padding.
  - Booleans & Null: `true`, `false`, `null`.
  - Delimiters: `[`, `]`, `<<`, `>>`.
  - Keywords: `obj`, `endobj`, `stream`, `endstream`, `xref`, `trailer`, `startxref`, `R`.
- **Containers:** Arrays (`[...]`), Dictionaries (`<<...>>`), Streams (`<< /Length N >>\nstream\n...\nendstream`).
- **Cross-Reference Tables:** Classic ASCII xref tables with multiple contiguous or disjoint subsections, free/in-use flags (`f`/`n`), and chained `/Prev` tables.
- **Page Tree:** Single-level and multi-level hierarchical `/Pages` nodes with `/Kids` and `/Count` traversal.
- **Content Stream Operators:**
  - Graphics State: `q`, `Q`, `cm`.
  - Text State & Positioning: `BT`, `ET`, `Tf`, `Tm`, `Td`, `TD`, `T*`, `Tj`, `TJ`.
  - Path Construction & Painting: `re`, `m`, `l`, `c`, `h`, `S`, `s`, `f`, `F`, `f*`, `B`, `B*`, `b`, `b*`.
  - XObjects: `Do`.
  - Unknown fallback preserved gracefully.

---

## 5. Unsupported PDF Syntax (v0.1 Scope)

- Cross-reference streams (PDF 1.5+ compressed `/XRef` streams).
- Object streams (PDF 1.5+ `/ObjStm`).
- Stream decompression filters (FlateDecode, ASCIIHexDecode, LZWDecode, DCTDecode).
- Standard encryption / password-protected security handlers.
- Linearized (Fast Web View) hint tables.

---

## 6. Parser Limits

- **Maximum Recursion Depth:** 64 (prevents stack overflow on malicious nested structures).
- **Maximum Page Tree Depth:** 32.
- **Header Search Window:** First 1,024 bytes.
- **StartXRef Search Window:** Last 2,048 bytes.

---

## 7. Lazy-Resolution Model & Telemetry

The `ObjectStore` maintains an internal lookup cache and records the following metrics:
- `objects_known`: Total entries discovered in the XRef table.
- `objects_resolved`: Number of indirect objects parsed from the byte stream on first access.
- `cache_hits`: Number of times a resolved object was retrieved directly from the memory cache.
- `bytes_parsed`: Cumulative byte count consumed during indirect object parsing.

---

## 8. Benchmark Baseline (Apple M-Series / Rust 1.93 Release)

| Benchmark | Throughput / Latency | Detail |
|---|---|---|
| **Lexer Throughput** | **104.61 MB/s** | 1,000,000 tokens in 47.59 ms |
| **Object Parser** | **54.00 MB/s** | 10,000 objects in 20.84 ms |
| **Document Open & XRef** | **2,048 ns/op** | 2.05 μs per document open |
| **Page Tree Resolution** | **766 ns/op** | 766 ns per page lookup |
| **Content Stream Parser** | **59.41 MB/s** | 90,000 instructions in 11.88 ms |

---

## 9. Verification & Quality Gates

```text
======================================================================
GATE                      STATUS      DETAILS
======================================================================
cargo fmt --check         PASSED      Formatted according to rustfmt standard
cargo clippy              PASSED      0 warnings with -D warnings & clippy::pedantic
cargo test                PASSED      61 unit & integration tests passing (100%)
cargo bench               PASSED      Micro-benchmark suite verified
cargo build --release     PASSED      Clean release binary compiled
npm run lint              PASSED      0 errors, 0 warnings
npm run typecheck         PASSED      0 errors (tsc --noEmit)
npm test                  PASSED      24 suites, 628 unit tests passing
npm run build             PASSED      Next.js Turbopack build exit code 0
npx playwright test       PASSED      23 / 23 E2E browser tests passing
======================================================================
```

---

## 10. Next Milestones (StarPDF v0.2+)

1. **Stream Filters & Decoders:** Pure Rust implementations of `FlateDecode` (zlib/deflate) and `ASCIIHexDecode`.
2. **Compressed Streams:** Support for PDF 1.5+ `/XRef` cross-reference streams and `/ObjStm` object streams.
3. **WebAssembly Bindings (`wasm-bindgen`):** Expose StarPDF to browser contexts via WebAssembly worker thread.
4. **Incremental Update Engine:** Append modifications directly to original PDF bytes without rewriting unchanged streams.
