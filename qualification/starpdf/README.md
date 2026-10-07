# StarPDF corpus qualification

Run from the repository root:

```sh
python3 qualification/starpdf/run.py --regressions
```

The command writes `qualification/starpdf/out/qualification.json` and `report.md`, then exits nonzero for observed StarPDF failures, unexplained changed objects, or unexpected oracle disagreements. Every fixture is checked against its recorded SHA-256 before use. Temporary edited PDFs and rendered PNGs are deleted at the end of each fixture. No field PDF is added by this workstream.

`corpus.json` links to already committed fixtures and their source manifests. Its `categories` list is the required taxonomy; `category_coverage` in the result shows empty categories. The regression matrix runs the established tests for FreeText, native text, multilingual fonts, images, forms, annotations, page operations, and xref recovery. Fixture provenance is the adjacent original MANIFEST, including producer and generator details. New fixture entries require an explicit source, license or permission, SHA-256, and intended edit applicability. Do not commit private or copyrighted field PDFs without permission.

The Rust example is a test-only subprocess probe. It measures StarPDF open, page parsing, first-page access, text extraction, mutation planning, export, reopen, and re-extraction. Poppler rasterizes the first page before and after controlled edits and supplies page-count checks; PDF.js independently opens and extracts text where locally installed. qpdf runs `--check` if available. Missing tools and inapplicable stages are explicit `UNSUPPORTED` records. Each subprocess has a timeout; crash, timeout, duration, and sampled peak RSS are retained. `psutil` and Pillow improve memory and pixel metrics but are optional.

For each controlled form edit, the probe snapshots serialized effective objects before and after export, hashes them with the StarPDF PDF-object hash domain, constructs a `DocumentDelta`, and calls `MutationEnvelope::compare` to produce a `MutationCertificate`. The plan's object set is only a provisional expected allowlist; it is **not** independent mutation authority. The certificate is correctly `REFUSED` until complete structural and visual closures can be proved. `unexplained_change_count` currently counts changed objects outside that plan and targets zero; zero alone does not certify mutation correctness. Pixel differences are observational and have no unexplained-area classification yet.

The report is a baseline, not a speed claim. StarPDF has no production raster or hit-test implementation exposed at this base, and AP generation is included in mutation planning rather than timed separately. Those timings are marked unsupported. Poppler render time is an oracle measurement, never attributed to StarPDF.
