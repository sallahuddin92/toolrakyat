# SmartPDF Deep Audit & Production Architecture Report

**Date:** 2026-08-20  
**Repository:** ToolRakyat (`https://github.com/sallahuddin92/toolrakyat`)  
**Audited Commit Baseline:** `b094a2e9dacdf68afca17712c4c1c89845e89deb`  
**Primary Target Route:** `/tools/pdf/editor`  
**Target API Proxy:** `/api/pdf/[[...path]]`  

---

## 1. Executive Summary

A comprehensive architectural and forensic audit was conducted on **SmartPDF / Advanced PDF Editor** in the ToolRakyat repository. 

### Key Findings
1. **Current Status: 100% Placeholder / Unwired.** Navigating to `/tools/pdf/editor` renders the `ToolPageShell` with a `ToolPlaceholder` component stating `"Tool wiring pending: This tool is marked implemented, but its UI wiring isn't added yet."` No editor UI, upload dropzone, canvas renderer, or toolbars are wired to the route.
2. **Missing Backend Microservice.** The API route `src/app/api/pdf/[[...path]]/route.ts` is an HTTP reverse proxy forwarding requests to `process.env.BACKEND_URL || "http://localhost:8000"`. Archaeological investigation across the entire Git history confirms that no Python, FastAPI, or standalone PDF backend ever existed in this repository. All proxy requests fail with HTTP 500 (`"Failed to connect to backend"`).
3. **Prerequisite Libraries Installed.** The project `package.json` already contains the requisite dependencies for full in-browser PDF manipulation: `pdf-lib` (v1.17.1), `pdfjs-dist` (v5.6.205), `react-pdf` (v10.4.1), `tesseract.js` (v7.0.0), `zustand` (v5.0.12), `react-dropzone` (v15.0.0), and `react-resizable-panels` (v4.10.0), alongside pre-bundled `public/pdf.worker.min.mjs` (1.24 MB).
4. **Documentation Discrepancy.** `README.md` claims SmartPDF is working with OCR, structural analysis, and true content editing, and references `docs/SMARTPDF_BETA_CHECKLIST.md` and `/tools/pdf/editor/uat`. Neither the checklist document nor the UAT route existed.
5. **No Existing Test Coverage.** There are currently 0 unit tests, 0 integration tests, and 0 Playwright E2E tests for SmartPDF or `/tools/pdf/editor`.

---

## 2. Current Architecture

```mermaid
flowchart TD
    subgraph Browser ["Browser Client (Next.js App Router)"]
        User([User / Browser])
        Route["/tools/pdf/editor<br/>(src/app/tools/[category]/[slug]/page.tsx)"]
        Shell["ToolPageShell<br/>(src/components/tools/ToolPageShell.tsx)"]
        Placeholder["ToolPlaceholder<br/>('Tool wiring pending')"]
        
        User -->|Navigates to /tools/pdf/editor| Route
        Route --> Shell
        Shell --> Placeholder
    end

    subgraph UnusedAssets ["Unwired In-Repo PDF Assets"]
        PDFJS["pdfjs-dist + react-pdf"]
        PDFLib["pdf-lib"]
        Tesseract["tesseract.js"]
        Worker["public/pdf.worker.min.mjs"]
        TestAssets["test-assets/*.pdf (9 fixtures)"]
    end

    subgraph DeadProxy ["Dead Reverse Proxy"]
        ProxyRoute["/api/pdf/[[...path]]<br/>(src/app/api/pdf/[[...path]]/route.ts)"]
        ExtService["http://localhost:8000<br/>(NON-EXISTENT)"]
        
        ProxyRoute -.->|fetch() fails| ExtService
    end
```

---

## 3. File / Component Map

| Path | Purpose | Current State |
|---|---|---|
| `src/app/tools/[category]/[slug]/page.tsx` | Dynamic tool route handler | Resolves `pdf-editor` from registry; renders `ToolPlaceholder`. |
| `src/lib/tools/registry.ts` | Tool registry configuration | Defines `pdf-editor` with slug `editor`, category `pdf`, max 50MB, `isImplemented: true`. |
| `src/app/api/pdf/[[...path]]/route.ts` | Next.js route handler proxy | Forwards `GET`/`POST` to `http://localhost:8000/documents/${path}`. Dead stub. |
| `public/pdf.worker.min.mjs` | PDF.js web worker asset | Statically hosted in `public/` (1.24 MB). Valid, but unreferenced. |
| `src/lib/tools/file-validation.ts` | Upload file validator | Validates PDF magic bytes (`%PDF-`), MIME types, and size limits. |
| `src/lib/tools/temp-files.ts` | Temporary file lifecycle | Manages `/tmp/toolrakyat` workspaces with 30-min cleanup. |
| `test-assets/` | PDF test fixtures | Contains 9 test fixtures (AcroForms, multi-page, scanned, invalid). |
| `src/components/tools/implementations/` | Tool implementation directory | Contains 5 AkaunKemas tools and 1 text tool; **0 SmartPDF components**. |

