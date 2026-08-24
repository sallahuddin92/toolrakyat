# SmartPDF 1.0 RC2 Release Notes

SmartPDF 1.0 Release Candidate 2 is a local-first PDF editor powered by the StarPDF Rust WebAssembly engine.

## Release identity

- **Qualified product SHA:** `003f70f0c0cca83a1373d403d21f1a937c7eb7ea`
- **Qualified branch:** `main`
- **Previous release candidate:** `v1.0.0-rc.1` remains unchanged
- **RC2 tag:** Not created
- **Manual acceptance:** `HUMAN PENDING`

The RC2 preparation commit contains documentation only. The qualified product implementation remains the SHA above.

## Adaptive multilingual font runtime

RC2 adds a real end-to-end adaptive font runtime for native PDF text editing:

- **HarfRust GSUB/GPOS shaping:** Complex text is shaped through OpenType substitution and positioning data, producing real glyph IDs, clusters, advances, and offsets instead of per-character cmap approximation.
- **Automatic Unicode and script handling:** Bidirectional and script-aware planning selects appropriate shaping and fallback behavior for Arabic, Jawi, Hebrew, Devanagari, Japanese, Simplified Chinese, Traditional Chinese, Korean, and mixed-script text.
- **Document-font reuse first:** StarPDF checks compatible fonts already present in the PDF before using a bundled fallback.
- **Lazy bundled fallbacks:** Noto fallback assets are fetched and registered only when replacement text requires them. They are not embedded in the WebAssembly binary, and browser or system fonts are not authoritative for export.
- **Locale-aware CJK selection:** Japanese, Korean, Simplified Chinese, and Traditional Chinese replacements select their corresponding bundled CJK font where the text provides enough script or orthographic context.
- **Native PDF fallback embedding:** New fallbacks use Type0 fonts with `Identity-H`, `CIDFontType2` descendants, explicit CID-to-GID mapping, and valid embedded TrueType font programs.
- **Real font subsetting:** Skera-backed retain-GID subsetting removes unused font data while preserving glyph addressing required by exported content streams.
- **Exact Unicode recovery:** Generated ToUnicode CMaps use occurrence-specific CIDs so export, reopen, extraction, search, and copy semantics preserve logical Unicode even when RTL glyphs are stored in visual order.

## Qualified multilingual coverage

Automated export-to-reopen qualification covers:

- Arabic
- Jawi, including `ڤ`, `ڠ`, `ڽ`, and `چ`
- Hebrew
- Devanagari
- Japanese
- Simplified Chinese
- Traditional Chinese
- Korean
- Mixed Latin, Arabic, and CJK text

The qualification asserts non-missing shaped glyph IDs, exact reopened Unicode, valid Type0/CIDFontType2 and ToUnicode structures, successful full-text search, stable target coordinates, preserved unrelated text coordinates, bounded subsets, and reopenable exports.

## Text movement safety

SmartPDF retains its conservative text-move dependency policy. Independent text can be moved by updating safe text positioning operators. Moves that would disturb dependent downstream text are refused with a user-facing safety message rather than shifting unrelated content or producing a corrupt export.

## Automated qualification

GitHub Actions workflow: [SmartPDF CI for the qualified product SHA](https://github.com/sallahuddin92/toolrakyat/actions/runs/32681263824)

| Gate | Result |
| :--- | :---: |
| StarPDF Rust formatting, Clippy, all-features tests, release build | PASS |
| Web ESLint, TypeScript, Vitest, Next.js production build | PASS |
| Playwright Chromium, one worker | PASS |
| Playwright Firefox, one worker | PASS |
| Playwright WebKit, one worker | PASS |

Chromium, Firefox, and Playwright WebKit are qualified. **Safari was not directly tested.** Playwright WebKit qualification must not be represented as direct Safari qualification.

## Bundled font licensing

The seven bundled Noto fallback assets have recorded upstream sources, embedded versions, SHA-256 hashes, and SIL Open Font License 1.1 terms in `THIRD_PARTY_NOTICES.md`. The complete OFL 1.1 text ships with the font assets.

## Release boundaries

- Manual acceptance using real PDFs is still required and remains `HUMAN PENDING`.
- Vertical writing and layouts requiring unsafe paragraph reflow remain subject to safe refusal.
- Encrypted PDFs remain protected by typed refusal behavior.
- Existing digital-signature structures may be preserved during incremental export, but cryptographic signature verification is not performed.
- RC2 must not be tagged until the manual acceptance document is completed and approved by a human release owner.
