# StarPDF Engine v0.12B — Cross-Document Merge and Split

## Qualification identity

- Starting SHA: `f11343a9c6c90cc056ad766465ed37a5f9b9d92c`
- Ending SHA: the local commit containing this document
- Runtime version: `0.12.1`
- Scope: bounded cross-document page import, selected-page merge, whole-document merge, and range split
- Production policy: local-only processing in Rust/WASM; no upload service or proprietary PDF SDK

## Architecture

The complete-document builder validates every source before returning output. A build follows:

`sources -> page selection validation -> page/inheritance resolution -> dependency remap -> catalog/AcroForm assembly -> complete serialization -> structural reopen verification`

Imported identity is the tuple `(source document index, object number, generation, clone scope)`. Destination object numbers are allocated monotonically with checked arithmetic. This prevents equal object numbers in separate PDFs from aliasing. A visited remap table reuses a dependency imported more than once from the same source. Recursive arrays, dictionaries, stream dictionaries, and unknown structurally valid indirect references are traversed under explicit depth, object, stream-byte, and output-byte limits.

Page outputs use a rebuilt flat `/Pages` tree. Inherited `/Resources`, `/MediaBox`, `/CropBox`, `/BleedBox`, `/TrimBox`, `/ArtBox`, and `/Rotate` values are materialized on imported pages. Encoded stream bytes and filter dictionaries are preserved; content streams are not decoded and recompressed merely for import.

## Merge and selected-page merge

`PdfDocument::merge_documents` accepts two or more source byte slices and preserves document order and page order. `PdfDocument::merge_selected` accepts an explicit ordered list of `(document_index, page_index)` values. Selections may interleave documents and repeat a page.

A repeated page receives a distinct page dictionary and annotation clone scope. Immutable/shared dependencies from the same source reuse one destination mapping. References from the current page back to itself are rewritten to that occurrence. A reference from another object to a repeated source page resolves deterministically to the first imported occurrence.

Tests cover two documents, three documents, arbitrary ordering, repeated selections, colliding source object numbers, reopen, text order, and subsequent move, duplicate, delete, and extract operations.

## Split

`PdfDocument::split_document` accepts zero-based, half-open `PageRange` values. Each non-empty, in-range, non-overlapping range is built atomically as a standalone PDF and structurally reopened before the output vector is returned. Gaps are allowed. Empty, out-of-range, and overlapping ranges are refused with typed errors. No partial output vector is returned when a range fails.

## Forms and field collisions

All imported supported field graphs are assembled under one destination `/AcroForm`. Source-local field and widget identity is preserved. A top-level partial-name collision is resolved deterministically by keeping the first name and renaming later roots with a suffix such as `__starpdf_d2_f0_1`; child hierarchy remains below that renamed root. Equal names never cause unrelated fields or radio groups to be joined.

Each imported top-level field receives its source AcroForm `/DR` and `/DA` when it does not already define them. This isolates same-named resources such as `/Helvetica` across source AcroForms. The destination AcroForm retains the primary document defaults. Regression coverage merges two copies of the independent form fixture, verifies unique fully qualified names and distinct widgets, checks text/checkbox/radio/choice and multi-widget counts, and independently mutates same-named text fields from both sources after merge.

Safe boundary:

- A document containing forms is imported only when every source page is selected exactly once.
- Repeated or partial page selection from a form document is refused as `PARTIAL_FIELD_IMPORT_REFUSED`.
- XFA import is refused.
- Ambiguous or malformed field graphs retain the existing typed-refusal behavior.

## Annotations and navigation

Annotation objects, appearance streams, popup/reply dependencies, actions, and unknown structurally valid dependencies are remapped recursively. `/P` references to the imported current page are rewritten to the destination page. Annotation dictionaries are cloned per repeated page while logical field/widget objects remain source-shared.

Direct page references in destinations and GoTo arrays are remapped when the target page is included. A required reference to an excluded page is refused instead of leaving a dangling reference. The primary document's `/Names`, `/Outlines`, and `/OpenAction` are preserved only when every primary page is selected exactly once. Navigation trees from later merge inputs are not combined in v0.12B.

## Resources

Compatibility coverage verifies inherited resources and boxes, cardinal rotation, embedded TrueType and supported Type0 resources, ToUnicode/descendant dependencies, images, Form XObjects, ExtGState/color-related references reached through resource dictionaries, encoded streams, and annotation appearances. Resource names in content streams are not rewritten because each imported page retains its corresponding resource dictionary.

The correctness-first deduplication policy reuses the same source object within a source document. Semantically equal objects from different PDFs are not globally hashed or deduplicated.

## Catalog, metadata, and writer

The first input document owns PDF version, `/Lang`, metadata, viewer preferences, page mode/layout, mark information, output intents, optional-content properties, trailer `/Info`, and trailer `/ID`. Incompatible document metadata and XMP packets are not merged.

