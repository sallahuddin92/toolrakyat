# SmartPDF / StarPDF Project Context & Engineering Overview

**Document ID:** DOC-CTX-001  
**Status:** CANONICAL QUALIFIED (Phase 0 / Pre-Implementation)  
**Product:** SmartPDF  
**Engine:** StarPDF  
**Repository:** `sallahuddin92/toolrakyat`  
**Current Qualified Integration Baseline:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` on branch `arch/integrated`  
**Dedicated Architecture Branch:** `docs/rust-first-architecture`  

---

## 1. Executive Summary & Identity

SmartPDF is a world-class, local-first professional PDF application powered by the StarPDF engine. Designed to provide Adobe Acrobat Pro-class capability, familiarity, and workflow breadth, SmartPDF is engineered for users who demand professional document manipulation without compromising on privacy, memory safety, or structural precision.

Unlike conventional PDF applications that either wrap legacy C/C++ libraries prone to security vulnerabilities or offer shallow web overlays that damage underlying document structures, SmartPDF is built on a **pure-Rust foundation with zero C-dependencies**. It guarantees mathematical proof that edits are contained within strict, declared boundaries where **UNEXPLAINED CHANGE = 0**.

### 1.1 Product vs. Engine Distinction
- **SmartPDF:** The end-user application suite, comprising the desktop application (macOS, Windows, Linux), web workspace, UI design tokens, panels, tools, shortcuts, and user-facing workflows.
- **StarPDF:** The low-level, high-performance, memory-safe PDF systems engine written in Rust. StarPDF owns parsing, COS object graph representation, xref recovery, incremental revisions, HarfRust font shaping, subsetting, layout calculation, execution contracts, security gating, and hardware-accelerated rendering.

### 1.2 Dual Release Track Strategy
- **TRACK A (Current Product & RC2 Release Candidate):** Stabilize and qualify the existing SmartPDF web application and StarPDF engine on `main` and `arch/integrated`. Completes manual acceptance testing (`docs/SMARTPDF_RC2_MANUAL_ACCEPTANCE.md`), verifies the 32-fixture qualification corpus (`qualification/starpdf/corpus.json`), and freezes `v1.0.0-rc.2`. Architecture work in Track B must NEVER rewrite, delay, or invalidate Track A deliverables.
- **TRACK B (Rust-First Desktop & Architecture Redevelopment):** Strangler migration creating `starpdf-app-core`, `starpdf-render-wgpu`, `smartpdf-desktop`, bringing 100% of state and rendering authority into Rust.

---

## 2. Baseline Status & Current Qualified Capabilities

The verified integration baseline at commit `f7ff85464b190b74c89c1f8be9862be37090dd7f` on branch `arch/integrated` represents an operational, qualified WebAssembly and React application. The repository has qualified the following capabilities across 171 automated Playwright E2E suites and extensive Rust unit/integration tests:

1. **Robust PDF Parsing & Recovery:**
   - Full support for standard xref tables, hybrid xrefs, compressed xref streams (`/Type /XRef`), and object streams (`/Type /ObjStm`).
   - Fault-tolerant recovery of malformed xref tables, missing `startxref` tokens, and broken or cyclic `/Prev` revision chains.
   - Clean structural healing export that normalizes damaged documents into canonical xref streams.
2. **Native Existing-Text Editing & Typography:**
   - In-place single-span and multi-span text replacement within content streams, preserving original baseline, font size, and color.
   - Groundbreaking **Adaptive Multilingual Font Runtime** powered by **HarfRust** (OpenType GSUB/GPOS shaping), **Fontations/Skrifa**, and **Skera** subsetting.
   - Comprehensive multilingual and complex script support: Arabic, Jawi (including specific characters `ڤ`, `ڠ`, `ڽ`, `چ`), Hebrew (RTL), Devanagari, Japanese, Simplified Chinese, Traditional Chinese, Korean, and mixed-script text.
   - Occurrence-specific `/ToUnicode` CMap generation guaranteeing 100% exact Unicode copy/paste and full-text search upon export and reopen.
   - Safe text repositioning: direct `Tm` matrix editing with automatic dependency verification (refuses moves that disturb dependent downstream text).
3. **Raster Image Manipulation:**
   - Enumeration of image XObjects, extraction of bounds, color spaces, and dimensions.
   - In-place image replacement (JPEG/PNG) with automatic shared XObject isolation (`is_shared` check clones XObject so modifying page 1 never corrupts cloned instances on page 2).
   - Direct spatial drag-move, corner resizing, and deletion of images.
   - Insertion of handwritten signatures and stamp images with alpha transparency.
4. **Vector Graphics & Paths:**
   - Enumeration of rectangles, lines, and complex Bézier path operator sequences.
   - In-place mutation of RGB stroke colors, fill colors, and line widths.
   - Direct manipulation of vector bounding boxes and path deletion.
5. **Interactive AcroForms & Widgets:**
   - Traversal of AcroForm field hierarchies and terminal widget resolution.
   - Spatial editing and state synchronization for text fields, checkboxes (`/AS` and `/V` synchronization), radio button groups, and dropdown choice menus.
   - ISO 32000-compliant `/AP` (Appearance Stream) regeneration with embedded font programs.
6. **Markup & FreeText Annotations:**
   - Discovery and editing of FreeText, Highlight, Underline, Strikeout, Square, Circle, Line, and Ink annotations.
   - Live on-canvas FreeText editing with real-time appearance regeneration and styling controls (`freetext-edit-session.ts`).
7. **Document Assembly & Page Management:**
   - Page tree mutations: reorder, delete, duplicate, and insert blank pages with full multi-level undo/redo.
   - Multi-page organizer rail with Shift/Ctrl multi-selection and drag-and-drop.
   - Page extraction (selected pages, ranges, burst) and multi-document merging.
8. **Security & Cryptographic Gating:**
   - Detection of Standard R2–R6 and Public-Key encryption; fail-closed refusal (`DecryptionUnavailable`). Actual decryptor is TARGET.
   - Detection of `/ByteRange` and digital signature contents; auditing of post-signing modifications. Cryptographic verification is `NOT_PERFORMED` in baseline.
   - Execution contracts foundation (`StarNodeId`, `RenderNodeId`, `EvidenceLevel`, `ObjectState`, `DocumentDelta`, `MutationEnvelope`, `MutationCertificate`, $C_s(\Delta)$, $C_v(\Delta)$).

---

## 3. Language & Technology Ownership Policy

To guarantee security, deterministic performance, and long-term maintainability, the project strictly enforces language boundaries:

```
┌────────────────────────────────────────────────────────────────────────┐
│                              RUST                                      │
│  OWNS THE PRODUCT. Authoritative for 100% of PDF state, parsing,       │
│  COS model, xref, recovery, security, encryption, signatures, fonts,   │
│  shaping, text, images, vectors, forms, annotations, OCR, redaction,   │
│  comparison, page tree, mutations, incremental writer, app-core,       │
│  selection, history, persistence, autosave, renderer (wgpu), CLI, MCP. │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
         ┌─────────────────────────┴──────────────────────────┐
         ▼                                                    ▼
