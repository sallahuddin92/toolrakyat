# StarPDF Engine v0.12A — Core Page Operations

## Release identity

- Starting SHA: `7bb1fa193c41d49a4f08044a27e6fec2e2c3d28d`
- Ending SHA: the local commit containing this document (`feat(starpdf): add core page operations`)
- Scope: delete, move/reorder, duplicate, insert blank, and single-source page extraction
- Explicitly excluded: cross-document merge/import, split orchestration, parser redesign, document-protection changes, existing-text editing, and image editing

## Architecture

The public API uses typed `PageEdit`, `PageOperationPlan`, `PageOperationType`, and `PageOperationLimits` values. Planning is separate from serialization and never mutates the caller's source bytes. An operation follows:

```text
source document
→ validate page tree and operation
→ calculate destination page state
→ materialize inherited page properties
→ serialize privately
→ reopen and structurally validate
→ return complete bytes
```

Plans are atomic: intermediate byte buffers are private, and a failing later edit returns an error without changing the source document.

## Operation behavior

### Delete

`delete_page(index)` uses an incremental update. It validates the complete page tree, materializes inherited state on retained pages, writes a flat root `/Pages /Kids` array, updates `/Count`, reparents retained pages, reopens the result, and verifies the page count. The original byte prefix is preserved. Deleting the final page is refused because StarPDF v0.12A requires at least one page.

Deletion of a page containing a widget is refused with `PARTIAL_FIELD_IMPORT_REFUSED`, rather than leaving a broken field graph. Catalogs containing named destinations, outlines, or open actions are conservatively refused because v0.12A does not yet prove that every navigation target survives deletion.

### Reorder

`move_page(from_index, to_index)` uses an incremental update and preserves the original page objects. Content, resources, annotations, widgets, boxes, rotation, and reference-based destinations therefore remain attached to the same page identity. A same-position move returns the original bytes unchanged.

### Duplicate

`duplicate_page(index, destination_index)` uses the standalone graph builder. The destination receives a new page dictionary and occurrence-scoped annotation copies. Annotation `/P` references are remapped to the corresponding destination page. Shared non-page dependencies from the same source object are copied once and reused; streams keep their encoded bytes and filter dictionaries.

Duplicating a page from a document containing forms is currently refused unless the resulting selection represents every source page exactly once. This deliberately avoids cloning a partial or ambiguously shared logical field tree.

### Blank page

`insert_blank_page(index, width, height, rotation)` uses an incremental update. Width and height must be finite, positive, and no greater than 20,000 points. Rotation must be `0`, `90`, `180`, or `270`. A blank page has an explicit empty resource dictionary and no `/Contents`, which is a valid empty-page representation.

### Extract

`extract_pages(&[indexes])` builds a new standalone PDF. Selection order is preserved. Repeated indexes are supported for pages without forms, so `[2, 2]` produces two distinct page objects with two copies of page-scoped annotation state. Empty and out-of-range selections are rejected.

The output contains a new catalog, a normalized flat page tree, selected reachable objects, classic xref table, trailer, `startxref`, and EOF marker. It does not depend on pdf-lib.

## Page tree and inherited state

`PageTree::validate_and_collect` validates `/Page` and `/Pages` types, `/Kids`, `/Parent`, `/Count`, cycles, depth, and bounded page count before an operation. Output trees may be normalized to a flat shape.

The effective `/Resources`, `/MediaBox`, `/CropBox`, `/BleedBox`, `/TrimBox`, `/ArtBox`, and `/Rotate` values are materialized from ancestors before pages leave their original tree. Boxes must contain four finite values with positive bounded dimensions. Rotation is never silently normalized.

## Reachability and resource preservation

The single-source standalone builder recursively remaps all structurally valid indirect references encountered through dictionaries, arrays, and stream dictionaries. It is not limited to a resource whitelist. This preserves fonts, font descriptors, embedded font streams, `ToUnicode`, descendant fonts, XObjects, images, ExtGState, color spaces, patterns, shading, properties, annotation appearances, and unknown reachable objects.

The remap table guarantees that one source object is copied once per required scope. Cycles terminate through a preallocated visited mapping. Content and resource streams normally preserve their encoded bytes; resource names inside content streams are not rewritten.

## Forms, widgets, and annotations

A complete supported AcroForm graph is copied when every source page is selected exactly once. Tests cover text fields, checkboxes, radio groups, choice fields, and multi-widget fields. Form defaults and resources are preserved. XFA and partial multi-page field extraction return typed unsupported/refusal errors.

