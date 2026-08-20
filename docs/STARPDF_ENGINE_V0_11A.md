# StarPDF Engine v0.11A - complex preservation and revision integrity

## Build identity and outcome

- Starting SHA: `27890a72195e66ff42405a31a108fcf1cb072d54`
- Ending SHA: the single local milestone commit reported in the final handoff; a commit cannot contain its own content-derived SHA.
- Scope: ordinary PDF revision integrity, structural preservation, ambiguous field-graph safety, browser compatibility, and hostile-input robustness.

This milestone resumed an interrupted working tree without resetting it. It preserves broader protected-document detection work already present when the scope narrowed, but v0.11A does not extend or claim signature verification, signing, decryption, password recovery, or protection bypass.

## Hybrid references and revision chains

Xref traversal is latest-first and records bounded internal revision metadata: revision index, xref kind, xref offset, previous offset, and hybrid xref-stream offset. Classic tables, xref streams, `/XRefStm`, and `/Prev` share one visited-offset set. Cycles, repeated offsets, non-backward links, out-of-file offsets, entry-count overflow, width/count conversion overflow, and the existing 64-revision depth limit return typed errors.

Two deterministic topologies pass:

1. classic xref -> `/XRefStm` -> xref stream -> `/Prev` -> later classic update;
2. xref stream -> classic update -> xref stream update.

The second topology starts with a compressed object entry and proves that later uncompressed definitions supersede it. Classic-to-classic replacement, freed objects, and reused object numbers with new generations also have direct regressions. Object resolution verifies both object number and generation at the parsed object boundary; it never falls back to an older generation merely because it is easier to parse.

## Effective trailer and document identity

Current-revision trailer keys are inserted before older revisions, so the latest present value wins while absent `/Root`, `/Info`, `/Encrypt`, and `/ID` values inherit from older revisions. `/Prev` and `/XRefStm` are read from the current revision dictionary rather than the merged effective trailer. Incremental output removes stale `/XRefStm`, writes a new `/Prev`, and otherwise clones the effective trailer.

Regression coverage proves a changed second `/ID` in a later revision wins, a normal two-element ID survives unrelated export, a missing ID remains missing, and a malformed ID remains byte-semantically preserved rather than being regenerated.

## Metadata, catalog, and unknown-object preservation

An unrelated annotation update preserves `/Info`, XMP metadata, `/Lang`, `/ViewerPreferences`, `/PageMode`, `/PageLayout`, `/OpenAction`, `/Names`, outlines, and an unknown catalog dictionary. StarPDF does not need to understand these values to preserve them. Incremental writing retains the complete input as an exact prefix; unmodified dictionaries, streams, resources, unsupported annotations, and unknown objects remain in their original revisions. Only explicitly replaced or generated objects are serialized into the appended revision.

Repeated-update and producer regressions continue to prove collision-free object allocation, valid `/Prev` links, reopenability, root continuity, and preservation of unrelated annotations and appearances.

## Field graph classification

Stable classifications are `CANONICAL_FIELD`, `MULTI_WIDGET_FIELD`, `ORPHAN_WIDGET`, `AMBIGUOUS_WIDGET_GROUP`, and `MALFORMED_FIELD_GRAPH`. Canonical identity comes from indirect references and actual `/Parent`/`/Kids` relationships. Names, appearance states, coordinates, and visual similarity are never sufficient to merge widgets.

Same-name orphan radios remain separately inspectable and are classified ambiguous. Contradictory child `/Parent` references are classified malformed. In either case, mutation requiring radio-group identity returns a typed atomic refusal before mutation planning or output. Existing canonical, nested, inherited, single-widget, and multi-widget producer cases remain green.

## Corpus and compatibility

The meaningful corpus contains 33 PDFs: 22 established independent/local producer fixtures and 11 deterministic synthetic complex fixtures. Independent authoring/producer-family coverage remains five: Chrome/Skia, LibreOfficeDev, Quartz/CUPS, Apple PDFKit annotation authoring, and pdf-lib. Synthetic PDFs are clearly labeled and are not counted as independent producers. Full per-file structure and classification are in `engine/starpdf/tests/fixtures/v0_11_complex/MANIFEST.md`.

