import type { PageBounds, PdfBox, PdfObjectRef, RenderNodeId, SourceLocation, StarNodeId } from "../contracts";
import { createRenderNodeId, createStarNodeId, renderStarNode } from "../identity";

export type Matrix = readonly [number, number, number, number, number, number];
export type Color = readonly [number, number, number];
export interface Origin {
  readonly object: PdfObjectRef;
  readonly source: SourceLocation;
  readonly occurrence: string;
  readonly runtime_id: string;
}
interface BaseNode {
  readonly id: RenderNodeId;
  readonly star_node: StarNodeId;
  readonly origin: Origin;
  readonly bounds: PageBounds;
  readonly z: number;
  readonly clip?: RenderNodeId;
}
export interface GlyphRun extends BaseNode {
  readonly kind: "GlyphRun";
  readonly text: string;
  readonly baseline: readonly [number, number];
  readonly font: string;
  readonly size: number;
  readonly color: Color;
  readonly transform: Matrix;
}
export interface ImageInstance extends BaseNode {
  readonly kind: "ImageInstance";
  readonly image_key: string;
  readonly transform: Matrix;
}
export interface PathInstance extends BaseNode {
  readonly kind: "PathInstance";
  readonly geometry: { readonly kind: "rect"; readonly rect: PdfBox } |
    { readonly kind: "line"; readonly from: readonly [number, number]; readonly to: readonly [number, number] };
  readonly fill: Color | null;
  readonly stroke: Color | null;
  readonly line_width: number;
}
export interface ClipNode extends BaseNode {
  readonly kind: "ClipNode";
  readonly rect: PdfBox;
}
export interface FormInstance extends BaseNode {
  readonly kind: "FormInstance";
  readonly children: readonly RenderNodeId[];
  readonly transform: Matrix;
}
export type RenderPrimitive = GlyphRun | ImageInstance | PathInstance | ClipNode | FormInstance;
export interface FallbackRegion { readonly bounds: PageBounds; readonly reason: string; readonly runtime_id: string }
export interface RenderPageIR {
  readonly page_index: number;
  readonly primitives: readonly RenderPrimitive[];
  readonly fallback: readonly FallbackRegion[];
}
export function makeNodeId(document: string, origin: Origin, primitive: string): { star_node: StarNodeId; id: RenderNodeId } {
  const star_node = createStarNodeId(document, origin.object, origin.occurrence);
  return { star_node, id: createRenderNodeId(star_node, primitive) };
}
export function validatePageIR(page: RenderPageIR): void {
  const ids = new Set<string>();
  for (const node of page.primitives) {
    if (ids.has(node.id) || renderStarNode(node.id) !== node.star_node || node.bounds.page_index !== page.page_index ||
        !validBox(node.bounds.rect) || !Number.isInteger(node.z)) throw new Error("Invalid render IR identity, bounds or z order");
    ids.add(node.id);
  }
  for (const node of page.primitives) if (node.clip && !ids.has(node.clip)) throw new Error("Missing clip node");
}
export function validBox(b: PdfBox): boolean { return b.every(Number.isFinite) && b[0] <= b[2] && b[1] <= b[3]; }
export function intersects(a: PdfBox, b: PdfBox): boolean {
  return a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1];
}
export function union(a: PdfBox, b: PdfBox): PdfBox {
  return [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(a[2], b[2]), Math.max(a[3], b[3])];
}
