# StarPDF Engine v0.9 — compatibility and font embedding

## Build identity

- Starting SHA: `1dd30a3d42658a9087bc4b06080d9cc5fea41c40`
- Ending SHA: the single local milestone commit reported by `git rev-parse HEAD` in the final handoff. A Git commit cannot embed its own content-derived SHA.
- Date/environment: 2026-08-20, Apple Silicon, Rust 1.93.0, Chromium, WASM worker, Next.js 16.2.4.

## Production result

StarPDF now automatically resolves an embedded TrueType/SFNT appearance font, proves Unicode coverage, calculates the glyph set, closes TrueType composite dependencies, emits a reparsed subset stream and descriptor/font objects, binds the new resource into `/AP`, and exports it incrementally. The TypeScript caller does not construct font objects. Exact subsets are reused within one atomic mutation when source identity, source checksum, base/resource identity, composite mode, and sorted glyph IDs match.

The subset keeps original glyph IDs and `.notdef`, preserves SFNT tables and metrics, rebuilds `glyf`/long `loca`, recalculates table checksums and `checkSumAdjustment`, caps input/output, and reparses output before it can enter a plan. Composite traversal is iterative, detects active-path cycles, limits depth to 32, and limits closure to 4,096 glyphs. CFF outlines remain a typed refusal.

Widget appearance `/BBox` and `/Matrix` generation is exact for 0°, 90°, 180°, and 270°. Page `/Rotate` is validated through a bounded parent chain; widget `/MK /R` controls the Form XObject transform. Text, checkbox/radio, and choice appearances share the same matrix rules. Arbitrary rotation and non-finite/degenerate geometry are refused.

Type0 appearance output is limited to `Identity-H`/`Identity-V`, one valid descendant CIDFont, a resolvable embedded SFNT, and identity/default `CIDToGIDMap`. Unicode maps through the embedded cmap to a proved glyph/CID. Other mappings return `UNSUPPORTED_COMPOSITE_MAPPING`; extraction success is not used as proof of appearance encoding.

WASM/worker/client version `0.9.0` exposes both appearance outcome and glyph mapping quality. Mapping reports `EXACT`, `FALLBACK`, `UNREPRESENTABLE` on refusal, or `NOT_APPLICABLE`. Heavy font work remains in Rust/WASM; there is no server dependency.

## Producer corpus and compatibility

The committed local corpus contains 12 authored fixtures from three independent producers: Google Chrome 151.0.7922.140, LibreOfficeDev 26.8.0.0.alpha0, and macOS Quartz/CUPS 26.5. It spans PDF 1.3/1.4/1.7, classic xref, one/two pages, landscape geometry, embedded simple TrueType, CID TrueType/Type0, WinAnsi/MacRoman/Identity-H, and ToUnicode. The existing `smartpdf-form.pdf` supplements xref-stream, object-stream, AcroForm, and widget coverage. Full per-fixture provenance and results are in `engine/starpdf/tests/fixtures/v0_9_compat/MANIFEST.md`.

All 12 independent fixtures pass open, exact page count, native extraction, search, form/annotation inspection, generated Square appearance, prefix-preserving incremental export, reopen, and semantic reinspection. Supplemental deterministic tests cover rotated page/widget dictionaries, forms, annotations, automatic subsets, deduplication, and three sequential exports. The browser suite performs PDF.js rendering and targeted-region comparison.

Compatibility bugs found and fixed:

1. Chrome and LibreOffice may split one logical word across geometrically overlapping spans. Search now joins only bounded same-line fragments whose projected gap is within 10% of font size; a regression preserves multi-box geometry.
2. pdf-lib may place the intended embedded font only in an existing widget `/AP /N /Resources`. Mutation now performs a bounded recovery from that resource dictionary instead of silently falling back.
3. Multiline/list coverage text used newline separators that are layout controls, not rendered glyphs. Subsetting and verification now exclude CR/LF while each rendered line is verified independently.
4. Checkbox calls targeting a parent field now discover bounded child `/Kids` widgets and regenerate their appearances.

No broad object scanning or parser-validation bypass was added. The remaining real-producer gap is producer-authored AcroForms, arbitrary original annotations, and page `/Rotate`; these are explicit supplemental tests rather than fabricated producer provenance.

## Incremental, atomic, and bounded behavior

A deterministic regression performs seed export, reopen, text/subset/90° appearance export, reopen, another subset export, reopen, then subset plus annotation export. Every prior generation remains the exact output prefix, `/Prev` chaining reopens, and allocation does not collide.

