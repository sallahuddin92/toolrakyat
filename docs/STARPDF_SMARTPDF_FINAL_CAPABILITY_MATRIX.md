# StarPDF / SmartPDF Final Capability Matrix (Phase 8 RC Freeze)

This document provides the final, complete capability inventory of the StarPDF Rust/WASM engine and SmartPDF web application upon Release Candidate qualification.

---

## 1. Classification Summary

| Classification | Count | Description |
| :--- | :---: | :--- |
| **`FULLY_EXPOSED`** | **42** | Complete Rust engine -> WASM -> TS Client -> SmartPDF UI -> Direct Manipulation -> Export/Reopen pipeline fully functional and qualified. |
| **`PARTIALLY_EXPOSED`** | **1** | Bounded direct manipulation exposed (e.g. vector styling/bounds manipulation), with granular internal operations (individual bezier node control) not exposed. |
| **`ENGINE_READY_UI_MISSING`** | **2** | Engine and WASM bindings exist and are tested, but user-facing UI triggers are omitted. |
| **`INTENTIONALLY_NOT_EXPOSED`** | **1** | Omitted intentionally to prevent document corruption or layout destruction. |
| **`SAFE_REFUSAL`** | **2** | Deterministically refused at engine/UI boundaries with clear typed user feedback to guarantee document integrity. |
| **`ENGINE_GAP`** | **1** | Engine core capability exists in Rust but lacks WASM binding or pipeline integration. |
| **TOTAL AUDITED** | **49** | Comprehensive coverage across all document editing, manipulation, and workflow domains. |

---

## 2. Detailed Capability Breakdown

### A. Document Lifecycle & Security

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 1 | **Document Open & Parsing** | `FULLY_EXPOSED` | Robust parsing of standard, hybrid xref, linearized, and multi-revision documents. |
| 2 | **Fault-Tolerant Recovery** | `FULLY_EXPOSED` | Recovers corrupt xref tables, reconstructs missing `startxref`, fixes binary stream lengths. |
| 3 | **Document Metadata & Metrics** | `FULLY_EXPOSED` | Extracts title, author, producer, page count, and dimensions across all pages. |
| 4 | **Digital Signature Detection** | `FULLY_EXPOSED` | Non-destructive byte-range preservation; displays signature notice without claiming crypto validation. |
| 5 | **Password Encryption Refusal** | `SAFE_REFUSAL` | Deterministically refuses Standard & Public-Key encrypted files with `ENCRYPTED_DOCUMENT`. |
| 6 | **Stale Handle Protection** | `FULLY_EXPOSED` | Safe handle lifecycle in Rust engine registry preventing double-free or stale document access. |
| 7 | **Document Close / Reset** | `FULLY_EXPOSED` | Explicit memory reclamation on document transitions with 0 leaks. |

### B. Native Text Editing & Typography

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 8 | **Text Extraction & Bounds** | `FULLY_EXPOSED` | Glyph and span-level bounding box calculation with rotation and CTM transformation. |
| 9 | **Batch All-Text Extraction** | `ENGINE_READY_UI_MISSING` | `starpdf_extract_all_text` available in WASM/Client; UI text export not exposed. |
| 10 | **Text Editability Check** | `FULLY_EXPOSED` | Verifies font type, encoding, and `/ToUnicode` CMap before permitting edit. |
| 11 | **Single-Span Text Replacement** | `FULLY_EXPOSED` | In-place text mutation preserving font size, color, baseline, and downstream layout. |
| 12 | **Multi-Span Text Replacement** | `FULLY_EXPOSED` | Atomic replacement across grouped heading/body spans with layout safety locks. |
| 13 | **Native Text Deletion / Clear** | `FULLY_EXPOSED` | Removes string operators and advances while preserving downstream glyph layout. |
| 14 | **Native Text Drag Move** | `FULLY_EXPOSED` | Moves text span / group `Tm` matrix coordinates via direct spatial dragging. |
| 15 | **Non-Rewritable Font Refusal** | `SAFE_REFUSAL` | Displays clean read-only banner for Type3 or unmapped fonts (`"This text can't be safely rewritten."`). |
| 16 | **Type0 / Identity-H CMaps** | `FULLY_EXPOSED` | Full support for Unicode composite fonts and CJK glyph mappings. |
| 17 | **Arbitrary Text Paragraph Reflow** | `INTENTIONALLY_NOT_EXPOSED` | Arbitrary multi-line reflow across subsetted embedded fonts omitted to prevent visual corruption. |

