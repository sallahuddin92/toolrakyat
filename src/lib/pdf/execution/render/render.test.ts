import { describe, expect, it, vi } from "vitest";
import { createCoordinateTransform } from "../coordinates";
import { createRenderNodeId, createStarNodeId } from "../identity";
import type { DocumentDelta, PdfBox } from "../contracts";
import type { StarPdfImageInfo, StarPdfTextSpan, StarPdfVectorGraphicInfo } from "../../starpdf-types";
import { adaptRuntimePage, affectedVisualBounds, type RuntimePage } from "./adapter";
import { compareWithPdfjs } from "./differential";
import { type RenderPageIR, type PathInstance } from "./ir";
import { ExperimentalRenderer } from "./renderer";
import { SpatialIndex } from "./spatial";

const object = { number: "12", generation: 0 };
const path = (id: number, rect: PdfBox, z = id): PathInstance => {
  const star_node = createStarNodeId("doc", object, `page:0/path:${id}`);
  return { id: createRenderNodeId(star_node, "path:0"), star_node, origin: { object, source: { kind: "FileOffset", offset: "42" },
    occurrence: `page:0/path:${id}`, runtime_id: `vec-${id}` }, kind: "PathInstance", bounds: { page_index: 0, rect }, z,
    geometry: { kind: "rect", rect }, fill: [1, 0, 0], stroke: null, line_width: 0 };
};
const empty = (): DocumentDelta => ({ added_objects: [], modified_objects: [], removed_references: [], source_range_edits: [],
  resource_changes: [], semantic_changes: [], affected_pages: [], before_hashes: [], after_hashes: [] });
const transform = (rotation: 0 | 90 | 180 | 270, scale = 1) => createCoordinateTransform({ media_box: [0, 0, 100, 100],
  rotation, scale, user_unit: 1, device_pixel_ratio: 1 });
function fakeCanvas(pixels = new Uint8ClampedArray(16)) {
  const context = { save: vi.fn(), restore: vi.fn(), fillRect: vi.fn(), fill: vi.fn(), stroke: vi.fn(), beginPath: vi.fn(), rect: vi.fn(),
    clip: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), fillText: vi.fn(), setTransform: vi.fn(), drawImage: vi.fn(),
    getImageData: () => ({ data: pixels }) };
  return { width: 2, height: 2, getContext: () => context, context };
}