---

## 4. Upload Pipeline

- **Intended Pipeline:** `File Drop → Magic Byte Validation → PDF.js Document Loading → Memory Buffer Store`.
- **Actual Current Reality:**
  - `src/lib/tools/file-validation.ts` has a functional `validateUploadedFile()` utility that checks `%PDF-` header magic bytes and size constraints (verified by unit tests in `src/lib/tools/file-validation.test.ts`).
  - However, no dropzone component is rendered on `/tools/pdf/editor`. The upload pipeline cannot be triggered from the UI.

---

## 5. Parsing & Rendering

- **Intended Pipeline:** `pdfjs-dist` worker parses PDF structure → renders viewport to `<canvas>` + text layer / annotation layer via `react-pdf`.
- **Actual Current Reality:**
  - `pdfjs-dist` and `react-pdf` are installed in `package.json`.
  - `public/pdf.worker.min.mjs` is present in the public folder.
  - Zero rendering components are instantiated. No `<canvas>`, `<Document>`, or `<Page>` elements are mounted for SmartPDF.

---

## 6. Editing Model

- **Intended Pipeline:** Text modification, additions, annotations, shape drawing, signatures, and redactions.
- **Actual Current Reality:**
  - **No Content-Stream Modifier:** There is no engine in the codebase that parses PDF content streams (`BT`/`ET` text blocks) or reflows font glyphs.
  - **No Overlay Layer:** There is no canvas overlay or DOM overlay component currently implemented to capture user annotations, text overlays, or drawings.

---

## 7. Form Support

- **Intended Pipeline:** AcroForm widget detection, interactive form filling, checkbox toggling, radio selection, and form export.
- **Actual Current Reality:**
  - `pdf-lib` supports full AcroForm inspection and manipulation (`doc.getForm().getFields()`, `field.setText()`, `field.check()`, `form.flatten()`).
  - Test fixtures `test-assets/smartpdf-form.pdf` and `test-assets/smartpdf-adobe-like-form.pdf` contain valid AcroForm fields (`PDFTextField`, `PDFCheckBox`, `PDFRadioGroup`).
  - However, no form detection or UI binding is currently wired.

---

## 8. Image Support

- **Intended Pipeline:** Extract existing images, insert images, scale/position, delete/replace images.
- **Actual Current Reality:**
  - Not implemented. `pdf-lib` has `doc.embedPng()` / `doc.embedJpg()` and `page.drawImage()`, but no UI or manipulation hooks exist.

---

## 9. Page Operations

- **Intended Pipeline:** Add blank page, delete page, rotate page, reorder pages, duplicate page.
- **Actual Current Reality:**
  - `pdf-lib` natively supports `doc.addPage()`, `doc.removePage()`, `doc.copyPages()`, and `page.setRotation()`.
  - Not implemented in UI.

---

## 10. Export Architecture

- **Intended Pipeline:** Save modified PDF (either keeping editable AcroForms/annotations or flattening layers into static PDF streams).
- **Actual Current Reality:**
  - `src/lib/akaunkemas/pdf-export.ts` demonstrates `pdf-lib` document creation and `doc.save()` returning `Uint8Array` for cashbook exports.
  - No export pipeline exists for SmartPDF.

---

## 11. External Microservice Dependency

### Detailed Audit of `/api/pdf/[[...path]]`
1. **Expected Microservice:** The proxy attempts to connect to `http://localhost:8000/documents/...` via `fetch()`.
2. **Repository Search:**
   ```bash
   git log --all --full-history -- "**/*pdf*"
   rg -n "BACKEND_URL|localhost:8000" src
   ```
3. **Findings:**
   - No Python backend, Docker container, or microservice source code exists in this repository or in any historical commit.
   - The route handler blindly forwards incoming requests with zero authentication, zero path validation, and zero payload sanitization.
   - When called, it catches network failure and returns:
     ```json
     { "error": "Failed to connect to backend", "status": 500 }
     ```
