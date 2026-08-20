# StarPDF Engine v0.8

**Milestone:** Production Appearance Fidelity, Embedded Font Resolution, Rich Form Rendering, and Compatibility Expansion

**Starting SHA:** `d2e847d57be9007b9a0ab486377a605e7d5e69ec`

**Milestone commit:** the commit containing this document

**Date:** August 20, 2026

## Production capabilities

StarPDF v0.8 regenerates deterministic appearances for the supported visual annotation and AcroForm mutation surface, reuses compatible embedded TrueType fonts, checks glyph coverage, renders comb/multiline/list fields, and preserves atomic byte-prefix incremental export. The browser API remains worker-hosted and local-only. The crate retains `#![forbid(unsafe_code)]` and zero production `.unwrap()`/`.expect()` calls.

The documented mutable form surface now renders 9 of 9 supported operations: single-line, password, multiline, and comb text; checkbox; radio; combo-box single selection; list-box single selection; and list-box multi-selection. This is 100% of the documented v0.8 surface, not 100% of all PDF form behavior. Push-button authoring, signature appearances, rich-text `/RV`, JavaScript actions, and arbitrary viewer UI state are outside the supported surface.

## Annotation appearance regeneration

Visual updates to FreeText, Highlight, Underline, StrikeOut, Square, Circle, Ink, and Line reconstruct an appearance from the updated dictionary and replace stale `/AP` atomically. The stable status vocabulary is `AP_REGENERATED`, `AP_PRESERVED`, `AP_NOT_REQUIRED`, and `AP_UNSUPPORTED`. Link and Text annotations retain viewer-native behavior when no explicit visual appearance is required. Unknown visible subtypes are refused instead of reporting a false regeneration result.

Line annotations implement `/L`, `/LE`, `/C`, `/IC`, `/BS`, `/Border`, `/Contents`, and a padded `/Rect`. Supported endings are None, Square, Circle, Diamond, OpenArrow, and ClosedArrow. Geometry, widths, colors, contents, stream bytes, and line endings are bounded and validated.

## Embedded-font strategy and glyph coverage

Appearance font resolution follows field `/DR`, AcroForm `/DR`, and inherited page resources using the name selected by `/DA`. Font dictionaries, descriptors, and unfiltered or Flate-decoded `FontFile2`/SFNT `FontFile3` streams are resolved through the existing font parser. The generated Form XObject references the original indirect font resource, so repeated appearances do not duplicate the embedded font.

Simple fonts encode through their declared encoding and validate the embedded cmap when present. Type0 fonts are supported for appearance writing only when `Identity-H` or `Identity-V` and an identity/default `CIDToGIDMap` make glyph-ID character codes deterministic. Non-identity composite mappings, unsupported filters/formats, and missing glyphs return an explicit `UNREPRESENTABLE`/typed limitation. Helvetica fallback is marked `FALLBACK` and is used only when it represents every requested character; it is never claimed as exact.

## TrueType subsetting foundation

The bounded subsetter accepts SFNT fonts containing `glyf`, `loca`, `head`, and `maxp`. It preserves original glyph IDs, always retains `.notdef`, closes over composite dependencies to depth 32, rebuilds long `loca` and `glyf`, preserves remaining tables, recomputes table checksums and `head.checksumAdjustment`, and reparses the output before returning it. CFF/OpenType-CFF subsetting is explicitly unsupported. The current foundation produces valid deterministic subset bytes but is not yet wired into mutation output because reuse of an existing embedded resource is safer than replacing shared font objects.

## Rich form rendering semantics

- Comb mode requires the comb flag and `/MaxLen` in `1..=4096`, rejects multiline/password combinations, refuses over-limit values, divides the widget into equal cells, and places one Unicode scalar in each cell.
- Multiline mode honors explicit LF breaks, performs deterministic whitespace-first character fallback wrapping, applies `/Q` left/center/right alignment, uses a consistent font-size-derived line height, clips to the field rectangle, and caps output at 2,048 lines and 256 KiB.
- Combo boxes render one visible selected or editable value.
- List boxes support single and flag-enabled multi-selection, synchronize `/V` and strictly ordered `/I`, honor bounded `/TI` as the top visible option, and draw deterministic selection highlighting. Scrolling UI state beyond `/TI` is not claimed.
- Parent fields with child Widget annotations update `/V`/flags on the field and regenerate `/AP` on every bounded child widget in the same transaction.

## Incremental writer and resource behavior

All font, layout, annotation, and resource work is completed in the mutation overlay before serialization. A failure returns no partial output. Sequential exports preserve every prior input as an exact byte prefix; new appearance streams and annotation objects receive collision-checked object numbers, and `/Size`, `/Prev`, xref entries, and references reopen consistently. Incremental growth remains capped at 64 MiB.

## Compatibility corpus and differential validation

The curated corpus uses only repository and locally generated material. It is intentionally small and is not an exhaustive compatibility claim.

