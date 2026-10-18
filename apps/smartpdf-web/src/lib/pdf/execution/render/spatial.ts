import type { PdfBox, Point } from "../contracts";
import { intersects, union, type RenderPrimitive } from "./ir";

interface Branch { readonly bounds: PdfBox; readonly left?: Branch; readonly right?: Branch; readonly items?: readonly RenderPrimitive[] }
const byPaint = (a: RenderPrimitive, b: RenderPrimitive) => b.z - a.z || b.id.localeCompare(a.id);
/** Packed bounding-volume tree. Rebuilt only on changed pages; queries prune disjoint subtrees. */
export class SpatialIndex {
  private readonly root?: Branch;
  constructor(items: readonly RenderPrimitive[]) { this.root = build([...items]); }
  query(bounds: PdfBox): RenderPrimitive[] {
    const out: RenderPrimitive[] = [];
    const visit = (branch?: Branch) => {
      if (!branch || !intersects(branch.bounds, bounds)) return;
      if (branch.items) { for (const item of branch.items) if (intersects(item.bounds.rect, bounds)) out.push(item); }
      else { visit(branch.left); visit(branch.right); }
    };
    visit(this.root);
    return out.sort(byPaint);
  }
  point(point: Point): RenderPrimitive[] { return this.query([point[0], point[1], point[0], point[1]]); }
}
function build(items: RenderPrimitive[]): Branch | undefined {
  if (!items.length) return undefined;
  const bounds = items.reduce<PdfBox>((b, item) => union(b, item.bounds.rect), items[0].bounds.rect);
  if (items.length <= 8) return { bounds, items };
  const axis = bounds[2] - bounds[0] >= bounds[3] - bounds[1] ? 0 : 1;
  items.sort((a, b) => (a.bounds.rect[axis] + a.bounds.rect[axis + 2]) -
    (b.bounds.rect[axis] + b.bounds.rect[axis + 2]) || a.id.localeCompare(b.id));
  const middle = Math.floor(items.length / 2);
  return { bounds, left: build(items.slice(0, middle)), right: build(items.slice(middle)) };
}
