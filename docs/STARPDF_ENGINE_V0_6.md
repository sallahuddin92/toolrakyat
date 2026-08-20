# StarPDF Engine v0.6 — AcroForm Parsing, Annotation Structures & Native PDF Mutation Foundation

**Status:** Completed & Validated  
**Date:** 2026-08-20  
**Repository:** `toolrakyat` (`engine/starpdf`)  
**Safety & Concurrency:** `#![forbid(unsafe_code)]`, 0 `.unwrap()` / `.expect()` in production sources, single-threaded WASM / multi-worker compatible.

---

## 1. Executive Summary

StarPDF Engine v0.6 establishes the first **native document-mutation foundation** within the proprietary Rust PDF engine. Prior versions (v0.1 through v0.5) delivered container parsing, font handling, coordinate-aware text extraction, search geometry, and the browser WebAssembly runtime. Milestone v0.6 empowers StarPDF to understand document forms (`/AcroForm`), widget and page annotations (`/Annots`), build deterministic mutation plans, and write incremental PDF updates adhering strictly to ISO 32000-1 (PDF 1.7) section 7.5.6.

All mutations preserve original document bytes verbatim, appending only modified indirect objects and a contiguous classic cross-reference subsection with an updated trailer dictionary referencing `/Prev`.

---

## 2. Architectural Components

```
                      Document Catalog
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
      /Pages Tree                        /AcroForm
            │                                 │
     Page Dictionary                          ▼
    /Annots -> Widgets ◄──────────► /Fields -> Field Tree
            │                                 │
            └───────────────┬─────────────────┘
                            ▼
                     MutationEngine
                    (PdfChange Batch)
                            │
                            ▼
                      MutationPlan
                 (Modified Objects Map)
                            │
                            ▼
                    IncrementalWriter
                 (Verbatim Prefix Append)
```

### 2.1 AcroForm & Field Hierarchy Parser (`forms/`)
- **Structure Discovery:** Locates Catalog `/AcroForm`, resolves `/Fields` array, and traverses the `/Kids` tree recursively up to depth limit 32 and 1,000 fields maximum.
- **Cycle Detection:** Maintains a `HashSet<ObjectRef>` of visited nodes to prevent circular reference attacks in hostile PDFs.
- **Inheritance Resolution:** Resolves and propagates inheritable field attributes from ancestor dictionaries to child nodes:
  - `/FT` (Field Type: `/Tx`, `/Btn`, `/Ch`, `/Sig`)
  - `/Ff` (Field Flags: multiline, password, radio, pushbutton, combo, multiselect, readonly, required)
  - `/DA` (Default Appearance string)
  - `/Q` (Quadding / text alignment: 0=Left, 1=Center, 2=Right)
  - `/Opt` (Options list for Choice fields)
- **Field vs. Widget Disambiguation:**
  - *Standard Hierarchical:* Non-terminal nodes with `/Kids` containing `/T` (child fields) vs terminal nodes with `/Kids` lacking `/T` (multiple widget annotations).
  - *Merged Field/Widget Dictionary:* Single-widget terminal fields where field dictionary directly contains `/Subtype /Widget`, `/Rect`, `/AP`, and `/AS`.
- **Accurate Form Field Classification:**
  - Text fields (`/Tx` -> single line or multiline, password flag)
  - Checkboxes (`/Btn` without Radio/PushButton flags) with dynamic on-state discovery from `/AP /N` subkeys (e.g. `/Yes`, `/1`, `/CustomOn`)
  - Radio button groups (`/Btn` with Bit 17) with mutual exclusion tracking
  - Push buttons (`/Btn` with Bit 16)
  - Combo boxes (`/Ch` with Bit 18) and List boxes (`/Ch` without Bit 18)
  - Signature fields (`/Sig`)

### 2.2 Annotation Foundation (`annotation/`)
- **Page Annotation Resolver:** Extracts annotations from page `/Annots` arrays up to 2,000 per page.
- **Subtype Support:** Supports all standard subtypes (`Widget`, `Text`, `Link`, `FreeText`, `Line`, `Square`, `Circle`, `Highlight`, `Underline`, `StrikeOut`, `Stamp`, `Ink`, `Popup`, `FileAttachment`, `Sound`, `Movie`, `Screen`) and preserves `Unknown(String)` variants without failure.
- **Geometric Fidelity:** Captures `[f64; 4]` bounding boxes, flags (`/F`), appearance state (`/AS`), and contents (`/Contents`).

### 2.3 Mutation Engine (`mutation/`)
- **Discrete Operations (`PdfChange`):**
  - `SetTextField { field_ref, value }`
  - `SetCheckbox { field_ref, widget_refs, checked }`
  - `SetRadio { parent_ref, selected_widget_ref, on_state }`
  - `SetChoice { field_ref, value }`
  - `SetAppearanceState { widget_ref, state_name }`