The v0.11A ordinary-document fixtures pass open, revision resolution, page/form/annotation inspection, supported mutation where structurally legal, exact-prefix incremental export, and reopen. Ambiguous and malformed radio group mutations are `REFUSED_SAFELY`, not guessed. Existing unknown-annotation and producer-preservation regressions remain green.

## Safety and fuzzing

Production retains `#![forbid(unsafe_code)]`, zero exact `.unwrap()`/`.expect()` calls, typed failures, checked arithmetic, bounded xref counts and traversal, bounded field trees/widgets, bounded generated objects, and bounded incremental growth.

There are 33 configured cargo-fuzz targets. Thirteen 10-second final-source campaigns across nine unique targets executed 8,782,444 inputs:

| Target | Executions |
|---|---:|
| `fuzz_signature_security` | 502,780 |
| `fuzz_encryption_detection` | 324,543 |
| `fuzz_revision_chain` | 765,972 |
| `fuzz_field_graph_classification` | 516,592 |
| `fuzz_xref_stream` | 1,400,101 |
| `fuzz_document_open` | 775,312 |
| `fuzz_acroform` | 538,961 |
| `fuzz_mutation` | 349,190 |
| `fuzz_incremental_writer` | 981,067 |
| final `fuzz_field_graph_classification` rerun | 516,219 |
| final `fuzz_acroform` rerun | 519,988 |
| final `fuzz_mutation` rerun | 312,044 |
| final `fuzz_xref_stream` rerun | 1,279,675 |

Crashes: 0. Hangs: 0. Regression reproducers: 0. The two broader security targets are recorded because they had already run before the scope narrowed; v0.11A relies directly on the revision, xref, document-open, field, mutation, and writer campaigns.

## Performance

Second warm native measurements:

| Operation | Result |
|---|---:|
| Lexer | 124.23 MB/s |
| Object parser | 50.25 MB/s |
| FlateDecode | 1,725.53 MB/s |
| Revision-chain traversal | 5,365 ns/op |
| Hybrid/latest-object resolution | 9,559 ns/op |
| Effective trailer lookup | 8 ns/op |
| Field-graph classification | 21,297 ns/op |
| Metadata-preserving mutation plan | 21,656 ns/op |
| General mutation plan | 3,902 ns/op |
| Incremental serialization | 357 ns/op |
| Export/reopen | 6,242 ns/op |

The selected second warm run places unchanged lexer, object-parser, FlateDecode, incremental-serialization, and export/reopen measurements within 15% of the v0.10 historical references. A separate final-gate run measured lexer throughput at 87.96 MB/s while object parsing and FlateDecode remained near their references; the lexer code is untouched and the same-session range (86.93-124.23 MB/s) indicates environmental benchmark variance, not enough evidence for a code-attributable regression. General mutation planning is not an unchanged workload because the interrupted tree adds document-policy inspection before planning; no check was removed or weakened.

## WASM, worker, and ToolRakyat

The checked-in WASM bindings were rebuilt from the final Rust source and remain byte-identical in both browser locations. Stable field-graph classification reaches TypeScript. Existing bounded handles, stale-handle rejection, deterministic close, typed worker errors, and local-only processing remain intact. v0.11A adds no backend dependency, upload path, telemetry, or UI redesign.

## Known limitations and readiness

- Revision-order validation proves bounded backward linkage and deterministic precedence; it does not repair arbitrary corrupt xref histories.
- Unknown structures are preserved, not made editable.
- Ambiguous or contradictory radio graphs are inspection-only for group-dependent mutations.
- Synthetic fixtures prove rare structures but do not add independent producer coverage.
- Broader protected-document work in the interrupted tree is outside the v0.11A production claim; cryptographic verification and decryption are not implemented.

Production readiness: ready for the stated v0.11A boundary after the recorded Rust, fuzz, browser, and ToolRakyat gates pass. Recommended next step: collect additional redistributable independent-producer hybrid and multi-revision PDFs before expanding mutation support; do not infer correctness from synthetic coverage alone.

Patentability claims: **NONE**.