describe("experimental render execution", () => {
  it("produces deterministic identities and refuses unresolved provenance", () => {
    const span = { page_index: 0, span_id: "p0_s0_i1_o0", stream_index: 0, x: 2, y: 3, width: 8, height: 9,
      text: "Hello", rotation: 0, font_size: 12, font_family: "sans-serif", editability_code: "EDITABLE_NATIVE_TEXT" } as StarPdfTextSpan;
    const page: RuntimePage = { page_index: 0, page_bounds: [0, 0, 100, 100], coverage: "complete",
      text: [span], images: [], graphics: [], annotations: [] };
    const provenance = { resolve: () => ({ object, source: { kind: "FileOffset" as const, offset: "42" } }) };
    const a = adaptRuntimePage({ document_id: "doc", page, provenance });
    expect(adaptRuntimePage({ document_id: "doc", page, provenance })).toEqual(a);
    expect(a.primitives[0].star_node).toContain("sn1/");
    expect(a.primitives[0].origin.object).toEqual(object);
    expect(adaptRuntimePage({ document_id: "doc", page, provenance: { resolve: () => null } }).fallback[0].reason)
      .toBe("missing source object");
    expect(adaptRuntimePage({ document_id: "doc", page: { ...page, coverage: "unknown" }, provenance }).fallback[0].reason)
      .toBe("paint coverage unverified");
  });
  it("queries viewport and pointer candidates in topmost z order", () => {
    const index = new SpatialIndex([path(1, [0, 0, 10, 10]), path(2, [5, 5, 15, 15]), path(3, [90, 90, 100, 100])]);
    expect(index.point([6, 6]).map(n => n.z)).toEqual([2, 1]);
    expect(index.query([89, 89, 101, 101]).map(n => n.z)).toEqual([3]);
  });
  it("adapts images and simple lines while rejecting unavailable pixels", () => {
    const image = { image_id: "img", page_index: 0, stream_index: 0, instruction_index: 2,
      rect: [2, 2, 12, 12], transform: [10, 0, 0, 10, 2, 2], is_nested_form: false } as StarPdfImageInfo;
    const line = { graphic_id: "line", page_index: 0, stream_index: 0, start_instruction_index: 1,
      graphic_type: "Line", bounds: [0, 0, 10, 10], transform: [1, 0, 0, 1, 0, 0],
      line_x1: 0, line_y1: 0, line_x2: 10, line_y2: 10, line_width: 1,
      is_stroked: true, is_filled: false, stroke_color_rgb: [0, 0, 0] } as StarPdfVectorGraphicInfo;
    const page: RuntimePage = { page_index: 0, page_bounds: [0, 0, 100, 100], coverage: "complete",
      text: [], images: [image], graphics: [line], annotations: [] };
    const provenance = { resolve: () => ({ object, source: { kind: "FileOffset" as const, offset: "42" } }) };
    const result = adaptRuntimePage({ document_id: "doc", page, provenance, image_keys: new Map([["img", "pixels"]]) });
    expect(result.primitives.map(v => v.kind)).toEqual(["PathInstance", "ImageInstance"]);
    expect(result.fallback).toEqual([]);
    expect(adaptRuntimePage({ document_id: "doc", page, provenance }).fallback[0].reason).toBe("image pixels unavailable");
  });
  it("computes old and new bounds for text style, FreeText, image move/delete and AP changes", () => {
    const before: RenderPageIR = { page_index: 0, primitives: [path(1, [1, 1, 5, 5])], fallback: [
      { bounds: { page_index: 0, rect: [20, 20, 30, 30] }, runtime_id: "annot:7:0", reason: "AP unsupported" }] };
    const after: RenderPageIR = { page_index: 0, primitives: [path(1, [2, 2, 6, 6])], fallback: before.fallback };
    const delta: DocumentDelta = { ...empty(), affected_pages: [0], semantic_changes: [[before.primitives[0].star_node, {
      text: { before: "a", after: "b" }, font: null, geometry: { before: before.primitives[0].bounds, after: after.primitives[0].bounds },
      other: ["FreeText", "image move", "annotation AP"] }]] };
    expect(affectedVisualBounds(delta, [before], [after]).map(b => b.rect)).toEqual([[1, 1, 5, 5], [2, 2, 6, 6], [20, 20, 30, 30]]);
    expect(affectedVisualBounds({ ...empty(), modified_objects: [object] }, [before], []).map(b => b.rect)).toEqual([[1, 1, 5, 5]]);
  });
  it("uses PDF.js for unsupported visible content and preserves hit identity", async () => {
    const canvas = fakeCanvas();
    vi.stubGlobal("document", { createElement: () => canvas });
    const renderPdfjs = vi.fn(async () => {});
    const ir: RenderPageIR = { page_index: 0, primitives: [path(1, [0, 0, 10, 10]), path(2, [5, 5, 15, 15])], fallback: [
      { bounds: { page_index: 0, rect: [4, 4, 8, 8] }, runtime_id: "annot:1", reason: "AP unsupported" }] };
    const renderer = new ExperimentalRenderer();
    await renderer.open({ loadPage: async () => ir, renderPdfjs, resolveImage: () => null }, { document_id: "doc", revision: "1" });
    const request = { page_index: 0, transform: transform(0), viewport_bounds: [0, 0, 20, 100] as PdfBox };
    const frame = await renderer.renderViewport(request);
    expect(frame.ok && frame.value.mode).toBe("pdfjs-fallback");
    expect(renderPdfjs).toHaveBeenCalledOnce();
    const hits = renderer.hitTest(0, [6, 6]);
    expect(hits.ok && hits.value.map(h => h.star_node)).toEqual([ir.primitives[1].star_node, ir.primitives[0].star_node]);
    vi.unstubAllGlobals();
  });
  it("draws supported primitives at zoom and all page rotations", async () => {
    const canvas = fakeCanvas();
    vi.stubGlobal("document", { createElement: () => canvas });
    const ir: RenderPageIR = { page_index: 0, primitives: [path(1, [1, 2, 10, 12])], fallback: [] };
    const renderer = new ExperimentalRenderer();
    await renderer.open({ loadPage: async () => ir, renderPdfjs: vi.fn(), resolveImage: () => null }, { document_id: "doc", revision: "1" });
    for (const rotation of [0, 90, 180, 270] as const) {
      const result = await renderer.renderViewport({ page_index: 0, transform: transform(rotation, 2), viewport_bounds: [0, 0, 200, 200] });
      expect(result.ok && result.value.mode).toBe("experimental");
    }
    expect(canvas.context.rect).toHaveBeenCalledTimes(4);
    vi.unstubAllGlobals();
  });
  it("redraws only delta bounds on a cached experimental frame", async () => {
    const canvas = fakeCanvas();
    vi.stubGlobal("document", { createElement: () => canvas });
    let ir: RenderPageIR = { page_index: 0, primitives: [path(1, [1, 1, 5, 5]), path(2, [80, 80, 90, 90])], fallback: [] };
    const renderer = new ExperimentalRenderer();
    await renderer.open({ loadPage: async () => ir, renderPdfjs: vi.fn(), resolveImage: () => null },
      { document_id: "doc", revision: "1" });
    const request = { page_index: 0, transform: transform(0), viewport_bounds: [0, 0, 100, 100] as PdfBox };
    const first = await renderer.renderViewport(request);
    expect(first.ok && first.value.rendered_nodes).toHaveLength(2);
    const old = ir.primitives[0].bounds;
    ir = { ...ir, primitives: [path(1, [2, 2, 6, 6]), ir.primitives[1]] };
    const next = ir.primitives[0].bounds;
    await renderer.update({ document_id: "doc", revision: "2" }, { ...empty(), affected_pages: [0],
      semantic_changes: [[ir.primitives[0].star_node, { text: null, font: null, geometry: { before: old, after: next }, other: [] }]] });
    const second = await renderer.renderViewport(request);
    expect(second.ok && second.value.rendered_nodes).toEqual([ir.primitives[0].id]);
    expect(renderer.invalidatedBounds(0)).toEqual([]);
    vi.unstubAllGlobals();
  });
  it("records pixel divergence and bounds instead of hiding it", () => {
    const a = fakeCanvas(new Uint8ClampedArray(16));
    const b = fakeCanvas(new Uint8ClampedArray([0, 0, 255, 0, ...Array(12).fill(0)]));
    const ir: RenderPageIR = { page_index: 0, primitives: [path(1, [0, 0, 1, 1])], fallback: [] };
    const record = compareWithPdfjs({ page_index: 0, mode: "experimental", rendered_nodes: [ir.primitives[0].id], fallback: [],
      canvas: a as unknown as HTMLCanvasElement }, b as unknown as HTMLCanvasElement, ir, rect => rect);
    expect(record.pixel_delta).toEqual({ differing_pixels: 1, max_channel_delta: 255, mean_channel_delta: 255 / 16 });
    expect(record.bounds_delta[0].pdf_bounds).toEqual([0, 0, 1, 1]);
  });
});