4. **Feasibility of In-Browser Replacement:**
   - **Form filling & AcroForms:** 100% achievable in-browser using `pdf-lib`.
   - **Page operations (merge/split/reorder/rotate/delete):** 100% achievable in-browser using `pdf-lib`.
   - **Annotations / text additions / signatures / redactions:** 100% achievable in-browser using `react-pdf` + canvas overlay + `pdf-lib` flattener.
   - **Client-Side OCR:** Achievable in-browser using `tesseract.js` (WebWorker).
   - **Conclusion:** An external microservice is **NOT required** for core PDF viewing, form filling, page manipulation, signing, and annotation workflows. Eliminating the external microservice dependency removes infrastructure overhead, eliminates server hosting costs, and guarantees 100% client-side privacy.

---

## 12. Capability Matrix

| Capability | Implementation | Verified | Limitation | Status |
|---|---|---|---|---|
| **Document Upload** | `src/lib/tools/file-validation.ts` | Yes (Unit tests) | UI dropzone not mounted on route | `PARTIAL` |
| **Multi-page Rendering** | None | No | No UI component | `NOT_IMPLEMENTED` |
| **Page Navigation & Thumbnails** | None | No | No UI component | `NOT_IMPLEMENTED` |
| **Zoom / Pan / Fit Viewport** | None | No | No UI component | `NOT_IMPLEMENTED` |
| **Page Rotation (UI)** | None | No | No UI component | `NOT_IMPLEMENTED` |
| **Large File Handling** | Registry max 50MB | Yes (Config) | Browser memory limits apply on client | `PARTIAL` |
| **Corrupted PDF Recovery** | None | No | Throws parsing exception | `NOT_IMPLEMENTED` |
| **Encrypted PDF Handling** | None | No | Password prompts not implemented | `NOT_IMPLEMENTED` |
| **Extract Existing Text** | `pdfjs-dist` (library only) | No | Unwired | `NOT_IMPLEMENTED` |
| **True Content-Stream Text Edit** | None | No | Fundamental PDF format limitation | `NOT_IMPLEMENTED` |
| **Overlay Text Insertion** | `pdf-lib` (library only) | No | Unwired | `NOT_IMPLEMENTED` |
| **Font Selection & Sizing** | None | No | Unwired | `NOT_IMPLEMENTED` |
| **AcroForm Field Detection** | `pdf-lib` (library only) | No | Unwired | `NOT_IMPLEMENTED` |
| **Interactive Form Filling** | None | No | Unwired | `NOT_IMPLEMENTED` |
| **Form Flattening** | `pdf-lib` `form.flatten()` | No | Unwired | `NOT_IMPLEMENTED` |
| **Image Insertion / Signature** | `pdf-lib` `embedPng` | No | Unwired | `NOT_IMPLEMENTED` |
| **Page Delete / Reorder** | `pdf-lib` (library only) | No | Unwired | `NOT_IMPLEMENTED` |
| **Freehand / Highlight Drawing** | None | No | Unwired | `NOT_IMPLEMENTED` |
| **Editable PDF Export** | `pdf-lib` `save()` | No | Unwired | `NOT_IMPLEMENTED` |
| **Flattened PDF Export** | `pdf-lib` `save()` | No | Unwired | `NOT_IMPLEMENTED` |
| **Backend API Proxy** | `src/app/api/pdf/[[...path]]` | Yes (Code) | Target backend does not exist | `BROKEN` |

---

## 13. Native vs Overlay Editing Analysis

### The Truth About "PDF Text Editing"
The PDF specification (ISO 32000-1) is a digital print layout language, not a structured word processor document format. A PDF page is composed of low-level graphics operators:
- `BT` (Begin Text) / `ET` (End Text)
- `Tf` (Select Font and Size)
- `Tm` (Set Text Matrix / Coordinates)
- `Tj` / `TJ` (Show Text String / Glyph Array with Kerning)

Existing text in compiled PDFs is often fragmented across multiple operators, with non-standard font encodings, subsetted embedded font dictionaries (missing glyph tables for unused characters), and absolute positioning without flow or line wrap awareness.