### C. Raster Images

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 18 | **Image Enumeration & Bounds** | `FULLY_EXPOSED` | Extracts image XObject references, dimensions, filters, and page coordinates. |
| 19 | **Image Replacement** | `FULLY_EXPOSED` | Replaces JPEG/PNG image data in-place with isolated XObject cloning. |
| 20 | **Image Removal / Delete** | `FULLY_EXPOSED` | Cleans `Do` operators and resource references. |
| 21 | **Image Drag Move & Resize** | `FULLY_EXPOSED` | Direct manipulation overlay updating transformation matrices atomically. |
| 22 | **Image Insertion (Signature/Stamp)** | `FULLY_EXPOSED` | Inserts JPEG/PNG stamps and handwritten signatures at target canvas coordinates. |
| 23 | **Shared Image Paint Isolation** | `FULLY_EXPOSED` | Clones shared XObject resources so modifying page 1 never corrupts other pages. |

### D. Vector Graphics & Paths

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 24 | **Vector Shape Enumeration** | `FULLY_EXPOSED` | Enumerates rectangles, lines, and complex bezier path operator sequences. |
| 25 | **Color & Line Width Styling** | `FULLY_EXPOSED` | Updates RGB stroke/fill colors and line widths in-place. |
| 26 | **Vector Shape Deletion** | `FULLY_EXPOSED` | Removes vector stream segments cleanly. |
| 27 | **Vector Move & Resize** | `FULLY_EXPOSED` | Direct bounding box manipulation for rectangles and lines. |
| 28 | **Vector Mark Creation** | `FULLY_EXPOSED` | Adds checkmarks, crossmarks, rectangles, and lines for form signing. |
| 29 | **Bezier Node Path Editing** | `PARTIALLY_EXPOSED` | Vector paths are styled/moved as bounded units; individual curve knot editing omitted. |

### E. Interactive AcroForms & Widgets

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 30 | **AcroForm Discovery & Trees** | `FULLY_EXPOSED` | Traverses field hierarchies, resolves terminal children, and maps `/Rect`. |
| 31 | **Text Field Editing** | `FULLY_EXPOSED` | Spatial input updates `/V` and generates Type1/TrueType `/AP` appearance stream. |
| 32 | **Checkbox Toggling** | `FULLY_EXPOSED` | Direct click toggles state and synchronizes `/AS` and `/V` entries. |
| 33 | **Radio Button Selection** | `FULLY_EXPOSED` | Mutually exclusive radio group selection with appearance regeneration. |
| 34 | **Dropdown / Choice Selection** | `FULLY_EXPOSED` | Updates selected choice export value and regenerates visible appearance. |
| 35 | **Multi-Select Listbox Values** | `ENGINE_READY_UI_MISSING` | Engine supports multi-value arrays; UI presents standard single select. |
| 36 | **Appearance `/AP` Regeneration** | `FULLY_EXPOSED` | ISO 32000 compliant appearance generation for all standard widgets. |

### F. Markup Annotations

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 37 | **Annotation Discovery** | `FULLY_EXPOSED` | Detects FreeText, Highlight, Underline, Square, Circle, Ink, and Link annotations. |
| 38 | **FreeText Annotation Editing** | `FULLY_EXPOSED` | Updates `/Contents` with font, border, color controls and appearance regeneration. |
| 39 | **Annotation Creation** | `FULLY_EXPOSED` | Creates FreeText, Ink (Draw), Highlight, and Shape annotations on canvas. |
| 40 | **Annotation Deletion** | `FULLY_EXPOSED` | Removes annotation dictionary from `/Annots` array cleanly. |
| 41 | **Link Annotation Navigation** | `FULLY_EXPOSED` | Displays link destinations without confusing text editing overlays. |

### G. Page Operations & Multi-Document Workflows

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 42 | **Page Mutations (Delete, Duplicate, Move, Blank)** | `FULLY_EXPOSED` | Page tree mutations (`/Kids`, `/Count`, parent references) with undo/redo. |
| 43 | **Multi-Page Organizer & Reordering** | `FULLY_EXPOSED` | Multi-selection (Shift/Ctrl), HTML5 drag-and-drop reordering, edge auto-scroll. |
| 44 | **Document Split & Extraction** | `FULLY_EXPOSED` | Selected-page extraction, comma-range split, and single-page burst extraction. |
| 45 | **Multi-Document Merge & Import** | `FULLY_EXPOSED` | Appends full documents or imports selected page ranges into target positions. |

### H. Search, File UX & Writers

| # | Capability | Classification | Notes |
| :-: | :--- | :---: | :--- |
| 46 | **Full Text Search & Highlighting** | `FULLY_EXPOSED` | StarPDF search bounding box overlay with zoom/rotation support and active hit focus. |
| 47 | **Incremental Save / Export** | `FULLY_EXPOSED` | Append-only xref revision export preserving original structure and digital signatures. |
| 48 | **Minimal Scratchpad Writer** | `FULLY_EXPOSED` | In-memory clean PDF generator for scratchpads and empty documents. |
| 49 | **Complete Standalone Compaction** | `ENGINE_GAP` | `CompleteWriter` defragmenter exists in Rust core; WASM export omitted in current build. |
