# StarPDF / SmartPDF Capability Utilization Matrix

This matrix provides an exhaustive, classified inventory of every StarPDF engine capability and its integration state in SmartPDF.

## Utilization Categories

- **`FULLY_INTEGRATED`**: Mature in Rust engine, bound in WASM, wired in TS client, and directly actionable in the primary SmartPDF UI.
- **`PARTIALLY_INTEGRATED`**: Bound in WASM/TS client, but UI only reaches a subset of available operations.
- **`ENGINE_READY_UI_MISSING`**: Implemented in Rust, exported via WASM, and supported in `StarPdfClient`, but has no active user-facing UI trigger.
- **`WASM_BINDING_MISSING`**: Implemented in the Rust engine core, but lacks a `#[wasm_bindgen]` export in `engine/starpdf/src/wasm/`.
- **`UI_PRESENT_ENGINE_LIMITED`**: UI provides bounded interaction within documented StarPDF capabilities.
- **`DIAGNOSTIC_ONLY`**: Accessible only via document properties or debug diagnostics modal.
- **`INTENTIONALLY_UNSUPPORTED`**: Excluded by design with safe, deterministic typed refusals.

---

## 1. Document Lifecycle & Security

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Document Open & Parsing** | Yes (`PdfDocument::open`) | `starpdf_open` | `StarPdfClient.openDocument` | Dropzone / Open Button | **`FULLY_INTEGRATED`** | Handles hybrid xref, object streams, linearized docs. |
| **Fault-Tolerant Recovery** | Yes (`recovery.rs`) | Internal to `starpdf_open` | Automatic | Automatic on Open | **`FULLY_INTEGRATED`** | Reconstructs missing startxref, fixes stream lengths. |
| **Document Metadata & Counts**| Yes (`document.info`) | `starpdf_get_info` | `doc.getInfo()` | Doc Info Modal | **`FULLY_INTEGRATED`** | Title, author, page count, form field count. |
| **Digital Signature Detection**| Yes (`security.rs`) | `starpdf_get_security_info`| `doc.getSecurityInfo()` | Info Badge / Warning | **`FULLY_INTEGRATED`** | Non-verification warning, preserves byte ranges. |
| **Password Encryption Refusal**| Yes (`security.rs`) | Typed `ENCRYPTED_DOCUMENT`| Typed Error Catch | Refusal Alert Banner | **`INTENTIONALLY_UNSUPPORTED`** | Refuses encrypted files without corruption. |
| **Stale Handle Protection** | Yes (`registry.rs`) | Typed `INVALID_HANDLE` | `StarPdfDocumentHandle` | History & Reset Lifecycle | **`FULLY_INTEGRATED`** | Closing or replacing documents frees handles cleanly. |
| **Document Close / Reset** | Yes (`registry.rs`) | `starpdf_close` | `doc.close()` | Open New Document Dialog | **`FULLY_INTEGRATED`** | Zero memory leak on document transitions. |

---

## 2. Native Text Editing & Typography

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Text Extraction & Bounds** | Yes (`TextExtractor`) | `starpdf_extract_page_text`| `doc.extractPageText()` | Canvas Text Overlay | **`FULLY_INTEGRATED`** | Exact glyph bounding boxes and rotation CTM. |
| **Batch All-Text Extraction** | Yes (`TextExtractor`) | `starpdf_extract_all_text` | `doc.extractAllText()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Useful for "Export Plain Text / Markdown". |
| **Text Editability Check** | Yes (`TextEditability`) | `starpdf_get_text_editability`| `doc.getTextEditability()` | Contextual Action Bar | **`FULLY_INTEGRATED`** | Checks font program, encoding, and glyph subset. |
| **Native Text Replacement** | Yes (`ContentStreamEditor`)| `starpdf_replace_text` | `doc.replaceText()` | Context Text Input + Apply | **`FULLY_INTEGRATED`** | Preserves CTM matrix, font family, and font size. |
| **Non-Rewritable Refusal** | Yes (`EditabilityCode`) | `refusal_reason` | `span.refusal_reason` | Contextual Read-Only Badge| **`FULLY_INTEGRATED`** | Displays `"This text can't be safely rewritten."` |
| **Type0 / Identity-H CMaps** | Yes (`UnicodeCMap`) | Internal font decoder | Automatic | Overlay & Selection | **`FULLY_INTEGRATED`** | Accurate unicode character mapping for CJK/subsets. |
| **Arbitrary Text Paragraph Reflow**| No | No | No | No | **`INTENTIONALLY_UNSUPPORTED`** | Outside scope; reflow across unmapped fonts risks corruption. |

---

## 3. Raster Images

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Image Enumeration & Bounds**| Yes (`ImageExtractor`) | `starpdf_enumerate_images` | `doc.enumerateImages()` | Canvas Image Overlay | **`FULLY_INTEGRATED`** | Width, height, filter, transform matrix, bounds. |
| **Image Replacement** | Yes (`ImageEditor`) | `starpdf_replace_image` | `doc.replaceImage()` | Contextual "Replace" Button | **`FULLY_INTEGRATED`** | Replaces JPEG/PNG bytes, clones if shared across pages. |
| **Image Removal** | Yes (`ImageEditor`) | `starpdf_remove_image` | `doc.removeImage()` | Contextual "Remove" Button | **`FULLY_INTEGRATED`** | Removes Do operator and XObject reference. |
| **Image Insertion** | Yes (`ImageEditor`) | `starpdf_add_image` | `doc.addImage()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Inserts JPEG/PNG at custom coordinates. |
| **Shared Image De-duplication**| Yes (`ImageEditor`) | `clone_if_shared: true` | Automatic in client | Transparent | **`FULLY_INTEGRATED`** | Modifying an image on page 1 does not corrupt page 2. |

