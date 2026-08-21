# STARPDF v0.16 — Performance, Memory & Large-Document Qualification Specification

## 1. Executive Summary & Objective

**StarPDF v0.16** establishes the performance, memory, and scalability baseline for the StarPDF engine. The objective is to empirically quantify engine throughput, process memory behavior (Resident Set Size via macOS mach task info), and incremental save output characteristics across 10-page, 100-page, and 500-page document profiles in optimized release mode (`opt-level = 3`).

---

## 2. Deterministic Test Corpus Architecture

Large binary fixtures are generated programmatically to maintain repository hygiene:

1. **Text-Heavy Documents (10, 100, 500 Pages)**:
   - 20 text lines per page (200, 2,000, and 10,000 spans)
   - File sizes: 10p = 21.0 KB, 100p = 209.1 KB, 500p = 1,053.7 KB
2. **Vector-Heavy Documents (10, 100, 500 Pages)**:
   - 10 vector graphics per page (100, 1,000, and 5,000 shapes)
   - Mixed geometry: rectangles (`re`), paths (`m`, `l`, `h`), stroke/fill (`S`, `s`, `f`, `B`), colors (`rg`, `RG`)
   - File sizes: 10p = 7.2 KB, 100p = 70.4 KB, 500p = 351.6 KB
3. **Image-Heavy Documents (10, 100, 500 Pages)**:
   - 1 JPEG Image XObject per page (`DCTDecode`, `/DeviceRGB`)
4. **Form-Heavy Documents (10, 100, 500 Pages)**:
   - 2 AcroForm text fields per page (20, 200, and 1,000 fields) with widget annotations

---

## 3. Reconciled Scaling Measurements & Empirical Classifications

All metrics captured under uniform release-mode execution (`target/release`, `opt-level = 3`):

| Workload | 10 Pages (Median) | 100 Pages (Median) | 500 Pages (Median) | 10 $\to$ 100 Ratio (10x input) | 100 $\to$ 500 Ratio (5x input) | Per-Unit Trend (10p $\to$ 100p $\to$ 500p) | Empirical Classification |
|---|---|---|---|---|---|---|---|
| **Document Open** | 15.58 µs | 64.04 µs | 219.71 µs | **4.11x** | **3.43x** | 1.56 µs/p $\to$ 0.64 µs/p $\to$ 0.44 µs/p | **SUBLINEAR_OBSERVED** |
| **Text Extraction** | 0.525 ms | 6.05 ms | 62.43 ms | **11.52x** | **10.32x** | 52.5 µs/p $\to$ 60.5 µs/p $\to$ 124.9 µs/p | **SUPERLINEAR_OBSERVED** |
| **Search Query** | 0.573 ms | 6.97 ms | 66.79 ms | **12.16x** | **9.58x** | 57.3 µs/p $\to$ 69.7 µs/p $\to$ 133.6 µs/p | **SUPERLINEAR_OBSERVED** |
| **Vector Enum** | 0.095 ms | 1.81 ms | 29.12 ms | **19.05x** | **16.09x** | 0.95 µs/s $\to$ 1.81 µs/s $\to$ 5.82 µs/s | **SUPERLINEAR_OBSERVED** |
| **Image Enum** | 0.048 ms | 1.84 ms | 40.37 ms | **38.33x** | **21.94x** | 4.80 µs/img $\to$ 18.40 µs/img $\to$ 80.74 µs/img | **SUPERLINEAR_OBSERVED** |
| **Forms Enum** | 0.042 ms | 1.43 ms | 30.48 ms | **34.05x** | **21.31x** | 2.10 µs/f $\to$ 7.15 µs/f $\to$ 30.48 µs/f | **SUPERLINEAR_OBSERVED** |
| **Standalone Write** | 0.139 ms | 4.02 ms | 83.51 ms | **28.92x** | **20.77x** | 13.9 µs/p $\to$ 40.2 µs/p $\to$ 167.0 µs/p | **SUPERLINEAR_OBSERVED** |

---

## 4. Investigation of 100 $\to$ 500 Page Scaling Behavior

1. **Document Open (`SUBLINEAR_OBSERVED`)**:
   - Amortized parser setup costs cause per-page parsing time to decrease from 1.56 µs/page on 10p down to 0.44 µs/page on 500p.
2. **Text Extraction & Search (`SUPERLINEAR_OBSERVED`)**:
   - Latency growth from 100 $\to$ 500 pages (10.32x for extraction, 9.58x for search) exceeds the 5x input increase.
   - *Root Cause*: Page tree traversal executes sequentially across all 500 content streams, resolving font dictionaries and character widths for 10,000 text spans without intermediate span indexing or caching.
