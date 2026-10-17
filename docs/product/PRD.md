# SmartPDF / StarPDF Product Requirements Document (PRD)

**Document ID:** DOC-PRD-001  
**Status:** CANONICAL QUALIFIED (Phase 0 / Pre-Implementation)  
**Product:** SmartPDF  
**Core Engine:** StarPDF  
**Repository:** `sallahuddin92/toolrakyat`  
**Current Baseline Commit:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` (`arch/integrated`)  
**Target Milestone:** Desktop Alpha / Rust-First Application Core  

---

## 1. Product Definition & Vision

SmartPDF is a world-class, professional-grade desktop and web PDF application designed to deliver Adobe Acrobat Pro-class capability, familiarity, and workflow breadth, powered by the local-first, memory-safe **StarPDF** Rust engine.

SmartPDF rejects the traditional compromise between bloated, opaque legacy PDF suites and rudimentary web PDF viewers. By combining:
1. **Acrobat-Class Capability:** Comprehensive editing, page manipulation, forms, digital signatures, OCR, redaction, document comparison, accessibility remediation, and high-fidelity export.
2. **Rust-First Core Engine:** 100% Rust-owned parser, COS model, font shaping, spatial indexing, mutation verification, security gating, and native hardware-accelerated rendering (`wgpu`), with zero C-dependencies.
3. **StarPDF Architectural Rigor:** Proof-gated partial editability, provenance-preserving selection, content-addressed semantic deduplication, strict mutation envelopes, mutation certificates, and delta-native invalidation where **UNEXPLAINED CHANGE = 0**.

SmartPDF operates **local-first and privacy-first**: no user documents, biometric signatures, or text layers are transmitted to external servers without explicit user intent.

---

## 2. Release Track Strategy

To guarantee operational stability, the project maintains two strictly independent engineering tracks:

- **TRACK A (Current Product & RC2 Release Candidate):**
  - Scope: Stabilize, audit, and qualify the existing SmartPDF web application and StarPDF engine on `main` and `arch/integrated`.
  - Deliverables: Complete manual acceptance testing (`docs/SMARTPDF_RC2_MANUAL_ACCEPTANCE.md`), verify the 32-fixture qualification corpus (`qualification/starpdf/corpus.json`), and release `v1.0.0-rc.2`.
  - Invariant: Architecture work in Track B must NEVER rewrite, delay, or invalidate Track A deliverables.
- **TRACK B (Rust-First Desktop & Architecture Redevelopment):**
  - Scope: The phased Strangler migration detailed in this documentation suite: creating `starpdf-app-core`, `starpdf-render-wgpu`, `smartpdf-desktop`, and bringing 100% of state and rendering authority into Rust.
  - Deliverables: Desktop alpha on macOS, Windows, Linux, and promoted native GPU renderer.

---

## 3. Problems & Opportunities

### 3.1 The Legacy PDF Dilemma
- **Security Vulnerabilities:** Decades-old C/C++ PDF codebases suffer recurring memory corruption vulnerabilities (buffer overflows, use-after-free).
- **Opaque Document Corruption:** Traditional editors often destroy underlying PDF structures, silently strip digital signatures, drop tag trees, or corrupt subsetted font encodings upon saving.
- **Subscription Lock-in & Privacy Intrusion:** Dominant market solutions mandate cloud uploads for basic document operations, creating compliance liabilities for legal, healthcare, and enterprise users.

### 3.2 The Web PDF Tool Dilemma
- **Superficial Feature Sets:** Most web tools merely overlay HTML elements on top of static raster canvases or PDF.js previews without true content-stream mutation.
- **Uncontrolled Data Sharing:** "Free" online PDF converters routinely ingest sensitive documents onto remote servers.
- **Fragile Font & Script Support:** Handling of complex scripts (Arabic, Jawi, Hebrew, Devanagari) and CJK typography is notoriously broken, resulting in missing glyphs or scrambled character order.

### 3.3 The SmartPDF Opportunity
- Deliver a native, cross-platform desktop application (macOS, Windows, Linux) and client-side Web/WASM application powered by the same verified Rust engine.
- Provide surgical, provenance-aware mutations where users can inspect exact diffs of what changed in the underlying PDF COS graph.

---

## 4. Goals & Non-Goals

### 4.1 Primary Goals
- **G-01: Acrobat-Class Capability Breadth:** Support the complete spectrum of professional PDF tasks across 24 functional domains.
- **G-02: Total Rust Ownership:** Migrate all authoritative state, mutation logic, selection, history, permissions, file export, and rendering into Rust.
- **G-03: Local-First Private Processing:** Guarantee that zero document bytes or metadata leave the user's workstation during editing, OCR, or conversion.
- **G-04: Non-Destructive Mutation (`UNEXPLAINED CHANGE = 0`):** Enforce mathematical proof that editing a specific paragraph, image, or form field alters *only* that object and its declared structural/visual closure.
- **G-05: Superior Multilingual Typography:** Maintain industry-leading support for complex, RTL, and CJK scripts using HarfRust shaping, Skera subsetting, and adaptive font fallback.
- **G-06: Unified Headless & Automation API:** Expose identical engine semantics across GUI, CLI, and Model Context Protocol (MCP) servers.

### 4.2 Non-Goals
- **NG-01: Adobe Visual Cloning:** SmartPDF will adopt familiar professional UX workflow patterns (Home, All Tools, Document Workspace) but will **never** copy Adobe trademarks, proprietary icons, exact hex colors, or trade dress. SmartPDF features its own bespoke design system.
- **NG-02: Big-Bang Architecture Rewrites:** Existing working systems (e.g., qualified Rust/WASM engine, PDF.js visual oracle, Playwright test suites) will be preserved and migrated incrementally via Strangler patterns.
- **NG-03: Bypassing PDF Security or Cracking Passwords:** SmartPDF will strictly enforce PDF security policies. Decryption requires valid user/owner credentials; password cracking or permission bypass is prohibited by design.
- **NG-04: Superficial Office Conversion:** SmartPDF will not ship a low-fidelity, lossy internal DOCX/XLSX converter just to claim "pure Rust." High-fidelity office conversion will use calibrated external conversion adapters.
- **NG-05: AI Authority Over Document State:** AI/LLM assistants may propose actions, generate summaries, or draft form responses, but will never directly mutate PDF bytes without passing through StarPDF validation and user approval.

---

## 5. Functional Requirements (PRD-FR)

Status classifications:
- `CURRENT`: Fully operational and qualified in baseline (`arch/integrated`).
- `PARTIAL`: Partially implemented or demonstrated in test harnesses; matures in planned phases.
- `TARGET`: Architectural target for future phase (not implemented in baseline).
- `DEFERRED`: Planned for post-v1.0 milestone.
- `SAFE_REFUSAL` / `UNSUPPORTED_BY_POLICY`: Intentionally refused to guarantee document safety and cryptographic integrity.

| ID | Category | Requirement Description | Priority | Current Status | Target Status | Acceptance Criteria & Implementation Evidence |
| :--- | :--- | :--- | :---: | :---: | :---: | :--- |
| **PRD-FR-001** | File | Open standard, hybrid xref, and linearized PDFs | P0 | CURRENT | CURRENT | Opens valid PDFs without error. *Evidence:* `engine/starpdf/src/document/document.rs`, `tests/parser_tests.rs`. |
| **PRD-FR-002** | File | Fault-tolerant recovery of malformed xrefs and missing `/Prev` | P0 | CURRENT | CURRENT | Recovers corrupt tables, reconstructs `startxref`, detects `/Prev` cycles. *Evidence:* `engine/starpdf/src/xref/resolver.rs`, `tests/xref_prev_recovery_tests.rs`. |
| **PRD-FR-003** | File | Standard password authentication flow (R2–R6) | P0 | SAFE_REFUSAL | TARGET | Detects R2–R6 dictionaries; fail-closed refusal active (`DecryptionUnavailable`). Actual decryptor is TARGET. *Evidence:* `engine/starpdf/src/security.rs:430`. |
| **PRD-FR-004** | File | Clean document close and memory reclamation | P0 | CURRENT | CURRENT | Explicit memory deallocation; zero memory leaks across document cycles. *Evidence:* `engine/starpdf/src/wasm/registry.rs`, `src/lib/pdf/starpdf.test.ts`. |
| **PRD-FR-005** | Edit PDF | Native existing-text single and multi-span replacement | P0 | CURRENT | CURRENT | In-place text mutation preserving font size, color, baseline, and layout. *Evidence:* `engine/starpdf/src/mutation/text_edit.rs`, `tests/existing_text_editing_v0_13_tests.rs`. |
| **PRD-FR-006** | Edit PDF | Multilingual text shaping (Arabic, Jawi, Hebrew, CJK, Devanagari) | P0 | CURRENT | CURRENT | HarfRust shaping, Skera subsetting, ToUnicode mapping; zero scrambled glyphs. *Evidence:* `engine/starpdf/src/font/shaping.rs`, `tests/multilingual_font_tests.rs`. |
| **PRD-FR-007** | Edit PDF | Native text styling (family, size, bold, italic, color, alignment) | P1 | CURRENT | CURRENT | Applies style patches, checks font catalog, embeds subsets as required. *Evidence:* `engine/starpdf/src/font/style.rs`, `tests/basic_text_formatting_tests.rs`. |
| **PRD-FR-008** | Edit PDF | Safe native text drag move with dependency check | P1 | CURRENT | CURRENT | Updates `Tm` matrix coordinates; refuses moves that disturb dependent text. *Evidence:* `engine/starpdf/src/mutation/text_move.rs`, `tests/text_move_v0_21_tests.rs`. |
| **PRD-FR-009** | Edit PDF | Arbitrary multi-line paragraph reflow across subsetted fonts | P3 | UNSUPPORTED_BY_POLICY | UNSUPPORTED_BY_POLICY | Refused to prevent font corruption and unintended visual overlap. *Evidence:* `docs/STARPDF_SMARTPDF_FINAL_CAPABILITY_MATRIX.md:49`. |
| **PRD-FR-010** | Edit PDF | Image enumeration, bounds inspection, and selection | P0 | CURRENT | CURRENT | Extracts image XObject references, dimensions, color spaces, coordinates. *Evidence:* `engine/starpdf/src/image/extractor.rs`, `tests/image_operations_v0_14_tests.rs`. |
| **PRD-FR-011** | Edit PDF | Image replacement (JPEG/PNG) with shared XObject isolation | P0 | CURRENT | CURRENT | Replaces image data in-place; clones shared XObjects automatically. *Evidence:* `engine/starpdf/src/image/editor.rs:72`, `src/tests/e2e/phase2db-images.spec.ts`. |
| **PRD-FR-012** | Edit PDF | Image drag move, resize, and delete | P0 | CURRENT | CURRENT | Direct overlay manipulation updates transformation matrix or removes `Do` call. *Evidence:* `engine/starpdf/src/image/editor.rs`, `tests/image_operations_v0_14_tests.rs`. |
| **PRD-FR-013** | Edit PDF | Vector graphics enumeration, color, and line styling | P1 | CURRENT | CURRENT | Enumerates paths, rectangles, lines; updates RGB stroke/fill and line width. *Evidence:* `engine/starpdf/src/vector/extractor.rs`, `tests/vector_operations_v0_15_tests.rs`. |
| **PRD-FR-014** | Edit PDF | Bounded vector shape move, resize, and delete | P1 | CURRENT | CURRENT | Modifies bounding box of rectangles/lines; deletes path operator sequences. *Evidence:* `engine/starpdf/src/vector/editor.rs`, `tests/vector_operations_v0_15_tests.rs`. |
| **PRD-FR-015** | Edit PDF | Fine-grained Bézier node / knot editing | P3 | PARTIAL | DEFERRED | Bounded styling supported; granular control point editing deferred. *Evidence:* `docs/STARPDF_SMARTPDF_FINAL_CAPABILITY_MATRIX.md:71`. |
| **PRD-FR-016** | Page Org | Page reorder, duplicate, delete, and insert blank | P0 | CURRENT | CURRENT | Updates `/Kids`, `/Count`, parent references; supports multi-level undo/redo. *Evidence:* `engine/starpdf/src/page_ops/builder.rs`, `tests/page_operations_v0_12_tests.rs`. |
| **PRD-FR-017** | Page Org | Multi-page thumbnail organizer with drag-and-drop | P0 | CURRENT | CURRENT | Shift/Ctrl multi-selection, visual reordering, edge auto-scrolling. *Evidence:* `src/components/tools/implementations/pdf/PdfThumbnailRail.tsx`. |
| **PRD-FR-018** | Page Org | Document split and page extraction (ranges, burst) | P0 | CURRENT | CURRENT | Extracts selected pages or comma-separated ranges into valid new PDFs. *Evidence:* `src/components/tools/implementations/pdf/SmartPdfSplitModal.tsx`, `tests/page_operations_v0_12_tests.rs`. |
| **PRD-FR-019** | Page Org | Multi-document merge and page range import | P0 | CURRENT | CURRENT | Appends documents or imports selected pages into target document indices. *Evidence:* `engine/starpdf/src/page_ops/incremental.rs`, `tests/page_merge_v0_12b_tests.rs`. |
| **PRD-FR-020** | Page Org | Page rotation (90°, 180°, 270°) and cropping | P1 | PARTIAL | TARGET | Updates `/Rotate` in page dictionary. Viewport visual crop tool is TARGET. *Evidence:* `engine/starpdf/src/appearance/rotation.rs`, `tests/page_tree_tests.rs`. |
| **PRD-FR-021** | Annotations | FreeText annotation creation and live in-place editing | P0 | CURRENT | CURRENT | Edits `/Contents`, updates font, color, border, regenerates `/AP` stream. *Evidence:* `src/lib/pdf/freetext-edit-session.ts`, `tests/annotation_mutation_tests.rs`. |
| **PRD-FR-022** | Annotations | Standard markup (Highlight, Underline, Strikeout, Squiggly) | P0 | CURRENT | CURRENT | Creates standard text markup annotations matching underlying text spans. *Evidence:* `engine/starpdf/src/annotation/types.rs`, `tests/annotation_tests.rs`. |
| **PRD-FR-023** | Annotations | Geometric annotations (Square, Circle, Line, Arrow, Polygon) | P1 | CURRENT | CURRENT | Interactive creation and manipulation with configurable stroke/fill styles. *Evidence:* `engine/starpdf/src/annotation/generator.rs`, `tests/annotation_tests.rs`. |
| **PRD-FR-024** | Annotations | Sticky notes, comment threads, and status workflow | P1 | PARTIAL | TARGET | Discovers point annotations. Threaded replies and review workflow are TARGET. *Evidence:* `engine/starpdf/src/annotation/parser.rs`. |
| **PRD-FR-025** | Forms | AcroForm field discovery and hierarchical tree traversal | P0 | CURRENT | CURRENT | Discovers text fields, checkboxes, radios, choice widgets, resolves `/Rect`. *Evidence:* `engine/starpdf/src/forms/parser.rs`, `tests/acroform_tests.rs`. |
| **PRD-FR-026** | Forms | Form field value updating and appearance (`/AP`) generation | P0 | CURRENT | CURRENT | Updates `/V`, `/AS`, regenerates ISO 32000-1 compliant appearance streams. *Evidence:* `engine/starpdf/src/appearance/generator.rs`, `tests/acroform_tests.rs`. |
| **PRD-FR-027** | Forms | Form field creation and editing (designer mode) | P1 | PARTIAL | TARGET | Fill mode qualified. Canvas drag-to-create designer mode is TARGET. *Evidence:* `engine/starpdf/src/forms/widget.rs`. |
| **PRD-FR-028** | Forms | Form data import and export (XFDF, FDF, JSON, CSV) | P1 | PARTIAL | TARGET | In-memory values updateable. Standalone XFDF/JSON import/export is TARGET. *Evidence:* `engine/starpdf/src/wasm/api.rs`. |
| **PRD-FR-029** | Fill & Sign | Quick signature placement (drawn, typed, image stamp) | P0 | CURRENT | CURRENT | Inserts transparent PNG signature stamp at clicked canvas coordinates. *Evidence:* `engine/starpdf/src/image/editor.rs:add_image`, `src/tests/e2e/smartpdf-fill-sign-state.spec.ts`. |
| **PRD-FR-030** | Signatures | Cryptographic digital signature detection & byte-range audit | P0 | CURRENT | CURRENT | Detects `/ByteRange`, `/Contents`, checks if modifications occurred post-signing. *Evidence:* `engine/starpdf/src/security.rs:parse_byte_range`, `tests/compat_security_tests.rs`. |
| **PRD-FR-031** | Signatures | Cryptographic signature validation (X.509, PKCS#7/CMS) | P1 | TARGET | TARGET | Cryptographic verification is currently `NOT_PERFORMED`. X.509 validation is TARGET. *Evidence:* `engine/starpdf/src/wasm/api.rs:82`. |
| **PRD-FR-032** | Signatures | Digital signing with certificate and timestamp authority | P1 | TARGET | TARGET | PAdES / RFC 3161 digital signing pipeline is TARGET (Phase 8). |
| **PRD-FR-033** | OCR | Production OCR orchestration (scan-to-searchable-PDF) | P1 | TARGET | TARGET | Client prototype uses web worker `tesseract.js`; native Rust engine orchestration is TARGET. *Evidence:* `package.json:tesseract.js`. |
| **PRD-FR-034** | OCR | Multilingual OCR and editable text layer generation | P2 | TARGET | TARGET | Multi-script OCR and text layer generation (`3 Tr`) is TARGET. |
| **PRD-FR-035** | Redaction | Permanent data redaction (text, vector, raster, metadata) | P0 | TARGET | TARGET | Zero content-stream redaction currently exists in StarPDF. True stream operator deletion is TARGET (Phase 8). |
| **PRD-FR-036** | Redaction | Redaction inspection and verification diff | P1 | TARGET | TARGET | Pre-export sanitization verification diff is TARGET (Phase 8). |
| **PRD-FR-037** | Compare | Rust-owned multi-mode comparison (text, layout, pixel diff) | P1 | TARGET | TARGET | `differential-smoke.ts` exists for internal renderer tests; end-user comparison tool is TARGET (Phase 8). |
| **PRD-FR-038** | Accessibility | Tagged PDF inspection and reading order tree visualization | P1 | PARTIAL | TARGET | Outlines and OCGs inspected (`engine/starpdf/src/compatibility.rs`). Tagged `/StructTreeRoot` editing is TARGET. |
| **PRD-FR-039** | Accessibility | Accessibility checker and PDF/UA-1 remediation tools | P2 | TARGET | TARGET | Automated rule audit and PDF/UA-1 remediation is TARGET (Phase 8). |
| **PRD-FR-040** | Export | Incremental export preserving signatures and structure | P0 | CURRENT | CURRENT | Appends xref revisions without touching previous byte ranges. *Evidence:* `engine/starpdf/src/writer/incremental.rs`, `tests/incremental_writer_tests.rs`. |
| **PRD-FR-041** | Export | Canonical / structural healing export | P0 | CURRENT | CURRENT | Writes cleaned xref stream, resolves orphan objects, normalizes trailer. *Evidence:* `engine/starpdf/src/writer/complete.rs`, `tests/recovered_xref_stream_export_tests.rs`. |
| **PRD-FR-042** | Export | Standard conversion profiles (Text, Markdown, HTML, JSON) | P1 | PARTIAL | TARGET | Plain text extraction verified (`engine/starpdf/src/text/extractor.rs`). Structured Markdown/HTML/JSON is TARGET. |
| **PRD-FR-043** | Automation | StarPDF CLI for batch processing, extraction, and forms | P1 | TARGET | TARGET | Subprocess probe exists (`qualification_probe.rs`); standalone user-facing CLI binary is TARGET (Phase 8). |
| **PRD-FR-044** | Automation | Model Context Protocol (MCP) server for AI pairing | P1 | CURRENT | TARGET | Operational TypeScript MCP server exists (`../toolrakyat/src/mcp/starpdf/`); native Rust port is TARGET. |

---

## 6. Non-Functional Requirements (PRD-NFR)

| ID | Category | Requirement Description | Priority | Target Metric | Current Status | Acceptance Criteria & Measurement Evidence |
| :--- | :--- | :--- | :---: | :---: | :---: | :--- |
| **PRD-NFR-001** | Performance | Application Cold Startup Time | P0 | < 800 ms native | TARGET | Web shell cold startup measured ~1,200 ms. Native desktop target is < 400 ms. |
| **PRD-NFR-002** | Performance | Document Open Time (100-page PDF) | P0 | < 300 ms | CURRENT | Measured ~180 ms in Rust engine test suite on standard desktop hardware. |
| **PRD-NFR-003** | Performance | First-Page Viewport Render | P0 | < 50 ms native | TARGET | PDF.js web rasterization measured ~85 ms. Native `wgpu` render target is < 30 ms. |
| **PRD-NFR-004** | Performance | Scrolling & Panning Frame Rate | P0 | 60–120 fps | CURRENT | 60 fps steady in browser DOM viewport; 120 fps target for native `wgpu` swapchain. |
| **PRD-NFR-005** | Performance | In-Place Text Replacement Latency | P0 | < 25 ms | CURRENT | Measured ~14 ms from keystroke commit to HarfRust shaping and content stream patch. |
| **PRD-NFR-006** | Performance | Spatial Pointer Hit-Testing | P0 | < 10 µs | DEMONSTRATED | Measured ~5.8 µs per query over 10,000 synthetic primitives in `src/lib/pdf/execution/render/benchmark.ts`. Native Rust BVH is TARGET. |
| **PRD-NFR-007** | Performance | Memory Consumption (Idle) | P0 | < 60 MB RSS native | TARGET | Current browser process uses ~110 MB RSS. Native desktop target is < 35 MB. |
| **PRD-NFR-008** | Performance | Memory Scaling (1,000-page PDF) | P0 | < 250 MB RSS | CURRENT | Measured ~195 MB RSS in Rust engine via lazy object stream and xref resolution. |
| **PRD-NFR-009** | Reliability | Structural Invariant: `UNEXPLAINED CHANGE = 0` | P0 | 100% | DEMONSTRATED | Certified in `engine/starpdf/tests/execution_contracts_tests.rs`. Production mutation gating is TARGET. |
| **PRD-NFR-010** | Reliability | Crash Recovery & Autosave | P0 | < 2 s loss | TARGET | Current web shell has 220 ms debounce and 25-step undo in RAM. Persistent atomic disk journal is TARGET. |
| **PRD-NFR-011** | Security | Memory Safety & Fuzzing | P0 | Zero Panic | CURRENT | Zero panics across `fuzz_hardening_tests.rs` and cargo-fuzz parser suites. |
| **PRD-NFR-012** | Security | Fail-Closed Security Handler | P0 | 100% | CURRENT | Rejects unsupported encryption with typed error; prohibits unauthorized signed edits. *Evidence:* `security.rs:430`. |
| **PRD-NFR-013** | Privacy | Pure Local Processing | P0 | 0 network bytes | CURRENT | Verified 0 outbound network requests during all document editing operations. |
| **PRD-NFR-014** | Accessibility | Keyboard Navigation & WCAG 2.1 AA | P1 | 100% | PARTIAL | Full keyboard navigation in web editor; full screen reader AT support is TARGET. |
| **PRD-NFR-015** | Platforms | Cross-Platform Parity | P0 | macOS/Win/Linux | PARTIAL | Fully qualified on Web (Chromium/Firefox/WebKit). Native desktop platforms are TARGET. |

---

## 7. Platform & Hardware Targets

### 7.1 Desktop Native (Track B)
- **macOS:** Apple Silicon (M1/M2/M3/M4) & Intel x86_64, macOS 12 (Monterey) through macOS 15+. Metal GPU backend via `wgpu`.
- **Windows:** Windows 10 & Windows 11 (x86_64 & ARM64). DirectX 12 / Vulkan GPU backend via `wgpu`.
- **Linux:** Modern distributions (Ubuntu 22.04+, Fedora 38+, Arch). Vulkan GPU backend via `wgpu`, Wayland & X11 support.

### 7.2 Web / WebAssembly (Track A & Track B)
- **Browsers:** Modern Evergreen browsers with WebAssembly and WebGPU support (Chromium 113+, Firefox 120+, Safari 18+). Fallback to WebGL2 or software canvas where WebGPU is unavailable.
- **Client-Side Storage:** OPFS (Origin Private File System) and IndexedDB for private document caching and autosave journals.

---

## 8. Export Profiles & Standards Compliance

Export profiles are owned authoritatively by the Rust engine (`starpdf-writer`):

1. **`StandardPdf`:** Clean incremental update or compact rewrite conforming to ISO 32000-1 (PDF 1.7) or ISO 32000-2 (PDF 2.0).
2. **`IncrementalPdf`:** Strictly append-only xref revision. Preserves existing signed byte ranges and historical revisions.
3. **`CanonicalPdf`:** Structural healing export. Normalizes xref tables into clean xref streams, removes unreachable garbage objects, and repairs broken dictionary references.
4. **`PdfA` (PDF/A-1b, PDF/A-2b):** Embeds all fonts, sets color management dictionaries (OutputIntents / ICC profiles), disables forbidden features (JavaScript, audio/video).
5. **`FlattenedPdf`:** Renders interactive AcroForm widgets and markup annotations permanently into page content streams (`/Contents`), removing widget dictionaries.
6. **`RedactedPdf`:** Sanitized export with all marked content, hidden text, and cropped raster data permanently erased from the object store.
7. **`StructuredExport`:** Extracted representations including Text (`.txt`), Markdown (`.md`), HTML5 (`.html`), JSON (`.json`), and Form Data (`.xfdf`, `.fdf`, `.csv`).