---

## 4. Vector Graphics & Paths

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Vector Enumeration** | Yes (`VectorExtractor`)| `starpdf_enumerate_graphics`| `doc.enumerateGraphics()` | Canvas Vector Overlay | **`FULLY_INTEGRATED`** | Rectangle, line, path bounds and color extraction. |
| **Color & Line Width Mutation**| Yes (`VectorEditor`) | `starpdf_update_graphic` | `doc.updateGraphic()` | Context Color & Width inputs| **`FULLY_INTEGRATED`** | Updates RGB stroke/fill and stroke thickness. |
| **Vector Shape Deletion** | Yes (`VectorEditor`) | `starpdf_delete_graphic` | `doc.deleteGraphic()` | Context "Delete" Button | **`FULLY_INTEGRATED`** | Removes vector instruction stream range cleanly. |
| **Rectangle Addition** | Yes (`VectorEditor`) | `starpdf_add_rectangle` | `doc.addRectangle()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Adds custom rect with stroke/fill to page content. |
| **Line Addition** | Yes (`VectorEditor`) | `starpdf_add_line` | `doc.addLine()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Adds custom line with stroke color and thickness. |
| **Arbitrary Curve / Path Move**| Bounded to Rect/Line | Bounded | Bounded | Bounded | **`UI_PRESENT_ENGINE_LIMITED`** | Complex bezier paths are selectable/deletable/stylable. |

---

## 5. Interactive AcroForms & Widgets

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **AcroForm Discovery** | Yes (`AcroFormParser`) | `starpdf_get_form_fields` | `doc.getFormFields()` | Canvas Field Overlay | **`FULLY_INTEGRATED`** | Traverses field trees, resolves inheritance. |
| **Text Field Editing** | Yes (`FormField::set_text`)| `starpdf_set_text_field` | `doc.setTextField()` | Direct Spatial Context Input| **`FULLY_INTEGRATED`** | Updates `/V` and generates Type1/TrueType `/AP`. |
| **Checkbox Toggling** | Yes (`set_checkbox`) | `starpdf_set_checkbox` | `doc.setCheckbox()` | Direct Spatial Click / Context| **`FULLY_INTEGRATED`** | Synchronizes `/AS` and `/V` states. |
| **Radio Button Selection** | Yes (`set_radio`) | `starpdf_set_radio` | `doc.setRadio()` | Direct Spatial Click / Context| **`FULLY_INTEGRATED`** | Mutually exclusive widget state updates. |
| **Dropdown / Choice Selection**| Yes (`set_choice`)| `starpdf_set_choice` | `doc.setChoice()` | Direct Spatial Select Input | **`FULLY_INTEGRATED`** | Updates `/V` and regenerates visible appearance. |
| **Multi-Select Listbox Values**| Yes (`set_choice_values`)| `starpdf_set_choice_values`| `doc.setChoiceValues()`| Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Supports array of selected export strings. |
| **Automatic `/AP` Generation** | Yes (`AppearanceGenerator`)| Internal | Automatic | Transparent | **`FULLY_INTEGRATED`** | Compliant with PDF 1.7 / ISO 32000. |

---