3. **Vector, Image & Form Enumeration (`SUPERLINEAR_OBSERVED`)**:
   - Latency growth from 100 $\to$ 500 pages (16.09x for vectors, 21.94x for images, 21.31x for forms) significantly exceeds the 5x input increase.
   - *Root Cause*: Whole-document enumeration methods allocate and clone unified result vectors containing thousands of structured DTOs (e.g. 5,000 vector shapes, 1,000 form field descriptors).
4. **Standalone Serialization Writing (`SUPERLINEAR_OBSERVED`)**:
   - Latency growth from 100 $\to$ 500 pages (20.77x) exceeds the 5x input increase.
   - *Root Cause*: Full serialization performs deep cloning of the entire indirect object graph, re-indexing cross-reference tables, and stringifying offsets over thousands of objects.

---

## 5. Process Memory Qualification (Resident Set Size via macOS Mach Task Info)

Memory measured via macOS `mach_task_basic_info` (`resident_size`):

- **Baseline Process RSS**: `1.97 MB` (2,064,384 bytes)
- **10-Page Document**:
  - Baseline: 2.23 MB
  - Peak (during extraction & search): **3.52 MB**
  - After Close / Drop: **3.52 MB**
- **100-Page Document**:
  - Baseline: 3.80 MB
  - Peak (during extraction & search): **6.52 MB**
  - After Close / Drop: **6.52 MB**
- **500-Page Document**:
  - Baseline: 7.39 MB
  - Peak (during extraction & search): **18.69 MB**
  - After Close / Drop: **18.69 MB**

### 200 Repeated Open $\to$ Edit $\to$ Save $\to$ Close Cycles
- **cycle0**: `18.70 MB` (19,611,648 bytes)
- **cycle1**: `18.91 MB` (19,824,640 bytes)
- **cycle5**: `18.95 MB` (19,873,792 bytes)
- **cycle10**: `18.98 MB` (19,906,560 bytes)
- **cycle20**: `19.02 MB` (19,939,328 bytes)
- **cycle40**: `19.05 MB` (19,972,096 bytes)
- **cycle60**: `19.12 MB` (20,054,016 bytes)
- **cycle80**: `19.12 MB` (20,054,016 bytes)
- **cycle100**: `19.12 MB` (20,054,016 bytes)
- **cycle125**: `19.12 MB` (20,054,016 bytes)
- **cycle150**: `19.12 MB` (20,054,016 bytes)
- **cycle175**: `19.12 MB` (20,054,016 bytes)
- **cycle200**: `19.12 MB` (20,054,016 bytes)
- **Peak RSS**: `19.12 MB` (20,054,016 bytes)

### Growth Slopes
- **0 $\to$ 20**: `16,384.0 bytes/cycle` (initial allocator arena acquisition)
- **20 $\to$ 40**: `1,638.4 bytes/cycle`
- **40 $\to$ 100**: `1,365.3 bytes/cycle`
- **100 $\to$ 150**: `0.0 bytes/cycle`
- **150 $\to$ 200**: `0.0 bytes/cycle`
- **Total Delta (cycle200 - cycle0)**: `+432 KB` (+442,368 bytes)
- **Classification**: **`PLATEAU_OBSERVED`**

---

## 6. Deterministic 10-Save Output Growth

- **Initial Document Size**: 23,958 B (20-page document)
- **Save 1**: 25,003 B (delta: +1,045 B)
- **Save 2**: 26,048 B (delta: +1,045 B)
- **Save 3**: 27,093 B (delta: +1,045 B)
- **Save 4**: 28,138 B (delta: +1,045 B)
- **Save 5**: 29,183 B (delta: +1,045 B)
- **Save 6**: 30,228 B (delta: +1,045 B)
- **Save 7**: 31,273 B (delta: +1,045 B)
- **Save 8**: 32,318 B (delta: +1,045 B)
- **Save 9**: 33,363 B (delta: +1,045 B)
- **Save 10**: 34,409 B (delta: +1,046 B)
- **Total Delta**: **+10,451 B**
- **Mean Delta**: **1,045.1 B/save** (Min: +1,045 B, Max: +1,046 B)

---

## 7. Verification & Quality Gates

| Gate | Target | Result | Status |
|---|---|---|---|
| `#![forbid(unsafe_code)]` | Engine crate level | Strictly enforced | **PASS** |
| Production `unwrap()` / `expect()` | Zero in production code | 0 occurrences | **PASS** |
| Rust Formatting | `cargo fmt --check` | Clean | **PASS** |
| Cargo Clippy | All targets & features (`-D warnings`) | 0 warnings | **PASS** |
| Rust Test Suite | Large document qualification | **3 passed, 0 failed** | **PASS** |
