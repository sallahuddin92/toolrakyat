import type { ContractResult, DocumentDelta, PageBounds, PdfBox, Point, RenderHit, Renderer, RendererDocumentState, ViewportRequest } from "../contracts";
import { renderStarNode } from "../identity";
import { affectedVisualBounds } from "./adapter";
import { intersects, validatePageIR, type ImageInstance, type RenderPageIR, type RenderPrimitive } from "./ir";
import { SpatialIndex } from "./spatial";

export interface RenderDocument {
  loadPage(pageIndex: number): Promise<RenderPageIR>;
  /** Production PDF.js page renderer, used for all unsupported visible content. */
  renderPdfjs(viewport: ViewportRequest, context: CanvasRenderingContext2D): Promise<void>;
  resolveImage(key: string): CanvasImageSource | null;
}
export interface RenderFrame {
  readonly mode: "experimental" | "pdfjs-fallback";
  readonly page_index: number;
  readonly rendered_nodes: readonly string[];
  readonly screen_targets?: readonly { readonly render_node: string; readonly viewport_bounds: PdfBox }[];
  readonly fallback: readonly { readonly runtime_id: string; readonly reason: string }[];
  readonly canvas: HTMLCanvasElement;
}
interface IndexedPage { ir: RenderPageIR; index: SpatialIndex }
const success = <T>(value: T): ContractResult<T> => ({ ok: true, value });
const refuse = (reason: string): ContractResult<never> => ({ ok: false, refusal: { code: "UnsupportedOperation", reason } });

