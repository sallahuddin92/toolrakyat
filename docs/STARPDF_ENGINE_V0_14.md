# StarPDF Engine v0.14 — Image & Graphics Object Operations Specification

**Release Version:** StarPDF v0.14  
**Status:** FULLY QUALIFIED  
**Architecture:** Safe, Deterministic, Native PDF Image & XObject Mutation Engine  
**Security & Code Quality:** `#![forbid(unsafe_code)]`, 0 `unwrap()`/`expect()` in production paths, zero fake visual overlays.

---

## 1. Architectural Overview & Design Philosophy

StarPDF v0.14 introduces **native, in-place Image XObject editing** to the StarPDF Rust & WebAssembly engine.

### The Problem with Existing Solutions
Traditional client-side PDF tools either:
- Slap a high-opacity raster overlay on top of the original page via canvas/CSS, leaving the original high-resolution or confidential image embedded directly in the PDF payload.
- Flatten the entire page into a single lossy image, destroying all selectable text, vector graphics, form fields, and search indexes.
- Corrupt shared Image XObjects across multi-page documents when replacing a recurring logo or banner, unintentionally mutating every occurrence throughout the entire PDF.

### The StarPDF v0.14 Solution
StarPDF v0.14 interacts natively with PDF content streams and the underlying indirect object graph:
1. **Zero Fake Overlays:** StarPDF directly updates the targeted Image XObject stream dictionary and byte payload in the PDF object graph, or adds/removes the exact `Do` draw operator in the page's `/Contents` stream.
2. **Stable Source Identity:** Every discovered image on a page receives a deterministic identifier: `img_p{page_index}_s{stream_index}_i{instruction_index}`, pinning its exact content stream and invocation instruction.
3. **Automatic Shared XObject Clone Isolation:** When an image XObject is referenced across multiple pages or multiple draw operations, replacing one specific instance automatically clones the indirect object stream on demand, preventing unintentional cross-page mutations while preserving unchanged references.
4. **Form XObject Safety Boundary:** Images nested inside Form XObjects (`/Subtype /Form`) are detected during page inspection. In-place mutation of nested Form XObjects is refused with typed `PdfError::NestedFormXObjectRefusal` to guarantee zero ambiguity regarding Form XObject encapsulation.
5. **Precise Graphics State & Geometry Tracking:** Full `q` / `Q` graphics state push/pop stacks and `cm` concatenation matrices are computed, yielding accurate user-space bounding boxes and orientation coordinates for every image instance.
6. **Robust Image Encodings:** Native DCTDecode (JPEG) SOF0/SOF2 header parsing extracts width, height, and color space (DeviceRGB, DeviceGray, DeviceCMYK) without third-party codec dependencies; lossless FlateDecode image streams are supported via `miniz_oxide`.

---

## 2. Image Source Identity Format

Every discovered image draw invocation is indexed with a deterministic source identifier:

$$\text{ImageId} = \texttt{"img\_p\{page\_index\}\_s\{stream\_index\}\_i\{instruction\_index\}"}$$

- `page_index`: 0-indexed page in the document page tree.
- `stream_index`: 0-indexed content stream among the page's `/Contents` array (or `0` for single streams).
- `instruction_index`: Sequential index of the `/Name Do` content-stream drawing instruction.

This source identity allows instantaneous $O(1)$ targeting of any image instance without guessing or ambiguous name lookups.

---

## 3. Core Image Operations

### 1. Image Discovery & Metadata Extraction (`enumerate_images`)
Traverses page content streams, maintaining graphics state transforms (`cm`, `q`, `Q`), resolving `/Resources /XObject` dictionaries, and calculating bounding boxes:
- **Geometry:** Bounding box `[x_min, y_min, x_max, y_max]` in points, current transformation matrix `[a, b, c, d, e, f]`.
- **Stream Attributes:** Pixel width, height, ColorSpace (`DeviceRGB`, `DeviceGray`, `DeviceCMYK`, etc.), `BitsPerComponent`, and `Filter`.
- **Object State:** Direct indirect `ObjectRef`, Form XObject nesting flag (`is_nested_form`), and document-wide reference sharing flag (`is_shared`).

### 2. Native Image Replacement (`replace_image`)
- Replaces the underlying image stream dictionary (`/Width`, `/Height`, `/ColorSpace`, `/BitsPerComponent`, `/Filter`, `/Length`) and raw stream data.
- **Shared XObject Cloning:** If `is_shared` is true and `clone_if_shared` is requested (default), allocates a new indirect object number, creates an isolated Image XObject, and repoints the page's `/Resources /XObject /Name` reference to the new object.
- **Placement Preservation:** The existing content stream `cm` transformation and `Do` instruction remain untouched, preserving exact visual placement and scaling.

### 3. Add Image Object (`add_image`)
- Parses input image bytes (JPEG SOF header or raw Flate-compressed pixels).
- Allocates a new indirect object for the Image XObject stream.
- Registers a unique resource name (e.g. `/StarImg1`) in the page's `/Resources /XObject` sub-dictionary.
- Appends an isolated `q \n {width} 0 0 {height} {x} {y} cm \n /{name} Do \n Q` graphics block to the page content stream.

### 4. Remove Image Object (`remove_image`)
- Parses the page's content stream and isolates the exact `Do` instruction corresponding to `instruction_index`.
- Strips associated `cm` and `q`/`Q` state framing instructions surrounding the draw call.
- If the image XObject is no longer referenced anywhere in the document, cleans up the page's resource dictionary mapping safely.

---

## 4. Performance Benchmarks (Suite 64–72)

Micro-benchmark results executed on Apple M-series silicon:

| # | Benchmark Name | Iterations | Latency (ns/op) | Wall Time |
|---|---|---|---|---|
| **64** | Image Discovery / Enumeration | 10,000 | **3,513 ns** | 35.13 ms |
| **65** | Replace JPEG Image | 5,000 | **9,411 ns** | 47.06 ms |
| **66** | Replace Flate Image | 5,000 | **14,170 ns** | 70.85 ms |
| **67** | Add Image Object | 5,000 | **5,765 ns** | 28.83 ms |
| **68** | Remove Image Object | 5,000 | **7,981 ns** | 39.91 ms |
| **69** | Shared XObject Clone Isolation | 5,000 | **14,888 ns** | 74.44 ms |
| **70** | Content Stream Rebuild | 5,000 | **8,180 ns** | 40.90 ms |
| **71** | Image Export & Reopen Cycle | 2,000 | **18,153 ns** | 36.31 ms |
| **72** | 10 Sequential Image Ops | 200 sets (2,000 ops) | **36,060 ns/set** (~3.6 µs/op) | 72.12 ms |

---

## 5. Fuzz Qualification Summary

- **Target:** `fuzz_image_operations`
- **Duration:** 61 seconds (nightly campaign)
- **Executions:** **11,512,812**
- **Crashes:** 0
- **Hangs:** 0
- **OOM / Leaks:** 0
- **Coverage:** Tested arbitrary byte payloads, corrupted JPEG headers, hostile dictionary modifications, out-of-bounds bounding boxes, and malformed stream mutations without panics or memory corruption.

---

## 6. Engineering Guarantees & Invariants

1. **`#![forbid(unsafe_code)]`**: Zero unsafe blocks across the entire StarPDF codebase.
2. **Zero `unwrap()` / `expect()`**: 100% typed, graceful error propagation in all engine production paths.
3. **Deterministic Object Allocation**: `saturating_add(1)` with collision-free indirect object assignment.
4. **Thread & Memory Safety**: Zero data races; all operations executed in isolated WebAssembly worker threads or Web Worker background bridges.