Merge, import, extract, and split use the complete writer. It emits a header, indirect objects, preserved streams, a classic xref table, trailer, `/Root`, `/Size`, `startxref`, and `%%EOF`, then reopens and structurally validates the result. Construction is all-or-nothing. Same-document delete, move, and blank insert retain the v0.12A incremental path.

## Resource limits and safety

Default bounds include:

- 16 source documents
- 10,000 total source pages and 10,000 selected pages
- 100,000 imported objects and remap entries
- dependency depth 64
- 512 MiB imported stream bytes
- 768 MiB output bytes
- 2,000 annotations per page
- 4,000 form fields
- 4,096 resources per page

All count and object-number growth uses checked arithmetic. Encrypted or structurally signed documents retain the existing safe refusal policy for complete graph rebuilding. Production code contains zero `.unwrap()`/`.expect()` calls and the crate retains `#![forbid(unsafe_code)]`.

## WASM, worker, and SmartPDF

The synchronized `0.12.1` bindings expose imported-page insertion, whole-document merge, selected-page merge, and range split. Public and application WASM binaries are byte-identical. Typed worker messages perform all graph work off the React main thread, transfer result buffers, preserve typed error classification, and close registry handles.

SmartPDF adds one minimal **Add PDF** control. It reads selected local files, transfers them to the dedicated StarPDF worker, merges the active document followed by the added documents, and reloads the result. No document bytes are sent to a server. Playwright verifies the resulting five-page order/count and PDF.js rendering within the existing page-operation flow.

## Tests and compatibility evidence

Rust qualification: 214 tests pass. New v0.12B coverage includes:

- two- and three-document merge
- arbitrary and repeated selected-page merge
- source object-number collision isolation
- imported-page insertion
- non-overlapping split ranges and invalid-range refusal
- inherited resource, box, and rotation materialization
- annotations and appearance preservation
- first-document metadata preservation
- same-name AcroForm field/radio isolation and independent mutation
- merge followed by v0.12A reorder, duplicate, delete, and extract
- document/page limit refusal without partial output

ToolRakyat qualification: 643 Vitest tests and 37 Chromium Playwright tests pass. The production Next.js build, lint, and typecheck gates pass. The three-page qualification output was reopened with Poppler and every rendered page was visually inspected; ordering, text, vector graphics, and appearances were intact.

## Performance baseline

Apple Silicon release benchmark results from the final qualification run:

| Workload | Result |
|---|---:|
| Cross-document import, 1 page | 80.109 µs/op |
| Cross-document import, 10 repeated pages | 168.609 µs/op |
| Merge 2 multi-page documents | 147.916 µs/op |
| Merge 3 documents | 219.434 µs/op |
| Resource-heavy graph remap | 298.055 µs/op |
| AcroForm collision remap | 581.996 µs/op |
| Merge standalone write | 88.874 µs/op |
| Merge output reopen | 5.660 µs/op |

These are new workloads and are not directly comparable to parser-only historical baselines. Existing parser, mutation, incremental writer, and reopen benchmarks remain in the suite.

## Fuzz status

There are 34 configured cargo-fuzz targets. v0.12B adds `fuzz_page_document_ops`, which frames seeded source documents and exercises whole merge, selected-page merge, imported-page insertion, split, writer output, and reopen paths.

A bounded v0.12B campaign was started before the interrupted session, but the installed stable compiler rejected the nightly-only sanitizer flags before libFuzzer began executing. Completed libFuzzer executions: **0**. No campaign crash or hang result exists because the target never started. The continuation instruction explicitly prohibited another cargo-fuzz campaign, so no retry was made. Existing deterministic fuzz-hardening tests pass, but this does not substitute for a completed libFuzzer campaign.

## Known limitations and readiness

- Partial/repeated selection from documents with AcroForms is refused.
- XFA, encrypted complete rebuilds, and signed complete rebuilds are refused.
- Navigation trees from later input documents are not merged.
- Primary navigation is retained only for a complete, once-only primary selection.
- References to excluded destination pages are refused rather than removed heuristically.
- Top-level field-name collisions are renamed; no semantic reconciliation of independently authored field hierarchies is attempted.
- Cross-document byte-equality deduplication is intentionally absent.
- Output uses a rebuilt flat page tree and classic xref table rather than preserving source xref/object-stream layout.
- The v0.12B libFuzzer campaign remains unexecuted under the explicit continuation stop condition.

The ordinary production gates are green and the supported merge/split surface is ready for local production use within these boundaries. Full fuzz qualification remains an explicitly documented follow-up before broadening the hostile-input claim.

Recommended v0.13: navigation-tree import/reconstruction and safely broader partial-form extraction, preceded by completing the bounded v0.12B libFuzzer campaign on a compatible nightly toolchain.
