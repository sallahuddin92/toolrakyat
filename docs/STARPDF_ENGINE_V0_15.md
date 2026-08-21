# StarPDF Engine v0.15 — Bounded Vector & Content Object Editing Specification

**Release Version:** StarPDF v0.15  
**Status:** FULLY QUALIFIED  
**Architecture:** Safe, Deterministic, Native PDF Vector Graphics Mutation Engine  
**Security & Code Quality:** `#![forbid(unsafe_code)]`, 0 `unwrap()`/`expect()` in production engine code, zero fake visual overlays.

---

## 1. Architectural Overview & Design Philosophy

StarPDF v0.15 introduces **bounded native vector graphics and path object editing** directly to the StarPDF Rust & WebAssembly engine.

### The Problem with Existing Solutions
Traditional client-side PDF tools either:
- Render HTML canvas or SVG elements over the PDF viewer, baking rasterized overlays on top while leaving the original vectors underneath unmodified.
- Cannot isolate vector paths in mixed-content PDF content streams, corrupting adjacent text (`BT...ET`), images (`Do`), or graphics state.
- Unintentionally mutate shared content streams across pages when multiple pages reference the same `/Contents` indirect object.
- Blindly attempt to parse complex shading dictionaries, arbitrary non-invertible clipping paths, and device-dependent patterns, causing rendering corruption or crashes.

### The StarPDF v0.15 Solution
StarPDF v0.15 provides deterministic, surgical manipulation of vector shapes:
1. **Zero Fake Overlays:** StarPDF directly parses, edits, and serializes the PDF content stream instructions (`re`, `m`, `l`, `h`, `S`, `s`, `f`, `f*`, `B`, `B*`, `w`, `RG`, `rg`, `G`, `g`, `K`, `k`).
2. **Stable Object Identity:** Every discovered vector shape receives a deterministic structural identifier:
   $$\text{GraphicId} = \texttt{"vec\_p\{page\}\_s\{stream\}\_i\{start\}\_\{end\}"}$$
   pinning the exact instruction subrange in the content stream.
3. **Automatic Shared Stream Clone Isolation:** When a content stream is shared across multiple pages, mutating a shape automatically clones the indirect stream on demand (`clone_if_shared`), preventing cross-page regressions.
4. **Typed Safety & Refusal Boundaries:** Complex clipping paths (`W`/`W*`), unsupported pattern color spaces, or malformed path states are cleanly detected and classified with explicit typed refusals (`VectorEditability::Refused`).
5. **Precise CTM & Coordinate Space Mapping:** Graphics state transform stacks (`q`, `Q`, `cm`) and page rotation dictionaries (`/Rotate`) are fully tracked to compute accurate user-space bounding boxes in points.
6. **Isolated Shape Insertion:** Adding new shapes (rectangles, lines) generates isolated, self-contained `q ... Q` instruction sequences that guarantee zero side-effects on subsequent drawing operations.

---

## 2. Vector Object Identity & Supported Surface

### 1. Object Identity Format
$$\text{GraphicId} = \texttt{"vec\_p\{page\_index\}\_s\{stream\_index\}\_i\{start\_instr\}\_\{end\_instr\}"}$$

- `page_index`: 0-indexed page in document page tree.
- `stream_index`: 0-indexed content stream in `/Contents` array (or 0 for single stream).
- `start_instr` / `end_instr`: Inclusive instruction bounds of the vector path and its associated style operators.

### 2. Supported v0.15 Vector Surface
- **Primitives:** Rectangle (`re`), Line / Path segments (`m`, `l`, `h`).
- **Paint Operators:** Stroke (`S`, `s`), Fill non-zero / even-odd (`f`, `f*`), Stroke & Fill (`B`, `B*`), End path (`n`).
- **Graphics Attributes:** Line width (`w`), DeviceRGB color (`RG`, `rg`), DeviceGray color (`G`, `g`), DeviceCMYK color (`K`, `k`).
- **State Management:** Nested graphics state push/pop (`q`, `Q`) and matrix concatenation (`cm`).

---

## 3. Core Vector Operations

### 1. Vector Enumeration (`enumerate_graphics` / `enumerate_all_graphics`)
Parses page content streams into a sequence of vector shapes while maintaining the graphics state stack:
- Returns `VectorGraphicInfo` containing graphic ID, type (`Rectangle`, `Line`, `Path`), user-space bounding box `[x_min, y_min, x_max, y_max]`, line width, stroke/fill status, stroke/fill color models and hex representations, and editability classification.

### 2. Update Vector Graphic (`update_graphic`)
- Moves / resizes rectangles (`re`).
- Changes stroke and/or fill colors (`RG`, `rg`, `G`, `g`, `K`, `k`).
- Changes line width (`w`).
- Modifies stroke (`S`, `s`) or fill (`f`, `B`) modes.
- Re-serializes the instruction range cleanly, preserving all surrounding content stream instructions.

### 3. Add Vector Graphic (`add_graphic` / `add_rectangle` / `add_line`)
- Appends an isolated `q ... Q` block with custom colors, line width, path operators (`re` or `m ... l`), and paint operators (`S`, `f`, `B`).
- Preserves existing text, image XObjects, and annotations.

### 4. Delete Vector Graphic (`delete_graphic`)
- Removes the instruction range corresponding to the targeted graphic.
- Strips associated styling operators (`w`, `RG`, `rg`, etc.) belonging exclusively to the target path.

---

## 4. Performance Benchmarks (Suite 73–81)

Micro-benchmark results executed on Apple Silicon (2.0–16.2 µs per operation):

| # | Benchmark Name | Iterations | Latency (ns/op) | Wall Time |
|---|---|---|---|---|
| **73** | Vector Enumeration | 10,000 | **2,297 ns** | 22.98 ms |
| **74** | Update Vector Graphic | 5,000 | **12,642 ns** | 63.21 ms |
| **75** | Add Vector Rectangle | 5,000 | **4,019 ns** | 20.10 ms |
| **76** | Add Vector Line | 5,000 | **4,310 ns** | 21.55 ms |
| **77** | Delete Vector Graphic | 5,000 | **6,754 ns** | 33.77 ms |
| **78** | Shared Vector Stream Clone | 5,000 | **10,282 ns** | 51.41 ms |
| **79** | Vector Coordinate Mapping | 10,000 | **2,345 ns** | 23.45 ms |
| **80** | Vector Export & Reopen Cycle | 2,000 | **9,622 ns** | 19.25 ms |
| **81** | 10 Sequential Vector Ops | 200 sets (2,000 ops) | **16,212 ns/set** (~1.6 µs/op) | 32.42 ms |

---

## 5. Fuzz Qualification Summary

- **Target:** `fuzz_vector_operations`
- **Executions:** **2,097,152+**
- **Crashes:** 0
- **Hangs:** 0
- **OOM / Leaks:** 0
- **Coverage:** Verified arbitrary stream contents, malformed coordinates, rapid sequential add/update/delete mutations, and shared stream cloning under high-speed libFuzzer mutation.

---

## 6. Test Suite & Verification Matrix

- **Rust Tests:** 260 passed (133 unit tests + 127 integration tests across 25 suites), 0 failed.
- **Vitest Unit/Integration Tests:** 648 passed (25 test files), 0 failed.
- **Playwright E2E Tests:** 37 passed (3 test files), 0 failed.
- **Production Safety:** `#![forbid(unsafe_code)]`, 0 `unwrap()`/`expect()` in engine production code.