### Implementation Reality
- **True Content-Stream Modification:** Reflowing or modifying existing text within existing streams without breaking layout or font subsetting requires a full desktop-class PDF layout engine. SmartPDF possesses no such engine.
- **Overlay Editing (Industry Standard for Web):** The robust, reliable method used by production web PDF editors (e.g. Smallpdf, PDF24, DocHub) combines:
  1. **AcroForm Native Editing:** Form fields are real PDF interactive widgets (`/Tx`, `/Btn`, `/Ch`) that store values natively in the PDF structure.
  2. **Visual Overlay Layer:** User-placed text, shapes, highlights, whiteouts, and signatures are managed in a reactive overlay state in the browser.
  3. **Composited PDF-Lib Generation:** Upon export, overlays are written into the PDF as native content streams or annotations, with optional flattening.
- **Conclusion:** SmartPDF documentation must honestly describe this capability as **Interactive Form Filling, Annotations, and Visual Overlay Editing**, rather than claiming arbitrary native content-stream text replacement.

---

## 14. SmartPDF-Specific Test Coverage

| Test Type | Current Count | Target Count (Production) | Status |
|---|---|---|---|
| **Unit Tests (`vitest`)** | 0 | 25+ | ❌ None |
| **Integration Tests** | 0 | 10+ | ❌ None |
| **Playwright E2E Tests** | 0 | 12+ | ❌ None |
| **Overall SmartPDF Test Coverage** | **0%** | **>90%** | ❌ None |

*(Note: The global test suites pass 100% because they cover AkaunKemas SaaS, CSV parsers, limits, and tool registry, but zero tests cover SmartPDF).*

---

## 15. UAT Results

Executing the UAT protocol from `README.md` against `/tools/pdf/editor`:

| Step | Document Class | Expected Behavior | Actual Behavior | Result |
|---|---|---|---|---|
| 1 | Normal Digital PDF (`test-assets/edit-test.pdf`) | Render pages, enable text overlay | Displays "Tool wiring pending" | ❌ FAILED |
| 2 | Scanned PDF (`test-assets/scanned-test.pdf`) | Render image page, offer OCR | Displays "Tool wiring pending" | ❌ FAILED |
| 3 | AcroForm PDF (`test-assets/smartpdf-form.pdf`) | Detect and render form inputs | Displays "Tool wiring pending" | ❌ FAILED |
| 4 | Multi-Page PDF (`test-assets/multi-page.test.pdf`) | Page list, thumbnails, pagination | Displays "Tool wiring pending" | ❌ FAILED |
| 5 | Corrupt PDF (`test-assets/invalid.pdf`) | Clear user-facing error message | Not accessible | ❌ FAILED |
| 6 | Export Pipeline | Download valid `.pdf` | Not accessible | ❌ FAILED |

---

## 16. Broken Behaviors

1. **Proxy Endpoint Returns 500:** Any call to `/api/pdf/*` fails with 500 error due to hardcoded connection to `http://localhost:8000`.
2. **Missing UAT Route:** `README.md` references `/tools/pdf/editor/uat`, which returns 404.
3. **Missing Checklist File:** `README.md` links to `docs/SMARTPDF_BETA_CHECKLIST.md`, which was never created.

---

## 17. Missing Capabilities

1. Interactive client component (`SmartPdfEditor.tsx`).
2. Reactive canvas and PDF viewer viewport powered by `pdfjs-dist` / `react-pdf`.
3. Multi-page thumbnail sidebar with active page selection and page operations.
4. AcroForm detection and interactive form-field sync using `pdf-lib`.
5. Overlay annotation engine (text boxes, freehand draw, highlight, rectangles, signatures).
6. Zoom controls (Zoom In, Zoom Out, Fit Width, Fit Page, 50%-200%).
7. Dual-mode exporter (Export Editable PDF vs. Export Flattened PDF).
8. Client-side OCR worker adapter via `tesseract.js`.

---

## 18. Security / File Safety

1. **Client-Side Privacy Advantage:** Transitioning SmartPDF to 100% in-browser processing guarantees that confidential customer documents (financial statements, tax invoices, IC copies) never leave the user's browser, eliminating data privacy risks and server-side data retention liabilities.
2. **Dead Proxy Removal:** Removing or securing `/api/pdf/[[...path]]` eliminates an unauthenticated open forwarder.
3. **Upload File Validation:** Retain `validateUploadedFile()` to enforce `%PDF-` magic byte verification and strict 50MB file size limits prior to parsing.

---

## 19. Performance Risks

1. **Client Memory with Large PDFs:** Rendering high-resolution canvases for 100+ page PDFs can exhaust mobile/browser memory if all pages are mounted simultaneously.
   - *Mitigation:* Implement virtualized page rendering (render only visible viewport page ±1 buffer page).
