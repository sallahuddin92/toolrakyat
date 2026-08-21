# SmartPDF / StarPDF Applicationization Audit — Phase 0

## 1. Executive Summary & Context

- **Baseline Commit**: `bc622f3`
- **Engine**: StarPDF (Rust/WASM custom engine, zero native C/C++ dependencies)
- **Client Product**: SmartPDF (Local-first web PDF editor)
- **Audit Objective**: Complete inventory of all StarPDF engine capabilities, WASM exports, TypeScript client bridges, and current SmartPDF UI reachability to establish a desktop-class application architecture without reinventing existing engine logic.

---

## 2. Complete StarPDF Engine Inventory

The StarPDF Rust engine (`engine/starpdf/src/`) is organized into 19 modular subsystems:

| Subsystem / Module | Rust Source Path | Core Types & Public APIs | Mutates PDF? | Bounded / Safe? | WASM Exposed? | Current UI Reachability |
| :--- | :--- | :--- | :---: | :---: | :---: | :--- |
| **Document Store & Lifecycle** | `document/document.rs`, `object_store.rs` | `PdfDocument`, `ObjectStore`, `PageTree` | Yes | Yes (bounded memory & handles) | **YES** (`starpdf_open`, `starpdf_get_info`, `starpdf_close`) | Fully integrated |
| **Object Stream Decompression** | `document/object_stream.rs` | `ObjectStreamReader`, `DecodedObjectStream` | No | Yes (max 4096 objects/stream) | **YES** (Internal parser) | Transparently active |
| **Parser & Lexer** | `syntax/parser.rs`, `lexer.rs`, `tokenizer.rs` | `PdfParser`, `PdfTokenizer`, `PdfToken`, `PdfObject` | No | Yes (depth <= 64, tokens <= 1M) | **YES** (Internal) | Transparently active |
| **Cross-Reference & Streams** | `xref/xref_resolver.rs`, `stream_parser.rs` | `XrefResolver`, `XrefTable`, `XrefStreamParser` | Yes | Yes (hybrid table + streams) | **YES** (Internal) | Transparently active |
| **Security & Permissions** | `security.rs` | `WasmSecurityInfo`, `SecurityHandler`, `SignedDocument` | No | Yes (typed non-verification & refusals) | **YES** (`starpdf_get_security_info`) | Fully integrated (Badges & Refusal) |
| **Incremental & Minimal Writer** | `writer/incremental.rs`, `minimal.rs` | `IncrementalWriter`, `MinimalWriter`, `Serializer` | Yes | Yes (appends clean revisions) | **YES** (`starpdf_export_incremental`, `starpdf_create_minimal_pdf`) | Fully integrated |
| **Complete Standalone Writer** | `writer/complete.rs` | `CompleteWriter` | Yes | Yes (rewrites compact PDF) | **NO** (Engine-only) | Missing WASM binding |
| **Text Extraction & CTM** | `text/extractor.rs`, `matrix.rs` | `TextExtractor`, `PageText`, `TextSpan`, `Matrix2D` | No | Yes (exact glyph bounds & transforms) | **YES** (`starpdf_extract_page_text`, `starpdf_extract_all_text`) | Fully integrated |
| **Text Editability & Fonts** | `text/editability.rs`, `font/` | `TextEditability`, `Font`, `SimpleEncoding`, `UnicodeCMap` | No | Yes (checks font programs & glyph tables) | **YES** (`starpdf_get_text_editability`) | Fully integrated |
| **Native Text Mutation** | `mutation/content_stream.rs`, `engine.rs` | `ContentStreamEditor`, `MutationPlan`, `PdfChange` | Yes | Yes (in-place replacement without reflow drift) | **YES** (`starpdf_replace_text`) | Fully integrated |
| **Document Search Index** | `search/search_index.rs`, `matcher.rs` | `DocumentSearchIndex`, `PageSearchIndex`, `SearchResult` | No | Yes (exact bounding-box hits across pages) | **YES** (`starpdf_search`) | Fully integrated |
| **Interactive AcroForms** | `forms/parser.rs`, `field.rs`, `widget.rs` | `AcroForm`, `FormField`, `FieldType`, `WidgetAnnotation` | Yes | Yes (tree traversal, inheritance resolution) | **YES** (`starpdf_get_form_fields`, `starpdf_set_*`) | Fully integrated |
| **Appearance Stream Gen** | `appearance/generator.rs`, `text_field.rs`, etc. | `AppearanceGenerator`, `DefaultAppearance`, `PdfColor` | Yes | Yes (Type1/TrueType `/AP` generation) | **YES** (Automatic during form/annot set) | Fully integrated |
| **Markup Annotations** | `annotation/parser.rs`, `generator.rs` | `AnnotationParser`, `AnnotationGenerator`, `Annotation` | Yes | Yes (FreeText, Highlight, Square, Circle, Link, etc.) | **YES** (`starpdf_get_annotations`, `starpdf_add/update/remove`) | Partially integrated (FreeText mutate; add/delete UI missing) |
| **Raster Images** | `image/extractor.rs`, `editor.rs` | `ImageExtractor`, `ImageEditor`, `ImageXObjectInfo` | Yes | Yes (shared image cloning & replacement) | **YES** (`starpdf_enumerate_images`, `starpdf_replace/add/remove`) | Fully integrated |
| **Vector Graphics & Paths** | `vector/extractor.rs`, `editor.rs` | `VectorExtractor`, `VectorEditor`, `VectorGraphicInfo` | Yes | Yes (color, line width, rect, line mutation) | **YES** (`starpdf_enumerate_graphics`, `starpdf_update/add/delete`) | Fully integrated |
| **Page Mutation & Reorder** | `page_ops/editor.rs`, `plan.rs` | `IncrementalPageEditor`, `PageOperationPlan`, `PageEdit` | Yes | Yes (reorder, duplicate, delete, insert blank) | **YES** (`starpdf_move/duplicate/delete/insert_blank_page`) | Fully integrated |
| **Cross-Doc Import & Merge** | `page_ops/document_builder.rs` | `DocumentBuilder`, `PageSource`, `PageRange` | Yes | Yes (object renumbering & resource merging) | **YES** (`starpdf_insert_imported_page`, `starpdf_merge_documents`, `starpdf_merge_selected`) | Partially integrated (Toolbar merge active; selected page merge UI missing) |
| **Document Split & Extract** | `page_ops/document_builder.rs` | `DocumentBuilder`, `PageRange` | Yes | Yes (extracts standalone sub-documents) | **YES** (`starpdf_extract_pages`, `starpdf_split_document`) | Partially integrated (Extract active; split ranges UI missing) |
| **Fault-Tolerant Recovery** | `document/recovery.rs` | `PdfDocument::open_with_recovery` | No | Yes (startxref repair, stream length reconcile) | **YES** (Automatic inside `starpdf_open`) | Fully integrated |