┌──────────────────────────────────┐      ┌──────────────────────────────┐
│            TYPESCRIPT            │      │            PYTHON            │
│  Transitional web shell only.    │      │  Offline ML/Research only.   │
│  Strictly limited to existing    │      │  Restricted to model         │
│  SmartPDF Next.js web UI and     │      │  training, OCR evaluation,   │
│  minimal WASM host glue.         │      │  and corpus qualification.   │
│  MUST NOT retain authority for   │      │  Standard users NEVER        │
│  state, mutation, or rendering.  │      │  require Python installed.   │
└──────────────────────────────────┘      └──────────────────────────────┘
```

---

## 4. Key Architectural Invariants & Differentiators

StarPDF introduces several foundational innovations that distinguish it from legacy PDF software:

1. **Proof-Gated Partial Editability:** Documents are never classified simply as "editable" or "read-only." Editability is computed per object and occurrence based on font embedding, encoding, encryption, and cryptographic signatures.
2. **Provenance-Preserving Selection:**
   $$\text{Screen Pixel} \longleftrightarrow \text{RenderNodeId} \longleftrightarrow \text{StarNodeId} \longleftrightarrow \text{Semantic Object} \longleftrightarrow \text{PDF Operator} \longleftrightarrow \text{Source Byte Range}$$
   Every selection maintains a bidirectional link from the screen pixel down to the exact byte offset in the PDF file.
3. **Mutation Envelopes & Certificates (`UNEXPLAINED CHANGE = 0`):**
   Prior to applying any mutation, an expected `MutationEnvelope` is declared. Upon execution, `MutationEnvelope::compare` verifies that actual changes match expected changes. Any uncertified object or range modification halts promotion immediately.
4. **Delta State Evolution:**
   $$G(t+1) = G(t) \oplus \Delta$$
   Document updates are explicit, ordered, and mathematically reversible, ensuring predictable multi-level undo/redo.
5. **Structural and Visual Closures:**
   - $C_s(\Delta)$: The complete set of structural dependencies (objects, fonts, resources) affected by an edit.
   - $C_v(\Delta)$: The exact visual bounding boxes on affected pages requiring invalidation and repaint.
6. **Delta-Native Rendering:**
   Mutations do not trigger full document re-parsing. The rendering compiler ingests the exact same `DocumentDelta` to update the spatial index and issue localized GPU draw calls.
7. **Content-Addressed Deduplication:**
   Deterministic semantic hashing (SHA-256 with `StarPDF.semantic.v1` domain separation) prevents redundant parsing, shaping, tessellation, or GPU texture uploads.

---

## 5. Rendering Pipeline & Migration Strategy

### 5.1 Current vs. Target Renderer
- **Current Production Renderer:** **PDF.js (v5.6)** running in the browser. It serves as the primary visual display and differential pixel oracle.
- **Experimental Prototype:** A lightweight TypeScript canvas adapter (`src/lib/pdf/execution/render/`) featuring a packed Bounding Volume Hierarchy (BVH) spatial index (~5.8 µs per query benchmarked over 10,000 synthetic rects on local desktop hardware).
- **Target Production Renderer:** **`starpdf-render-wgpu`**, a native hardware-accelerated GPU pipeline written in Rust:
  - Compiles `StarGraph` into **Star Render IR** (`GlyphRun`, `ImageInstance`, `PathInstance`, `ClipNode`, `FormInstance`).
  - Queries primitives via a SIMD-accelerated packed BVH in Rust.
  - Blits via `wgpu` using Metal on macOS, DirectX 12 / Vulkan on Windows, Vulkan on Linux, and WebGPU in browsers.
- **Strangler Rule:** PDF.js will **NOT** be removed until `starpdf-render-wgpu` independently proves 99.5%+ visual parity across a 1,000-fixture target compatibility corpus.

---

## 6. Important Repository Paths

```
toolrakyat/
├── apps/
│   └── smartpdf-desktop/         <-- (Target) Native desktop application shell
├── engine/
│   └── starpdf/                  <-- Core Rust engine (Cargo workspace)
│       ├── src/
│       │   ├── syntax/           <-- Lexer, parser, COS objects
│       │   ├── xref/             <-- Xref table/stream, recovery
│       │   ├── document/         <-- PdfDocument, ObjectStore, pages
│       │   ├── font/             <-- HarfRust, Skera, font catalog, CMap
│       │   ├── mutation/         <-- Text, image, vector mutation engines
│       │   ├── execution/        <-- StarNodeId, Delta, Envelopes, Closures
│       │   ├── security.rs       <-- Encryption detection, permissions, signatures
│       │   ├── writer/           <-- Incremental & complete healing writers
│       │   └── wasm/             <-- WASM bindings & DTOs
│       └── tests/                <-- Extensive Rust test suites
├── qualification/
│   └── starpdf/                  <-- Regression harness (32 fixtures in corpus.json)
├── src/
│   ├── app/smartpdf/             <-- Fullscreen Next.js web application route
│   ├── components/tools/implementations/pdf/
│   │   ├── SmartPdfEditor.tsx    <-- Primary web editor shell
│   │   ├── PdfInteractiveOverlay.tsx <-- Direct manipulation canvas overlay
│   │   ├── PdfContextualToolbar.tsx  <-- Floating contextual action bar
│   │   └── PdfThumbnailRail.tsx  <-- Collapsible thumbnail & organizer rail
│   └── lib/pdf/
│       ├── starpdf-client.ts     <-- TypeScript client wrapping WASM
│       ├── commands/             <-- Typed command layer & history
│       └── execution/            <-- TS execution contracts & experimental renderer
└── docs/                         <-- Canonical project documentation
    ├── product/PRD.md            <-- Product Requirements Document
    ├── technical/TRD.md          <-- Technical Requirements Document
    ├── product/APP_FLOW.md       <-- User Application Flows
    ├── design/DESIGN_BRIEF.md    <-- Design Tokens & Interaction Brief
    ├── technical/BACKEND_SCHEMA.md <-- State Architecture & Schemas
    ├── planning/IMPLEMENTATION_PLAN.md <-- 12-Phase Strangler Roadmap
    ├── REQUIREMENT_TRACEABILITY.md     <-- End-to-End Traceability Matrix
    ├── PROJECT_CONTEXT.md        <-- This document
    └── AI_WORKFLOW.md            <-- Protocol for future AI/Codex agents
