# SmartPDF 1.0 Release Candidate Manual Acceptance Checklist
### Powered by StarPDF Engine

This checklist serves as the formal manual acceptance and verification script for human QA and release qualification.

---

## 1. Document Lifecycle & File UX

- [x] **Open Standard PDF**: Open PDF via toolbar "Open" button or file dropzone. Renders correctly with metadata.
- [x] **Drag & Drop PDF**: Drag and drop PDF file directly onto workspace canvas. Dropzone overlay appears; file opens cleanly.
- [x] **Unsaved Changes Warning**: Modify document and attempt to open another PDF. Confirm dialog appears with "Unsaved Changes" warning; cancelling preserves edits; confirming opens new file.
- [x] **Local First Status**: Status bar displays "Zero bytes uploaded • 100% Client-Side WebAssembly" and "Saved" / "Unsaved changes" indicator.

---

## 2. Direct Text Manipulation & Typography

- [x] **Select & In-Place Edit**: Click any editable text span on canvas. Context toolbar appears; changing text and applying updates canvas immediately.
- [x] **Multi-Span Heading Edit**: Select multi-span or fragmented text run. Edit applies atomically across all spans without character truncation.
- [x] **Layout Safety Lock**: Replacing text with shorter/longer string keeps downstream text in exact original coordinates without drift or overlap.
- [x] **Text Move**: Drag text selection to new spatial location on page. `Tm` matrix updates coordinates in PDF content stream.
- [x] **Text Clear / Delete**: Pressing "Clear" or "Delete" (or keyboard `Delete`/`Backspace`) removes text span cleanly.
- [x] **Safe Refusal Notice**: Selecting non-rewritable (Type3 or non-standard subset) text displays `"This text can't be safely rewritten."` with option to add FreeText instead.

---

## 3. Fill & Sign & Annotations

- [x] **Add Text**: Click "Add Text" tool and click anywhere on page. Places clean FreeText annotation without unwanted black borders or background fill.
- [x] **Checkmark & Crossmark**: Place checkmarks and crossmarks on flat form checkboxes. Automatically centers inside detected box bounds.
- [x] **Signature Image**: Upload transparent signature PNG/JPEG. Insert, drag move, resize, and delete.
- [x] **Draw / Ink**: Draw freehand lines on canvas. Produces valid Ink annotation.
- [x] **Markup Annotations**: Select FreeText, Highlight, Square, Circle, Link annotations. Context toolbar allows color, border, text mutation, or deletion.

---

## 4. Images & Vector Graphics

- [x] **Raster Images**: Select image on canvas. Drag to move, drag handles to resize, replace with another file, or delete.
- [x] **Shared XObject Isolation**: Modifying an image shared across multiple pages clones the resource so other pages remain untouched.
- [x] **Vector Shapes**: Select rectangles and lines. Change stroke color, fill color, line thickness, move, resize, or delete.
- [x] **Create Vector Shapes**: Add new rectangle or line directly via Fill & Sign tools.

---

## 5. AcroForms & Interactive Fields

- [x] **Text Fields**: Direct spatial click focuses field; typing updates `/V` value and regenerates `/AP` appearance stream.
- [x] **Checkboxes & Radios**: Clicking checkbox toggles check state; clicking radio button updates mutually exclusive option.
- [x] **Dropdowns**: Selecting choice updates displayed export value and appearance.
- [x] **Read-Only Forms**: Read-only fields refuse direct text mutation cleanly.

---

## 6. Page Organizer & Multi-Document Operations

- [x] **Multi-Selection**: `Click` selects page; `Cmd/Ctrl + Click` toggles selection; `Shift + Click` selects continuous page range.
- [x] **Drag & Drop Reorder**: Drag thumbnail card to reorder pages. Blue insertion bar previews target; drop updates page sequence atomically.
- [x] **Batch Duplicate & Delete**: Duplicate or delete multiple selected pages simultaneously. Safeguard prevents deleting all pages in document.
- [x] **Insert Blank Page**: Inserts clean blank page before/after active page.
- [x] **Extract Pages**: Downloads clean standalone sub-document containing selected pages.
- [x] **Split PDF**: Split document by comma-separated ranges (`1-2, 3-5`) or burst into single-page PDFs.
- [x] **Import & Merge**: Merge secondary PDF document or import selected page ranges into specified positions.

---

## 7. Search, Viewport & Contextual Toolbar

- [x] **Search Highlighting**: Press `Cmd/Ctrl + F` or click Search. Highlights all matches with distinct active hit outline.
- [x] **Search Navigation**: `Enter` / `Shift + Enter` cycles matches; auto-scrolls active match into view.
- [x] **Zoom & Rotation**: Search bounding boxes and interactive overlays align accurately across all zoom levels (25%–400%) and page rotations (0°, 90°, 180°, 270°).
- [x] **Contextual Toolbar Anchoring**: Contextual toolbar floats directly adjacent to the selected canvas element regardless of document length or scroll position.

---

## 8. Export, Reopen & Durability

- [x] **Incremental Export**: Press `Cmd/Ctrl + S` or click "Export Editable". Downloads clean, valid PDF file.
- [x] **Reopen & Re-Edit**: Reopening exported PDF in SmartPDF or external readers loads with all mutations intact and allows further edits.
- [x] **Undo / Redo**: Multi-cycle undo (`Cmd/Ctrl+Z`) and redo (`Cmd/Ctrl+Shift+Z` / `Ctrl+Y`) cleanly reverses and reapplies mutations.
- [x] **Zero Memory Leak**: Document transitions and closing release WebAssembly handles cleanly.

---

## 9. Provenance & Qualification Summary

- **Real-User Field Validation**: `NOT PERFORMED` (Strict provenance classification; no external real-user PDFs committed).
- **Development Corpus & Fixtures**: Chrome, macOS Quartz, LibreOffice, PDFKit, pdf-lib, synthetic suites all verified.
- **Cross-Browser Qualification**: Chromium (77/77), Firefox (77/77), Playwright WebKit (77/77) — 231/231 E2E tests passing.
- **Safari**: *Not directly tested (Playwright WebKit qualified).*
