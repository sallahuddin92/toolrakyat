# SmartPDF / StarPDF Product Requirements Document (PRD)

**Document ID:** DOC-PRD-001  
**Status:** CANONICAL DRAFT (Phase 0 / Pre-Implementation)  
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
2. **Rust-First Core Engine:** 100% Rust-owned parser, COS model, font shaping, spatial indexing, mutation verification, security gating, and native hardware-accelerated rendering (`wgpu`).
3. **StarPDF Architectural Rigor:** Proof-gated partial editability, provenance-preserving selection, content-addressed semantic deduplication, strict mutation envelopes, mutation certificates, and delta-native invalidation where **UNEXPLAINED CHANGE = 0**.

SmartPDF operates **local-first and privacy-first**: no user documents, biometric signatures, or text layers are transmitted to external servers without explicit user intent.

---

## 2. Problems & Opportunities

### 2.1 The Legacy PDF Dilemma
- **Security Vulnerabilities:** Decades-old C/C++ PDF codebases suffer recurring memory corruption vulnerabilities (buffer overflows, use-after-free).
- **Opaque Document Corruption:** Traditional editors often destroy underlying PDF structures, silently strip digital signatures, drop tag trees, or corrupt subsetted font encodings upon saving.
- **Subscription Lock-in & Privacy Intrusion:** Dominant market solutions mandate cloud uploads for basic document operations, creating compliance liabilities for legal, healthcare, and enterprise users.

### 2.2 The Web PDF Tool Dilemma
- **Superficial Feature Sets:** Most web tools merely overlay HTML elements on top of static raster canvases or PDF.js previews without true content-stream mutation.
- **Uncontrolled Data Sharing:** "Free" online PDF converters routinely ingest sensitive documents onto remote servers.
- **Fragile Font & Script Support:** Handling of complex scripts (Arabic, Jawi, Hebrew, Devanagari) and CJK typography is notoriously broken, resulting in missing glyphs or scrambled character order.

### 2.3 The SmartPDF Opportunity
- Deliver a native, cross-platform desktop application (macOS, Windows, Linux) and client-side Web/WASM application powered by the same verified Rust engine.
- Provide surgical, provenance-aware mutations where users can inspect exact diffs of what changed in the underlying PDF COS graph.

---

## 3. Goals & Non-Goals

### 3.1 Primary Goals
- **G-01: Acrobat-Class Capability Breadth:** Support the complete spectrum of professional PDF tasks across 24 functional domains.
- **G-02: Total Rust Ownership:** Migrate all authoritative state, mutation logic, selection, history, permissions, file export, and rendering into Rust.
- **G-03: Local-First Private Processing:** Guarantee that zero document bytes or metadata leave the user's workstation during editing, OCR, or conversion.
- **G-04: Non-Destructive Mutation (`UNEXPLAINED CHANGE = 0`):** Enforce mathematical proof that editing a specific paragraph, image, or form field alters *only* that object and its declared structural/visual closure.
- **G-05: Superior Multilingual Typography:** Maintain industry-leading support for complex, RTL, and CJK scripts using HarfRust shaping, Skera subsetting, and adaptive font fallback.
- **G-06: Unified Headless & Automation API:** Expose identical engine semantics across GUI, CLI, and Model Context Protocol (MCP) servers.

### 3.2 Non-Goals
- **NG-01: Adobe Visual Cloning:** SmartPDF will adopt familiar professional UX workflow patterns (Home, All Tools, Document Workspace) but will **never** copy Adobe trademarks, proprietary icons, exact hex colors, or trade dress. SmartPDF features its own bespoke design system.
- **NG-02: Big-Bang Architecture Rewrites:** Existing working systems (e.g., qualified Rust/WASM engine, PDF.js visual oracle, Playwright test suites) will be preserved and migrated incrementally via Strangler patterns.
- **NG-03: Bypassing PDF Security or Cracking Passwords:** SmartPDF will strictly enforce PDF security policies. Decryption requires valid user/owner credentials; password cracking or permission bypass is prohibited by design.
- **NG-04: Superficial Office Conversion:** SmartPDF will not ship a low-fidelity, lossy internal DOCX/XLSX converter just to claim "pure Rust." High-fidelity office conversion will use calibrated external conversion adapters.
- **NG-05: AI Authority Over Document State:** AI/LLM assistants may propose actions, generate summaries, or draft form responses, but will never directly mutate PDF bytes without passing through StarPDF validation and user approval.

