# SmartPDF / StarPDF Technical Requirements Document (TRD)

**Document ID:** DOC-TRD-001  
**Status:** CANONICAL DRAFT (Phase 0 / Pre-Implementation)  
**System:** SmartPDF Application & StarPDF Engine  
**Baseline Commit:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` (`arch/integrated`)  
**Target Architecture:** Rust-First Native Cross-Platform Application  

---

## 1. Architectural Philosophy & Invariants

SmartPDF is architected on the principle of **Authoritative Rust Ownership**. While existing web implementations provide an initial working baseline, the long-term system mandates that all PDF state, mutation semantics, selection identity, undo/redo history, security gating, file export, and native rendering reside in Rust.

### Core Mathematical Invariants
1. **Delta State Evolution:**
   $$G(t+1) = G(t) \oplus \Delta$$
   Every document state transition is an explicit, immutable delta applied to the authoritative document graph.
2. **Certified Zero Unexplained Change:**
   $$\text{UNEXPLAINED CHANGE}(\Delta) = 0$$
   A mutation is refused by the engine if any modified object, source range, or resource reference falls outside the declared `MutationEnvelope`.
3. **Bounded Structural Mutation Time:**
   $$T(\text{edit}) \approx \mathcal{O}(|C_s(\Delta)|)$$
   Execution time depends strictly on the size of the structural dependency closure $C_s(\Delta)$, never on total document size.
4. **Bounded Visual Invalidation:**
   $$T(\text{render}) \approx \mathcal{O}(|C_v(\Delta)| + \log N)$$
   Repaint time is bounded by the visual closure $C_v(\Delta)$ queried through a spatial index over $N$ primitives.
5. **Bounded Serialization Time:**
   $$T(\text{write}) \approx \mathcal{O}(|\Delta_{\text{objects}}|)$$
   Incremental save persists only modified and added objects, leaving existing byte ranges bit-for-bit unchanged.

---

## 2. Current Architecture vs. Target Architecture

### 2.1 Current Qualified Architecture (`arch/integrated` @ `f7ff854`)

In the current baseline, StarPDF operates as a Rust core compiled to WebAssembly, embedded inside a Next.js / React 19 web application shell (`SmartPdfEditor.tsx`). PDF.js serves as the primary rendering engine and DOM text overlay provider, while StarPDF executes semantic mutations, adaptive font shaping, and page operations.

```mermaid
graph TD
    subgraph Browser_Runtime [Browser Runtime (Next.js / React 19)]
        UI[SmartPdfEditor UI Shell]
        Overlay[PdfInteractiveOverlay & ContextualToolbar]
        Cmd[TS Command Layer: src/lib/pdf/commands]
        PDFJS[PDF.js 5.6 Canvas & TextLayer (Visual Oracle)]
    end

    subgraph WASM_Bridge [WebAssembly Bridge]
        Client[StarPdfClient: src/lib/pdf/starpdf-client.ts]
        WasmGlue[starpdf_wasm.js & serde-wasm-bindgen]
    end

    subgraph Rust_WASM_Engine [StarPDF Engine: engine/starpdf]
        WasmApi[WASM API: src/wasm/api.rs]
        Registry[Document Registry: src/wasm/registry.rs]
        Doc[PdfDocument & ObjectStore]
        MutationEngine[Mutation Engine: src/mutation]
        FontEngine[Adaptive Font Runtime: HarfRust + Skera]
        ExecutionShadow[Execution Contracts & Shadow State]
        Xref[Xref Recovery & Incremental Writer]
    end

    UI --> Overlay
    Overlay --> Cmd
    Cmd --> Client
    Client --> WasmGlue
    WasmGlue --> WasmApi
    WasmApi --> Registry
    Registry --> Doc
    Doc --> MutationEngine
    Doc --> FontEngine
    Doc --> ExecutionShadow
    Doc --> Xref
    Doc -.->|Export Bytes| Client
    Client -.->|Reload Bytes| PDFJS
    PDFJS -.->|Render Pixels| UI
