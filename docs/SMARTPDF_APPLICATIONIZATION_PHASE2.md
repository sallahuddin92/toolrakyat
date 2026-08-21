# SmartPDF Applicationization — Phase 2: Unified Selection & Command Lifecycle

## Overview

SmartPDF Applicationization Phase 2 establishes a unified, typed command infrastructure and direct-manipulation selection architecture on top of the Phase 1 dedicated application shell.

Phase 2 replaces fragmented ad-hoc mutation handlers with a strict, auditable transaction model:

$$\text{User Action} \longrightarrow \text{SmartPdfCommand} \longrightarrow \text{Validation} \longrightarrow \text{StarPDF Engine Operation} \longrightarrow \text{Document Refresh} \longrightarrow \text{Bounded History Snapshot} \longrightarrow \text{Dirty State} \longrightarrow \text{Selection Update} \longrightarrow \text{User Feedback}$$

---

## 1. Architecture Summary

### A. Selection Subsystem (`src/lib/pdf/selection/`)

1. **Selection Union**:
   - `SmartPdfSelection = TextSelection | ImageSelection | VectorSelection | FormSelection | AnnotationSelection | null`
   - Every active selection retains:
     - `type`: Target domain classification (`"text" | "image" | "vector" | "form" | "annotation"`)
     - `id`: Unique identifier (e.g. `span_id`, `image_id`, `graphic_id`, form field `name`, annotation `id`)
     - `pageIndex`: Zero-based target page
     - `pdfRect`: User-space bounding box `{ x, y, width, height }` in PDF points
     - `bounds`: Screen-space rendered box `{ left, top, width, height }` in CSS pixels

2. **Hit-Testing Priority**:
   - Strict stacking and precedence order:
     $$\text{form (z-30)} > \text{annotation (z-25)} > \text{text (z-20)} > \text{image (z-15)} > \text{vector (z-10)}$$
   - Active selected item elevates to `z-40` for unambiguous focus and keyboard interactions.

3. **Selection Invariants**:
   - **Post-Mutation Resolution** (`resolveSelectionAfterMutation`):
     - If the target object still exists on the active page, selection is preserved and its data payload is refreshed.
     - If the target object was deleted, replaced with an incompatible item, or if the page navigated away, selection deterministically clears to `null`.
   - **User Intent Clearing**:
     - Clicking the canvas background or empty workspace clears selection.
     - Pressing `Escape` clears selection.
     - Page navigation clears selection.

4. **Coordinate Math**:
   - `convertPdfRectToPixels`: Handles bottom-left PDF coordinate conversion with 0°, 90°, 180°, and 270° viewport rotations and continuous zoom scaling.

---

### B. Command Lifecycle Subsystem (`src/lib/pdf/commands/`)

1. **Core Interfaces**:
   - `SmartPdfCommand<TResult>`:
     - `readonly id: string`: Canonical command identifier (e.g. `text.replace`, `page.delete`)
     - `readonly label: string`: Human-readable description (used for toasts, history descriptions, and status bar activity indicator)
     - `readonly isMutating: boolean`: Declares whether the command modifies document state or structure
     - `execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult<TResult>>`
   - `SmartPdfCommandContext`: Encapsulates `sourceBytes`, `filename`, `currentPage`, `pageCount`, `selection`, `starPdfDoc`, `fieldValues`, `annotationValues`, and `inspectionResult`.
   - `SmartPdfCommandResult`: Encapsulates output `bytes`, `download`, `fieldValues`, `annotationValues`, `nextPage`, `nextSelection`, `clearSelection`, and `message`.

2. **Bounded 25-Snapshot History**:
   - Pure functional transaction stack implemented in `src/lib/pdf/commands/history.ts`.
   - Mutating commands push a new snapshot, discard redo branches, and enforce a hard boundary of 25 entries.
   - Non-mutating commands (selection, navigation, search, export) never push history and never mark dirty.
   - Undo/Redo functions (`undoHistory`, `redoHistory`) navigate snapshots and reload document proxies atomically.

3. **Command Implementations**:
   - **Text**: `ReplaceTextCommand`
   - **Images**: `ReplaceImageCommand`, `RemoveImageCommand`
   - **Vectors**: `UpdateVectorCommand`, `DeleteVectorCommand`
   - **Form Fields**: `SetFormFieldValueCommand`
   - **Annotations**: `UpdateAnnotationCommand`
   - **Page Operations**: `MovePageCommand`, `DuplicatePageCommand`, `DeletePageCommand`, `InsertBlankPageCommand`, `ExtractPagesCommand`
   - **Document Operations**: `MergeDocumentsCommand`, `ExportDocumentCommand`

---

## 2. Invariant Verification

| Invariant | Implementation Mechanism | Validation Result |
| :--- | :--- | :--- |
| **Atomic Document Refresh** | `loadDocument` loads proxy, inspects AcroForms, initialises StarPDF handle, and cleans up previous references without leaking WebGL/Worker memory | **VERIFIED** |
| **Bounded History Stack** | History snapshots bounded to 25 items via `pushHistorySnapshot` | **VERIFIED** (8 unit tests + E2E) |
| **Dirty State Integrity** | `isModified` marked only on successful mutating commands; reset on new document load; preserved on export | **VERIFIED** |
| **Selection Precedence** | Form > Annotation > Text > Image > Vector | **VERIFIED** (12 unit tests + E2E) |
| **Selection Auto-Clear** | Cleared on empty click, `Escape`, page navigation, or object deletion | **VERIFIED** |
| **Execution Busy State** | Status bar pulses active command; rapid invocations await or reject gracefully | **VERIFIED** |
| **Error Resilience** | StarPDF/pdf-lib errors translated to user-friendly messages via `formatPdfErrorMessage`; document state remains untouched | **VERIFIED** |

---

## 3. Test Qualification Results

- **Unit Tests (Vitest)**: 27 test files, **673 / 673 passed** (including selection geometry and bounded history suites)
- **Cross-Browser Playwright E2E**:
  - **Chromium**: 67 / 67 passed
  - **Firefox**: 67 / 67 passed
  - **Playwright WebKit (Safari)**: 67 / 67 passed
  - **Total**: **201 / 201 passed** (0 failures, 0 flaky)
- **Code Quality**:
  - `npm run lint`: **PASS** (0 errors, 0 warnings)
  - `npm run typecheck`: **PASS** (0 errors)
  - `npm run build`: **PASS** (Production Next.js bundle created successfully)
- **Engine Integrity**: StarPDF Rust/WASM codebase (`engine/starpdf/`) preserved without modification.
