import type { DocumentDelta, PageBounds, PdfBox, PdfObjectRef, SourceLocation } from "../contracts";
import type { StarPdfAnnotation, StarPdfImageInfo, StarPdfTextSpan, StarPdfVectorGraphicInfo } from "../../starpdf-types";
import { makeNodeId, validBox, type Color, type FallbackRegion, type RenderPageIR, type RenderPrimitive, type Origin } from "./ir";

export interface RuntimePage {
  readonly page_index: number;
  readonly page_bounds: PdfBox;
  /** Complete means an upstream scanner has accounted for every visible paint operation. */
  readonly coverage: "complete" | "unknown";
  readonly text: readonly StarPdfTextSpan[];
  readonly images: readonly StarPdfImageInfo[];
  readonly graphics: readonly StarPdfVectorGraphicInfo[];
  readonly annotations: readonly StarPdfAnnotation[];
}
/** Caller must resolve the actual content-stream owner. Stream index alone is not a PDF object identity. */
export interface ProvenanceResolver {
  resolve(kind: "text" | "image" | "graphic", runtimeId: string, pageIndex: number, streamIndex: number):
    { readonly object: PdfObjectRef; readonly source: SourceLocation } | null;
}
export interface RuntimeAdapterInput {
  readonly document_id: string;
  readonly page: RuntimePage;
  readonly provenance: ProvenanceResolver;
  readonly image_keys?: ReadonlyMap<string, string>;
}
const rgb = (value?: readonly number[]): Color | null => value && value.length === 3 && value.every(v => Number.isFinite(v) && v >= 0 && v <= 1)
  ? [value[0], value[1], value[2]] : null;