```

#### Current Characteristics & Limitations
- **Hybrid Authority:** While StarPDF owns content mutations, UI state (selection, current tool, history stack, zoom, viewport pan) is split between React component state (`SmartPdfEditor.tsx`), Zustand stores, and the TypeScript command layer (`src/lib/pdf/commands/history.ts`).
- **Full Canvas Reload on Edit:** After a mutation, StarPDF serializes an incremental PDF byte array, transfers it across the WASM boundary to TypeScript, and PDF.js re-parses and re-renders the modified page.
- **Browser Thread Dependency:** Although web workers handle heavier batch operations, UI interaction latency is subject to the JavaScript event loop and DOM layout engine.

---

### 2.2 Target Architecture: Pure Rust-First Desktop & Core System

In the target architecture, `starpdf-app-core` becomes the single authority for document lifecycle, tool state, selection, undo/redo, command dispatch, and viewport state. The renderer is a native GPU pipeline powered by `wgpu`. The UI layer (whether native desktop or Web/WASM) is a thin, declarative presentation shell that emits user intent and receives immutable render frames.

```mermaid
graph TD
    subgraph UI_Surface [SmartPDF UI Shell (Native Desktop or WASM Host)]
        Input[Keyboard, Pointer, Window Events]
        Panels[Docking Panels, Toolbars, Menus]
        ViewSurface[GPU Canvas / Texture Swapchain]
    end

    subgraph StarPDF_App_Core [starpdf-app-core (Rust)]
        SessionMgr[Document Session & Tab Manager]
        ToolState[Tool State & Command Router]
        HistoryMgr[Delta History & Undo/Redo Engine]
        SelModel[Provenance-Preserving Selection Model]
        CoordConv[Hierarchical Coordinate Engine]
    end

    subgraph StarPDF_Engine_Core [StarPDF Engine Core (Rust)]
        Parser[Zero-Copy Lexer & Object Parser]
        XrefCore[Xref, Object Streams & Recovery]
        StarGraph[Authoritative Object Graph & COS Model]
        SecHandler[Security, Encryption & Signatures]
        FontRuntime[HarfRust + Fontations + Skera]
        DeltaEngine[Mutation Engine + Cs(Δ) + Cv(Δ)]
        Certifier[Mutation Envelope & Certificate Validator]
    end

    subgraph Render_Pipeline [StarPDF Delta-Native Render Pipeline]
        Compiler[Render Compiler: StarGraph -> Star Render IR]
        RenderIR[Render IR Primitives with StarNodeId]
        SpatialIndex[Packed BVH Spatial Index]
        WgpuBackend[wgpu Hardware Rasterizer (Metal / DX12 / Vulkan / WebGPU)]
    end

    subgraph Output_Writers [StarPDF Writers]
        IncrementalWriter[Incremental Writer]
        HealingWriter[Structural Healing Writer]
        ExportProfiles[PDF/A, Flattened, Redacted, Text, JSON]
    end

    Input --> ToolState
    Panels --> ToolState
    ToolState --> SelModel
    ToolState --> HistoryMgr
    HistoryMgr --> DeltaEngine
    DeltaEngine --> Certifier
    Certifier --> StarGraph
    StarGraph --> Compiler
    Compiler --> RenderIR
    RenderIR --> SpatialIndex
    SpatialIndex --> WgpuBackend
    WgpuBackend --> ViewSurface
    StarGraph --> OutputWriters
    OutputWriters --> FileSystem[Local Filesystem / Storage]
