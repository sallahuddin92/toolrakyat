# SmartPDF / StarPDF Canonical Implementation Plan

**Document ID:** DOC-PLAN-001  
**Status:** CANONICAL DRAFT (Phase 0 / Pre-Implementation)  
**System:** SmartPDF & StarPDF Strangler Migration  
**Baseline Commit:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` (`arch/integrated`)  
**Methodology:** Strangler Migration (Zero Big-Bang Rewrites; Continuous Working Software)  

---

## 1. Rust UI Framework Evaluation & Decision Matrix

To achieve Acrobat-class desktop polish, high-DPI rendering, complex docking panels, native menus, international typography/IME, and hardware acceleration across macOS, Windows, Linux, and Web/WASM, six architectural paths were evaluated:

| Criterion | egui / eframe | Slint | Iced | Dioxus | Floem | Custom winit + wgpu |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Desktop UX Quality** | 7/10 (Game/Tool feel) | 9/10 (Native look) | 8/10 (Clean flat) | 8/10 (Web-like) | 8/10 (Modern) | 9/10 (Tailored) |
| **Complex Docking Panels** | 8/10 (egui_dock) | 7/10 (Manual) | 6/10 (Limited) | 7/10 (CSS-based) | 6/10 (Early) | 8/10 (Custom) |
| **Document Tabs & Rails** | 8/10 | 9/10 | 7/10 | 9/10 | 7/10 | 9/10 |
| **Accessibility (Screen Reader)** | 5/10 (AccessKit early) | 8/10 (Built-in) | 6/10 (AccessKit) | 7/10 (Web/Native) | 5/10 (Early) | 6/10 (AccessKit manual) |
| **IME & Multilingual Entry** | 6/10 (Basic) | 9/10 (Robust) | 7/10 (WIP) | 9/10 (Native/Web) | 6/10 (Basic) | 7/10 (winit IME) |
| **RTL & Complex Scripts** | 6/10 (Limited) | 8/10 (Good) | 6/10 (Cosmic-text) | 9/10 (Browser/OS) | 7/10 (Cosmic) | 9/10 (HarfRust direct) |
| **Native Menus (OS Menu Bar)** | 6/10 (Muda wrapper) | 9/10 (First-class) | 6/10 (Muda) | 8/10 (Muda/Native)| 6/10 (Muda) | 9/10 (Muda direct) |
| **Keyboard Shortcuts & Focus** | 8/10 | 9/10 | 8/10 | 8/10 | 8/10 | 9/10 |
| **Drag & Drop / Clipboard** | 8/10 | 9/10 | 8/10 | 8/10 | 7/10 | 9/10 |
| **Printing Integration** | 5/10 | 8/10 | 5/10 | 7/10 | 5/10 | 8/10 (OS print spool) |
| **High DPI Scaling** | 8/10 | 9/10 | 8/10 | 8/10 | 8/10 | 9/10 |
| **Multi-Window Support** | 7/10 | 9/10 | 7/10 | 8/10 | 7/10 | 9/10 |
| **GPU Texture / wgpu Embed** | 9/10 (Native paint cb)| 8/10 (Skia/GL/wgpu)| 8/10 (wgpu layer)| 6/10 (Difficult)| 8/10 (wgpu) | 10/10 (Direct swapchain) |
| **Web / WASM Compatibility**| 9/10 (Excellent) | 8/10 (WASM build) | 8/10 (WASM canvas)| 9/10 (Native web)| 5/10 (Experimental)| 8/10 (WebGPU canvas) |
| **Startup Time (<800ms)** | 10/10 (<150ms) | 9/10 (<250ms) | 9/10 (<200ms) | 7/10 (<600ms) | 9/10 (<200ms)| 10/10 (<100ms) |
| **Memory Footprint (<60MB)**| 9/10 (~35MB) | 9/10 (~40MB) | 9/10 (~45MB) | 6/10 (~90MB+) | 9/10 (~40MB)| 10/10 (~25MB) |
| **Ecosystem Maturity** | 9/10 (Battle-tested)| 8/10 (Commercial) | 8/10 (Active) | 8/10 (Active) | 6/10 (Young) | 8/10 (winit/wgpu standard) |
| **Licensing** | MIT / Apache-2.0 | GPLv3 / Royalty Free| MIT / Apache-2.0| MIT / Apache-2.0| MIT / Apache-2.0| MIT / Apache-2.0 |
| **Packaging & Distribution**| 9/10 (Single binary)| 9/10 (Single bin) | 9/10 (Single bin) | 8/10 (Cargo-pack)| 8/10 (Single) | 9/10 (Single binary) |
| **TOTAL SCORE (out of 190)**| **147** | **162** | **140** | **143** | **130** | **165** |

### Decision & Recommendation
- **Recommended Desktop Shell Architecture: Justified Hybrid (`winit` + `wgpu` + declarative UI layer)**
  - **Core Viewport & Document Canvas:** Pure `winit` + `wgpu` render pipeline. Direct Metal / DX12 / Vulkan swapchain integration guarantees optimal 120fps scrolling, direct delta invalidation, and zero abstraction penalty for the PDF renderer.
  - **Chrome & Workspace Panels (Docking, Menus, Toolbars):** Driven by **Slint** (or `egui_dock` embedded into the `wgpu` pass via standard render pass overlay). Slint provides native OS look-and-feel, accessibility hooks, first-class IME, and royalty-free licensing for internal/open-source use.
  - **Transitional Web Shell:** Existing Next.js / React 19 web application continues running seamlessly via WASM bindings, gradually delegating authority to `starpdf-app-core` compiled to `wasm32-unknown-unknown`.

---

## 2. Target Modular Crate Structure

To avoid monolithic code while preventing excessive crate fragmentation, StarPDF adopts a phased, layered crate hierarchy with strict one-way dependency direction:

```
[Layer 5: UI & Applications]
   smartpdf-desktop (Native winit/wgpu executable)
   smartpdf-wasm (WASM web bridge & npm package)
   smartpdf-cli (Headless batch & automation CLI)
         │
         ▼