---

## 3. WASM API & Worker Bridge Inventory

All 45 exported WASM APIs in `engine/starpdf/src/wasm/api.rs` are mapped through `public/starpdf.worker.js` and `src/lib/pdf/starpdf-client.ts`:

| Exported WASM API Name | Input Arguments | Output Type | Worker Message Type | TypeScript Client Method | Current SmartPDF UI Utilization |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `starpdf_version` | None | `String` | `init` | `StarPdfClient.init` | Version check & health |
| `starpdf_open` | `bytes: &[u8]` | `Result<u32, JsValue>` | `open` | `StarPdfClient.openDocument` | Document load |
| `starpdf_get_info` | `handle: u32` | `WasmDocumentInfo` | `info` | `doc.getInfo()` | Doc info modal & properties |
| `starpdf_get_security_info` | `handle: u32` | `WasmSecurityInfo` | `securityInfo` | `doc.getSecurityInfo()` | Security audit badge & alerts |
| `starpdf_get_page_count` | `handle: u32` | `u32` | `pageCount` | `doc.getPageCount()` | Page counter & thumbnail rail |
| `starpdf_extract_page_text` | `handle, page_index` | `WasmPageText` | `extractPage` | `doc.extractPageText(page)` | Canvas text overlay & selection |
| `starpdf_extract_all_text` | `handle: u32` | `Vec<WasmPageText>` | `extractAll` | `doc.extractAllText()` | Batch document extraction |
| `starpdf_replace_text` | `handle, page, span_id, text` | `WasmReplaceTextResult` | `replaceText` | `doc.replaceText(...)` | Direct text editing on canvas |
| `starpdf_get_text_editability` | `handle, page, span_id` | `WasmTextSpan` | `getTextEditability` | `doc.getTextEditability(...)` | Pre-flight editability check |
| `starpdf_search` | `handle, query, case_sensitive` | `Vec<WasmSearchResult>` | `search` | `doc.search(...)` | Toolbar search & match counts |
| `starpdf_validate` | `handle: u32` | `bool` | `validate` | `doc.validate()` | Structural validation |
| `starpdf_get_form_fields` | `handle: u32` | `Vec<WasmFormField>` | `getFormFields` | `doc.getFormFields()` | Form fields on canvas & values |
| `starpdf_set_text_field` | `handle, obj_num, gen, value` | `bool` | `setTextField` | `doc.setTextField(...)` | Direct form text editing |
| `starpdf_set_checkbox` | `handle, obj_num, gen, checked`| `bool` | `setCheckbox` | `doc.setCheckbox(...)` | Direct checkbox toggle |
| `starpdf_set_radio` | `handle, parent, widget, state`| `bool` | `setRadio` | `doc.setRadio(...)` | Direct radio selection |
| `starpdf_set_choice` | `handle, obj_num, gen, value` | `bool` | `setChoice` | `doc.setChoice(...)` | Dropdown value change |
| `starpdf_set_choice_values` | `handle, obj_num, gen, values` | `bool` | `setChoiceValues` | `doc.setChoiceValues(...)` | Multi-select listbox changes |
| `starpdf_get_annotations` | `handle, page_index` | `Vec<WasmAnnotation>` | `getAnnotations` | `doc.getAnnotations(page)` | Canvas markup annotation layer |
| `starpdf_add_annotation` | `handle, page, input` | `bool` | `addAnnotation` | `doc.addAnnotation(...)` | **API EXPOSED, UI MISSING** |
| `starpdf_update_annotation` | `handle, obj_num, gen, input` | `bool` | `updateAnnotation` | `doc.updateAnnotation(...)` | Direct annotation text editing |
| `starpdf_remove_annotation` | `handle, page, obj_num, gen` | `bool` | `removeAnnotation` | `doc.removeAnnotation(...)` | **API EXPOSED, UI MISSING** |
| `starpdf_enumerate_images` | `handle, page_index` | `Vec<WasmImageInfo>` | `enumerateImages` | `doc.enumerateImages(page)` | Canvas image overlay |
| `starpdf_replace_image` | `handle, page, id, bytes, clone`| `WasmImageMutationResult` | `replaceImage` | `doc.replaceImage(...)` | Direct image replace button |
| `starpdf_add_image` | `handle, page, bytes, x, y, w, h`| `WasmImageMutationResult` | `addImage` | `doc.addImage(...)` | API exposed (Direct UI missing) |
| `starpdf_remove_image` | `handle, page, id` | `WasmImageMutationResult` | `removeImage` | `doc.removeImage(...)` | Direct image remove button |
| `starpdf_enumerate_graphics` | `handle, page_index` | `Vec<WasmVectorGraphicInfo>`| `enumerateGraphics` | `doc.enumerateGraphics(page)` | Canvas vector overlay |
| `starpdf_enumerate_all_graphics`| `handle: u32` | `Vec<WasmVectorGraphicInfo>`| `enumerateAllGraphics` | `doc.enumerateAllGraphics()` | Batch vector inspection |
| `starpdf_update_graphic` | `handle, input` | `WasmVectorMutationResult` | `updateGraphic` | `doc.updateGraphic(...)` | Vector color & line width |
| `starpdf_add_rectangle` | `handle, input` | `WasmVectorMutationResult` | `addRectangle` | `doc.addRectangle(...)` | API exposed (Direct UI missing) |
| `starpdf_add_line` | `handle, input` | `WasmVectorMutationResult` | `addLine` | `doc.addLine(...)` | API exposed (Direct UI missing) |
| `starpdf_delete_graphic` | `handle, input` | `WasmVectorMutationResult` | `deleteGraphic` | `doc.deleteGraphic(...)` | Direct vector delete button |
| `starpdf_delete_page` | `handle, page_index` | `Vec<u8>` | `deletePage` | `doc.deletePage(page)` | Page operations delete button |
| `starpdf_move_page` | `handle, from, to` | `Vec<u8>` | `movePage` | `doc.movePage(from, to)` | Page operations move buttons |
| `starpdf_duplicate_page` | `handle, page, dest` | `Vec<u8>` | `duplicatePage` | `doc.duplicatePage(...)` | Page operations duplicate button |
| `starpdf_insert_blank_page` | `handle, page, w, h, rot` | `Vec<u8>` | `insertBlankPage` | `doc.insertBlankPage(...)` | Page operations blank button |
| `starpdf_extract_pages` | `handle, indices` | `Vec<u8>` | `extractPages` | `doc.extractPages(indices)` | Page operations extract button |
| `starpdf_insert_imported_page`| `handle, buf, imp_idx, dest` | `Vec<u8>` | `insertImportedPage` | `doc.insertImportedPage(...)` | Multi-doc page insertion |
| `starpdf_merge_documents` | `buffers: Vec<Vec<u8>>` | `Vec<u8>` | `mergeDocuments` | `StarPdfClient.mergeDocuments` | Toolbar merge button |
| `starpdf_merge_selected` | `buffers, page_sources` | `Vec<u8>` | `mergeDocuments` (sources) | `StarPdfClient.mergeSelected` | **API EXPOSED, UI MISSING** |
| `starpdf_split_document` | `handle, ranges` | `Vec<Vec<u8>>` | `splitDocument` | `doc.splitDocument(ranges)` | **API EXPOSED, UI MISSING** |
| `starpdf_get_appearance_status`| `handle: u32` | `String` | `getAppearanceStatus` | `doc.getAppearanceStatus()` | Diagnostic info |
| `starpdf_get_glyph_mapping_quality`| `handle: u32` | `String` | `getGlyphMappingQuality` | `doc.getGlyphMappingQuality()`| Diagnostic info |
| `starpdf_export_incremental` | `handle: u32` | `Vec<u8>` | `exportIncremental` | `doc.exportIncremental()` | Export Editable button |
| `starpdf_close` | `handle: u32` | `bool` | `close` | `doc.close()` | Document reset & cleanup |
| `starpdf_create_minimal_pdf` | `text: &str` | `Vec<u8>` | `createMinimal` | `StarPdfClient.createMinimal` | Empty scratchpad creation |