Annotations remain attached to moved pages. Duplicate/extract operations clone page-scoped annotation graphs, remap `/P`, and preserve `/AP`, links, URI actions, and unknown annotation dictionaries when their dependencies remain valid. References to excluded pages return `EXCLUDED_PAGE_TARGET` rather than producing a dangling reference.

## Standalone writer

`CompleteWriter` emits sequential generation-zero objects, a bounded classic xref table, catalog root, trailer, `startxref`, and EOF. It recomputes stream `/Length`, strips incremental-only `/Prev` and `/XRefStm`, refuses `/Encrypt`, and enforces object/output bounds. Every complete output is reopened with StarPDF and structurally validated before it is returned.

## Resource limits and safety

Default page-operation limits include:

- 10,000 source/output or selected pages
- 100,000 copied objects and remap entries
- dependency depth 64
- 512 MiB total copied stream bytes
- 768 MiB output
- 2,000 annotations per page
- 4,000 form fields
- 4,096 top-level resource entries per page

Counts and object allocation use checked arithmetic. The crate retains `#![forbid(unsafe_code)]`; production `engine/starpdf/src` contains zero `.unwrap()` and `.expect()` calls. No existing parser or safety limit was weakened. Per the v0.12A scope, fuzzing infrastructure was not modified.

## WASM, worker, and SmartPDF

The WASM API version is `0.12.0` and exposes `starpdf_delete_page`, `starpdf_move_page`, `starpdf_duplicate_page`, `starpdf_insert_blank_page`, and `starpdf_extract_pages`. Transforming calls atomically replace the worker registry bytes only after the output reopens. Extraction returns a new document without replacing the source handle. Pending form/annotation mutations must be exported or reset first.

The typed worker protocol exposes `deletePage`, `movePage`, `duplicatePage`, `insertBlankPage`, and `extractPages`. SmartPDF runs these graph operations in a dedicated worker. The minimal UI adds current-page controls and thumbnail extraction-selection checkboxes; it provides progress and typed error feedback without adding a server dependency.

The source and public WASM copies are byte-identical with SHA-256 `72b676a1c744efb5eb034d86aa328804882e041d1a68df121cefe57cd79c413e`.

## Verification

### Compatibility and rendering

Twelve new deterministic Rust tests cover first/middle/last deletion, all requested move positions, no-op and invalid indexes, nested-page-tree normalization, independent page/annotation duplication, repeated extraction selection, blank geometry and rotation, inherited resources and boxes, embedded fonts, annotations and appearances, complete supported field graphs, images/XObjects, mixed xref history, incremental sources, metadata/navigation, resource limits, atomicity, serialization, and reopen.

The full Rust suite passes: 207 tests. The full Vitest suite passes: 642 tests. The full Playwright suite passes: 37 tests. Browser coverage performs all five v0.12A operations through the worker, downloads an extracted PDF, reopens it, and renders it with PDF.js.

A representative `[1, 0, 1]` extraction was also reopened by Poppler as a three-page PDF and every page was rasterized at 144 DPI. Visual inspection confirmed the expected `B, A, B` order with intact text and page geometry.

### Benchmarks

One warm release run on the qualification machine recorded:

| Workload | Result |
| --- | ---: |
| Delete page | 82,823 ns/op |
| Move page | 93,529 ns/op |
| Duplicate page | 115,003 ns/op |
| Insert blank page | 103,508 ns/op |
| Extract one page + write | 59,006 ns/op |
| Extract ten selected pages + write | 143,955 ns/op |
| Standalone graph rebuild/write | 58,872 ns/op |
| Reopen generated output | 3,884 ns/op |

Existing parser benchmarks remain present. The new page-operation measurements include source parsing, validation, serialization, and output reopen where performed by the production API; they are initial v0.12A baselines rather than comparisons to earlier workloads.

## Known limitations and readiness

- Cross-document page import and merge are intentionally deferred to v0.12B.
- Split is intentionally deferred; clients can orchestrate non-overlapping extraction only after a stable range contract is defined.
- Partial AcroForm extraction, duplicated form pages, and XFA are refused safely.
- Delete refuses complex catalog navigation rather than attempting incomplete target repair.
- Navigation entries are preserved in standalone output only when every source page is selected exactly once; excluded-page targets are refused.
- v0.12A does not claim exhaustive PDF compatibility.

Within these documented boundaries, v0.12A is production-ready for ordinary delete, reorder, duplicate, blank insertion, and standalone extraction. Recommended v0.12B scope is cross-document import/merge with explicit field-name collision and navigation policies; it should not begin automatically from this milestone.