```

---

## 7. Migration Roadmap Summary

The evolution of SmartPDF follows an 11-phase Strangler migration pattern:
- **Phase 0:** Canonical documentation & baseline freeze *(Completed in this run)*.
- **Phase 1:** Ingest `starpdf-app-core` underneath current web UI; conduct desktop UI spike.
- **Phase 2:** Transfer authority of selection, editing sessions, and history into Rust.
- **Phase 3:** Mature `StarGraph` and enforce `MutationCertificate` checks.
- **Phase 4:** Port Render IR and packed BVH spatial index into Rust.
- **Phase 5:** Build native `smartpdf-desktop` shell (`winit` + `wgpu`).
- **Phase 6:** Implement `wgpu` hardware rasterization vertical slices.
- **Phase 7:** Implement professional desktop UI workspace.
- **Phase 8:** Expand Acrobat-class features (Redaction, Compare, Signatures, OCR, Accessibility).
- **Phase 9:** Promote `starpdf-render-wgpu` to primary renderer.
- **Phase 10:** Decommission React editor state authority in web shell.
- **Phase 11:** Mature production packaging (macOS DMG, Windows MSIX, Linux Flatpak/AppImage, WASM).

---

## 8. Immediate Next Milestone: Phase 1

The immediate next objective is **Phase 1: Rust App-Core Ingestion Under Current SmartPDF**:
- Create `crates/starpdf-app-core`.
- Implement `SessionManager` and `DocumentSession` in Rust.
- Expose session and tab query bindings through WASM.
- Implement the UI Framework Desktop Spike (`apps/smartpdf-desktop-spike`) to evaluate Slint + `winit` + `wgpu` against the 7-point decision gate.
- Hook `SmartPdfEditor.tsx` to read document session status directly from Rust without altering current visual behavior.