---

## 4. User Personas & Core Workflows

### 4.1 Target Personas
1. **The Legal & Compliance Professional:** Requires verifiable document integrity, cryptographic digital signatures, strict byte-range preservation, and unrecoverable, permanent redaction.
2. **The Enterprise Administrator / Accountant:** Manages interactive AcroForms, batches invoices, extracts tabular data to CSV, organizes and merges large multi-page reports.
3. **The Multilingual Publisher / Scholar:** Edits existing documents containing mixed Latin, Arabic/Jawi, Hebrew, or CJK scripts, requiring precise typography, zero font corruption, and valid ToUnicode mapping.
4. **The Privacy-Conscious Power User:** Refuses cloud-based PDF utilities for confidential personal, financial, or medical documents.
5. **The Automation Engineer / AI Agent:** Automates PDF workflows, extraction, splitting, merging, and form filling via CLI and MCP protocols.

### 4.2 Core Workflows
- **WF-01: Document Review & Annotation:** Open multi-gigabyte or complex PDF -> navigate via bookmarks/thumbnails -> highlight text, insert FreeText comments, draw shapes -> export incremental revision.
- **WF-02: Surgical Content Editing:** Select existing text run -> edit typo or update date -> StarPDF verifies font editability -> shapes replacement text via HarfRust -> subsets font -> updates content stream in-place -> preserves downstream layout.
- **WF-03: Forensic Redaction:** Search sensitive terms or marquee box -> mark redaction -> inspect underlying text/vector/image data -> execute redaction -> underlying bytes physically deleted and sanitized -> export RedactedPdf.
- **WF-04: Form Design & Completion:** Open government/tax PDF -> auto-detect fields -> fill interactive fields with validation -> cryptographically sign -> flatten or save incremental revision.
- **WF-05: Document Organization & Assembly:** Drag-and-drop reorder pages -> extract page ranges -> merge external documents -> rotate orientation -> export cleaned canonical PDF.
- **WF-06: Document Intelligence & Accessibility:** Audit tagged structure tree -> auto-detect reading order and headings -> add alt-text to images -> verify PDF/UA compliance -> export accessible PDF.

---

## 5. Functional Requirements (PRD-FR)

Status classifications:
- `CURRENT`: Fully operational and qualified in baseline (`arch/integrated`).
- `PARTIAL`: Partially implemented or bounded direct manipulation; matures in planned phases.
- `TARGET`: Architectural target for future phase.
- `DEFERRED`: Planned for post-v1.0 milestone.
- `UNSUPPORTED_BY_POLICY`: Intentionally refused to guarantee document safety and cryptographic integrity.