```

---

## 3. Subsystem Technical Specifications

### 3.1 StarPDF Structural Core (`starpdf-syntax`, `starpdf-cos`, `starpdf-xref`)
- **Zero-Copy Parser:** Lexer operates on memory-mapped slices (`&[u8]`) with strict boundary checks, preventing heap allocations for tokens, object identifiers, and operators.
- **Xref & Stream Engine:**
  - Classic table parsing, xref stream decoding (`/Type /XRef`), and compressed object stream extraction (`/Type /ObjStm`).
  - Resilient recovery: Scans backwards from end-of-file for `startxref`, tracks `/Prev` links with cycle detection, isolates corrupt revisions, and falls back to brute-force object reconstruction if trailers are truncated.
- **COS Object Model:** Strict type-safe representation of PDF primitives (`Boolean`, `Integer`, `Real`, `String`, `Name`, `Array`, `Dictionary`, `Stream`, `Null`, `Reference`).

### 3.2 Provenance & Execution Contracts (`starpdf-execution`)
Established in baseline contracts (`engine/starpdf/src/execution/`):
- **`StarNodeId`:** Opaque, canonical identifier for an object occurrence:
  `sn1/<doc_hex>/<obj_num>/<generation>/<occurrence_path_hex>`
  Guarantees stability across coordinate moves and revision updates.
- **`RenderNodeId`:** Visual primitive identifier linking directly to structural identity:
  `rn1/<canonical_starnodeid_hex>/<primitive_key_hex>`
- **`EvidenceLevel`:**
  - `VERIFIED`: Proven by direct, undamaged xref reference and matching checksum.
  - `RECOVERED_VERIFIED`: Reconstructed via structural recovery with validated page reachability.
  - `INFERRED`: Derived via heuristic catalog/stream discovery; flagged for user inspection.
  - `UNTRUSTED`: Unchecked or corrupt structure; prohibited from participating in certified mutations.
- **`MutationEnvelope`:** Strict contract defining target objects, allowable byte ranges, affected visual bounds, expected font/text changes, and forbidden structural dependencies.
- **`MutationCertificate`:** Result of executing `MutationEnvelope::compare`:
  Returns `EXPLAINED`, `UNEXPLAINED`, or `REFUSED`. Any deviation outside the envelope yields an immediate halt (`UNEXPLAINED CHANGE = 0`).
- **Closures:**
  - $C_s(\Delta)$ (Structural Closure): All indirect objects, resources, and byte ranges structurally modified.
  - $C_v(\Delta)$ (Visual Closure): All screen/page bounding boxes affected by old and new rendering extents.

### 3.3 Text & Multilingual Font Subsystem (`starpdf-fonts`, `starpdf-text`)
- **OpenType Shaping:** Powered by **HarfRust**, executing full GSUB (glyph substitution, ligatures, contextual alternates) and GPOS (glyph positioning, kerning, mark placement).
- **Subsetting & Embedding:** Powered by **Skera** and **write-fonts**, generating compact TrueType font programs retaining only active glyph IDs.
- **Script Handling & Bidi:** `unicode-bidi` segments text runs into directional levels; script segmentation dispatches appropriate HarfRust shaping engines for:
  - Arabic and Jawi (supporting specific Jawi characters `\u06A4`, `\u06A0`, `\u06BD`, `\u0686`).
  - Hebrew (RTL with diacritical mark positioning).
  - Devanagari (complex conjuncts and reordering).
  - CJK (Unified Ideographs across Japanese, Simplified Chinese, Traditional Chinese, Korean).
- **Exact Unicode Extraction:** Generates `/ToUnicode` CMaps mapping occurrence-specific CIDs back to UTF-32 code points, guaranteeing perfect copy/paste and full-text search.

### 3.4 Hardware-Accelerated Native Renderer (`starpdf-render-wgpu`)
The long-term rendering target replaces PDF.js with a native, delta-aware GPU pipeline:
1. **Render Compiler:** Compiles `StarGraph` page content streams into **Star Render IR**:
   - `GlyphRun`: Glyph IDs, positions, font asset reference, fill/stroke colors.
   - `ImageInstance`: Decoded texture handle, sampling filter, affine matrix, mask reference.
   - `PathInstance`: Tessellated triangles or Bézier curves, line width, join/cap styles, stroke/fill paints.
   - `ClipNode`: Bounding clip geometry or stencil mask hierarchy.
   - `FormInstance`: Sub-stream appearance reference with local transform.
   - `ShadingInstance` / `PatternInstance`: Axial/radial gradients, tiling patterns.
   - `AnnotationPrimitive` / `FormWidgetPrimitive`: Native appearance representations.
2. **Spatial Index:** **Packed Bounding Volume Hierarchy (BVH)**.
   - Empirically benchmarked (`benchmark.ts`): ~5.8 µs per query over 10,000 primitives (17× faster than linear scan).
   - Provides $O(\log N)$ point hit-testing, marquee selection, and viewport culling.
3. **GPU Backend (`wgpu`):**
   - Metal on macOS.
   - DirectX 12 / Vulkan on Windows.
   - Vulkan on Linux.
   - WebGPU in modern browsers.
4. **Delta-Native Viewport Updates:** Repaints only the bounding union of $C_v(\Delta)$, avoiding full-page re-rasterization.

### 3.5 Security, Signatures & Redaction (`starpdf-security`, `starpdf-signatures`, `starpdf-redaction`)
- **Security Handler:** Supports standard PDF encryption (R2 through R6, 40-bit RC4 to 256-bit AES). Authenticates user and owner passwords; validates permission flags (`/P`). Fails closed on malformed or unsupported crypt filters.
- **Digital Signatures:**
  - Audits `/ByteRange` coverage; detects post-signing byte tampering.
  - Validates CMS/PKCS#7 signatures against X.509 certificate chains.
  - Creates append-only signed revisions with designated timestamp authorities (RFC 3161).
- **Forensic Redaction:**
  - True physical deletion: Splices content stream operators, removes marked glyphs/paths, tiles and re-encodes underlying raster images, purges unreachable objects.
  - Sanitizes hidden metadata, bookmarks, annotations, and form field entries within the redaction zone.

---

## 4. Cross-Platform Abstraction Architecture

To support macOS, Windows, Linux, and Web/WASM cleanly without leaking platform details into `starpdf-app-core`, StarPDF establishes platform abstraction traits:

```mermaid
classDiagram
    class PlatformHost {
        <<interface>>
        +open_file_dialog(filter) Future~Option~PathBuf~~
        +save_file_dialog(default_name) Future~Option~PathBuf~~
        +read_clipboard() Future~Option~ClipboardData~~
        +write_clipboard(data) Future~Result~~
        +request_render() void
        +set_cursor(cursor_type) void
        +open_url(url) void
    }

    class NativeDesktopHost {
        -winit_window: Arc~Window~
        -rfd_dialogs: RfdAdapter
        -arboard_clipboard: ArboardAdapter
        +open_file_dialog()
        +write_clipboard()
    }

    class WebWasmHost {
        -web_sys_window: web_sys::Window
        -dom_canvas: web_sys::HtmlCanvasElement
        +open_file_dialog()
        +write_clipboard()
    }

    PlatformHost <|-- NativeDesktopHost
    PlatformHost <|-- WebWasmHost