const bounds = (page_index: number, rect: PdfBox): PageBounds => ({ page_index, rect });
export function adaptRuntimePage({ document_id, page, provenance, image_keys }: RuntimeAdapterInput): RenderPageIR {
  const primitives: RenderPrimitive[] = [], fallback: FallbackRegion[] = [];
  const paintOrder = new Map<string, readonly [number, number]>();
  let z = 0;
  const reject = (rect: PdfBox, runtime_id: string, reason: string) => fallback.push({ bounds: bounds(page.page_index, rect), runtime_id, reason });
  const source = (kind: "text" | "image" | "graphic", id: string, stream: number, occurrence: string): Origin | null => {
    const link = provenance.resolve(kind, id, page.page_index, stream);
    return link ? { ...link, occurrence, runtime_id: id } : null;
  };
  for (const span of page.text) {
    const rect: PdfBox = [span.x, span.y, span.x + span.width, span.y + span.height];
    const origin = source("text", span.span_id, span.stream_index, `page:${page.page_index}/text:${span.span_id}`);
    const color = rgb(span.fill_color) ?? [0, 0, 0];
    if (!validBox(rect) || !origin || !/^[\u0009\u000a\u000d\u0020-\u024f]*$/.test(span.text) ||
        span.rotation !== 0 || !Number.isFinite(span.font_size) || span.font_size <= 0 ||
        span.editability_code === "UNSUPPORTED_COMPLEX_SCRIPT" || span.editability_code === "UNSUPPORTED_VERTICAL_WRITING") {
      reject(rect, span.span_id, !origin ? "missing source object" : "unsupported text shaping or geometry"); continue;
    }
    const ids = makeNodeId(document_id, origin, "glyph:0");
    primitives.push({ ...ids, origin, kind: "GlyphRun", bounds: bounds(page.page_index, rect), z: z++, text: span.text,
      baseline: [span.x, span.y], font: span.font_family ?? "sans-serif", size: span.font_size, color,
      transform: [1, 0, 0, 1, 0, 0] });
    paintOrder.set(ids.id, [span.stream_index, span.instruction_index]);
  }
  for (const image of page.images) {
    const rect = image.rect as PdfBox;
    const origin = source("image", image.image_id, image.stream_index, `page:${page.page_index}/image:${image.image_id}`);
    const image_key = image_keys?.get(image.image_id);
    if (!validBox(rect) || !origin || !image_key || image.is_nested_form || image.transform[1] !== 0 || image.transform[2] !== 0) {
      reject(rect, image.image_id, !origin ? "missing source object" : !image_key ? "image pixels unavailable" : "nested form image"); continue;
    }
    const ids = makeNodeId(document_id, origin, "image:0");
    primitives.push({ ...ids, origin, kind: "ImageInstance", bounds: bounds(page.page_index, rect), z: z++, image_key,
      transform: image.transform });
    paintOrder.set(ids.id, [image.stream_index, image.instruction_index]);
  }
  for (const graphic of page.graphics) {
    const rect = graphic.bounds as PdfBox;
    const origin = source("graphic", graphic.graphic_id, graphic.stream_index, `page:${page.page_index}/graphic:${graphic.graphic_id}`);
    const fill = graphic.is_filled ? rgb(graphic.fill_color_rgb) : null;
    const stroke = graphic.is_stroked ? rgb(graphic.stroke_color_rgb) : null;
    const geometry = graphic.graphic_type === "Rectangle" && graphic.rect_x !== undefined && graphic.rect_y !== undefined && graphic.rect_w !== undefined && graphic.rect_h !== undefined
      ? { kind: "rect" as const, rect: [graphic.rect_x, graphic.rect_y, graphic.rect_x + graphic.rect_w, graphic.rect_y + graphic.rect_h] as PdfBox }
      : graphic.graphic_type === "Line" && graphic.line_x1 !== undefined && graphic.line_y1 !== undefined && graphic.line_x2 !== undefined && graphic.line_y2 !== undefined
        ? { kind: "line" as const, from: [graphic.line_x1, graphic.line_y1] as const, to: [graphic.line_x2, graphic.line_y2] as const } : null;
    if (!validBox(rect) || !origin || !geometry || graphic.transform.some((v, i) => v !== [1, 0, 0, 1, 0, 0][i]) ||
        (graphic.is_filled && !fill) || (graphic.is_stroked && !stroke) || !Number.isFinite(graphic.line_width)) {
      reject(rect, graphic.graphic_id, !origin ? "missing source object" : "unsupported path or paint"); continue;
    }
    const ids = makeNodeId(document_id, origin, "path:0");
    primitives.push({ ...ids, origin, kind: "PathInstance", bounds: bounds(page.page_index, rect), z: z++, geometry, fill, stroke,
      line_width: graphic.line_width });
    paintOrder.set(ids.id, [graphic.stream_index, graphic.start_instruction_index]);
  }
  for (const annotation of page.annotations) {
    // AP streams may contain arbitrary PDF graphics. Keep PDF.js authoritative until AP parsing exists.
    reject(annotation.rect as PdfBox, `annot:${annotation.object_num}:${annotation.object_gen}`, `annotation AP ${annotation.subtype} unsupported`);
  }
  if (page.coverage !== "complete") reject(page.page_bounds, `page:${page.page_index}`, "paint coverage unverified");
  const sorted = primitives.sort((a, b) => {
    const x = paintOrder.get(a.id)!, y = paintOrder.get(b.id)!;
    return x[0] - y[0] || x[1] - y[1] || a.id.localeCompare(b.id);
  }).map((node, index) => ({ ...node, z: index }));
  return { page_index: page.page_index, primitives: sorted, fallback };
}

export function affectedVisualBounds(delta: DocumentDelta, before: readonly RenderPageIR[], after: readonly RenderPageIR[]): PageBounds[] {
  const changed = new Set(delta.semantic_changes.map(([id]) => id));
  const objectKeys = new Set([...delta.modified_objects, ...delta.added_objects].map(o => `${o.number}:${o.generation}`));
  const out: PageBounds[] = [];
  for (const [id, change] of delta.semantic_changes) {
    if (change.geometry) out.push(change.geometry.before, change.geometry.after);
    if (change.other.some(v => /annotation|appearance|AP|image|FreeText/i.test(v))) changed.add(id);
  }
  for (const page of [...before, ...after]) {
    for (const node of page.primitives) if (changed.has(node.star_node) || objectKeys.has(`${node.origin.object.number}:${node.origin.object.generation}`)) out.push(node.bounds);
    if (delta.affected_pages.includes(page.page_index)) for (const region of page.fallback) out.push(region.bounds);
  }
  for (const index of delta.affected_pages) if (!out.some(bound => bound.page_index === index)) {
    for (const page of [...before, ...after]) if (page.page_index === index)
      for (const node of page.primitives) out.push(node.bounds);
  }
  return [...new Map(out.map(b => [`${b.page_index}:${b.rect.join(",")}`, b])).values()];
}