2. **PDF.js Web Worker Offloading:** Heavy PDF decoding must run inside `public/pdf.worker.min.mjs` rather than blocking the main UI thread.
3. **Large Canvas Scaler:** Limit rendering canvas resolution multiplier (e.g. `window.devicePixelRatio` capped at 2.0) to prevent canvas memory allocation crashes.

---

## 20. Recommended Production Architecture

### **Option A: Pure In-Browser Client-Side Architecture (RECOMMENDED)**

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         BROWSER RUNTIME (Next.js)                        │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                    SmartPdfEditor Component UI                     │  │
│  │  ┌──────────────┐ ┌──────────────────────────────┐ ┌────────────┐  │  │
│  │  │  Thumbnails  │ │     Canvas / Page Viewport   │ │  Toolbar   │  │  │
│  │  │   Sidebar    │ │  (react-pdf + SVG Overlays)  │ │  & Actions │  │  │
│  │  └──────────────┘ └──────────────────────────────┘ └────────────┘  │  │
│  └───────────────────────────────────┬────────────────────────────────┘  │
│                                      │                                   │
│  ┌───────────────────────────────────▼────────────────────────────────┐  │
│  │                       Zustand Editor Store                         │  │
│  │   - documentBytes: Uint8Array                                      │  │
│  │   - pages: PageMeta[] (rotation, dimensions, pageNum)              │  │
│  │   - forms: AcroFormField[] (text, check, radio, select)            │  │
│  │   - overlays: OverlayElement[] (text, draw, sign, stamp)           │  │
│  │   - history: Undo / Redo Stack                                     │  │
│  └──────────────────┬─────────────────────────────────┬───────────────┘  │
│                     │                                 │                  │
│       ┌─────────────▼───────────────┐   ┌─────────────▼──────────────┐   │
│       │    pdfjs-dist Web Worker    │   │           pdf-lib          │   │
│       │  - Fast page rendering      │   │  - Form parsing & sync     │   │
│       │  - Text layer extraction    │   │  - Page ops (rotate/reorder│   │
│       │  - Viewport scaling         │   │  - Dual-mode PDF export    │   │
│       └─────────────────────────────┘   └────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

### Why Option A is Superior:
- **Zero Server Infrastructure:** No Python service, Docker host, or external API costs.
- **Maximum Data Privacy:** Fully compliant with Malaysian PDPA and enterprise requirements (files never leave device).
- **Vercel / Serverless Compatibility:** Zero risk of serverless execution timeouts or 4.5MB payload limits on large PDF files.
- **Instant Responsiveness:** Local canvas rendering and immediate UI feedback.

---

## 21. Prioritized Backlog

### P0 — Blocking Defects
1. **Remove / Refactor Dead Backend Proxy (`/api/pdf/[[...path]]`):** Disconnect or replace with safe local API if server-side processing is ever needed.
2. **Implement Core SmartPDF Editor Shell (`SmartPdfEditor.tsx`):** Replace placeholder on `/tools/pdf/editor` with functional upload dropzone and editor layout.
3. **Initialize PDF.js Web Worker:** Wire `public/pdf.worker.min.mjs` cleanly into `react-pdf` / `pdfjs-dist` to prevent main-thread UI blocking.

### P1 — Required for Reliable Beta
1. **Interactive Document Viewer:** Multi-page thumbnail rail, primary canvas viewport, zoom controls (50%-200%, fit width, fit page), and page navigation.
2. **Native AcroForm Form-Filling:** Inspect form fields via `pdf-lib`, render interactive HTML form inputs over PDF widget locations, and persist values back to PDF document.
3. **Visual Overlay Annotator:** Add text boxes, signatures, highlights, rectangles, and freehand drawing.
4. **Dual-Mode Exporter:** Export editable PDF (preserving AcroForm fields) vs. Export flattened PDF (burning all layers into native graphics stream).
5. **Page Operations:** Rotate page (90°/180°/270°), reorder pages, and delete pages.
6. **SmartPDF Test Suite:** Comprehensive unit tests for PDF operations and Playwright E2E tests for upload, edit, and export workflows.

### P2 — Required for Production
1. **Client-Side OCR (Tesseract.js):** Detect scanned / image-only pages and extract searchable text.
2. **Password-Protected PDF Support:** Detect encrypted PDFs, prompt for user password, and decrypt via `pdfjs-dist` / `pdf-lib`.
3. **Undo / Redo History Stack:** Full state time-travel for edits and annotations.
4. **Touch & Mobile Viewport Support:** Responsive pinch-to-zoom and mobile toolbar.

### P3 — Enhancements
1. **Table Extraction to CSV / Excel:** Extract tabular data from financial statements directly to CSV.
2. **Custom Watermarking & Page Numbering:** Direct visual stamping in editor.

---

## 22. Recommended Next Slice

### Slice Title:
**SmartPDF Phase 1 — In-Browser Document Viewer, AcroForm Engine & Exporter**

### Problem:
`/tools/pdf/editor` is a non-functional placeholder, and `/api/pdf/[[...path]]` is a dead proxy. Users cannot view, fill, or export PDFs.

### Reason for Priority:
Establishes the robust client-side foundation, restores genuine product utility to SmartPDF, leverages existing `pdf-lib` and `pdfjs-dist` packages, and eliminates reliance on the non-existent microservice.

### Exact Files Likely Involved:
1. `src/components/tools/implementations/pdf/SmartPdfEditor.tsx` (New main editor component)
2. `src/components/tools/implementations/pdf/PdfThumbnailRail.tsx` (New page thumbnail sidebar)
3. `src/components/tools/implementations/pdf/PdfPageCanvas.tsx` (New viewport renderer)
4. `src/components/tools/implementations/pdf/PdfToolbar.tsx` (New editor actions toolbar)
5. `src/lib/pdf/pdf-engine.ts` (New `pdf-lib` helper for document loading, form extraction, form filling, rotation, and export)
6. `src/lib/pdf/pdf-engine.test.ts` (New unit test suite for PDF operations)
7. `src/app/tools/[category]/[slug]/page.tsx` (Wire `pdf-editor` to `SmartPdfEditor`)
8. `src/tests/e2e/smartpdf-editor.spec.ts` (New Playwright E2E suite)

### Test Fixtures to Use:
- `test-assets/smartpdf-form.pdf` (AcroForm verification)
- `test-assets/multi-page.test.pdf` (Multi-page navigation & thumbnail verification)
- `test-assets/edit-test.pdf` (Standard text PDF)
- `test-assets/invalid.pdf` (Error boundary verification)

### Acceptance Criteria:
1. Navigating to `/tools/pdf/editor` renders the full SmartPDF editor interface.
2. Dropping a PDF uploads and renders page canvases with zoom and page navigation.
3. AcroForm fields (text fields, checkboxes, radio groups) are detected, editable, and synced.
4. "Export PDF" downloads a valid PDF preserving filled data.
5. Re-opening exported PDF in Acrobat/browser preserves filled fields.
6. 100% pass on `npm run lint`, `npm run typecheck`, `npm test`, `npm run build`, and `npx playwright test`.

---

## 23. Production Readiness Score

```text
┌─────────────────────────────────────────────────────────────┐
│                   SMARTPDF READINESS SCORE                  │
├──────────────────────────────┬──────────────────────────────┤
│ Category                     │ Score                        │
├──────────────────────────────┼──────────────────────────────┤
│ Rendering                    │  0 / 100                     │
│ Editing                      │  0 / 100                     │
│ Forms                        │  0 / 100                     │
│ Export                       │  0 / 100                     │
│ Reliability                  │ 15 / 100                     │
│ Security / Privacy           │ 40 / 100                     │
├──────────────────────────────┼──────────────────────────────┤
│ OVERALL SMARTPDF             │  9 / 100                     │
└──────────────────────────────┴──────────────────────────────┘
```

### Deductions Explained:
- **Rendering (0/100):** -100 because no page rendering component is wired to the route.
- **Editing (0/100):** -100 because no overlay or content modification engine is implemented.
- **Forms (0/100):** -100 because AcroForm detection and interactive filling are not mounted.
- **Export (0/100):** -100 because no download or PDF compilation pipeline is wired.
- **Reliability (15/100):** +15 for valid `pdf-lib` and `pdfjs-dist` packages installed; -85 for non-existent backend proxy returning 500 and dead route placeholders.
- **Security / Privacy (40/100):** +40 for file validation utilities and potential 100% client-side privacy model; -60 for dead proxy exposing unvalidated forwarding logic.
- **Overall SmartPDF (9/100):** Reflects that SmartPDF is currently an architectural specification and package dependency skeleton rather than an executable product.