| Fixture/class | Producer | PDF/xref | Relevant content | Result | Differential class |
|---|---|---|---|---|---|
| `smartpdf-form.pdf` | pdf-lib | PDF 1.7, xref stream/object streams | text, checkbox, radio | Rust/WASM mutate, sequential reopen, PDF.js visible render | AGREE |
| `smartpdf-adobe-like-form.pdf` | pdf-lib | PDF 1.7, xref stream/object streams | three AcroForm fields | parse/validate/reopen | AGREE |
| `multi-page.test.pdf` | pdf-lib | PDF 1.7, xref stream/object streams | two pages/text | parse, page traversal, PDF.js render | AGREE |
| `scanned-test.pdf` | pdf-lib | PDF 1.7, xref stream/object streams | image-only page | zero text/forms, valid page | AGREE |
| locally generated rich fixture | pdf-lib + fontkit | PDF 1.7, classic xref | embedded TrueType, child widgets, comb, multiline, multi-list | mutate/reopen and targeted PDF.js visual difference | AGREE |
| StarPDF annotation generations | StarPDF incremental writer | preserved base + classic incremental xref | eight regenerated AP subtypes; detailed Line | StarPDF reopen and PDF.js targeted visual difference | AGREE |
| `invalid.pdf` | deliberate invalid bytes | invalid | corrupt header | StarPDF/pdf-lib/PDF.js refuse | AGREE |

Relevant semantic values are compared with pdf-lib, while appearance visibility is compared through PDF.js targeted regions. No implementation was changed solely to mimic another parser.

## Fuzzing and resource limits

There are 23 registered real cargo-fuzz targets. New bounded libFuzzer campaigns covered annotation/Line regeneration, embedded-font resolution, TrueType subsetting, and combined comb/multiline/list/resource generation. Clean summary passes plus the final affected resolver rerun recorded 2,214,760 inputs with zero crashes, hangs, or regression artifacts; preceding 10-second passes also completed without findings.

Key limits include: 16 MiB embedded/subset font bytes; 64 SFNT tables; format-bounded 65,535 glyph IDs; 4,096 requested/closed subset glyphs; 65,536 cmap mappings/groups and 4,096 format-4 segments; 256 appearance resources; 64 resource ancestors; 4,096 comb cells; 2,048 multiline lines; 5,000 list options; 1,000 multi-select indexes; 256 KiB form and annotation appearance streams; 1 MiB values/annotation contents; 2,000 widgets/generated objects; 500 mutations; and 64 MiB incremental growth. Checked arithmetic and finite geometry validation precede allocation or serialization.

## Benchmarks

The second warm native run measured: lexer 137.29 MB/s, object parser 58.65 MB/s, FlateDecode 1,916.07 MB/s, font resolution 1,952 ns/op, glyph coverage 270 ns/op, TrueType subset 822 ns/op, text appearance 1,378 ns/op, comb 5,188 ns/op, multiline 3,301 ns/op, list box 4,355 ns/op, annotation AP regeneration 1,993 ns/op, mutation planning 1,148 ns/op, incremental serialization 361 ns/op, export/reopen 6,432 ns/op, and resource export/reopen 18,950 ns/op. Repeated warm results did not show a material greater-than-15% regression on an unchanged v0.7 workload.

## Browser integration

WASM reports version `0.8.0`, exposes stable rich choice/annotation DTOs plus appearance status, and keeps parsing/font work in the Web Worker. Vitest covers the typed protocol and handle lifecycle. Playwright performs actual mutation, two-generation export/reopen, PDF.js rendering, and bounded region comparisons. StarPDF uploads no PDF bytes to a server.

## Known limitations

- StarPDF delegates display to PDF.js and is not a custom renderer.
- CFF/CFF2, variable-font subsetting, non-identity Type0 appearance encodings, arbitrary CMaps, and unsupported font stream filters are refused. TrueType subsetting is a validated foundation, not yet an automatic export optimization.
- Rotated widget appearance matrices, rich text `/RV`, bidi shaping, complex-script shaping, kerning, vertical typography, and sophisticated paragraph layout are not implemented.
- List-box transient scroll/focus state beyond `/TI`, push-button appearance authoring, signature appearance/cryptography, and arbitrary annotation subtypes remain unsupported.
- Compatibility evidence covers pdf-lib, StarPDF, and PDF.js classes available locally; no Microsoft Office, LibreOffice, macOS Preview-produced, encrypted, signed, or hybrid-reference fixture was available in this milestone.

## Production readiness and recommended v0.9

v0.8 is production-ready for its documented local, bounded annotation and AcroForm mutation surface. Supported outputs reopen consistently in Rust and PDF.js, failures are typed and atomic, browser memory stayed within the configured 2 GiB fuzz ceiling (observed campaigns remained well below it), and the full gate matrix is required at the milestone commit.

After reviewing this milestone, a possible v0.9 should prioritize a larger locally sourced producer corpus, rotated widget matrices, careful automatic TrueType subset embedding, and additional composite-font/CMap support. OCR, arbitrary existing-text editing, encryption, signatures, cloud processing, and a custom renderer remain out of scope.
