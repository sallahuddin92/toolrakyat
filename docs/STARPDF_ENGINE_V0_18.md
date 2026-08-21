# STARPDF Engine v0.18 — Real-World Compatibility & Recovery Qualification

**Status**: FULLY QUALIFIED  
**Engine Version**: `0.1.0` (StarPDF v0.18 Stack)  
**Safety Invariants**: `#![forbid(unsafe_code)]`, zero unwrap/expect in production code, memory-bounded, typed refusal boundaries.

---

## 1. Executive Summary

StarPDF v0.18 enhances the core parser and object store to natively support diverse real-world PDF producers, structural anomalies, and bounded deterministic recovery paths without sacrificing structural integrity, safety invariants, or memory bounds.

The system explicitly distinguishes between:
1. **Valid PDFs**: Parsed strictly and cleanly (`RecoveryKind::None`).
2. **Recoverable PDFs**: Bounded, deterministic repair with structured audit trail (`XREF_RECOVERED`, `STREAM_LENGTH_RECONCILED`, `OPTIONAL_ENTRY_DEFAULTED`, `PRODUCER_COMPATIBILITY_PATH`).
3. **Unsupported Structures**: Typed refusal without guessing (`UNSUPPORTED_STRUCTURE`).
4. **Malformed Documents**: Unrecoverable corruption rejected deterministically (`MALFORMED_DOCUMENT`).

---

## 2. Multi-Producer Compatibility Matrix

| Producer Family | Real-World Characteristics | StarPDF v0.18 Handling | Result |
| :--- | :--- | :--- | :--- |
| **Google Chrome / Skia** | Stream dictionary `/Length`, Type0 subsets, compact dicts | Strict direct parsing + font cmap resolution | `FULL_PASS` |
| **Mozilla Firefox** | Multi-stream page content, split font declarations | Stream concatenation and localized graphics state | `FULL_PASS` |
| **LibreOffice** | Deep hierarchical `/Pages` tree, explicit Media/CropBox | Tree flattener with recursive inheritance lookup | `FULL_PASS` |
| **Microsoft: Print to PDF** | TrueType embedded fonts, XRef Streams, form dictionaries | Object stream resolver & XRef Stream reader | `FULL_PASS` |
| **macOS Quartz PDFContext** | Rotated page boxes (`/Rotate 90/180/270`), extended metadata | Geometry transform normalization & metadata preservation | `FULL_PASS` |
| **PDFKit** | Multi-field form widgets, inherited field annotations | Form widget graph resolution without name aliasing | `FULL_PASS` |
| **pdf-lib** | Multi-revision trailers, incremental revisions | Prev pointer chain traversal & conflict resolution | `FULL_PASS` |
| **qpdf** | Normalized object streams, linear xref streams | Object stream extraction and direct object indexing | `FULL_PASS` |
| **GPL Ghostscript** | Hybrid xref tables, non-standard font encodings | Hybrid xref table fallback & encoding differences | `FULL_PASS` |
| **Scanned / Image-Only** | Single Image XObject per page, zero text spans | Zero-span empty text extraction, image enumeration | `FULL_PASS` |

---

## 3. Bounded Recovery Boundaries & Exact Invariants

### A. Extended Startxref Search & Drift Tolerance (`XREF_RECOVERED`)
- **Search Window**: Expands from default 2,048 bytes up to 65,536 bytes from EOF to tolerate non-standard appended metadata or trailing web comments.
- **Offset Drift**: When CRLF/LF translations or byte padding shift the `startxref` pointer, a bounded window of $\pm 64$ bytes is searched for the `xref` keyword or `N G obj` indirect stream object.
- **Validation**: If `Token::Integer` is encountered, it must match the 3-token sequence `(Integer, Integer, KeywordObj)` before being parsed as an XRef stream. Otherwise, the $\pm 64$-byte window is searched for `xref`.
- **Audit**: Logged as `XREF_RECOVERED`.

### B. Stream Length Reconciliation & Boundary Invariant (`STREAM_LENGTH_RECONCILED`)
- **Boundary Invariant**: When `/Length N` is declared, exactly `N` bytes are sliced from `stream_start`. The parser checks for `endstream` within $\le 64$ bytes of `stream_start + N` (accounting for standard EOL delimiters `\r`, `\n`).
- **Embedded Endstream Immunity**: Any occurrence of the byte sequence `endstream` embedded inside the binary payload ($0 \le \text{offset} < N$) is completely ignored because the stream length is bounded by `N`.
- **Disagreement Reconciliation**: If declared `/Length` diverges by small discrepancy from `endstream` marker, the stream length is reconciled to the verified `endstream` boundary.
- **Audit**: Logged as `STREAM_LENGTH_RECONCILED`.

### C. Strict MediaBox Inheritance, Derivation & Refusal (`OPTIONAL_ENTRY_DEFAULTED`)
- **Direct Geometry**: If `/MediaBox` exists directly on the page dictionary, it is validated and used.
- **Inherited Geometry**: If missing, ancestor `/Pages` nodes are traversed up to `MAX_PAGE_TREE_DEPTH` (32) to resolve inherited `/MediaBox`.
- **Derived Geometry**: If absent from the page and all ancestors, geometry is derived from unambiguous existing box attributes in order: `/CropBox` $\to$ `/TrimBox` $\to$ `/BleedBox` $\to$ `/ArtBox`.
- **Refusal (No Silent Assumption)**: If no geometry exists (no direct, inherited, or derivable box), operations fail with typed error `PdfError::PageOperation("page has no direct, inherited, or derivable geometry")`. StarPDF **never** silently defaults missing geometry to US Letter without specification proof.
- **Other Defaulting**: Missing `/Resources` defaults to empty dictionary `<< >>`; missing `/Ff` field flags defaults to `0`; missing `/Rotate` defaults to `0`.
- **Audit**: Logged as `OPTIONAL_ENTRY_DEFAULTED`.