Font resolution, coverage, subset/resource construction, AP generation, and plan validation complete before bytes are emitted. Missing glyph, malformed font, cyclic composite, 65th unique font resource, unsupported rotation, and unsupported Type0 mapping abort the batch. Resource limits include 16 MiB source/subset fonts, 65,535 source glyphs, 4,096 subset glyphs/dependencies, depth 32, 64 subset resources per mutation, 256 appearance resources/states, 2,000 generated objects, 500 batch changes, 1 MiB field values, bounded ancestry, appearance streams, and incremental growth. Checked arithmetic is used for offsets and allocation.

Production source retains `#![forbid(unsafe_code)]` and has zero exact `.unwrap()`/`.expect()` occurrences.

## Fuzzing

There are 26 configured cargo-fuzz targets. New targets are `fuzz_rotated_widget`, `fuzz_type0_auto_embedding`, and `fuzz_search_fragment_recovery`. The Type0 target is seeded with a valid Liberation Sans TTF and exercises AP-resource recovery, automatic subsetting, two-widget deduplication, rotations, plan creation, and export. Affected targets rerun were font subsetter, embedded font resolver, rich appearance, incremental writer, mutation, and text appearance.

The final recorded nine-target campaign set ran for 10 seconds per target and executed 11,823,541 inputs: rotated widget 3,938,960; Type0/automatic embedding 6,179; search recovery 443,183; font subsetter 3,132,222; embedded resolver 212,603; rich appearance 287,039; incremental writer 1,300,445; mutation 2,387,797; text appearance 115,113. Crashes: 0. Hangs: 0. Regression reproducers: 0.

## Performance

Final warm native measurements:

| Operation | Result |
|---|---:|
| Lexer | 133.48 MB/s |
| Object parser | 60.63 MB/s |
| FlateDecode | 2,001.72 MB/s |
| Font resolution | 1,883 ns/op |
| TrueType subset | 944 ns/op |
| Composite closure + subset | 1,072 ns/op |
| Text appearance | 1,306 ns/op |
| Rotated matrix | 97 ns/op |
| Automatic Type0 embedding plan | 173,819 ns/op |
| Two-widget dedup plan | 296,512 ns/op |
| Mutation plan | 1,110 ns/op |
| Incremental serialization | 382 ns/op |
| Export/reopen | 6,389 ns/op |
| Subset export/reopen | 17,251 ns/op |

The final warm lexer result is within 3% of the v0.8 reference. Two earlier runs at 83–85 MB/s were investigated: this milestone has no lexer/parser code change, and the final same-build rerun recovered to 133.48 MB/s, identifying transient local load as the comparison discrepancy. No safety check was removed.

## Visual validation

Chromium/PDF.js reopens the incremental output and verifies semantic form state plus a targeted rendered-region change. The rich fixture uses one embedded TrueType field with two widgets, 90° and 270° `/MK /R`, a 180° comb widget, multiline text, and a multi-select list. A separate real Chrome/Skia Identity-H Type0 fixture receives a pdf-lib AcroForm field bound to its page font; StarPDF proves the producer subset's coverage, automatically embeds a new exact subset, reopens the semantic value, and changes the field's expected render region. Appended bytes are structurally checked for generated `/Matrix`, `/FontFile2`, and `/SPF` resources, and mapping quality is `EXACT`. The annotation case regenerates and renders FreeText, Square, Circle, Line, Highlight, Underline, StrikeOut, Ink, and checkbox/radio state. Pixel delta is supporting evidence, not the sole assertion.

## Gates, limitations, and readiness

Rust fmt, clippy with warnings denied, tests, release build, benchmarks, fuzz listing, ToolRakyat lint/typecheck/637 Vitest tests/build, and 26 Playwright tests pass. Generated WASM bindings and both browser copies are synchronized.

Known limitations: CFF/CFF2 subsetting is refused; non-identity composite mappings are refused; arbitrary rotation is refused; subset reuse is transaction-local rather than persisted across separate incremental generations; page rotation is validated but does not alter the widget-local AP matrix; producer-authored rotated/forms/annotation corpus gaps remain; visual comparison covers representative cases rather than every producer fixture.

Production readiness: ready for the supported v0.9 boundary, with typed refusal outside it. Recommended next milestone: controlled compatibility expansion using producer-authored AcroForms/annotations and CFF analysis, without weakening parser or resource bounds.
