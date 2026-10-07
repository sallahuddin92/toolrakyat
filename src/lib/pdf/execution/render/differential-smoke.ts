/** Real PDF.js pixel oracle. Run with `npx tsx src/lib/pdf/execution/render/differential-smoke.ts`. */
import { createCanvas, DOMMatrix, ImageData, Path2D } from "@napi-rs/canvas";
import { PDFDocument, rgb, StandardFonts } from "pdf-lib";
import { createCoordinateTransform } from "../coordinates";
import { createRenderNodeId, createStarNodeId } from "../identity";
import { compareWithPdfjs } from "./differential";
import type { RenderPageIR } from "./ir";
import { ExperimentalRenderer } from "./renderer";

async function main() {
Object.assign(globalThis, { DOMMatrix, ImageData, Path2D,
  document: { createElement: () => createCanvas(1, 1) } });
const pdfjs = await import("pdfjs-dist/legacy/build/pdf.mjs");
const pdf = await PDFDocument.create();
const page = pdf.addPage([100, 100]);
page.drawRectangle({ x: 10, y: 20, width: 30, height: 20, color: rgb(1, 0, 0) });
const second = pdf.addPage([100, 100]);
second.drawText("Latin", { x: 10, y: 60, size: 12, font: await pdf.embedFont(StandardFonts.Helvetica), color: rgb(0, 0, 0) });
const bytes = await pdf.save();
const loading = pdfjs.getDocument({ data: new Uint8Array(bytes), useSystemFonts: true, disableFontFace: true });
const pdfDocument = await loading.promise;
const pdfPage = await pdfDocument.getPage(1);
const viewport = pdfPage.getViewport({ scale: 1 });
const oracle = createCanvas(100, 100);
await pdfPage.render({ canvasContext: oracle.getContext("2d") as never, canvas: oracle as never, viewport }).promise;
const object = { number: "1", generation: 0 };
const origin = { object, source: { kind: "Generated" as const }, occurrence: "page:0/path:0", runtime_id: "rect" };
const star_node = createStarNodeId("differential", object, origin.occurrence);
const ir: RenderPageIR = { page_index: 0, primitives: [{ id: createRenderNodeId(star_node, "path:0"), star_node, origin,
  kind: "PathInstance", bounds: { page_index: 0, rect: [10, 20, 40, 40] }, z: 0,
  geometry: { kind: "rect", rect: [10, 20, 40, 40] }, fill: [1, 0, 0], stroke: null, line_width: 0 }],
  fallback: [] };
const renderer = new ExperimentalRenderer();
await renderer.open({ loadPage: async () => ir, renderPdfjs: async (request, context) => {
  await pdfPage.render({ canvasContext: context as never, canvas: context.canvas as never,
    viewport: pdfPage.getViewport({ scale: 1 }) }).promise;
}, resolveImage: () => null }, { document_id: "differential", revision: "1" });
const request = { page_index: 0, transform: createCoordinateTransform({ media_box: [0, 0, 100, 100], rotation: 0, scale: 1,
  user_unit: 1, device_pixel_ratio: 1 }), viewport_bounds: [0, 0, 100, 100] as const };
const result = await renderer.renderViewport(request);
if (!result.ok) throw new Error(result.refusal.reason);
const comparison = compareWithPdfjs(result.value, oracle as never, ir, request.transform.pdfBoxToViewport);
const ink = (canvas: typeof oracle) => {
  const data = canvas.getContext("2d").getImageData(0, 0, 100, 100).data;
  let count = 0; for (let i = 0; i < data.length; i += 4) if (data[i] < 250 || data[i + 1] < 250 || data[i + 2] < 250) count++;
  return count;
};
console.log(JSON.stringify(comparison, null, 2));
console.log(JSON.stringify({ page_index: 0, prototype_ink_pixels: ink(result.value.canvas as never), pdfjs_ink_pixels: ink(oracle) }));
const pdfPage2 = await pdfDocument.getPage(2);
const textOrigin = { object: { number: "2", generation: 0 }, source: { kind: "Generated" as const }, occurrence: "page:1/text:0", runtime_id: "latin" };
const textStar = createStarNodeId("differential", textOrigin.object, textOrigin.occurrence);
const textIR: RenderPageIR = { page_index: 1, primitives: [{ id: createRenderNodeId(textStar, "glyph:0"), star_node: textStar,
  origin: textOrigin, kind: "GlyphRun", bounds: { page_index: 1, rect: [10, 60, 40, 72] }, z: 0, text: "Latin",
  baseline: [10, 60], font: "Helvetica", size: 12, color: [0, 0, 0], transform: [1, 0, 0, 1, 0, 0] }], fallback: [] };
const textRenderer = new ExperimentalRenderer();
await textRenderer.open({ loadPage: async () => textIR, renderPdfjs: async () => {}, resolveImage: () => null },
  { document_id: "differential", revision: "1" });
const textResult = await textRenderer.renderViewport({ ...request, page_index: 1 });
if (!textResult.ok) throw new Error(textResult.refusal.reason);
const textOracle = createCanvas(100, 100);
await pdfPage2.render({ canvasContext: textOracle.getContext("2d") as never, canvas: textOracle as never,
  viewport: pdfPage2.getViewport({ scale: 1 }) }).promise;
console.log(JSON.stringify(compareWithPdfjs(textResult.value, textOracle as never, textIR, request.transform.pdfBoxToViewport), null, 2));
console.log(JSON.stringify({ page_index: 1, prototype_ink_pixels: ink(textResult.value.canvas as never), pdfjs_ink_pixels: ink(textOracle) }));
const fallbackIR: RenderPageIR = { page_index: 1, primitives: [], fallback: [
  { bounds: { page_index: 1, rect: [10, 58, 50, 72] }, runtime_id: "annotation", reason: "annotation AP unsupported" }] };
const fallbackRenderer = new ExperimentalRenderer();
await fallbackRenderer.open({ loadPage: async () => fallbackIR, renderPdfjs: async (_request, context) => {
  await pdfPage2.render({ canvasContext: context as never, canvas: context.canvas as never,
    viewport: pdfPage2.getViewport({ scale: 1 }) }).promise;
}, resolveImage: () => null }, { document_id: "differential", revision: "1" });
const fallbackResult = await fallbackRenderer.renderViewport({ ...request, page_index: 1 });
if (!fallbackResult.ok || fallbackResult.value.mode !== "pdfjs-fallback") throw new Error("PDF.js fallback failed");
console.log(JSON.stringify({ page_index: 1, supported: [], fallback: fallbackResult.value.fallback, pixel_delta: null,
  bounds_delta: [], failure_reason: null }, null, 2));
await pdfDocument.destroy();
}
main().catch(error => { console.error(error); process.exitCode = 1; });