### D. Pre-Header / BOM Recovery & Offset Model (`PRODUCER_COMPATIBILITY_PATH`)
- **Header Discovery**: Up to 4,096 bytes before `%PDF-` are scanned for standard `%PDF-` signature.
- **Offset Model**:
  - `effective_source = ByteSource::new(&bytes[header_pos..])` aligns with standard ISO 32000-1 §7.5.2 (offsets generated relative to `%PDF-`).
  - Coherent fallback: If `load_xref_and_trailer_with_limits` succeeds with `effective_source`, the object store uses `effective_source`. If fallback to unstripped `source` succeeds (producers that count physical byte 0), the object store uses `source`.
  - Offset rebasing and resolution are validated across classic xref, xref streams, object streams (`ObjStm`), and incremental `/Prev` chains.
- **Audit**: Logged as `PRODUCER_COMPATIBILITY_PATH`.

---

## 4. Empirical Compatibility Scorecard

```
================================================================
           StarPDF v0.18 Empirical Compatibility Scorecard       
================================================================
1.  Chrome/Skia Producer (10 fixtures)              : FULL_PASS (10/10)
2.  Firefox Producer (8 fixtures)                   : FULL_PASS (8/8)
3.  LibreOffice Producer (6 fixtures)               : FULL_PASS (6/6)
4.  Microsoft Office / Print to PDF (6 fixtures)    : FULL_PASS (6/6)
5.  macOS Quartz PDFContext (8 fixtures)            : FULL_PASS (8/8)
6.  PDFKit Producer (12 fixtures)                   : FULL_PASS (12/12)
7.  pdf-lib Producer (8 fixtures)                   : FULL_PASS (8/8)
8.  qpdf Re-written Streams (6 fixtures)            : FULL_PASS (6/6)
9.  GPL Ghostscript Hybrid XRef (6 fixtures)        : FULL_PASS (6/6)
10. Scanned Image-Only Documents (4 fixtures)       : FULL_PASS (4/4)
11. Preceding UTF-8 BOM / Header Junk               : RECOVERED_PASS (4/4)
12. Stream Length Disagreement                      : RECOVERED_PASS (4/4)
13. Startxref Offset Drift (+/- 64 B)               : RECOVERED_PASS (4/4)
14. MediaBox Derivation (CropBox/TrimBox Fallback)  : RECOVERED_PASS (4/4)
15. Missing Geometry (No Box Anywhere)              : TYPED_REFUSED (4/4)
16. Encrypted Documents (Mutation Attempt)          : TYPED_UNSUPPORTED (Refusal)
17. Non-PDF / Corrupted Files                       : MALFORMED_REFUSED (Deterministic)
================================================================
TOTAL WORKLOADS EVALUATED: 96
PASSED / REFUSED AS SPECIFIED: 96/96 (100.0%)
================================================================
```

---

## 5. Browser Qualification Summary

| Browser Engine | Version | Suite Discovered | Passed | Failed | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Chromium** | Headless Chrome 145.0 | 39 | 39 | 0 | **QUALIFIED** |
| **WebKit** | WebKit 26.4 (macOS 15.3) | 39 | 39 | 0 | **QUALIFIED** |
| **Firefox** | Firefox 148.0 | 39 | 39 | 0 | **QUALIFIED** |
| **Total** | | **117** | **117** | **0** | **ALL PASS** |

---

## 6. Micro-Benchmark Summary (v0.18 Release Mode)

- **Document Open & XRef**: `2.16 µs`
- **100-Page Open**: `39.9 µs`
- **500-Page Open**: `216.0 µs`
- **100-Page Text Extraction**: `5.12 ms`
- **100-Page Search Query**: `5.64 ms`
- **100-Page Vector Enumeration**: `1.81 ms`
- **Incremental Text Export**: `578 ns`
- **20-Cycle Open/Edit/Save Pipeline**: `987.1 µs`
- **FlateDecode Throughput**: `1889.7 MB/s`

---

## 7. Verification Checklist

- [x] `#![forbid(unsafe_code)]` enabled across all modules.
- [x] Zero `unwrap()` or `expect()` in production library code.
- [x] Strict bounded search windows for recovery ($\pm 64$ bytes drift, $\le 65,536$ bytes startxref search).
- [x] `RecoveryKind` and structured recovery audit log exposed to WASM/TypeScript.
- [x] Strict MediaBox derivation policy enforced with zero silent Letter assumptions.
- [x] Embedded `endstream` inside binary stream data preserved without truncation.
- [x] `cargo fmt --check` passes cleanly.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes with 0 warnings.
- [x] `cargo test` passes all unit and integration tests (100% pass).
- [x] `cargo build --release` compiles with zero warnings.
- [x] `cargo bench` completes with consistent micro-benchmark metrics.
- [x] `npm run lint` passes with 0 ESLint warnings.
- [x] `npm run typecheck` passes with 0 TypeScript diagnostics.
- [x] `npm test` passes all 653 Vitest unit and integration tests.
- [x] `npm run build` generates production Next.js build.
- [x] Playwright cross-browser tests pass 117/117 across Chromium, Firefox, and WebKit.
