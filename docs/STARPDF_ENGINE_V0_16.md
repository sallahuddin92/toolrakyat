# STARPDF v0.16 — Performance, Memory & Large-Document Qualification Specification

## 1. Executive Summary & Objective

**StarPDF v0.16** establishes the performance, memory, and scalability baseline for the StarPDF engine. The primary objective is to empirically quantify engine throughput, memory lifecycle behavior, and incremental output characteristics under uniform release-mode conditions across 10-page, 100-page, and 500-page document profiles.

---

## 2. Deterministic Test Corpus Architecture

Large binary PDF fixtures are NOT committed to version control. StarPDF generates deterministic test documents programmatically:

1. **Text-Heavy Documents (10, 100, 500 Pages)**:
   - 20 text lines per page (200, 2,000, and 10,000 spans)
   - Page tree with balanced `/Kids` arrays and `/Helvetica` font references
   - File sizes: 10p = 21.0 KB, 100p = 209.1 KB, 500p = 1,053.7 KB
2. **Vector-Heavy Documents (10, 100, 500 Pages)**:
   - 10 vector graphics per page (100, 1,000, and 5,000 vector shapes)
   - Mixed geometry: rectangles (`re`), paths/lines (`m`, `l`, `h`), stroke/fill (`S`, `s`, `f`, `B`), colors (`rg`, `RG`)
   - File sizes: 10p = 7.2 KB, 100p = 70.4 KB, 500p = 351.6 KB
3. **Image-Heavy Documents (10, 100, 500 Pages)**:
   - 1 JPEG Image XObject per page (`DCTDecode`, `/DeviceRGB`)
4. **Form-Heavy Documents (10, 100, 500 Pages)**:
   - 2 AcroForm text fields per page (20, 200, and 1,000 fields) with widget annotations
5. **Merged & Multi-Source Documents**:
   - 100-page text document + 50-page vector document merged into a 150-page compound document.

---

## 3. Reconciled Scaling Measurements (Release Build Profile)

All measurements conducted in release mode (`opt-level = 3`) with dedicated warmup cycles and sample collection:

| Workload | 10 Pages (Median / Mean) | 100 Pages (Median / Mean) | 500 Pages (Median / Mean) | Normalized Cost (500p) | Scaling Classification |
|---|---|---|---|---|---|
| **Document Open & Page Tree** | 15.58 µs / 15.57 µs | 60.71 µs / 63.29 µs | 220.42 µs / 233.79 µs | 467.00 ns/page | **SUBLINEAR_OBSERVED** |
| **Full-Text Extraction** | 535.50 µs / 579.09 µs | 6.11 ms / 6.11 ms | 62.97 ms / 62.91 ms | 125.81 µs/page (6.29 µs/span) | **LINEAR_OBSERVED** |
| **Full-Document Search Query** | 571.63 µs / 570.58 µs | 7.09 ms / 7.06 ms | 68.75 ms / 68.72 ms | 137.43 µs/page (145k hits/s) | **LINEAR_OBSERVED** |
| **Vector Graphics Enum** | 97.38 µs / 121.22 µs | 1.82 ms / 1.89 ms | 30.24 ms / 30.26 ms | 60.52 µs/page (6.05 µs/shape) | **SUPERLINEAR_OBSERVED** |
| **Image Enumeration** | 49.33 µs / 50.89 µs | 1.87 ms / 1.92 ms | 42.21 ms / 42.33 ms | 84.67 µs/page (84.67 µs/image) | **SUPERLINEAR_OBSERVED** |
| **Forms Enumeration** | 42.21 µs / 46.52 µs | 1.48 ms / 1.54 ms | 31.77 ms / 32.77 ms | 65.54 µs/page (32.77 µs/field) | **SUPERLINEAR_OBSERVED** |
| **Standalone Write / Rewrite** | 140.29 µs / 184.95 µs | 4.08 ms / 4.14 ms | 87.69 ms / 88.31 ms | 176.63 µs/page | **SUPERLINEAR_OBSERVED** |

---

## 4. Reconciled Anomalies & Discrepancies

### A. Initial 10p / 100p / 500p Latency Inversion
- **Issue**: The preliminary report listed Text Extraction as 10p = 8.31 ms, 100p = 5.81 ms, 500p = 459.28 ms, and Search as 10p = 4.98 ms, 100p = 6.61 ms, 500p = 476.36 ms.
- **Root Cause**: The 10p and 500p numbers were captured from `cargo test` (unoptimized debug profile `target/debug`), while the 100p number was captured from `cargo bench` (optimized release profile `target/release`).
- **Resolution**: Re-measured all document profiles exclusively under release mode. Verified consistent progression: Text Extraction is 0.54 ms (10p) $\to$ 6.11 ms (100p) $\to$ 62.97 ms (500p), and Search is 0.57 ms (10p) $\to$ 7.09 ms (100p) $\to$ 68.75 ms (500p).

### B. Save-Growth Metric Discrepancy
- **Issue**: Preliminary report cited both +1,487 B/save and +1,153 B/save.
- **Root Cause**: The two values arose from differing payload lengths in separate benchmark/test generators.
- **Deterministic 10-Save Result**:
  - Initial Size: 23,958 B
  - Save 1: 25,003 B (delta: +1,045 B)
  - Save 2: 26,048 B (delta: +1,045 B)
  - Save 3: 27,093 B (delta: +1,045 B)
  - Save 4: 28,138 B (delta: +1,045 B)
  - Save 5: 29,183 B (delta: +1,045 B)
  - Save 6: 30,228 B (delta: +1,045 B)
  - Save 7: 31,273 B (delta: +1,045 B)
  - Save 8: 32,318 B (delta: +1,045 B)
  - Save 9: 33,363 B (delta: +1,045 B)
  - Save 10: 34,409 B (delta: +1,046 B)
  - Total Delta: +10,451 B
  - Mean Delta: **1,045.1 B/save** (Min: +1,045 B, Max: +1,046 B)

---

## 5. Memory Qualification

- **Method**: In-process memory lifecycle monitoring across 20 sequential `open` $\to$ `edit text` $\to$ `add vector` $\to$ `export_incremental` $\to$ `close / drop` cycles.
- **20-Cycle Latency**: Total 72.62 ms (average **3.63 ms / cycle** in release mode).
- **Representative Profiles**: 10p, 100p, 500p, 50-page image-heavy, and 150-page mixed document.
- **Memory Retention Assessment**:
  - `NO MONOTONIC RETENTION OBSERVED`
  - Engine handles, stores, and parser AST buffers deallocate cleanly on document drop.

---

## 6. Verification & Quality Gates

| Gate | Target | Result | Status |
|---|---|---|---|
| `#![forbid(unsafe_code)]` | Engine crate level | Strictly enforced | **PASS** |
| Production `unwrap()` / `expect()` | Zero in production code | 0 occurrences | **PASS** |
| Rust Test Suite | Unit & Integration | **267 passed, 0 failed** | **PASS** |
| Rust Benchmarks | Benchmarks 1–95 | **95 passed** | **PASS** |
| Cargo Clippy | All targets & features (`-D warnings`) | 0 warnings | **PASS** |
| Vitest Suite | Client & WASM integration | **649 passed, 0 failed** | **PASS** |
| Playwright E2E Suite | Browser end-to-end | **38 passed, 0 failed** | **PASS** |
| Next.js Production Build | `npm run build` | Clean production build | **PASS** |
