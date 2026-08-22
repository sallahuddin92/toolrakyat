# StarPDF v0.11A final qualification

Qualification date: 2026-08-20 (Asia/Kuala_Lumpur)

## Repository

- HEAD: `7bb1fa193c41d49a4f08044a27e6fec2e2c3d28d`
- Accepted milestone: StarPDF v0.11A
- Pre-existing modified fixtures, preserved and excluded from any commit:
  - `engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-choice-radio.pdf`
  - `engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf`
- No unexpected source modification was found and no regression fix was required.
- This qualification adds only this report. No feature, parser, WASM API, or document-protection behavior was changed. No commit or push was made.

The regression commands ran against the current working tree. Consequently, the two named pre-existing fixture paths contained their user-modified bytes during the tests; they were not altered by this qualification.

## Rust gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test` | PASS: 195 passed, 0 failed, 0 ignored |
| `cargo build --release` | PASS |
| `cargo bench` | PASS, including one warm-up and five recorded full-suite runs |
| Production `.unwrap()` / `.expect()` scan | PASS: 0 exact matches in `engine/starpdf/src` |
| Unsafe Rust scan | PASS: 0 `unsafe` tokens in `engine/starpdf/src` |
| Crate invariant | PASS: `engine/starpdf/src/lib.rs` retains `#![forbid(unsafe_code)]` |

## ToolRakyat gates

| Gate | Result |
| --- | --- |
| `npm run lint` | PASS |
| `npm run typecheck` | PASS |
| `npm test` | PASS: 25 files, 640 tests |
| `npm run build` | PASS: Next.js 16.2.4 production build |
| `npx playwright test` | PASS: 36 tests |

The production build emitted the existing development-only warning that `AUTH_SECRET` was absent and the development fallback was used. It did not fail the build.

## Five-run performance baseline

One complete warm-up preceded five complete recorded `cargo bench` runs. Throughput is higher-is-better; latency is lower-is-better.

| Workload | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: |
| Lexer | 118.89 MB/s | 127.49 MB/s | 128.80 MB/s | 144.63 MB/s |
| Object parser | 49.32 MB/s | 53.07 MB/s | 52.92 MB/s | 57.82 MB/s |
| FlateDecode | 1,983.55 MB/s | 1,989.69 MB/s | 1,993.38 MB/s | 2,009.98 MB/s |
| Mutation planning | 3,924 ns/op | 3,960 ns/op | 4,010.00 ns/op | 4,196 ns/op |
| Incremental serialization/export | 358 ns/op | 366 ns/op | 364.80 ns/op | 371 ns/op |
| Export/reopen | 6,356 ns/op | 6,509 ns/op | 6,528.40 ns/op | 6,791 ns/op |

The lexer, object-parser, FlateDecode, incremental-serialization, and export/reopen benchmark definitions retain the v0.10 input and iteration semantics. Their medians differ from the recorded v0.10 values by -10.09%, -10.22%, +1.23%, +1.10% latency, and +1.07% latency respectively. None is an adverse degradation above 15%.

General mutation planning is **NOT DIRECTLY COMPARABLE** to v0.10. Although the benchmark call site remains `PdfDocument::apply_mutation`, v0.11A's existing document-level path performs additional bounded policy and field-graph inspection before constructing the mutation plan. It is directly comparable to the recorded v0.11A workload: 3,960 ns/op median versus 3,902 ns/op recorded (+1.49% latency). No reproducible greater-than-15% regression was confirmed, and no validation, resource limit, bounds check, or typed failure was removed.

## Ordinary compatibility smoke test

The existing committed test families ran without adding fixtures or recovery behavior:

| Area | Existing evidence | Result |
| --- | --- | --- |
| Ordinary multi-page PDF | `real_world_tests`, `real_world_text_tests`, browser page navigation | PASS |
| Text extraction/search | `real_world_text_tests`, `search_tests`, v0.9 compatibility matrix | PASS |
| Forms | AcroForm tests plus v0.9/v0.10 compatibility matrices | PASS |
| Annotations | annotation parser/mutation tests plus v0.10 producer fixtures | PASS |
| TrueType appearance | v0.8 embedded TrueType/subset roundtrips and browser pixel validation | PASS |
| Supported Type0 appearance | existing Identity-H automatic-embedding browser path | PASS |
| Rotated widget | v0.8 rotation tests and PDFKit page/widget rotation browser path | PASS |
| Incremental revisions | incremental-writer tests and v0.10 prior-history fixture | PASS |
| Metadata-rich fixture | v0.11 metadata/catalog/info/document-ID preservation test and browser properties path | PASS |
| Mixed xref | hybrid xref and v0.11 mixed-revision precedence tests | PASS |
| Supported export/reopen | Rust prefix-preserving roundtrips and all v0.10 producer PDF.js reopen/render paths | PASS |

## WASM and worker smoke test

- WASM build: PASS with `cargo build --target wasm32-unknown-unknown --release --features wasm`.
- Generated bindings: PASS. `wasm-bindgen 0.2.127` regenerated the web bindings from the release artifact; generated JS, TypeScript declarations, and WASM were byte-identical to the committed source copy. The public and source WASM files also share SHA-256 `d8af3b1c1f62a683f53bc2ff82db4bbabd98176c32f767d1c7286277e52760cd`.
- Worker startup and document open: PASS in the full browser suite.
- Typed worker error path: PASS.
- Stale handle: PASS with `INVALID_HANDLE` after close.
- Cleanup: PASS; close removes the document handle and subsequent access refuses safely.
- Ordinary SmartPDF workflows: PASS through the complete 36-test Playwright suite.

## Recommendation

**GO for v0.12**, limited to its separately approved scope. StarPDF v0.11A's existing Rust, ToolRakyat, compatibility, WASM, worker, safety, and performance gates are green. This is ordinary qualification evidence, not a claim of exhaustive PDF compatibility.
