# StarPDF experimental renderer

This opt-in execution layer lives under `src/lib/pdf/execution/render`. No production page component imports it. PDF.js remains the production renderer, the fallback for unsupported visible content, and the pixel oracle.

## Data path and identity

`adaptRuntimePage` accepts runtime text, image, vector, and annotation records plus a provenance resolver. The resolver must supply the owning PDF object and source location for each content-stream primitive. The runtime's stream index alone does not identify an object; unresolved records become PDF.js fallback regions. The caller must also attest complete page paint coverage; the current extraction APIs do not provide that attestation, so their pages fall back to PDF.js until an upstream scanner does. Each IR primitive stores its source, object, `StarNodeId`, `RenderNodeId`, PDF bounds, and paint order. `RenderNodeId` embeds `StarNodeId`; hit testing returns both. Pixel targets are the primitive's transformed bounds in the frame canvas.

The IR includes `GlyphRun`, `ImageInstance`, `PathInstance`, `ClipNode`, and `FormInstance`, with numeric geometry, paint, transforms, and external image keys suitable for a later GPU buffer builder. The initial canvas backend draws Latin glyph runs, axis-aligned images, rectangles, and lines with simple fill or stroke. Unsupported content, arbitrary paths, nested forms, complex shaping, annotation AP streams, and unresolved image pixels cause a full visible-page PDF.js fallback. The full-page fallback is deliberate: region compositing would break z order when arbitrary unsupported PDF content overlaps supported primitives. `FormInstance` and `ClipNode` are represented in IR; form execution is still a fallback.

## Spatial index choice

`benchmark.ts` compares a linear scan, fixed grid, and packed bounding-volume tree over 10,000 deterministic rectangles and 5,000 mixed point/viewport queries. Measured on this checkout:

| Structure | Query time | Matches |
| --- | ---: | ---: |
| Linear scan | 517.1 ms | 266,713 |
| Fixed grid | 17.4 ms | 266,713 |
| Packed BVH | 29.1 ms | 266,713 |

The packed BVH is selected. It is slower than the grid in this regular synthetic case but still about 17× faster than the scan, around 5.8 µs/query. It avoids selecting a cell size or duplicating large primitives across cells. Balanced subdivision gives approximately logarithmic descent plus visible candidates for ordinary page layouts; heavily overlapping bounds can still degrade to a scan. The tree is rebuilt only for pages named by a delta. Query results are sorted by paint order so pointer hits return the topmost primitive first.

## Deltas and redraw

`affectedVisualBounds` unions old and new geometry and the bounds of nodes named by semantic or changed-object entries. It also includes prior and current annotation fallback regions on affected pages for AP changes. Image delete and move, FreeText, and native text style changes use the same identity and geometry path. When a cached experimental frame exists, the renderer clips repaint to those bounds and only redraws primitives intersecting them. Unknown affected-page changes conservatively invalidate all known primitives on that page. A visible unsupported region instead rerenders through PDF.js.

## Differential evidence

`differential-smoke.ts` generates a two-page PDF and compares the canvas backend with a real PDF.js render through `@napi-rs/canvas` at 100 × 100 pixels. The red rectangle had 600 nonwhite pixels in both outputs and **0 differing pixels**. The Latin text had 142 nonwhite pixels in both outputs and **0 differing pixels**. `compareWithPdfjs` records supported IDs, fallback reasons, channel and pixel deltas, PDF and viewport bounds, and pixel access failures. A separate AP case records PDF.js fallback. These are narrow smoke fixtures; font substitutions, transparency, blend modes, complex transforms, and larger document corpora remain uncalibrated and will report divergence rather than silently pass.

Run:

```sh
npx vitest run src/lib/pdf/execution/render/render.test.ts
npx tsx src/lib/pdf/execution/render/benchmark.ts
npx tsx src/lib/pdf/execution/render/differential-smoke.ts
```