---

## 4. Complete UI Traceability Matrix

Every user action in SmartPDF traces through a strict typed pipeline:

```
[UI Trigger / Canvas Click]
       │
       ▼
[SmartPdfEditor Handler]
       │
       ▼
[StarPdfClient / Handle Method]
       │
       ▼
[StarPdfWorkerBridge / postMessage]
       │
       ▼
[public/starpdf.worker.js]
       │
       ▼
[StarPDF WASM Export]
       │
       ▼
[StarPDF Rust Core Mutation Engine]
       │
       ▼
[Incremental Byte Stream & Snapshot Refresh]
```

### Traceability Table

| UI Category | UI Control & Location | React Event Handler | TS Client API Call | Worker Request | StarPDF WASM Entry | StarPDF Rust Core Function |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **File** | Open File (Toolbar / Dropzone) | `handleOpenNewFileClick` | `StarPdfClient.openDocument(bytes)` | `open` | `starpdf_open` | `PdfDocument::open_with_recovery` |
| **File** | Export Editable (Toolbar) | `onExport("editable")` | `doc.exportIncremental()` | `exportIncremental` | `starpdf_export_incremental` | `IncrementalWriter::write` |
| **File** | Export Flattened (Dropdown) | `onExport("flattened")` | `pdf-lib` + StarPDF fallback | Direct bytes | N/A (Client Flattening) | Native `/NeedAppearances` lock |
| **File** | Add PDF / Merge (Toolbar) | `mergeInputRef.onChange` | `StarPdfClient.mergeDocuments([a, b])`| `mergeDocuments` | `starpdf_merge_documents` | `DocumentBuilder::merge` |
| **File** | Properties & Info (Toolbar) | `setShowInfoModal(true)` | `doc.getInfo()` | `info` | `starpdf_get_info` | `PdfDocument::info` |
| **Text** | Canvas Text Span (Overlay) | `onSelectItem({type: "text"})` | `doc.getTextEditability(page, id)` | `getTextEditability` | `starpdf_get_text_editability` | `TextExtractor::check_editability` |
| **Text** | Apply Text Edit (Context Bar) | `handleReplaceExistingText` | `doc.replaceText(page, id, text)` | `replaceText` | `starpdf_replace_text` | `ContentStreamEditor::replace_text` |
| **Image** | Canvas Image (Overlay) | `onSelectItem({type: "image"})` | `doc.enumerateImages(page)` | `enumerateImages` | `starpdf_enumerate_images` | `ImageExtractor::extract_page_images` |
| **Image** | Replace Image (Context Bar) | `handleReplaceImage` | `doc.replaceImage(page, id, bytes)` | `replaceImage` | `starpdf_replace_image` | `ImageEditor::replace_image` |
| **Image** | Remove Image (Context Bar) | `handleRemoveImage` | `doc.removeImage(page, id)` | `removeImage` | `starpdf_remove_image` | `ImageEditor::remove_image` |
| **Vector** | Canvas Graphic (Overlay) | `onSelectItem({type: "vector"})` | `doc.enumerateGraphics(page)` | `enumerateGraphics` | `starpdf_enumerate_graphics`| `VectorExtractor::extract_page_graphics` |
| **Vector** | Edit Stroke/Fill (Context Bar)| `handleUpdateGraphic` | `doc.updateGraphic(input)` | `updateGraphic` | `starpdf_update_graphic` | `VectorEditor::update_graphic` |
| **Vector** | Delete Shape (Context Bar) | `handleDeleteGraphic` | `doc.deleteGraphic(input)` | `deleteGraphic` | `starpdf_delete_graphic` | `VectorEditor::delete_graphic` |
| **Forms** | Canvas Form Field (Overlay) | `onSelectItem({type: "form"})` | `doc.getFormFields()` | `getFormFields` | `starpdf_get_form_fields` | `AcroFormParser::parse` |
| **Forms** | Text Field Input (Context Bar)| `handleFieldValueChange` | `doc.setTextField(num, gen, val)` | `setTextField` | `starpdf_set_text_field` | `FormField::set_text_value` + `/AP` |
| **Forms** | Checkbox Toggle (Context Bar) | `handleFieldValueChange` | `doc.setCheckbox(num, gen, val)` | `setCheckbox` | `starpdf_set_checkbox` | `FormField::set_checkbox_value` + `/AP`|
| **Forms** | Radio Select (Context Bar) | `handleFieldValueChange` | `doc.setRadio(...)` | `setRadio` | `starpdf_set_radio` | `FormField::set_radio_value` + `/AP` |
| **Forms** | Choice Select (Context Bar) | `handleFieldValueChange` | `doc.setChoice(num, gen, val)` | `setChoice` | `starpdf_set_choice` | `FormField::set_choice_value` + `/AP` |
| **Annots** | Canvas Annotation (Overlay) | `onSelectItem({type: "annot"})` | `doc.getAnnotations(page)` | `getAnnotations` | `starpdf_get_annotations` | `AnnotationParser::parse_page` |
| **Annots** | Edit FreeText (Context Bar) | `handleAnnotationChange` | `doc.updateAnnotation(...)` | `updateAnnotation` | `starpdf_update_annotation` | `AnnotationGenerator::update` |
| **Pages** | Delete Page (Ops Bar) | `page-delete.onClick` | `doc.deletePage(page)` | `deletePage` | `starpdf_delete_page` | `IncrementalPageEditor::delete_page` |
| **Pages** | Move Left/Right (Ops Bar) | `page-move-*.onClick` | `doc.movePage(from, to)` | `movePage` | `starpdf_move_page` | `IncrementalPageEditor::move_page` |
| **Pages** | Duplicate Page (Ops Bar) | `page-duplicate.onClick` | `doc.duplicatePage(page, dest)` | `duplicatePage` | `starpdf_duplicate_page` | `IncrementalPageEditor::duplicate_page`|
| **Pages** | Insert Blank (Ops Bar) | `page-insert-blank.onClick` | `doc.insertBlankPage(...)` | `insertBlankPage` | `starpdf_insert_blank_page` | `IncrementalPageEditor::insert_blank` |
| **Pages** | Extract Page (Ops Bar) | `page-extract.onClick` | `doc.extractPages([page])` | `extractPages` | `starpdf_extract_pages` | `DocumentBuilder::extract_pages` |
| **Search** | Search Query (Toolbar) | `onSearchChange` | `doc.search(query, options)` | `search` | `starpdf_search` | `DocumentSearchIndex::search` |
| **History**| Undo / Redo (Toolbar) | `onUndo` / `onRedo` | History Stack Uint8Array buffer swap | `open` (on restored snapshot) | `starpdf_open` | `PdfDocument::open_with_recovery` |

