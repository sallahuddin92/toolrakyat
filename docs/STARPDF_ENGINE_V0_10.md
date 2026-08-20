# StarPDF Engine v0.10 — cross-producer forms and annotations

## Build identity and outcome

- Starting SHA: `e3caf30e6bd99522a2445ef14262617fd9780c77`
- Ending SHA: the single local milestone commit reported in the final handoff; a commit cannot contain its own content-derived SHA.
- Environment: 2026-08-20, Apple Silicon, Rust 1.93, Swift 6.2.3, Chromium/PDF.js, browser WASM worker, Next.js 16.2.4.

StarPDF v0.10 ships bounded compatibility for canonical producer AcroForms and Apple PDFKit's page-level orphan widgets. It resolves inherited field properties, separates logical fields from widgets, preserves unrelated producer appearance states, reconciles `/NeedAppearances` only when visual completeness is proved, parses Link URI actions, and retains atomic incremental export. Existing SmartPDF flows gain this behavior without a UI redesign or server dependency.

## Corpus and compatibility matrix

The matrix contains 22 meaningful fixtures: 21 committed producer fixtures plus the existing `test-assets/smartpdf-form.pdf`. Five locally available authoring/producer families are represented: Chrome/Skia, LibreOfficeDev 26.8 alpha, Quartz/CUPS, Apple PDFKit with the Quartz serializer, and pdf-lib 1.17.1. PDFKit is a distinct annotation-authoring API but shares the Quartz serialization layer; this is not counted as an unrelated serializer.

Nine v0.10 additions cover PDF 1.3 classic xref and PDF 1.7 xref/object streams; one has two prior incremental updates. They include text, multiline text, checkbox, radio, combo, multiselect list, nested `/Kids`, a multi-widget field, duplicate names on distinct orphan widgets, inherited FT/Ff/DA/Q/Opt/V/DV/MaxLen, `/NeedAppearances`, N/R/D appearances, page 90° plus widget 270°, and FreeText/Highlight/Underline/StrikeOut/Square/Circle/Ink/Line/Link. Full structure-derived metadata is in `engine/starpdf/tests/fixtures/v0_10_compat/MANIFEST.md`.

All 22 fixtures are PASS. StarPDF and the reference/browser observations agree for supported semantics and representative visuals. No fixture is PARTIAL, FAIL, or INCONCLUSIVE. Unsupported synthetic font-program mutation cases are REFUSED_SAFELY but are not counted as producer fixtures.

## AcroForms, widgets, and appearances

Field traversal resolves direct or indirect FT, Ff, DA, Q, Opt, V, DV, and MaxLen through a maximum 32-level parent chain, with cycle detection. Mutation uses the same effective dictionary, including AcroForm DA/Q/DR defaults, while writing the leaf value. Options are capped at 5,000; fields at 1,000; page annotations/widgets at 2,000.

When `/AcroForm` is missing or omits a page widget, recovery walks only known page `/Annots` references. It never scans global objects and never aliases fields merely because `/T` strings match. Canonical parent fields update their bounded `/Kids` widgets; distinct PDFKit same-name orphan widgets remain distinct objects.

Generated `/AP /N` replaces only the owned normal appearance. Existing `/R` and `/D` entries and unrelated normal-state keys survive. `/NeedAppearances true` becomes false only after a successful field appearance mutation and a complete bounded proof that every field widget has a nonempty normal appearance; otherwise the flag is preserved.

Quarter-turn widget geometry remains limited to 0°/90°/180°/270°. The real PDFKit rotated fixture combines page `/Rotate 90`, widget `/MK /R 270`, and a producer appearance BBox/Matrix. Arbitrary rotations remain typed refusals.

## Annotation compatibility

Producer annotations retain Rect, QuadPoints, InkList, L, LE, C, IC, BS, Border, Contents, F, AP, and unrelated dictionary members through incremental updates. Link `/A` dictionaries are preserved and bounded URI values are exposed to the stable WASM DTO. Semantic-only changes preserve valid producer AP. Visual changes regenerate a supported N appearance before reporting `AP_REGENERATED`, while R/D survive.

Unknown annotation subtypes are not globally interpreted or deleted. Annotation count, strings, geometry arrays, appearance dictionaries/states, and generated streams remain bounded. A mixed multi-widget plus invalid-annotation batch proves failure produces no mutation output.

## Incremental histories

The two-revision fixture starts with a pdf-lib xref/object stream, then contains two classic StarPDF incremental sections and two `/Prev` links. A third update preserves the complete input as an exact prefix, keeps catalog identity and annotation population, allocates without collision, reopens, and exposes the new semantic value. Existing malformed-chain depth and cycle checks are unchanged.

## Advanced font assessment

