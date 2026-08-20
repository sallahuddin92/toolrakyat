# StarPDF Engine v0.7

**Milestone:** Appearance Stream Generation, Annotation Mutation, and Visual Roundtrip Validation

**Starting SHA:** `efb56d36d181a6f28b0422e30b0d1ba1e6f27b7c`

**Milestone commit:** the commit containing this document

**Date:** August 20, 2026

## Implemented Production Capabilities

StarPDF v0.7 can parse AcroForm default appearances, generate appearances for supported form controls, add/update/remove supported annotations transactionally, export byte-preserving incremental updates, and expose those operations through the browser Web Worker/WASM boundary. The production crate remains safe Rust with `#![forbid(unsafe_code)]` and no `.unwrap()` or `.expect()` in `engine/starpdf/src`.

## Appearance Architecture

Appearance generation is isolated under `appearance/`. Generators return typed errors and an explicit `AppearanceStatus`: `Generated`, `Regenerated`, `PreservedExisting`, `LogicalValueOnly`, or `Unsupported`. Batch mutation combines statuses without hiding a weaker visual result.

## /DA Parsing

The resource-bounded `/DA` parser recognizes font selection (`Tf`), gray (`g`), RGB (`rg`), and CMYK (`k`) operators. Malformed operands, non-finite values, invalid color ranges, and oversized input are rejected deterministically.

## Font Selection

Appearance generation uses the font resource named by `/DA` when it is usable and otherwise falls back to the standard Helvetica resource. Existing font resources are preserved; v0.7 does not embed or subset arbitrary fonts.

## Text Appearance

Single-line and multiline text fields produce bounded Form XObject appearance streams with escaped PDF strings, clipping, alignment, background/border drawing, and deterministic line placement. Password fields are masked. Text values and generated streams are size-limited.

## Checkbox / Radio Appearance

Checkbox and radio updates synchronize the field value and widget appearance state. Generated on/off appearances are deterministic and bounded. AcroForm parsing follows the PDF flag positions for radio (bit 16) and pushbutton (bit 17).

## Choice Appearance

Choice fields synchronize logical selection and visible text appearance for the supported single-selection path. Unsupported choice configurations remain explicit rather than silently claiming a rendered appearance.

## Annotation Mutation

Add mutation is implemented for Text, Link, FreeText, Highlight, Underline, StrikeOut, Square, Circle, and Ink annotations. Generated dictionaries include page association and type-specific geometry. Update changes supported semantic properties while preserving unrelated/vendor-private dictionary entries. Remove verifies the target is an annotation on the requested page before unlinking it.

## Atomic Transaction Model

All changes are validated and planned against an isolated object overlay. A failed change returns a typed error and emits no partial incremental output. Plans are capped at 1,000 mutations, pending browser changes at 500, page widgets at 2,000, and PDF object numbers at 9,999,999,999.

## Incremental Export

Export appends changed objects, a new cross-reference section, and trailer. The complete original input remains an exact byte prefix of the result. Growth is capped at 64 MiB, arithmetic is checked, and the resulting document is reopened in Rust, PDF.js, and pdf-lib validation paths.

## Visual Roundtrip Results

The Chromium test loads a real AcroForm PDF, initializes the v0.7 worker/WASM engine, changes text/checkbox/radio values, adds FreeText and Square annotations, exports incrementally, reopens through StarPDF, and reloads through PDF.js. A bounded canvas-region comparison detects a greater-than-2% pixel change. Stale handles return the typed `INVALID_HANDLE` error.

## cargo-fuzz Results

Bounded 10-second libFuzzer campaigns ran for all new targets and affected legacy targets. The campaigns executed 18,292,731 inputs in total with zero crashes, hangs, OOMs, or unique reproducers.

New target executions:

- `fuzz_da_parser`: 2,192,489
- `fuzz_text_appearance`: 121,822
- `fuzz_annotation_generator`: 71,012
- `fuzz_annotation_mutation`: 2,401,679 across an empty-corpus pass and a PDF-seeded pass

Affected legacy target executions:

- `fuzz_acroform`: 4,842,722
- `fuzz_mutation`: 2,618,066
- `fuzz_incremental_writer`: 1,332,963
- `fuzz_annotations`: 4,711,978

## Resource Limits

Annotation contents are capped at 1 MiB, URIs at 16 KiB, QuadPoints at 8,000 scalar values, InkLists at 1,000 paths/10,000 points per path/100,000 total points, and generated annotation appearance data at 256 KiB. Rectangles, colors, widths, font sizes, points, and geometry must be finite and semantically valid. Incremental output growth, mutation counts, widget counts, state names, object numbers, and pending WASM changes have explicit caps.

## Benchmarks

The warm native benchmark pass measured lexer throughput at 134.18 MB/s, object parsing at 61.84 MB/s, and FlateDecode at 2,026.76 MB/s. New hot-loop results were 1,204 ns/text appearance, 906 ns/checkbox appearance, 1,860 ns/annotation generation, 2,891 ns/annotation mutation planning, and 6,093 ns/export-plus-reopen. The cold pass was retained as diagnostic noise; the required warm rerun did not cross the 15% regression threshold against the stated v0.6 baselines.

## WASM / Browser Integration

The browser client and worker expose form and annotation reads, queued mutations, incremental export, and document close. Worker failures use typed codes: `INVALID_HANDLE`, `RESOURCE_LIMIT`, `UNSUPPORTED`, `INVALID_PDF`, and `ENGINE_ERROR`. Closing a handle releases the registry entry; no PDF bytes are uploaded by StarPDF.

## Real-World Validation

Differential tests validate mutated output using StarPDF plus pdf-lib, and the browser roundtrip validates rendering through PDF.js. The fixture covers text, checkbox, and radio fields plus FreeText and Square annotations. Corrupt documents and stale handles fail explicitly.

## Known Limitations

- v0.7 does not render PDFs; PDF.js remains the display engine.
- Existing annotation appearances are preserved during semantic annotation updates, so a viewer may display a stale appearance until a future regeneration path is implemented.
- Font embedding/subsetting, rich text, comb fields, multi-select choice rendering, signature appearances, and arbitrary custom annotation appearance editing are not implemented.
- Unsupported appearance cases remain available as logical-only mutations with an explicit status where safe; they are not reported as visually regenerated.

## Production Readiness

The v0.7 subsystem is ready for production use within the documented supported surface: safe, local, bounded form/annotation mutation with incremental export and explicit failure/status reporting. All Rust, application, browser, fuzz-registration, and release-build gates are required to remain green at the milestone commit.

## Recommended v0.8

Prioritize deterministic regeneration of existing annotation appearances, embedded-font resolution/subsetting, richer choice/comb-field rendering, and expanded real-world corpus validation while preserving the current transaction and resource-limit model.
