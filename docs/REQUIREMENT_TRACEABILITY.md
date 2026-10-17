# SmartPDF / StarPDF Requirement Traceability Matrix (RTM)

**Document ID:** DOC-RTM-001  
**Status:** CANONICAL DRAFT (Phase 0 / Pre-Implementation)  
**System:** SmartPDF & StarPDF End-to-End Traceability  
**Baseline Commit:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` (`arch/integrated`)  

---

## 1. Functional Requirements Traceability Matrix

| Requirement ID | Source Document | Current Implementation | Target Subsystem | Associated Tests | Current Status | Target Phase | Known Gap / Planned Evolution |
| :--- | :--- | :--- | :--- | :--- | :---: | :---: | :--- |
| **PRD-FR-001** | PRD §5 | `engine/starpdf/src/document/` | `starpdf-cos` | `parser_tests.rs`, `real_world_tests.rs` | `CURRENT` | Baseline | Fully qualified; linear scanning optimizations planned. |
| **PRD-FR-002** | PRD §5 | `engine/starpdf/src/xref/` | `starpdf-xref` | `xref_prev_recovery_tests.rs` | `CURRENT` | Baseline | Recovers corrupt `/Prev`; full object reconstruction in phase 3. |
| **PRD-FR-003** | PRD §5 | `engine/starpdf/src/security.rs` | `starpdf-security` | `compat_security_tests.rs` | `PARTIAL` | Phase 8 | Rejection is fail-closed; AES/RC4 decryption pipeline targeted. |
| **PRD-FR-004** | PRD §5 | `src/wasm/registry.rs` | `starpdf-app-core` | `starpdf.test.ts` | `CURRENT` | Baseline | Memory freed explicitly on close; RAII guard in app-core planned. |
| **PRD-FR-005** | PRD §5 | `engine/starpdf/src/mutation/` | `starpdf-mutation` | `existing_text_editing_v0_13_tests.rs` | `CURRENT` | Baseline | Single & multi-span in-place replacement verified. |
| **PRD-FR-006** | PRD §5 | `engine/starpdf/src/font/` | `starpdf-fonts` | `multilingual_font_tests.rs` | `CURRENT` | Baseline | HarfRust + Skera handles Arabic, Jawi, Hebrew, CJK, Devanagari. |
| **PRD-FR-007** | PRD §5 | `src/font/style.rs` | `starpdf-fonts` | `basic_text_formatting_tests.rs` | `CURRENT` | Baseline | Applies bold, italic, font family, color patches. |
| **PRD-FR-008** | PRD §5 | `src/mutation/text_move.rs` | `starpdf-mutation` | `text_move_v0_21_tests.rs` | `CURRENT` | Baseline | Moves `Tm` safely; refuses moves with dependent text. |
| **PRD-FR-009** | PRD §5 | `docs/FINAL_CAPABILITY_MATRIX` | N/A (Policy) | N/A | `UNSUPPORTED_BY_POLICY` | Permanent | Reflow across subsetted fonts refused to prevent corruption. |
| **PRD-FR-010** | PRD §5 | `src/image/extractor.rs` | `starpdf-images` | `image_operations_v0_14_tests.rs` | `CURRENT` | Baseline | Extracts XObject dimensions, color spaces, coordinates. |
| **PRD-FR-011** | PRD §5 | `src/image/editor.rs` | `starpdf-images` | `phase2db-images.spec.ts` | `CURRENT` | Baseline | Replaces images; isolates shared XObjects automatically. |
| **PRD-FR-012** | PRD §5 | `src/image/editor.rs` | `starpdf-images` | `image_operations_v0_14_tests.rs` | `CURRENT` | Baseline | Direct bounding box drag move, resize, and delete. |
| **PRD-FR-013** | PRD §5 | `src/vector/extractor.rs` | `starpdf-graphics` | `vector_operations_v0_15_tests.rs` | `CURRENT` | Baseline | Enumerates rectangles, lines, paths; updates RGB styling. |
| **PRD-FR-014** | PRD §5 | `src/vector/editor.rs` | `starpdf-graphics` | `vector_operations_v0_15_tests.rs` | `CURRENT` | Baseline | Modifies bounding coordinates; deletes path operators. |
| **PRD-FR-015** | PRD §5 | `docs/FINAL_CAPABILITY_MATRIX` | `starpdf-graphics` | N/A | `PARTIAL` | Phase 8 | Bounded styling supported; granular knot editing deferred. |
| **PRD-FR-016** | PRD §5 | `src/page_ops/` | `starpdf-page-ops` | `page_operations_v0_12_tests.rs` | `CURRENT` | Baseline | `/Kids` and `/Count` mutated with full undo/redo. |
| **PRD-FR-017** | PRD §5 | `PdfThumbnailRail.tsx` | `smartpdf-desktop` | `smartpdf-editor.spec.ts` | `CURRENT` | Phase 7 | React HTML5 drag-and-drop; native Slint rail in Phase 7. |
| **PRD-FR-018** | PRD §5 | `SmartPdfSplitModal.tsx` | `starpdf-page-ops` | `page_operations_v0_12_tests.rs` | `CURRENT` | Baseline | Splits and extracts page ranges or burst single pages. |
| **PRD-FR-019** | PRD §5 | `src/page_ops/incremental.rs`| `starpdf-page-ops` | `page_merge_v0_12b_tests.rs` | `CURRENT` | Baseline | Appends documents or imports selected page sets. |
| **PRD-FR-020** | PRD §5 | `src/page_ops/` | `starpdf-page-ops` | `page_tree_tests.rs` | `PARTIAL` | Phase 7 | Page `/Rotate` supported; visual crop tool in Phase 7. |
| **PRD-FR-021** | PRD §5 | `src/annotation/` | `starpdf-annotations`| `annotation_mutation_tests.rs` | `CURRENT` | Baseline | FreeText creation, font/color controls, `/AP` generation. |
| **PRD-FR-022** | PRD §5 | `src/annotation/types.rs` | `starpdf-annotations`| `annotation_tests.rs` | `CURRENT` | Baseline | Highlight, Underline, Strikeout annotation creation. |
| **PRD-FR-023** | PRD §5 | `src/annotation/generator.rs`| `starpdf-annotations`| `annotation_tests.rs` | `CURRENT` | Baseline | Square, Circle, Line annotations with configurable stroke/fill. |
| **PRD-FR-024** | PRD §5 | `src/annotation/parser.rs` | `starpdf-annotations`| `annotation_tests.rs` | `PARTIAL` | Phase 8 | Point comments detected; threaded replies in Phase 8. |
| **PRD-FR-025** | PRD §5 | `src/forms/parser.rs` | `starpdf-forms` | `acroform_tests.rs` | `CURRENT` | Baseline | Traverses form field hierarchy, resolves terminal `/Rect`. |
| **PRD-FR-026** | PRD §5 | `src/appearance/` | `starpdf-forms` | `acroform_tests.rs` | `CURRENT` | Baseline | Updates `/V`, `/AS`; regenerates ISO 32000 `/AP` streams. |
| **PRD-FR-027** | PRD §5 | `src/forms/widget.rs` | `starpdf-forms` | `acroform_tests.rs` | `PARTIAL` | Phase 8 | Fill mode fully exposed; form builder designer in Phase 8. |
| **PRD-FR-028** | PRD §5 | `src/wasm/api.rs` | `starpdf-forms` | `acroform_tests.rs` | `PARTIAL` | Phase 8 | Internal value updates supported; XFDF/JSON import in Phase 8. |
| **PRD-FR-029** | PRD §5 | `src/image/editor.rs` | `starpdf-images` | `phase2db-images.spec.ts` | `CURRENT` | Baseline | Inserts drawn or stamp signatures as transparent PNG. |
| **PRD-FR-030** | PRD §5 | `src/security.rs` | `starpdf-security` | `compat_security_tests.rs` | `CURRENT` | Baseline | Detects `/ByteRange`, `/Contents`, checks post-sign edits. |
| **PRD-FR-031** | PRD §5 | `src/security.rs` | `starpdf-signatures` | `compat_security_tests.rs` | `PARTIAL` | Phase 8 | Detection verified; cryptographic digest check in Phase 8. |
| **PRD-FR-032** | PRD §5 | Planned | `starpdf-signatures` | Target suite | `TARGET` | Phase 8 | PAdES / RFC 3161 digital signing pipeline targeted. |
| **PRD-FR-033** | PRD §5 | `tesseract.js` (Web only) | `starpdf-ocr` | Target suite | `PARTIAL` | Phase 8 | Client web OCR prototype; native Rust OCR orchestration targeted. |
| **PRD-FR-034** | PRD §5 | Planned | `starpdf-ocr` | Target suite | `TARGET` | Phase 8 | Multilingual OCR text layer generation (`3 Tr`) targeted. |
| **PRD-FR-035** | PRD §5 | Planned | `starpdf-redaction` | Target suite | `PARTIAL` | Phase 8 | UI redaction markers; true stream operator splicing targeted. |
| **PRD-FR-036** | PRD §5 | Planned | `starpdf-redaction` | Target suite | `TARGET` | Phase 8 | Pre-export redaction sanitization inspection targeted. |
| **PRD-FR-037** | PRD §5 | `src/lib/pdf/execution/` | `starpdf-compare` | `differential-smoke.ts` | `PARTIAL` | Phase 8 | Differential pixel smoke test; document compare tool in Phase 8. |
| **PRD-FR-038** | PRD §5 | `src/compatibility.rs` | `starpdf-accessibility`| `compat_security_tests.rs`| `PARTIAL` | Phase 8 | Outlines/OCGs inspected; structure tree editor in Phase 8. |
| **PRD-FR-039** | PRD §5 | Planned | `starpdf-accessibility`| Target suite | `TARGET` | Phase 8 | PDF/UA accessibility validation and remediation targeted. |
| **PRD-FR-040** | PRD §5 | `src/writer/incremental.rs` | `starpdf-writer` | `incremental_writer_tests.rs`| `CURRENT` | Baseline | Strict append-only xref revisions preserving prior bytes. |
| **PRD-FR-041** | PRD §5 | `src/writer/complete.rs` | `starpdf-writer` | `recovered_xref_stream_export_tests.rs`| `CURRENT` | Baseline | Normalized xref streams, garbage collection, trailer fix. |
| **PRD-FR-042** | PRD §5 | `src/text/extractor.rs` | `starpdf-convert` | `text_extractor_tests.rs` | `PARTIAL` | Phase 8 | Plain text extraction verified; Markdown/HTML in Phase 8. |
| **PRD-FR-043** | PRD §5 | Planned | `smartpdf-cli` | Target suite | `PARTIAL` | Phase 8 | Core engine headless; dedicated CLI binary in Phase 8. |
| **PRD-FR-044** | PRD §5 | `../toolrakyat/src/mcp/` | `starpdf-mcp` | `mcp-server.test.ts` | `CURRENT` | Phase 8 | MCP server working in TypeScript; Rust port in Phase 8. |

---

## 2. Non-Functional Requirements Traceability Matrix

| Requirement ID | Target Metric | Current Implementation / Measured Baseline | Target Subsystem | Status | Target Phase |
| :--- | :---: | :--- | :--- | :---: | :---: |
| **PRD-NFR-001** | Cold Startup < 800ms | ~1,200ms (Next.js web shell cold start) | `smartpdf-desktop` | `PARTIAL` | Phase 5 (<350ms native) |
| **PRD-NFR-002** | Open (100-page) < 300ms | ~180ms (Measured in Rust test suite) | `starpdf-cos` | `CURRENT` | Baseline |
| **PRD-NFR-003** | First-Page Render < 50ms | ~85ms (PDF.js initial rasterization) | `starpdf-render-wgpu` | `PARTIAL` | Phase 6 (<25ms GPU) |
| **PRD-NFR-004** | Frame Rate 60–120 fps | 60 fps (Browser DOM scroll) | `starpdf-render-wgpu` | `CURRENT` | Phase 6 (120 fps swapchain) |
| **PRD-NFR-005** | Text Edit Latency < 25ms | ~14ms (Rust mutation + HarfRust shaping) | `starpdf-mutation` | `CURRENT` | Baseline |
| **PRD-NFR-006** | Spatial Hit-Test < 10 µs | ~5.8 µs (Measured in `benchmark.ts` packed BVH) | `starpdf-render-ir` | `CURRENT` | Baseline |
| **PRD-NFR-007** | Memory (Idle) < 60 MB | ~110 MB (Browser process footprint) | `smartpdf-desktop` | `PARTIAL` | Phase 5 (<35MB native) |
| **PRD-NFR-008** | Memory (1k pages) < 250 MB | ~195 MB (Lazy object store in Rust) | `starpdf-cos` | `CURRENT` | Baseline |
| **PRD-NFR-009** | Unexplained Change = 0 | 100% verified on all promoted mutations | `starpdf-execution` | `CURRENT` | Baseline |
| **PRD-NFR-010** | Crash Loss < 2s | 5s autosave snapshot in React state | `starpdf-app-core` | `PARTIAL` | Phase 2 (Atomic journal) |
| **PRD-NFR-011** | Zero Engine Panic | Cargo-fuzz pass across parser and filters | `starpdf-syntax` | `CURRENT` | Baseline |
| **PRD-NFR-012** | Fail-Closed Security | Rejects unsupported encryption with typed error | `starpdf-security` | `CURRENT` | Baseline |
| **PRD-NFR-013** | Pure Local Processing | 0 network requests during editing workflows | All Subsystems | `CURRENT` | Baseline |
| **PRD-NFR-014** | Keyboard & WCAG 2.1 AA | 100% keyboard shortcut coverage in web editor | `smartpdf-desktop` | `CURRENT` | Baseline |
| **PRD-NFR-015** | Cross-Platform Parity | Web/Chromium/Firefox/WebKit qualified | `smartpdf-desktop` | `PARTIAL` | Phase 5 (Native multi-OS) |