| ID | Category | Requirement Description | Priority | Current Status | Target Status | Acceptance Criteria |
| :--- | :--- | :--- | :---: | :---: | :---: | :--- |
| **PRD-FR-001** | File | Open standard, hybrid xref, and linearized PDFs | P0 | CURRENT | CURRENT | Opens valid PDFs without error; resolves object hierarchies cleanly. |
| **PRD-FR-002** | File | Fault-tolerant recovery of malformed xrefs and missing `/Prev` | P0 | CURRENT | CURRENT | Recovers corrupt tables, reconstructs `startxref`, detects `/Prev` cycles. |
| **PRD-FR-003** | File | Standard password authentication flow (R2–R6) | P0 | PARTIAL | TARGET | Rejects encrypted files fail-closed; accepts valid passwords via Rust handler. |
| **PRD-FR-004** | File | Clean document close and memory reclamation | P0 | CURRENT | CURRENT | Explicit memory deallocation; zero memory leaks across document cycles. |
| **PRD-FR-005** | Edit PDF | Native existing-text single and multi-span replacement | P0 | CURRENT | CURRENT | In-place text mutation preserving font size, color, baseline, and layout. |
| **PRD-FR-006** | Edit PDF | Multilingual text shaping (Arabic, Jawi, Hebrew, CJK, Devanagari) | P0 | CURRENT | CURRENT | HarfRust shaping, Skera subsetting, ToUnicode mapping; zero scrambled glyphs. |
| **PRD-FR-007** | Edit PDF | Native text styling (family, size, bold, italic, color, alignment) | P1 | CURRENT | CURRENT | Applies style patches, checks font catalog, embeds subsets as required. |
| **PRD-FR-008** | Edit PDF | Safe native text drag move with dependency check | P1 | CURRENT | CURRENT | Updates `Tm` matrix coordinates; refuses moves that disturb dependent text. |
| **PRD-FR-009** | Edit PDF | Arbitrary multi-line paragraph reflow across subsetted fonts | P3 | UNSUPPORTED_BY_POLICY | UNSUPPORTED_BY_POLICY | Refused to prevent font corruption and unintended visual overlap. |
| **PRD-FR-010** | Edit PDF | Image enumeration, bounds inspection, and selection | P0 | CURRENT | CURRENT | Extracts image XObject references, dimensions, color spaces, coordinates. |
| **PRD-FR-011** | Edit PDF | Image replacement (JPEG/PNG) with shared XObject isolation | P0 | CURRENT | CURRENT | Replaces image data in-place; clones shared XObjects so page 1 edit leaves page 2 intact. |
| **PRD-FR-012** | Edit PDF | Image drag move, resize, and delete | P0 | CURRENT | CURRENT | Direct overlay manipulation updates transformation matrix or removes `Do` call. |
| **PRD-FR-013** | Edit PDF | Vector graphics enumeration, color, and line styling | P1 | CURRENT | CURRENT | Enumerates paths, rectangles, lines; updates RGB stroke/fill and line width. |
| **PRD-FR-014** | Edit PDF | Bounded vector shape move, resize, and delete | P1 | CURRENT | CURRENT | Modifies bounding box of rectangles/lines; deletes path operator sequences. |
| **PRD-FR-015** | Edit PDF | Fine-grained Bézier node / knot editing | P3 | PARTIAL | DEFERRED | Bounded styling supported; granular control point editing deferred. |
| **PRD-FR-016** | Page Org | Page reorder, duplicate, delete, and insert blank | P0 | CURRENT | CURRENT | Updates `/Kids`, `/Count`, parent references; supports multi-level undo/redo. |
| **PRD-FR-017** | Page Org | Multi-page thumbnail organizer with drag-and-drop | P0 | CURRENT | CURRENT | Shift/Ctrl multi-selection, visual reordering, edge auto-scrolling. |
| **PRD-FR-018** | Page Org | Document split and page extraction (ranges, burst) | P0 | CURRENT | CURRENT | Extracts selected pages or comma-separated ranges into valid new PDFs. |
| **PRD-FR-019** | Page Org | Multi-document merge and page range import | P0 | CURRENT | CURRENT | Appends documents or imports selected pages into target document indices. |
| **PRD-FR-020** | Page Org | Page rotation (90°, 180°, 270°) and cropping | P1 | PARTIAL | TARGET | Updates `/Rotate` and `/CropBox` in page dictionary with instant canvas refresh. |
| **PRD-FR-021** | Annotations | FreeText annotation creation and live in-place editing | P0 | CURRENT | CURRENT | Edits `/Contents`, updates font, color, border, regenerates `/AP` stream. |
| **PRD-FR-022** | Annotations | Standard markup (Highlight, Underline, Strikeout, Squiggly) | P0 | CURRENT | CURRENT | Creates standard text markup annotations matching underlying text spans. |
| **PRD-FR-023** | Annotations | Geometric annotations (Square, Circle, Line, Arrow, Polygon) | P1 | CURRENT | CURRENT | Interactive creation and manipulation with configurable stroke/fill styles. |
| **PRD-FR-024** | Annotations | Sticky notes, comment threads, and status workflow | P1 | PARTIAL | TARGET | Adds point annotations with threaded popup comments and review status. |
| **PRD-FR-025** | Forms | AcroForm field discovery and hierarchical tree traversal | P0 | CURRENT | CURRENT | Discovers text fields, checkboxes, radios, choice widgets, resolves `/Rect`. |
| **PRD-FR-026** | Forms | Form field value updating and appearance (`/AP`) generation | P0 | CURRENT | CURRENT | Updates `/V`, `/AS`, regenerates ISO 32000-1 compliant appearance streams. |
| **PRD-FR-027** | Forms | Form field creation and editing (designer mode) | P1 | PARTIAL | TARGET | Drag-to-create new form fields, assign names, export values, validation rules. |
| **PRD-FR-028** | Forms | Form data import and export (XFDF, FDF, JSON, CSV) | P1 | PARTIAL | TARGET | Bidirectional serialization of form field values without modifying page layout. |
| **PRD-FR-029** | Fill & Sign | Quick signature placement (drawn, typed, image stamp) | P0 | CURRENT | CURRENT | Inserts transparent PNG signature stamp at clicked canvas coordinates. |
| **PRD-FR-030** | Signatures | Cryptographic digital signature detection & byte-range audit | P0 | CURRENT | CURRENT | Detects `/ByteRange`, `/Contents`, checks if modifications occurred post-signing. |
| **PRD-FR-031** | Signatures | Cryptographic signature validation (X.509, PKCS#7/CMS) | P1 | PARTIAL | TARGET | Verifies digest over `/ByteRange` bytes against certificate public key. |
| **PRD-FR-032** | Signatures | Digital signing with certificate and timestamp authority | P1 | TARGET | TARGET | Applies cryptographically valid signature revision with PAdES compliance. |
| **PRD-FR-033** | OCR | Production OCR orchestration (scan-to-searchable-PDF) | P1 | PARTIAL | TARGET | Preprocesses image, runs OCR engine, embeds invisible text layer (`3 Tr`). |
| **PRD-FR-034** | OCR | Multilingual OCR and editable text layer generation | P2 | TARGET | TARGET | Recognizes Latin, Arabic, CJK text; aligns OCR bounding boxes with raster. |
| **PRD-FR-035** | Redaction | Permanent data redaction (text, vector, raster, metadata) | P0 | PARTIAL | TARGET | Physically removes underlying stream operators and pixels; zero leakage. |
| **PRD-FR-036** | Redaction | Redaction inspection and verification diff | P1 | TARGET | TARGET | Displays exact objects and byte ranges removed before finalizing export. |
| **PRD-FR-037** | Compare | Rust-owned multi-mode comparison (text, layout, pixel diff) | P1 | PARTIAL | TARGET | Generates visual difference overlays and side-by-side structural reports. |
| **PRD-FR-038** | Accessibility | Tagged PDF inspection and reading order tree visualization | P1 | PARTIAL | TARGET | Inspects `/StructTreeRoot`, `/RoleMap`, tags (`<P>`, `<H1>`, `<Table>`). |
| **PRD-FR-039** | Accessibility | Accessibility checker and PDF/UA-1 remediation tools | P2 | TARGET | TARGET | Validates alt-text, contrast, headings, tab order; guides user remediation. |
| **PRD-FR-040** | Export | Incremental export preserving signatures and structure | P0 | CURRENT | CURRENT | Appends xref revisions without touching previous byte ranges. |
| **PRD-FR-041** | Export | Canonical / structural healing export | P0 | CURRENT | CURRENT | Writes cleaned xref stream, resolves orphan objects, normalizes trailer. |
| **PRD-FR-042** | Export | Standard conversion profiles (Text, Markdown, HTML, JSON) | P1 | PARTIAL | TARGET | Structured semantic extraction utilizing reading order and font metrics. |
| **PRD-FR-043** | Automation | StarPDF CLI for batch processing, extraction, and forms | P1 | PARTIAL | TARGET | Headless Rust command-line tool exposing complete engine capabilities. |
| **PRD-FR-044** | Automation | Model Context Protocol (MCP) server for AI pairing | P1 | CURRENT | TARGET | Exposes StarPDF tools via stdio/JSON-RPC for Claude/Codex agent interactions. |

---

## 6. Non-Functional Requirements (PRD-NFR)

| ID | Category | Requirement Description | Priority | Target Metric | Acceptance Criteria |
| :--- | :--- | :--- | :---: | :---: | :--- |
| **PRD-NFR-001** | Performance | Application Cold Startup Time | P0 | < 800 ms | From binary launch to interactive Home/Workspace view on baseline hardware. |
| **PRD-NFR-002** | Performance | Document Open Time (100-page PDF) | P0 | < 300 ms | Parses catalog, builds xref map, extracts page tree into memory. |
| **PRD-NFR-003** | Performance | First-Page Viewport Render | P0 | < 50 ms | Rasterizes or issues GPU draw calls for initial viewport. |
| **PRD-NFR-004** | Performance | Scrolling & Panning Frame Rate | P0 | 60–120 fps | Smooth viewport movement without frame drops or jitter. |
| **PRD-NFR-005** | Performance | In-Place Text Replacement Latency | P0 | < 25 ms | From keystroke commit to content stream patch and canvas invalidation. |
| **PRD-NFR-006** | Performance | Spatial Pointer Hit-Testing | P0 | < 10 µs | BVH query returns topmost hit candidate under cursor. |
| **PRD-NFR-007** | Performance | Memory Consumption (Idle) | P0 | < 60 MB RSS | Baseline application footprint without open documents. |
| **PRD-NFR-008** | Performance | Memory Scaling (1,000-page PDF) | P0 | < 250 MB RSS | Bounded memory usage via lazy object stream and font loading. |
| **PRD-NFR-009** | Reliability | Structural Invariant: `UNEXPLAINED CHANGE = 0` | P0 | 100% | Zero mutations permitted if uncertified object or byte changes exist. |
| **PRD-NFR-010** | Reliability | Crash Recovery & Autosave | P0 | < 2 s loss | Unsaved editing sessions recoverable from atomic journal files. |
| **PRD-NFR-011** | Security | Memory Safety & Fuzzing | P0 | Zero Panic | 100% Rust engine core with cargo-fuzz regression against malformed inputs. |
| **PRD-NFR-012** | Security | Fail-Closed Security Handler | P0 | 100% | Rejects unsupported encryption/permissions; never exposes plaintext illegally. |
| **PRD-NFR-013** | Privacy | Pure Local Processing | P0 | 0 network bytes | Verification that zero file bytes are transmitted over external networks. |
| **PRD-NFR-014** | Accessibility | Keyboard Navigation & WCAG 2.1 AA | P1 | 100% | Full keyboard access, ARIA roles, high-contrast theme support. |
| **PRD-NFR-015** | Platforms | Cross-Platform Parity | P0 | macOS/Win/Linux | Feature-identical desktop binaries; web/WASM builds where feasible. |

---

## 7. Platform & Hardware Targets

### 7.1 Desktop Native
- **macOS:** Apple Silicon (M1/M2/M3/M4) & Intel x86_64, macOS 12 (Monterey) through macOS 15+. Metal GPU backend via `wgpu`.
- **Windows:** Windows 10 & Windows 11 (x86_64 & ARM64). DirectX 12 / Vulkan GPU backend via `wgpu`.
- **Linux:** Modern distributions (Ubuntu 22.04+, Fedora 38+, Arch). Vulkan GPU backend via `wgpu`, Wayland & X11 support.

### 7.2 Web / WebAssembly
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