## 6. Markup Annotations

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Annotation Discovery** | Yes (`AnnotationParser`)| `starpdf_get_annotations`| `doc.getAnnotations()` | Canvas Annotation Overlay | **`FULLY_INTEGRATED`** | FreeText, Highlight, Underline, Square, Ink, Link. |
| **FreeText Mutation** | Yes (`AnnotationGenerator`)| `starpdf_update_annotation`| `doc.updateAnnotation()`| Direct Context Text Input | **`FULLY_INTEGRATED`** | Updates `/Contents` and regenerates appearance stream. |
| **Link Destination Display** | Yes (`AnnotationSubtype::Link`)| Internal mapping | Direct data | Context Read-Only Link | **`FULLY_INTEGRATED`** | Displays link target without confusing text edits. |
| **Annotation Creation** | Yes (`AnnotationGenerator`)| `starpdf_add_annotation` | `doc.addAnnotation()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Adds FreeText, Highlight, Square, Circle, Line, Ink. |
| **Annotation Deletion** | Yes (`AnnotationParser`)| `starpdf_remove_annotation`| `doc.removeAnnotation()`| Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Removes annotation from page `/Annots` array. |
| **Appearance Regeneration** | Yes (`generator.rs`) | Internal | Automatic | Transparent | **`FULLY_INTEGRATED`** | Generates valid `/AP` XObject streams. |

---

## 7. Page Operations & Multi-Document Workflows

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Page Deletion** | Yes (`IncrementalPageEditor`)| `starpdf_delete_page` | `doc.deletePage()` | Page Operations Bar | **`FULLY_INTEGRATED`** | Removes from `/Kids`, decrements `/Count`. |
| **Page Reordering / Moving** | Yes (`IncrementalPageEditor`)| `starpdf_move_page` | `doc.movePage()` | Page Operations Bar | **`FULLY_INTEGRATED`** | Updates `/Kids` array and parent pointers. |
| **Page Duplication** | Yes (`IncrementalPageEditor`)| `starpdf_duplicate_page` | `doc.duplicatePage()` | Page Operations Bar | **`FULLY_INTEGRATED`** | Deep clones page dict and inherited resources. |
| **Blank Page Insertion** | Yes (`IncrementalPageEditor`)| `starpdf_insert_blank_page`| `doc.insertBlankPage()`| Page Operations Bar | **`FULLY_INTEGRATED`** | Creates clean blank page dictionary. |
| **Single Page Extraction** | Yes (`DocumentBuilder`) | `starpdf_extract_pages` | `doc.extractPages()` | Page Operations Bar | **`FULLY_INTEGRATED`** | Generates standalone single-page PDF. |
| **Multi-Page Arbitrary Extract**| Yes (`DocumentBuilder`) | `starpdf_extract_pages` | `doc.extractPages()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Supports non-contiguous page array extraction. |
| **Multi-Document Merge All** | Yes (`DocumentBuilder`) | `starpdf_merge_documents` | `StarPdfClient.mergeDocuments`| Toolbar "Add PDF" | **`FULLY_INTEGRATED`** | Appends documents in sequential order. |
| **Arbitrary Page Interleave Merge**| Yes (`DocumentBuilder`) | `starpdf_merge_selected` | `StarPdfClient.mergeSelected` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Custom page mapping `[(doc0, p0), (doc1, p2), ...]`.|
| **Multi-Range Document Split**| Yes (`DocumentBuilder`) | `starpdf_split_document` | `doc.splitDocument()` | Not Exposed in UI | **`ENGINE_READY_UI_MISSING`** | Splits single PDF into multiple independent files. |

---

## 8. Document Writers & Serialization

| Feature / Subsystem | Engine Support | WASM Binding | TS Client | SmartPDF UI | Status | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Incremental Save / Writer** | Yes (`IncrementalWriter`)| `starpdf_export_incremental`| `doc.exportIncremental()`| "Export Editable" Button | **`FULLY_INTEGRATED`** | Appends xref revision; preserves digital signatures. |
| **Minimal Scratchpad Writer** | Yes (`MinimalWriter`) | `starpdf_create_minimal_pdf`| `StarPdfClient.createMinimal`| Internal Scratchpad | **`FULLY_INTEGRATED`** | Generates valid 1-page PDF in 0ms. |
| **Complete Standalone Compaction**| Yes (`CompleteWriter`)| Missing WASM Export | No | No | **`WASM_BINDING_MISSING`** | Defragments multiple revisions into compact PDF. |