- **Integrity Validation:** Validates that targets exist and are dictionary objects. Enforces 1 MB value limits.
- **State Updating:**
  - Text: Mutates field `/V` to `PdfObject::String`.
  - Checkbox: Mutates field `/V` to `/Yes` (or custom on-state name) and `/Off`, updating both field `/AS` and all associated widget annotations' `/AS`.
  - Radio: Sets parent field `/V` to active state, updates active widget `/AS`, and sets all sibling widget `/AS` to `/Off`.
  - Choice: Updates `/V` to selected value string or name.

### 2.4 Incremental Writer (`writer/incremental.rs`)
- **Byte-Preserving Structure:** Copies the exact original PDF bytes without modifying a single pre-existing byte.
- **Object Serialization:** Appends modified indirect objects formatted as standard PDF objects (`N G obj ... endobj\n`).
- **Contiguous XRef Subsections:** Collects all modified object numbers, groups them into contiguous contiguous subsection runs (`0 1`, `8 2`, etc.), and writes standard 20-byte ASCII cross-reference entries (`{:010} 00000 n \n`).
- **Chained Trailer:** Generates a new trailer dictionary updating `/Size` (max object number + 1), preserving `/Root` and `/Info`, and establishing `/Prev` pointing to the previous `startxref` offset.
- **Terminator:** Emits standard `startxref\n<new_offset>\n%%EOF`.

---

## 3. WebAssembly API & Worker Integration

The WASM bridge was expanded in `engine/starpdf/src/wasm/` with high-performance exports:
- `starpdf_get_form_fields(handle: u32) -> JsValue`
- `starpdf_get_annotations(handle: u32, page_index: u32) -> JsValue`
- `starpdf_set_text_field(handle: u32, obj_num: u64, obj_gen: u16, value: &str) -> bool`
- `starpdf_set_checkbox(handle: u32, obj_num: u64, obj_gen: u16, checked: bool) -> bool`
- `starpdf_set_radio(handle: u32, parent_num: u64, parent_gen: u16, widget_num: u64, widget_gen: u16, on_state: &str) -> bool`
- `starpdf_set_choice(handle: u32, obj_num: u64, obj_gen: u16, value: &str) -> bool`
- `starpdf_export_incremental(handle: u32) -> Uint8Array`

`StarPdfClient` and `public/starpdf.worker.js` provide ergonomic async APIs with zero main-thread blocking.

---

## 4. Verification & Benchmark Audit

### 4.1 Native Rust Test Suite
- **125 Total Rust Tests** across 18 test suites (including `acroform_tests.rs`, `annotation_tests.rs`, `mutation_tests.rs`, `incremental_writer_tests.rs`, `fuzz_hardening_v0_6_tests.rs`).
- **100% Pass Rate**.

### 4.2 Benchmark Results (M-Series ARM64 / Release Mode)
| Component | Metric | StarPDF v0.6 Result |
| :--- | :--- | :--- |
| **Lexer Throughput** | MB/s | **102.53 MB/s** |
| **Object Parser** | MB/s | **60.67 MB/s** |
| **FlateDecode** | MB/s | **1,869.38 MB/s** |
| **SFNT Cmap Parsing** | latency | **105 ns/op** |
| **Text Extractor** | MB/s | **40.69 MB/s** |
| **Search Index Query** | latency | **767 ns/op** |
| **XRef Stream Parsing** | latency | **1,772 ns/op** |
| **ObjStm Extraction** | latency | **179 ns/op** |
| **Document Open & XRef** | latency | **2,102 ns/op** |
| **Page Tree Resolution** | latency | **791 ns/op** |
| **Incremental Serialization** | latency | **355 ns/op** |
| **Mutation Plan Evaluation** | latency | **514 ns/op** |

### 4.3 Fuzzing Hardening Targets
15 dedicated `cargo-fuzz` targets configured:
1. `fuzz_lexer`
2. `fuzz_object_parser`
3. `fuzz_xref_stream`
4. `fuzz_object_stream`
5. `fuzz_flatedecode`
6. `fuzz_predictor`
7. `fuzz_sfnt_font`
8. `fuzz_tounicode_cmap`
9. `fuzz_content_stream`
10. `fuzz_document_open`
11. `fuzz_text_search`
12. `fuzz_acroform`
13. `fuzz_annotations`
14. `fuzz_mutation`
15. `fuzz_incremental_writer`

### 4.4 Repository Gates
- **Rust fmt & clippy:** 0 warnings with `#![forbid(unsafe_code)]` and zero `.unwrap()`/`.expect()`.
- **Vitest:** 25 suites, 635 tests passed.
- **TypeScript:** 0 typecheck errors.
- **ESLint:** 0 lint errors.
- **Next.js Production Build:** Completed in 2.7s.
- **Playwright E2E:** 23/23 tests passed.