```

---

## 5. Testing, Qualification & Continuous Integration

### 5.1 Quality Gates
Every code promotion must satisfy the four-tier verification matrix:
1. **Engine Unit & Integration Tests (Rust):**
   - `cargo test --all-features`: 100% pass across parser, xref, mutations, font shaping, security, and execution contracts.
   - `cargo clippy -- -D warnings`: Deny all warnings.
   - `cargo fmt -- --check`: Strict formatting enforcement.
2. **Web Client & Integration Tests (TypeScript / Vitest):**
   - `npm run typecheck`: Strict TypeScript checking with zero errors.
   - `npm run test`: Vitest suite covering commands, selection, WASM bridge, and mock sessions.
3. **End-to-End Browser Qualification (Playwright):**
   - Chromium, Firefox, WebKit execution over 171 automated multi-step scenarios.
   - Validates document open, edit text, image replace, form fill, page reorder, export, and reopen.
4. **Differential Oracle Qualification (`qualification/starpdf`):**
   - Python-driven corpus harness executing controlled mutations across diverse PDF corpora.
   - Differential comparison against Poppler, PDF.js, and qpdf oracles.
   - Asserts **`unexplained_change_count == 0`** and zero visual divergence outside $C_v(\Delta)$.

### 5.2 Python & Machine Learning Boundary
- Python is restricted strictly to offline ML research, OCR training, dataset generation, and regression benchmarking (`qualification/`).
- Standard users and desktop installations **do not require Python**. Production OCR runs via self-contained native Rust engines (e.g., Tesseract C-bindings or ONNX Runtime).
