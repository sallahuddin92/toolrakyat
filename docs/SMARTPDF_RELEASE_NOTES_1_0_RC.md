# SmartPDF 1.0 Release Candidate Release Notes
### Powered by StarPDF Engine

SmartPDF 1.0 Release Candidate (RC) represents a comprehensive, desktop-grade, local-first PDF editor running entirely in-browser via StarPDF WebAssembly.

---

## 1. Product Identity & Architecture

- **Product Name**: SmartPDF
- **Engine**: StarPDF WebAssembly Rust Engine (`engine/starpdf`)
- **Execution Architecture**: 100% Client-Side WebAssembly (Zero bytes of PDF payloads uploaded to servers).
- **Safety Invariant**: `#![forbid(unsafe_code)]` in Rust engine core with strict bounds checking and memory isolation.

---

## 2. Major Capabilities Delivered

### A. Direct Manipulation & Native Typography
- **Direct Canvas Selection**: Select text spans, raster images, vector shapes, form fields, and annotations directly on the rendered page canvas.
- **Native Text Mutation**: In-place single-span and multi-span atomic text replacement with layout safety locks preventing downstream text displacement.
- **Text Move & Delete**: Drag text blocks spatially to update transformation matrices (`Tm`); delete or clear text content cleanly.
- **Safe Refusal on Complex Typography**: Type3 bitmap fonts or embedded fonts missing `/ToUnicode` CMaps produce safe, typed user notices (`"This text can't be safely rewritten."`) rather than lossy approximate substitutions.

### B. Fill & Sign & Markup Annotations
- **Type Anywhere**: Click anywhere to place FreeText annotations with clean, borderless defaults.
- **Quick Marks & Signatures**: Add checkmarks, crossmarks, lines, rectangles, and draw/ink annotations.
- **Image Signatures & Stamps**: Insert transparent PNG/JPEG signature images, move, and resize directly on canvas.
- **Full Annotation Suite**: Select, style, move, and delete FreeText, Ink, Highlight, Underline, Square, Circle, and Link annotations.

### C. Interactive AcroForms & Widgets
- **Direct Spatial Form Editing**: Edit text fields, toggle checkboxes, select mutually exclusive radio buttons, and choose dropdown options.
- **ISO 32000 Appearance Regeneration**: Automatically regenerates `/AP` appearance streams for maximum visual fidelity across external PDF readers.

### D. Advanced Page Organizer & Multi-Document Workflows
- **Left Rail Organizer**: Multi-selection via `Click`, `Cmd/Ctrl + Click`, and `Shift + Click` range selection.
- **Drag & Drop Reordering**: HTML5 drag-and-drop page reordering with edge auto-scrolling and cancellation safeguards.
- **Batch Operations**: Multi-page batch duplication, deletion, and blank page insertion.
- **Extraction & Splitting**: Extract selected pages, split documents by page ranges (e.g. `1-2, 3-5`), or burst into single-page PDFs.
- **Multi-Document Merge**: Append full secondary PDFs or import selected page ranges into target positions.

### E. Search, Keyboard Workflows & File UX
- **StarPDF Engine Search**: Direct engine bounding box highlighting with zoom, rotation, and multi-page support.
- **Global Keyboard Shortcuts**: `Cmd/Ctrl+F` (search), `Cmd/Ctrl+S` (export), `Cmd/Ctrl+Z` (undo), `Cmd/Ctrl+Shift+Z` / `Ctrl+Y` (redo), `Delete`/`Backspace` (delete), and `Escape` (dismissal). Strict typing safety suppresses global shortcuts when editing text inputs.
- **File Drag & Drop**: Native drag-and-drop file opening with unsaved-changes confirmation dialog.
- **Save State Lifecycle**: Dynamic `Saved` vs `Unsaved changes` status bar tracking.

---

## 3. Browser Qualification

| Browser Engine | Test Count | Result | Status |
| :--- | :---: | :---: | :---: |
| **Chromium** | 77 / 77 | 100% PASS | Qualified |
| **Firefox** | 77 / 77 | 100% PASS | Qualified |
| **Playwright WebKit** | 77 / 77 | 100% PASS | Qualified |
| **Safari** | — | — | *Not directly tested (Playwright WebKit qualified)* |

**Total Suite**: **231 / 231 Passed across all 3 engines.**

---

## 4. Known Limitations & Boundaries

1. **Specialized Fonts**: Non-rewritable fonts without standard `/ToUnicode` CMaps safely refuse native replacement with user notification.
2. **Arbitrary Paragraph Reflow**: Reflow across arbitrary multi-line paragraphs with external fonts is intentionally unsupported to preserve typographic integrity.
3. **Complex Bezier Nodes**: Complex bezier curve paths are manipulated as bounding units; individual knot/anchor point editing is not exposed.
4. **AcroForm Multi-Select Listboxes**: Multi-select listboxes are displayed as single-selection dropdowns in current UI.
5. **Standalone Compaction**: Rust engine `CompleteWriter` compaction is implemented; WASM export pipeline omitted in current build.
6. **Encrypted Documents**: Password-protected (standard or public-key) PDFs produce typed refusal (`ENCRYPTED_DOCUMENT`).
7. **Digital Signatures**: Existing signature byte ranges / structures are preserved where applicable on incremental export. Cryptographic public-key verification is **NOT PERFORMED / NOT SUPPORTED**.

---

## 5. Compliance & Legal Notice

- **Dependency Audit**: 41 production dependencies audited for permissive OSS compliance (MIT, Apache 2.0, BSD-2/3, ISC). 0 unresolved metadata review items.
- **Formal Legal Review**: **NOT PERFORMED**.
