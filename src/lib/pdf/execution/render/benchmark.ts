/** Run with `npx tsx src/lib/pdf/execution/render/benchmark.ts`. */
import { performance } from "node:perf_hooks";
import { createRenderNodeId, createStarNodeId } from "../identity";
import type { PdfBox } from "../contracts";
import { intersects, type PathInstance } from "./ir";
import { SpatialIndex } from "./spatial";

const items: PathInstance[] = [];
for (let i = 0; i < 10_000; i++) {
  const x = (i % 100) * 12, y = Math.floor(i / 100) * 12;
  const rect: PdfBox = [x, y, x + 8, y + 8];
  const object = { number: String(i + 1), generation: 0 };
  const star_node = createStarNodeId("benchmark", object, `page:0/path:${i}`);
  items.push({ id: createRenderNodeId(star_node, "path:0"), star_node,
    origin: { object, source: { kind: "Generated" }, occurrence: `page:0/path:${i}`, runtime_id: String(i) },
    kind: "PathInstance", bounds: { page_index: 0, rect }, z: i, geometry: { kind: "rect", rect }, fill: null, stroke: null, line_width: 0 });
}
const queries: PdfBox[] = Array.from({ length: 5000 }, (_, i) => {
  const x = (i * 37 % 1200), y = (i * 73 % 1200); return [x, y, x + (i % 2 ? 1 : 120), y + (i % 2 ? 1 : 120)];
});
const grid = new Map<string, PathInstance[]>();
const cell = 48;
for (const item of items) {
  const b = item.bounds.rect;
  for (let x = Math.floor(b[0] / cell); x <= Math.floor(b[2] / cell); x++)
    for (let y = Math.floor(b[1] / cell); y <= Math.floor(b[3] / cell); y++) {
      const key = `${x}:${y}`; const bin = grid.get(key) ?? []; bin.push(item); grid.set(key, bin);
    }
}
const queryGrid = (b: PdfBox) => {
  const found = new Set<PathInstance>();
  for (let x = Math.floor(b[0] / cell); x <= Math.floor(b[2] / cell); x++)
    for (let y = Math.floor(b[1] / cell); y <= Math.floor(b[3] / cell); y++)
      for (const item of grid.get(`${x}:${y}`) ?? []) if (intersects(item.bounds.rect, b)) found.add(item);
  return found.size;
};
const tree = new SpatialIndex(items);
const candidates = {
  linear: (b: PdfBox) => items.filter(v => intersects(v.bounds.rect, b)).length,
  fixed_grid: queryGrid,
  packed_bvh: (b: PdfBox) => tree.query(b).length,
};
for (const [name, query] of Object.entries(candidates)) {
  let count = 0;
  const start = performance.now();
  for (const bounds of queries) count += query(bounds);
  console.log(`${name}: ${(performance.now() - start).toFixed(1)} ms, ${count} hits`);
}