---

## 5. Top 10 Unused & Underused StarPDF Engine Capabilities

These capabilities are fully implemented in Rust and have WASM bindings, but lack intuitive application UI:

| Rank | Capability | Engine Status | WASM / Client Status | Current UI Status | User Value | UI Integration Work Required | Priority |
| :---: | :--- | :---: | :---: | :---: | :---: | :--- | :---: |
| **1** | **Multi-Range Document Split** (`starpdf_split_document`) | Complete | Complete | Missing | **High**: Allows extracting chapters, page groups, or splitting large documents. | Modal / dialog to define page ranges (e.g. `1-3, 4-7, 8-12`) and trigger multi-file zip/downloads. | **P0** |
| **2** | **Arbitrary Selected Page Merge** (`starpdf_merge_selected`) | Complete | Complete | Missing | **High**: Allows interleaving pages from multiple PDFs into a custom target order. | Multi-document visual organizer tray with drag-and-drop page placement. | **P0** |
| **3** | **Interactive Markup Annotation Creation** (`starpdf_add_annotation`) | Complete | Complete | Missing | **High**: Allows adding Highlight, Underline, FreeText, Square, Circle, and Ink to any PDF. | Annotation mode tool in top toolbar; drag-to-draw bounding box on canvas. | **P0** |
| **4** | **Annotation Deletion** (`starpdf_remove_annotation`) | Complete | Complete | Missing | **High**: Allows deleting unwanted highlights, sticky notes, or stamps. | Delete / Trash button on contextual annotation toolbar + keyboard `Delete` key. | **P0** |
| **5** | **Vector Shape Creation** (`starpdf_add_rectangle`, `starpdf_add_line`) | Complete | Complete | Missing | **Medium-High**: Allows drawing redactions, highlight boxes, separators, and callouts. | "Insert Shape" tool button with canvas click-and-drag drawing interaction. | **P1** |
| **6** | **Image Insertion** (`starpdf_add_image`) | Complete | Complete | Missing | **Medium-High**: Allows inserting company logos, signatures, and photographic stamps. | "Insert Image" button with file picker and drag-to-place canvas rect. | **P1** |
| **7** | **Multi-Page Batch Extraction** (`starpdf_extract_pages` with arbitrary array) | Complete | Complete | Partially used (single active page only) | **Medium**: Extract arbitrary non-contiguous pages (e.g. pages 2, 5, 9-11). | Checkbox selection in thumbnail rail + "Extract Selected (N)" button. | **P1** |
| **8** | **Search Match Bounding Box Canvas Highlighting** (`SearchResult.boxes`) | Complete | Complete | Underused (result count only) | **Medium**: Visually highlight search matches directly on the canvas with jump-to-result focus. | Overlay search highlight rects on `PdfInteractiveOverlay` during active search. | **P1** |
| **9** | **Complete Standalone PDF Compaction** (`CompleteWriter`) | Complete in Rust | Missing WASM | Missing | **Medium**: Cleans and defragments multiple revisions into a minimal compact PDF. | Export dropdown option: "Export Optimized / Compact Standalone PDF". | **P2** |
| **10** | **Glyph Mapping & Appearance Audit Diagnostics** (`get_glyph_mapping_quality`, `get_appearance_status`) | Complete | Complete | Diagnostic only | **Low-Medium**: Developer/power-user transparency on font subset status. | Expandable section inside Document Diagnostics modal. | **P3** |