The local corpus contains CFF 0, CFF2 0, and non-Identity Type0 0. All observed Type0 fonts use Identity-H and ToUnicode. There is therefore no evidence basis for a CFF subsetter or another composite reverse mapping in v0.10.

Font program classification is explicit: `TRUETYPE_SUPPORTED`, `CFF_DETECTED_UNSUPPORTED`, `CFF2_DETECTED_UNSUPPORTED`, or `UNKNOWN_FONT_PROGRAM`. Detection recognizes FontFile3 Type1C/CIDFontType0C, OpenType CFF/CFF2 tables, and TrueType only when glyf+loca are present. CFF bytes are never interpreted as TrueType. Extraction may continue through ToUnicode, but a mutation requiring unsupported glyph writing returns a typed refusal. Non-Identity composite appearance writing remains `UNSUPPORTED_COMPOSITE_MAPPING` unless reverse Unicode-to-CID mapping can be proved.

## Compatibility bugs and regressions

Five evidence-backed compatibility bugs were fixed:

1. Producer field values/defaults and other inherited attributes could be missed when direct/indirect values were mixed.
2. PDFKit widgets without `/AcroForm` were invisible; bounded page-annotation recovery now exposes them without name-based aliasing.
3. Regenerating `/AP /N` could discard producer `/R`, `/D`, or unrelated normal states.
4. `/NeedAppearances` had no proof-based reconciliation after successful regeneration.
5. Link URI actions and N/R/D appearance presence were not available for preservation verification through WASM.

Each has a deterministic local generator/fixture and Rust regression. Nine fixtures and three fuzz targets were added. No private or downloaded documents are included.

## Safety and fuzzing

Production retains `#![forbid(unsafe_code)]`, zero exact `.unwrap()`/`.expect()` calls, typed hostile-input failures, checked arithmetic, atomic mutation planning, and prior decompression/xref/font/output limits. New caps cover recovery attempts, fields, widgets, annotations, option arrays, AP entries/states, URI length, and inherited lookup depth.

There are 29 configured cargo-fuzz targets. Twelve 10-second campaigns across ten new/affected targets ran 31,396,831 executions: the initial set recorded form compatibility 3,623,757; annotation compatibility 3,634,438; font program detection 3,179,247; AcroForm 3,676,019; annotations 3,698,923; annotation regeneration 231,851; rotated widget 3,800,077; mutation 2,275,941; Type0 automatic embedding 74,792; and incremental writer 1,287,822. Final-source reruns added form compatibility 3,657,746 and mutation 2,256,218. Crashes 0, hangs 0, regression reproducers 0.

## Performance

Second warm native measurements:

| Operation | Result |
|---|---:|
| Lexer | 141.80 MB/s |
| Object parser | 59.11 MB/s |
| FlateDecode | 1,965.49 MB/s |
| Producer field traversal | 32,919 ns/op |
| Orphan widget resolution | 15,741 ns/op |
| Annotation/AP traversal | 6,765 ns/op |
| CFF/program detection | 30 ns/op |
| Type0 mapping/automatic embed plan | 185,813 ns/op |
| Mutation plan | 2,275 ns/op |
| Appearance regeneration | 1,917 ns/op |
| Incremental serialization | 362 ns/op |
| Export/reopen | 6,440 ns/op |

Unchanged lexer, parser, FlateDecode, incremental export, and export/reopen results remain within 15% of v0.9 references. The mutation-plan benchmark now intentionally includes valid catalog/AcroForm-default resolution and is not an unchanged workload; its additional cost is the bounded inheritance proof. No validation or safety check was removed.

## PDF.js, WASM, and production readiness

The browser worker performs open, enumeration, mutation, incremental export, reopen, and PDF.js rendering for every v0.10 fixture. Target-region pixel validation separately proves third-party text, checkbox, radio, choice, page/widget rotation, FreeText, Highlight, Line, and unrelated-appearance preservation. Semantic assertions and prefix checks accompany the pixel evidence.

WASM/worker capability version is 0.10.0. Stable DTOs add URI and N/R/D appearance-presence flags; raw parser internals are not exposed. Handle limits, stale-handle rejection, cleanup, typed errors, and entirely local processing remain intact.

Production readiness: ready for the stated v0.10 boundary. Known gaps are CFF/CFF2 appearance writing, non-Identity composite reverse mappings, arbitrary widget rotation, ambiguous orphan radio grouping without a canonical parent, and broader signed/encrypted/hybrid-reference producer coverage. Compatibility is intentionally not claimed as exhaustive.

Recommended v0.11: collect redistributable corpus evidence for CFF/CFF2 and non-Identity composite fonts before deciding whether a bounded writer is justified; otherwise prioritize signed/encrypted and hybrid-reference preservation testing.

Patentability claims: **NONE**.
