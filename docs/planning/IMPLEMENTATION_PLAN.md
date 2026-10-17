# SmartPDF / StarPDF Canonical Implementation Plan

**Document ID:** DOC-PLAN-001  
**Status:** CANONICAL QUALIFIED (Phase 0 / Pre-Implementation)  
**System:** SmartPDF & StarPDF Strangler Migration  
**Baseline Commit:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` (`arch/integrated`)  
**Methodology:** Strangler Migration (Zero Big-Bang Rewrites; Continuous Working Software)  

---

## 1. Dual Release Track Architecture

To maintain strict operational discipline and avoid disrupting active release operations, the engineering roadmap is divided into two decoupled tracks:

- **TRACK A: Current Product & RC2 Release Candidate:**
  - **Objective:** Finalize, qualify, and ship SmartPDF v1.0.0-rc.2 on `main` and `arch/integrated`.
  - **Focus:** Complete manual acceptance testing (`docs/SMARTPDF_RC2_MANUAL_ACCEPTANCE.md`), ensure 100% pass across 171 Playwright E2E suites, verify the 32-fixture qualification corpus (`qualification/starpdf/corpus.json`), and freeze the stable release candidate.
  - **Invariant:** Architecture work in Track B must never modify, delay, or invalidate Track A deliverables.
- **TRACK B: Rust-First Desktop & Architecture Redevelopment:**
  - **Objective:** Multi-phase Strangler migration to establish `starpdf-app-core`, `starpdf-render-wgpu`, and `smartpdf-desktop`.
  - **Focus:** Phased development on dedicated architecture branches (`docs/rust-first-architecture`, `feat/starpdf-app-core`, etc.), moving state authority into Rust, building the native `wgpu` render pipeline, and packaging native desktop binaries.

---

## 2. Rust UI Framework Evaluation & Spike Decision Gate

To deliver Acrobat-class desktop ergonomics, high-DPI rendering, complex docking panels, native menus, international typography/IME, and hardware acceleration across macOS, Windows, Linux, and Web/WASM, six architectural paths were qualitatively assessed:

| Architecture Candidate | Strengths | Technical Risks & Open Questions | Candidate Status |
| :--- | :--- | :--- | :---: |
| **egui / eframe** | Extremely lightweight; immediate-mode simplicity; excellent Web/WASM integration; zero-copy texture blitting. | Lacks native OS accessibility (AccessKit early); basic IME/RTL support; game-like UI styling requiring extensive custom theming. | Viable fallback for internal tools |
| **Slint** | Native platform styling; first-class accessibility hooks; built-in IME; clean declarative syntax; responsive layout engine. | Requires commercial license review for closed-source or royalty-free terms; wgpu pass embedding needs empirical validation. | **Leading Candidate** (To be evaluated in Phase 1 spike) |
| **Iced** | Clean Elm-inspired architecture; pure Rust; active community; wgpu-based rendering. | Complex docking panels are limited; IME support is in active development; multi-window management is early. | Secondary Candidate |
| **Dioxus** | Familiar React-like ergonomics; shared code across desktop and web; strong ecosystem. | Higher memory footprint; difficult direct `wgpu` swapchain integration; desktop relies on system webview or blitz renderer. | Less suitable for high-perf PDF canvas |
| **Floem** | Fine-grained reactivity; GPU-powered; modern widget design inspired by Lapce. | Immature ecosystem; API volatility; limited production hardening on Windows and Linux. | Experimental |
| **Custom `winit` + `wgpu` (Direct)** | 100% control over swapchain, render passes, and delta invalidation; sub-100ms startup; <25MB idle memory. | High maintenance burden: requires building custom docking, tabs, menus, and text input from scratch. | **Target for Canvas Viewport Only** |

### 2.1 Executable Phase 1 / Desktop-Spike Decision Gate
Prior to committing to a final desktop UI framework in Phase 5, an isolated **Spike Prototype** (`apps/smartpdf-desktop-spike`) will be implemented and evaluated against seven concrete pass/fail criteria:

1. **IME & Multilingual Composition:** User can compose complex text (Japanese Hiragana/Kanji, Arabic with shaping, diacritical marks) without input drop or visual glitches on macOS (AppKit), Windows (TSF), and Linux (Wayland text-input-v3 / IBus).
2. **Accessibility (AT / Screen Reader):** UI controls expose valid AccessKit nodes recognized by VoiceOver (macOS) and Narrator (Windows).
3. **GPU Render Pass Embedding:** Slint/UI chrome and `wgpu` PDF canvas can render within a single unified swapchain without offscreen texture readbacks or CPU memory copies.
4. **Frame Responsiveness:** Maintains <16ms frame time (60–120 fps) during simultaneous UI panel resizing and canvas panning.
5. **Cold Startup Footprint:** Binary cold startup < 400ms; idle RSS memory < 45MB on standard desktop hardware.
6. **Multi-Window & Native Menus:** Supports opening multiple document windows sharing engine resources; integrates with native OS menu bars via `muda`.
7. **Licensing & Distribution:** Satisfies project distribution requirements without restrictive runtime royalty obligations.

**Rule:** Slint + `winit` + `wgpu` remains a **candidate to be proven by the spike gate**, not a foregone conclusion.

---

## 3. Target Modular Crate Structure

To avoid monolithic code while preventing excessive crate fragmentation, StarPDF adopts a layered crate hierarchy with strict one-way dependency direction (zero circular dependencies):

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

## 4. Phased Implementation Roadmap (Strangler Migration)

### Phase 0: Canonical Documentation & Baseline Freeze (CURRENT)
- **Objective:** Establish the immutable canonical single source of truth across all product, architecture, design, and planning requirements.
- **Baseline:** Commit `f7ff85464b190b74c89c1f8be9862be37090dd7f` on branch `arch/integrated`.
- **Dependencies:** None.
- **Workstreams:** Documentation authoring only. Zero production code changes.
- **Deliverables:** `PRD.md`, `TRD.md`, `APP_FLOW.md`, `DESIGN_BRIEF.md`, `BACKEND_SCHEMA.md`, `IMPLEMENTATION_PLAN.md`, `REQUIREMENT_TRACEABILITY.md`, `PROJECT_CONTEXT.md`, `AI_WORKFLOW.md`, `AGENTS.md`.
- **Acceptance Criteria:** All 9 documents created, reviewed, verified; clean Git staging; committed and pushed to `docs/rust-first-architecture`.

---

### Phase 1: Rust App-Core Ingestion Under Current SmartPDF
- **Objective:** Create `starpdf-app-core` crate in Rust. Move document session lifecycle, tab management, and active tool states from React into Rust. Conduct the UI Framework Desktop Spike.
- **Baseline:** Phase 0 canonical documentation.
- **Dependencies:** Existing `engine/starpdf`.
- **Workstreams:**
  - Create `crates/starpdf-app-core`.
  - Implement `SessionManager`, `DocumentSession`, `TabState`.
  - Expose `starpdf_app_core` WASM bindings to Next.js.
  - Implement and evaluate `apps/smartpdf-desktop-spike` against the decision gate.
- **Crates:** `starpdf-app-core`, `starpdf`.
- **Code Areas:** `engine/starpdf/src/wasm`, `src/lib/pdf/starpdf-client.ts`, `SmartPdfEditor.tsx`.
- **Tests:** `cargo test -p starpdf-app-core`, Vitest session tests.
- **Performance Gates:** Session switch latency < 10ms.
- **Acceptance Criteria:** React editor queries `starpdf-app-core` for active document/tab metadata. Zero regressions in Playwright test suites. UI framework spike gate evaluated.
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
- **Performance Gates:** BVH hit-test < 10 µs per query in Rust (targeting the ~5.8 µs demonstrated in synthetic TS benchmark).
- **Acceptance Criteria:** Pointer click returns exact `RenderNodeId` and `StarNodeId` in sub-millisecond time.
- **Rollback:** Continue using PDF.js text layer DOM hit testing.
- **Stop Condition:** Benchmark confirms Rust BVH matches or exceeds synthetic prototype targets.

---

### Phase 5: Rust Desktop Shell Prototype (`smartpdf-desktop`)
- **Objective:** Build standalone native desktop binary running on macOS, Windows, and Linux using `winit` and selected UI framework.
- **Baseline:** Phase 4 completed; Phase 1 spike gate evaluated.
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
- **Performance Gates:** First-page render < 30ms (target); steady 60–120fps panning.
- **Acceptance Criteria:** 0 differing pixels on supported text and image primitives compared to oracle.
- **Rollback:** PDF.js fallback overlay on diverged tiles.
- **Stop Condition:** Differential test suite reports zero regressions across smoke corpus.

---

### Phase 7: Professional Rust Desktop Workspace
- **Objective:** Implement full desktop UI chrome (Home, All Tools, Document Workspace, Docking Rails, Contextual Toolbars).
- **Baseline:** Phase 6 completed.
- **Dependencies:** `smartpdf-desktop`, `starpdf-app-core`.
- **Workstreams:**
  - Embed declarative UI chrome overlay into the `wgpu` render pass.
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
  - Implement complex path tessellation, blend modes, and shadings.
  - Calibrate rendering across full 1,000-fixture target corpus.
- **Crates:** `starpdf-render-wgpu`.
- **Code Areas:** `apps/smartpdf-desktop/`, `engine/starpdf/`.
- **Tests:** Large-scale automated visual diff across PDF.js, Poppler, and StarPDF.
- **Performance Gates:** 99.5% visual equivalence with PDF.js across target corpus.
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

## 5. Parallel Workstream Matrix

The following streams can safely proceed concurrently without merge conflicts:

| Workstream | Focus Subsystem | Crates Involved | Can Run Concurrently With |
| :--- | :--- | :--- | :--- |
| **Stream A: App-Core** | Session, Tabs, History, Commands | `starpdf-app-core` | Stream B, Stream C |
| **Stream B: Render Pipeline** | Render IR, BVH, wgpu Shaders | `starpdf-render-ir`, `starpdf-render-wgpu` | Stream A, Stream D |
| **Stream C: Security & Crypto** | Signatures, X.509, Redaction | `starpdf-security`, `starpdf-signatures` | Stream A, Stream B |
| **Stream D: Automation & CLI** | CLI Tooling, MCP Server | `smartpdf-cli`, `starpdf-mcp` | Stream B, Stream C |