---

## 6. Critical Analysis of Product & Technical Gaps

### A. Biggest Product UX Gap: Isolated Tool Viewport vs. Full-Scale Desktop Application Shell
SmartPDF currently renders inside a restricted 2/3 container within the general ToolRakyat marketing tool layout. For desktop users working on large 50-page manuals, forms, or contracts, the application needs its own dedicated fullscreen application shell (`/smartpdf`) with standard menu bars, customizable tool modes, multi-select thumbnail organizer, and rich keyboard navigation.

### B. Biggest Text-Editing Gap: Granular Text Spans vs. Word/Line Presentation Grouping
In documents generated by PDFKit, LibreOffice, or Quartz, a single visual word or sentence can consist of 10 to 50 tiny glyph spans. While StarPDF maps each span with exact coordinates and CTM, presenting these tiny fragments as individual click targets is cumbersome. A presentation grouping layer is required to visually group contiguous glyph runs into selectable words/lines while preserving exact source `span_id` mappings for StarPDF mutation.

### C. Biggest Page-Management Gap: Single-Page Actions vs. Multi-Select Batch Operations
StarPDF engine supports multi-page deletion, arbitrary page reordering, multi-source merging, and arbitrary page extraction. The UI currently exposes single-page linear buttons in the top rail. Upgrading the Left Thumbnail Rail to support `Shift+Click` range selection, `Cmd/Ctrl+Click` multi-select, and drag-and-drop reordering will unlock the engine's full power.

---

## 7. UI vs. Engine Gap Classification Rules

To maintain high engineering velocity, all future feature development must strictly classify new functionality before touching code:

1. **`UI_ONLY`**: The Rust engine, WASM bindings, and TypeScript client already fully support the operation (e.g. Annotation deletion, multi-range split, search bounding-box highlights). Requires ZERO engine modifications.
2. **`WEB_RUNTIME_ONLY`**: Client-side UX coordination such as DOM drag-and-drop, keyboard focus traps, canvas presentation grouping, or local session persistence.
3. **`WASM_BINDING_REQUIRED`**: Rust engine core implements the functionality (e.g. `CompleteWriter` standalone defragmentation), but it lacks a `#[wasm_bindgen]` export in `engine/starpdf/src/wasm/api.rs`.
4. **`STARPDF_ENGINE_REQUIRED`**: Functionality requires novel PDF object graph parsing or mutation algorithms not yet in the Rust core.
5. **`UNSUPPORTED_BY_DESIGN`**: Intentionally refused operations (e.g. mutating password-encrypted PDFs without keys, re-flowing complex paragraphs across broken font subsets, or executing JavaScript actions).