/** Opt-in adapter. No production page component imports it. PDF.js remains the oracle and fallback. */
export class ExperimentalRenderer implements Renderer<RenderDocument, RenderFrame> {
  private document: RenderDocument | null = null;
  private state: RendererDocumentState | null = null;
  private readonly pages = new Map<number, IndexedPage>();
  private readonly dirty = new Map<number, PageBounds[]>();
  private readonly frames = new Map<string, RenderFrame>();
  async open(document: RenderDocument, state: RendererDocumentState): Promise<ContractResult<void>> {
    this.document = document; this.state = state; this.pages.clear(); this.dirty.clear(); this.frames.clear();
    return success(undefined);
  }
  currentState(): RendererDocumentState | null { return this.state; }
  private async page(index: number): Promise<IndexedPage> {
    const cached = this.pages.get(index);
    if (cached) return cached;
    if (!this.document) throw new Error("Renderer is not open");
    const ir = await this.document.loadPage(index);
    validatePageIR(ir);
    if (ir.page_index !== index) throw new Error("Page index mismatch");
    const page = { ir, index: new SpatialIndex(ir.primitives) };
    this.pages.set(index, page);
    return page;
  }
  async update(state: RendererDocumentState, delta: DocumentDelta): Promise<ContractResult<void>> {
    if (!this.document || !this.state || state.document_id !== this.state.document_id) return refuse("renderer document mismatch");
    const before = [...this.pages.values()].map(v => v.ir);
    const changed = new Set(delta.affected_pages);
    for (const [id, change] of delta.semantic_changes) {
      if (change.geometry) { changed.add(change.geometry.before.page_index); changed.add(change.geometry.after.page_index); }
      for (const page of before) if (page.primitives.some(node => node.star_node === id)) changed.add(page.page_index);
    }
    for (const page of before) if (page.primitives.some(node => [...delta.modified_objects, ...delta.added_objects].some(o =>
      o.number === node.origin.object.number && o.generation === node.origin.object.generation))) changed.add(page.page_index);
    for (const index of changed) this.pages.delete(index);
    this.state = state;
    const after: RenderPageIR[] = [];
    for (const index of changed) after.push((await this.page(index)).ir);
    this.invalidate(affectedVisualBounds(delta, before, after));
    return success(undefined);
  }
  invalidatedBounds(pageIndex: number): readonly PageBounds[] { return this.dirty.get(pageIndex) ?? []; }
  invalidate(bounds: readonly PageBounds[]): ContractResult<void> {
    for (const bound of bounds) this.dirty.set(bound.page_index, [...this.invalidatedBounds(bound.page_index), bound]);
    return success(undefined);
  }
  hitTest(pageIndex: number, pdfPoint: Point): ContractResult<readonly RenderHit[]> {
    const page = this.pages.get(pageIndex);
    if (!page) return refuse("page has not been loaded");
    return success(page.index.point(pdfPoint).filter(node => node.kind !== "ClipNode" && node.kind !== "FormInstance")
      .map(node => ({ render_node: node.id, star_node: renderStarNode(node.id), bounds: node.bounds })));
  }
  async renderViewport(viewport: ViewportRequest): Promise<ContractResult<RenderFrame>> {
    if (!this.document) return refuse("renderer is not open");
    const page = await this.page(viewport.page_index);
    const [left, top, right, bottom] = viewport.viewport_bounds;
    const corners = [[left, top], [right, top], [left, bottom], [right, bottom]] as const;
    const pdfPoints = corners.map(p => viewport.transform.viewportToPdf(p));
    const query: PdfBox = [Math.min(...pdfPoints.map(p => p[0])), Math.min(...pdfPoints.map(p => p[1])),
      Math.max(...pdfPoints.map(p => p[0])), Math.max(...pdfPoints.map(p => p[1]))];
    const visible = page.index.query(query).reverse();
    const fallback = page.ir.fallback.filter(region => intersects(region.bounds.rect, query));
    const unresolvedImages = visible.filter((node): node is ImageInstance => node.kind === "ImageInstance" && !this.document!.resolveImage(node.image_key));
    const unsupportedForms = visible.filter(node => node.kind === "FormInstance");
    const signature = [viewport.page_index, ...viewport.viewport_bounds,
      ...viewport.transform.pdfToViewport([0, 0]), ...viewport.transform.pdfToViewport([1, 0]),
      ...viewport.transform.pdfToViewport([0, 1])].join(":");
    const cached = this.frames.get(signature);
    const dirty = this.invalidatedBounds(viewport.page_index).filter(bound => intersects(bound.rect, query));
    if (cached && !dirty.length) return success(cached);
    const canvas = cached?.canvas ?? document.createElement("canvas");
    if (!cached) { canvas.width = Math.ceil(right - left); canvas.height = Math.ceil(bottom - top); }
    const ctx = canvas.getContext("2d");
    if (!ctx) return refuse("2D canvas unavailable");
    if (fallback.length || unresolvedImages.length || unsupportedForms.length) {
      await this.document.renderPdfjs(viewport, ctx);
      const frame: RenderFrame = { mode: "pdfjs-fallback", page_index: viewport.page_index, rendered_nodes: [], screen_targets: [], canvas,
        fallback: [...fallback.map(v => ({ runtime_id: v.runtime_id, reason: v.reason })),
          ...unresolvedImages.map(v => ({ runtime_id: v.origin.runtime_id, reason: "image pixels unavailable" })),
          ...unsupportedForms.map(v => ({ runtime_id: v.origin.runtime_id, reason: "form execution unsupported" }))] };
      for (const [key, other] of this.frames) if (key !== signature && other.page_index === viewport.page_index) this.frames.delete(key);
      this.frames.set(signature, frame); this.dirty.delete(viewport.page_index);
      return success(frame);
    }
    const partial = cached?.mode === "experimental" && dirty.length > 0;
    const redraw = partial ? visible.filter(node => dirty.some(bound => intersects(node.bounds.rect, bound.rect))) : visible;
    if (partial) {
      ctx.save(); ctx.beginPath();
      for (const bound of dirty) {
        const rect = viewport.transform.pdfBoxToViewport(bound.rect);
        ctx.rect(rect[0] - left, rect[1] - top, rect[2] - rect[0], rect[3] - rect[1]);
      }
      ctx.clip();
    }
    ctx.fillStyle = "white"; ctx.fillRect(0, 0, canvas.width, canvas.height);
    const clips = new Map(page.ir.primitives.filter(node => node.kind === "ClipNode").map(node => [node.id, node]));
    for (const node of redraw) draw(ctx, node, viewport, this.document, clips);
    if (partial) ctx.restore();
    this.dirty.delete(viewport.page_index);
    const frame: RenderFrame = { mode: "experimental", page_index: viewport.page_index,
      rendered_nodes: redraw.map(v => v.id), screen_targets: visible.map(node => ({ render_node: node.id,
        viewport_bounds: viewport.transform.pdfBoxToViewport(node.bounds.rect) })), fallback: [], canvas };
    for (const [key, other] of this.frames) if (key !== signature && other.page_index === viewport.page_index) this.frames.delete(key);
    this.frames.set(signature, frame);
    return success(frame);
  }
}
function css(color: readonly number[]): string { return `rgb(${color.map(v => Math.round(v * 255)).join(",")})`; }
function draw(ctx: CanvasRenderingContext2D, node: RenderPrimitive, viewport: ViewportRequest, doc: RenderDocument,
  clips: ReadonlyMap<string, RenderPrimitive>): void {
  const transform = viewport.transform;
  const [left, top] = viewport.viewport_bounds;
  const point = (p: Point): Point => { const q = transform.pdfToViewport(p); return [q[0] - left, q[1] - top]; };
  ctx.save();
  const clip = node.clip && clips.get(node.clip);
  if (clip?.kind === "ClipNode") {
    const rect = transform.pdfBoxToViewport(clip.rect);
    ctx.beginPath(); ctx.rect(rect[0] - left, rect[1] - top, rect[2] - rect[0], rect[3] - rect[1]); ctx.clip();
  }
  if (node.kind === "GlyphRun") {
    const [x, y] = point(node.baseline);
    const px = point([node.baseline[0] + 1, node.baseline[1]]);
    const py = point([node.baseline[0], node.baseline[1] + 1]);
    ctx.setTransform(px[0] - x, px[1] - y, x - py[0], y - py[1], x, y);
    ctx.fillStyle = css(node.color); ctx.font = `${node.size}px ${node.font}`; ctx.fillText(node.text, 0, 0);
  } else if (node.kind === "ImageInstance") {
    const image = doc.resolveImage(node.image_key)!;
    const rect = transform.pdfBoxToViewport(node.bounds.rect);
    ctx.drawImage(image, rect[0] - left, rect[1] - top, rect[2] - rect[0], rect[3] - rect[1]);
  } else if (node.kind === "PathInstance") {
    ctx.beginPath();
    if (node.geometry.kind === "rect") {
      const r = transform.pdfBoxToViewport(node.geometry.rect);
      ctx.rect(r[0] - left, r[1] - top, r[2] - r[0], r[3] - r[1]);
    } else {
      const a = point(node.geometry.from), b = point(node.geometry.to);
      ctx.moveTo(...a); ctx.lineTo(...b);
    }
    if (node.fill) { ctx.fillStyle = css(node.fill); ctx.fill(); }
    if (node.stroke) {
      const a = point([0, 0]), b = point([1, 0]);
      ctx.strokeStyle = css(node.stroke); ctx.lineWidth = node.line_width * Math.hypot(b[0] - a[0], b[1] - a[1]); ctx.stroke();
    }
  }
  ctx.restore();
}
