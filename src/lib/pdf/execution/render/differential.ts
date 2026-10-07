import type { PdfBox } from "../contracts";
import type { RenderFrame } from "./renderer";
import type { RenderPageIR } from "./ir";

export interface DifferentialRecord {
  readonly page_index: number;
  readonly supported: readonly string[];
  readonly fallback: readonly { readonly runtime_id: string; readonly reason: string }[];
  readonly pixel_delta: { readonly differing_pixels: number; readonly max_channel_delta: number; readonly mean_channel_delta: number };
  readonly bounds_delta: readonly { readonly render_node: string; readonly pdf_bounds: PdfBox; readonly viewport_bounds: PdfBox;
    readonly prototype_ink_bounds: PdfBox | null; readonly pdfjs_ink_bounds: PdfBox | null;
    readonly edge_delta: PdfBox | null }[];
  readonly failure_reason: string | null;
}
/** Compare actual canvas pixels; never suppress deviations when the experimental backend drew. */
export function compareWithPdfjs(frame: RenderFrame, oracle: HTMLCanvasElement, ir: RenderPageIR,
  pdfBoxToViewport: (rect: PdfBox) => PdfBox): DifferentialRecord {
  const a = frame.canvas.getContext("2d")?.getImageData(0, 0, frame.canvas.width, frame.canvas.height).data;
  const b = oracle.getContext("2d")?.getImageData(0, 0, oracle.width, oracle.height).data;
  if (!a || !b || frame.canvas.width !== oracle.width || frame.canvas.height !== oracle.height)
    return { page_index: frame.page_index, supported: frame.rendered_nodes, fallback: frame.fallback,
      pixel_delta: { differing_pixels: 0, max_channel_delta: 0, mean_channel_delta: 0 }, bounds_delta: [],
      failure_reason: "canvas dimensions or pixel access unavailable" };
  let differing = 0, sum = 0, max = 0;
  for (let i = 0; i < a.length; i += 4) {
    let pixelDiffers = false;
    for (let c = 0; c < 4; c++) { const d = Math.abs(a[i + c] - b[i + c]); sum += d; max = Math.max(max, d); pixelDiffers ||= d !== 0; }
    if (pixelDiffers) differing++;
  }
  const isolated = ir.primitives.length === 1 && frame.mode === "experimental";
  const prototype = isolated ? inkBounds(a, frame.canvas.width, frame.canvas.height) : null;
  const pdfjs = isolated ? inkBounds(b, oracle.width, oracle.height) : null;
  return { page_index: frame.page_index, supported: frame.rendered_nodes, fallback: frame.fallback,
    pixel_delta: { differing_pixels: differing, max_channel_delta: max, mean_channel_delta: sum / a.length },
    bounds_delta: ir.primitives.map(node => ({ render_node: node.id, pdf_bounds: node.bounds.rect,
      viewport_bounds: pdfBoxToViewport(node.bounds.rect), prototype_ink_bounds: prototype, pdfjs_ink_bounds: pdfjs,
      edge_delta: prototype && pdfjs ? [prototype[0] - pdfjs[0], prototype[1] - pdfjs[1],
        prototype[2] - pdfjs[2], prototype[3] - pdfjs[3]] : null })), failure_reason: null };
}
function inkBounds(data: Uint8ClampedArray, width: number, height: number): PdfBox | null {
  let left = width, top = height, right = -1, bottom = -1;
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const i = (y * width + x) * 4;
    if (data[i] < 250 || data[i + 1] < 250 || data[i + 2] < 250) {
      left = Math.min(left, x); top = Math.min(top, y); right = Math.max(right, x + 1); bottom = Math.max(bottom, y + 1);
    }
  }
  return right < 0 ? null : [left, top, right, bottom];
}