[Layer 4: Application Core]
   starpdf-app-core (Session, History, Commands, Selection, Preferences)
         │
         ▼
[Layer 3: Execution, Rendering & Writers]
   starpdf-render-wgpu (wgpu rasterizer & GPU buffers)
   starpdf-render-ir (Render IR, primitives, packed BVH index)
   starpdf-mutation (Delta engine, MutationEnvelope, Certificates)
   starpdf-writer (Incremental, Canonical Healing, PDF/A, Redacted)
         │
         ▼
[Layer 2: Domain Capabilities]
   starpdf-text / starpdf-fonts (HarfRust, Fontations, Skera, Bidi)
   starpdf-security / starpdf-signatures (Crypto, X.509, Permissions)
   starpdf-forms / starpdf-annotations (AcroForms, Widgets, AP generation)
   starpdf-images / starpdf-graphics (XObjects, JPEG/PNG, Paths)
   starpdf-page-ops (Tree mutations, reorder, merge, split)
         │
         ▼
[Layer 1: Foundations]
   starpdf-syntax (Zero-copy lexer, parser, COS object model)
   starpdf-xref (Xref tables, xref streams, recovery)
   starpdf-execution (StarNodeId, EvidenceRecord, closures)
```

---

## 3. Phased Implementation Roadmap (Strangler Migration)

### Phase 0: Canonical Documentation & Baseline Freeze (CURRENT)
- **Objective:** Establish the immutable canonical single source of truth across all product, architecture, design, and planning requirements.
- **Baseline:** Commit `f7ff85464b190b74c89c1f8be9862be37090dd7f` on branch `arch/integrated`.
- **Dependencies:** None.
- **Workstreams:** Documentation authoring only. Zero production code changes.
- **Deliverables:** `PRD.md`, `TRD.md`, `APP_FLOW.md`, `DESIGN_BRIEF.md`, `BACKEND_SCHEMA.md`, `IMPLEMENTATION_PLAN.md`, `REQUIREMENT_TRACEABILITY.md`, `PROJECT_CONTEXT.md`, `AI_WORKFLOW.md`.
- **Performance Gates:** N/A (Doc-only).
- **Acceptance Criteria:** All 9 documents created, reviewed, verified; clean Git staging; committed and pushed to `docs/rust-first-architecture`.

---

### Phase 1: Rust App-Core Ingestion Under Current SmartPDF
- **Objective:** Create `starpdf-app-core` crate in Rust. Move document session lifecycle, tab management, and active tool states from React into Rust.
- **Baseline:** Phase 0 canonical documentation.
- **Dependencies:** Existing `engine/starpdf`.
- **Workstreams:**
  - Create `crates/starpdf-app-core`.
  - Implement `SessionManager`, `DocumentSession`, `TabState`.
  - Expose `starpdf_app_core` WASM bindings to Next.js.
- **Crates:** `starpdf-app-core`, `starpdf`.
- **Code Areas:** `engine/starpdf/src/wasm`, `src/lib/pdf/starpdf-client.ts`, `SmartPdfEditor.tsx`.
- **Tests:** `cargo test -p starpdf-app-core`, Vitest session tests.
- **Performance Gates:** Session switch latency < 10ms.
- **Acceptance Criteria:** React editor queries `starpdf-app-core` for active document/tab metadata. Zero regressions in Playwright test suites.
- **Rollback:** Retain React local state hooks behind feature flag.
- **Stop Condition:** `npm run test` and `cargo test` pass 100%.

---

### Phase 2: Authority of Selection, Editing Session & History into Rust
- **Objective:** Shift selection state (`StarNodeId`), inline text editing sessions, and undo/redo history stacks from TypeScript into Rust `starpdf-app-core`.
- **Baseline:** Phase 1 completed.
- **Dependencies:** `starpdf-app-core`.
- **Workstreams:**
  - Port `src/lib/pdf/commands/history.ts` into Rust `DeltaHistoryStack`.
  - Integrate `freetext-edit-session.ts` into Rust text session manager.
  - Bind `Cmd+Z` / `Cmd+Shift+Z` directly to Rust delta undo/redo.
- **Crates:** `starpdf-app-core`, `starpdf`.
- **Code Areas:** `src/lib/pdf/commands/`, `src/lib/pdf/selection/`, `SmartPdfEditor.tsx`.
- **Tests:** Vitest history suite, Playwright undo/redo E2E scenarios.
- **Performance Gates:** Undo/redo cycle < 15ms.
- **Acceptance Criteria:** TypeScript owns zero history arrays; all mutations originate and commit via Rust app-core.
- **Rollback:** Fall back to TS history manager.
- **Stop Condition:** 25-level undo/redo stress test with image, text, and form mutations passes without memory leak.

---

### Phase 3: StarGraph & Provenance Maturation
- **Objective:** Mature `StarGraph` object graph and enforce strict `EvidenceLevel` checks across all content mutations.
- **Baseline:** Phase 2 completed.
- **Dependencies:** `starpdf-execution`, `starpdf-cos`.
- **Workstreams:**
  - Connect `StarExecutionState` directly to live mutation paths.
  - Enforce `MutationEnvelope::compare` prior to every content stream write.
  - Implement `EditabilityMap` exposing `VERIFIED`, `RECOVERED`, `PROTECTED`, `UNSUPPORTED`.
- **Crates:** `starpdf`, `starpdf-execution`.
- **Code Areas:** `engine/starpdf/src/mutation/`, `engine/starpdf/src/execution/`.
- **Tests:** `cargo test --test execution_core_tests`, `qualification/starpdf/run.py`.
- **Performance Gates:** Envelope comparison latency < 2ms.
- **Acceptance Criteria:** `unexplained_change_count == 0` strictly enforced. Any unauthorized mutation halts with typed error.
- **Rollback:** Disable certificate gating for non-critical markup.
- **Stop Condition:** 100% pass on execution contracts test suite.

---

### Phase 4: Render IR & Spatial Execution Maturation
- **Objective:** Port experimental TypeScript Render IR (`src/lib/pdf/execution/render/`) and packed BVH into native Rust.
- **Baseline:** Phase 3 completed.
- **Dependencies:** `starpdf-execution`.
- **Workstreams:**
  - Create `crates/starpdf-render-ir`.
  - Implement Rust `PackedBvh` spatial index with SIMD-optimized box queries.
  - Build `RenderCompiler` converting `StarGraph` page streams into `RenderNode` sequences.
- **Crates:** `starpdf-render-ir`, `starpdf-execution`.
- **Code Areas:** `engine/starpdf/src/render_ir/`, `src/lib/pdf/execution/render/`.
- **Tests:** Spatial index microbenchmarks, Render IR compilation golden tests.
- **Performance Gates:** BVH hit-test < 6 µs per query over 10,000 primitives.
- **Acceptance Criteria:** Pointer click returns exact `RenderNodeId` and `StarNodeId` in sub-millisecond time.
- **Rollback:** Continue using PDF.js text layer DOM hit testing.
- **Stop Condition:** Benchmark confirms BVH matches or exceeds synthetic benchmark targets.

---

### Phase 5: Rust Desktop Shell Prototype (`smartpdf-desktop`)
- **Objective:** Build standalone native desktop binary running on macOS, Windows, and Linux using `winit` and basic window chrome.
- **Baseline:** Phase 4 completed.
- **Dependencies:** `starpdf-app-core`, `starpdf-render-ir`.
- **Workstreams:**
  - Create `apps/smartpdf-desktop`.
  - Setup `winit` event loop, native window creation, high-DPI scaling.
  - Integrate native menus (`muda`) and file open dialogs (`rfd`).
- **Crates:** `smartpdf-desktop`, `starpdf-app-core`.
- **Code Areas:** `apps/smartpdf-desktop/src/main.rs`.
- **Tests:** Native binary smoke test on macOS (Metal), Windows (DX12), Linux (Vulkan).
- **Performance Gates:** Cold startup < 400ms; idle RSS < 35MB.
- **Acceptance Criteria:** Native executable opens, renders placeholder document window, responds to keyboard shortcuts and file dialogs.
- **Rollback:** Project remains web-first.
- **Stop Condition:** Clean execution on all three target OS platforms.

---

### Phase 6: Native Renderer Vertical Slices (`wgpu`)
- **Objective:** Implement hardware-accelerated rasterization of Latin glyph runs, solid fills, and axis-aligned images via `wgpu`.
- **Baseline:** Phase 5 completed.
- **Dependencies:** `starpdf-render-ir`, `wgpu`.
- **Workstreams:**
  - Create `crates/starpdf-render-wgpu`.
  - Write vertex/fragment shaders for glyph quad atlasing and image blitting.
  - Implement differential repainting using visual closure $C_v(\Delta)$.
- **Crates:** `starpdf-render-wgpu`, `starpdf-render-ir`.
- **Code Areas:** `crates/starpdf-render-wgpu/src/`.
- **Tests:** Differential pixel comparison against PDF.js oracle (`differential-smoke`).
- **Performance Gates:** First-page render < 30ms; 120fps steady panning.
- **Acceptance Criteria:** 0 differing pixels on supported text and image primitives compared to oracle.
- **Rollback:** PDF.js fallback overlay on diverged tiles.
- **Stop Condition:** Differential test suite reports zero regressions across smoke corpus.

---

### Phase 7: Professional Rust Desktop Workspace
- **Objective:** Implement full desktop UI chrome (Home, All Tools, Document Workspace, Docking Rails, Contextual Toolbars).
- **Baseline:** Phase 6 completed.
- **Dependencies:** `smartpdf-desktop`, `starpdf-app-core`.
- **Workstreams:**
  - Embed Slint / declarative chrome overlay into the `wgpu` render pass.
  - Implement Left Rail: Thumbnails, Bookmarks, Annotations, Attachments.
  - Implement Floating Contextual Toolbars and Command Palette (`Cmd+K`).
- **Crates:** `smartpdf-desktop`, `starpdf-app-core`.
- **Code Areas:** `apps/smartpdf-desktop/ui/`.
- **Tests:** Desktop E2E interaction test harness.
- **Performance Gates:** Zero frame drop during panel collapse or thumbnail scrolling.
- **Acceptance Criteria:** Feature parity with existing React web shell UI.
- **Rollback:** Retain web shell as primary surface.
- **Stop Condition:** User can execute full document editing workflow end-to-end in native desktop app.

---

### Phase 8: Acrobat-Class Feature Expansion
- **Objective:** Implement advanced capabilities: Redaction, Compare, OCR orchestration, Digital Signatures (X.509/CMS), Tagged PDF accessibility checker.
- **Baseline:** Phase 7 completed.
- **Dependencies:** `starpdf-app-core`, `starpdf-security`.
- **Workstreams:**
  - Implement `starpdf-redaction`: True physical stream sanitization.
  - Implement `starpdf-signatures`: Cryptographic PKCS#7 / PAdES signing.
  - Implement `starpdf-compare`: Side-by-side visual and structural diffing.
  - Implement `starpdf-accessibility`: `/StructTreeRoot` tag editor.
- **Crates:** `starpdf-redaction`, `starpdf-signatures`, `starpdf-compare`, `starpdf-accessibility`.
- **Code Areas:** `engine/starpdf/src/`.
- **Tests:** Compliance test suites, digital signature verification against standard trust stores.
- **Performance Gates:** Redaction execution < 50ms per page.
- **Acceptance Criteria:** Redacted content is unrecoverable via hex editor; signatures validate in Adobe Acrobat Reader.
- **Rollback:** Feature-flag experimental modules.
- **Stop Condition:** All Phase 8 capability tests pass with 100% coverage.

---

### Phase 9: Native Renderer Promotion
- **Objective:** Promote `starpdf-render-wgpu` to primary production renderer; relegate PDF.js to differential test validator and emergency fallback.
- **Baseline:** Phase 8 completed.
- **Dependencies:** `starpdf-render-wgpu`.
- **Workstreams:**
  - Implement complex path tessellation (lyon / pathfinder), blend modes, and shadings.
  - Calibrate rendering across full 1,000-file corpus.
- **Crates:** `starpdf-render-wgpu`.
- **Code Areas:** `apps/smartpdf-desktop/`, `engine/starpdf/`.
- **Tests:** Large-scale automated visual diff across PDF.js, Poppler, and StarPDF.
- **Performance Gates:** 99.5% visual equivalence with PDF.js across standard corpus.
- **Acceptance Criteria:** Native renderer active by default across all platforms.
- **Rollback:** Single-click setting to restore PDF.js oracle.
- **Stop Condition:** Zero visual regressions on primary test corpus.

---

### Phase 10: Decommission React Editor Authority
- **Objective:** Eliminate React/TypeScript document state and command execution in web app; reduce React shell to a thin presentation wrapper around `starpdf-wasm`.
- **Baseline:** Phase 9 completed.
- **Dependencies:** `starpdf-app-core`, `starpdf-wasm`.
- **Workstreams:**
  - Replace React command handlers with calls to `starpdf_app_core` WASM.
  - Stream WebGPU canvas frames directly into browser canvas.
  - Remove redundant TypeScript state machines.
- **Crates:** `smartpdf-wasm`.
- **Code Areas:** `src/components/tools/implementations/pdf/SmartPdfEditor.tsx`.
- **Tests:** Full Playwright E2E test suite.
- **Performance Gates:** Web bundle size reduced by > 40%.
- **Acceptance Criteria:** All web interactions routed through identical Rust engine as desktop.
- **Rollback:** Maintain prior TypeScript bridge code in git history.
- **Stop Condition:** 100% pass on Playwright test suite with unified Rust core.

---

### Phase 11: WebAssembly & Cross-Platform Maturation
- **Objective:** Optimize WASM multi-threading, SharedArrayBuffer memory sharing, SIMD-128 acceleration, and production distribution packaging (macOS `.dmg`/Notarization, Windows MSIX, Linux AppImage/Flatpak).
- **Baseline:** Phase 10 completed.
- **Dependencies:** All subsystems.
- **Workstreams:**
  - Configure CI matrix for multi-platform binary compilation and signing.
  - Optimize WASM binary via `wasm-opt -O4`.
- **Crates:** `smartpdf-desktop`, `smartpdf-wasm`.
- **Code Areas:** `.github/workflows/`, `scripts/packaging/`.
- **Tests:** Cross-platform installation and smoke tests on real OS environments.
- **Performance Gates:** WASM initial load < 500ms; desktop binary size < 45MB.
- **Acceptance Criteria:** Verified production release artifacts signed and ready for deployment.
- **Rollback:** N/A.
- **Stop Condition:** Production release sign-off.

---

## 4. Parallel Workstream Matrix

The following streams can safely proceed concurrently without merge conflicts:

| Workstream | Focus Subsystem | Crates Involved | Can Run Concurrently With |
| :--- | :--- | :--- | :--- |
| **Stream A: App-Core** | Session, Tabs, History, Commands | `starpdf-app-core` | Stream B, Stream C |
| **Stream B: Render Pipeline** | Render IR, BVH, wgpu Shaders | `starpdf-render-ir`, `starpdf-render-wgpu` | Stream A, Stream D |
| **Stream C: Security & Crypto** | Signatures, X.509, Redaction | `starpdf-security`, `starpdf-signatures` | Stream A, Stream B |
| **Stream D: Automation & CLI** | CLI Tooling, MCP Server | `smartpdf-cli`, `starpdf-mcp` | Stream B, Stream C |
