import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import type { ContractResult, CoordinateConfig, DocumentDelta, EvidenceRecord, PageBounds, Renderer,
  RendererDocumentState, ViewportRequest } from "./contracts";
import { createRenderNodeId, createStarNodeId, parseRenderNodeId, parseStarNodeId, renderStarNode, starNodeParts } from "./identity";
import { canonicalHashBytes, semanticHash, semanticNumberBytes } from "./hash";
import { createCoordinateTransform } from "./coordinates";
import { checkEvidence } from "./evidence";

const vector: { document: string; object_number: string; generation: number; occurrence: string; primitive: string;
  star_node: string; render_node: string; hash_fields: Record<string, string>; canonical_hash_input: string; semantic_hash: string } =
  JSON.parse(readFileSync(resolve("engine/starpdf/tests/contracts_vectors.json"), "utf8"));
const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString("hex");
const hashFields = Object.fromEntries(Object.entries(vector.hash_fields).map(([name, value]) => [name, Uint8Array.from(Buffer.from(value, "hex"))]));
const sampleNode = () => createStarNodeId(vector.document, { number: vector.object_number, generation: vector.generation }, vector.occurrence);
const emptyDelta: DocumentDelta = {
  added_objects: [], modified_objects: [], removed_references: [], source_range_edits: [], resource_changes: [],
  semantic_changes: [], affected_pages: [], before_hashes: [], after_hashes: [],
};

describe("execution identities", () => {
  it("matches Rust vectors and preserves u64 object numbers as decimal strings", () => {
    const node = sampleNode();
    expect(node).toBe(vector.star_node);
    expect(starNodeParts(node)).toEqual({ document: vector.document,
      object: { number: vector.object_number, generation: vector.generation }, occurrence: vector.occurrence });
    expect(parseStarNodeId(JSON.parse(JSON.stringify(node)))).toBe(node);
  });
  it("distinguishes document, generation and reused-object occurrences", () => {
    const object = { number: "7", generation: 0 };
    const node = createStarNodeId("doc", object, "page:0/paint:0");
    expect(createStarNodeId("doc", object, "page:0/paint:1")).not.toBe(node);
    expect(createStarNodeId("other-doc", object, "page:0/paint:0")).not.toBe(node);
    expect(createStarNodeId("doc", { ...object, generation: 1 }, "page:0/paint:0")).not.toBe(node);
  });
  it("links multiple render primitives to one StarNode", () => {
    const node = sampleNode(), render = createRenderNodeId(node, vector.primitive);
    expect(render).toBe(vector.render_node);
    expect(renderStarNode(parseRenderNodeId(render))).toBe(node);
    expect(createRenderNodeId(node, "glyph:1")).not.toBe(render);
  });
  it.each(["sn1/64/01/0/70", "sn1/64/1/+0/70", "sn1/FF/1/0/70", "sn1/ff/1/0/70",
    "sn1/64/18446744073709551616/0/70", "sn1/64/0/0/70", "sn1//1/0/70",
    "sn1/64/1/65536/70", "sn1/64/1/0/7", "sn1/64/1/0/70/extra"])("refuses invalid wire identity %s", (value) => {
    expect(() => parseStarNodeId(value)).toThrow("InvalidIdentity");
  });
  it("refuses malformed UTF-16 and empty render primitive keys", () => {
    expect(() => createStarNodeId("\ud800", { number: "1", generation: 0 }, "paint:0")).toThrow("InvalidIdentity");
    expect(() => createRenderNodeId(sampleNode(), "")).toThrow("InvalidIdentity");
  });
});

describe("operation-specific evidence", () => {
  const record: EvidenceRecord = { level: "RECOVERED_VERIFIED", proof: "local structure verified", source_revision: "2", provenance: null };
  it("accepts recovery only when the operation explicitly permits it", () => {
    expect(checkEvidence({ operation: "edit", accepted_levels: ["VERIFIED"], required_revision: "2" }, record).ok).toBe(false);
    expect(checkEvidence({ operation: "edit", accepted_levels: ["RECOVERED_VERIFIED"], required_revision: "2" }, record).ok).toBe(true);
  });
  it("refuses stale, missing and empty proof records", () => {
    const policy = { operation: "edit", accepted_levels: ["RECOVERED_VERIFIED"] as const, required_revision: "2" };
    expect(checkEvidence(policy, { ...record, source_revision: "1" })).toMatchObject({ ok: false, refusal: { code: "StaleRevision" } });
    expect(checkEvidence(policy, { ...record, source_revision: null })).toMatchObject({ ok: false, refusal: { code: "StaleRevision" } });
    expect(checkEvidence(policy, { ...record, proof: " " })).toMatchObject({ ok: false, refusal: { code: "InsufficientEvidence" } });
  });
});

describe("deterministic semantic hash profile", () => {
  it("matches the exact Rust binary input and SHA-256 vector", async () => {
    const input = { domain: "TextRun" as const, fields: hashFields };
    expect(hex(canonicalHashBytes(input))).toBe(vector.canonical_hash_input);
    expect(await semanticHash(input)).toBe(vector.semantic_hash);
    expect(await semanticHash({ ...input, fields: Object.fromEntries(Object.entries(hashFields).reverse()) })).toBe(vector.semantic_hash);
  });
  it("separates domains and changed semantic values", async () => {
    for (const domain of ["Font", "Image", "Annotation", "PdfObject"] as const) {
      expect(await semanticHash({ domain, fields: hashFields })).not.toBe(vector.semantic_hash);
    }
    expect(await semanticHash({ domain: "TextRun", fields: { ...hashFields, text: new TextEncoder().encode("changed") } })).not.toBe(vector.semantic_hash);
  });
  it("length-prefixes fields and encodes finite numbers consistently", async () => {
    expect(await semanticHash({ domain: "Font", fields: { a: new TextEncoder().encode("bc") } })).not.toBe(
      await semanticHash({ domain: "Font", fields: { ab: new TextEncoder().encode("c") } }));
    expect(hex(semanticNumberBytes(-0))).toBe("0000000000000000");
    expect(hex(semanticNumberBytes(1))).toBe("3ff0000000000000");
    expect(() => semanticNumberBytes(Infinity)).toThrow("InvalidHashInput");
    expect(() => canonicalHashBytes({ domain: "Font", fields: { 字体: new Uint8Array() } })).toThrow("InvalidHashInput");
  });
});

describe("shared coordinate convention", () => {
  it.each([
    [0, [20, 320]], [90, [80, 20]], [180, [180, 80]], [270, [320, 180]],
  ] as const)("maps rotation %s to a known anchor and back", (rotation, expected) => {
    const transform = createCoordinateTransform({ media_box: [0, 0, 150, 250], crop_box: [10, 20, 110, 220],
      rotation, scale: 1, user_unit: 2, device_pixel_ratio: 3 });
    expect(transform.pdfToViewport([20, 60])).toEqual(expected);
    expect(transform.viewportToPdf(expected)).toEqual([20, 60]);
    expect(transform.viewportToDevice(expected)).toEqual(expected.map((v) => v * 3));
    expect(transform.deviceToViewport(transform.viewportToDevice(expected))).toEqual(expected);
    for (const point of [[-20, -30], [10, 20], [110, 220], [40.5, 72.25]] as const) {
      expect(transform.viewportToPdf(transform.pdfToViewport(point))).toEqual(point);
    }
  });
  it("intersects crop and media boxes and transforms bounds without minimum size or clipping", () => {
    const transform = createCoordinateTransform({ media_box: [0, 0, 100, 100], crop_box: [-10, 20, 200, 120],
      rotation: 90, scale: 1, user_unit: 1, device_pixel_ratio: 1 });
    expect(transform.pdfToViewport([0, 20])).toEqual([0, 0]);
    expect(transform.pdfBoxToViewport([10, 30, 40, 50])).toEqual([10, 10, 30, 40]);
  });
  it("refuses invalid boxes, scales and points", () => {
    const config: CoordinateConfig = { media_box: [0, 0, 100, 100], rotation: 0, scale: 1, user_unit: 1, device_pixel_ratio: 1 };
    expect(() => createCoordinateTransform({ ...config, scale: 0 })).toThrow("InvalidCoordinates");
    expect(() => createCoordinateTransform({ ...config, crop_box: [110, 0, 120, 100] })).toThrow("InvalidCoordinates");
    expect(() => createCoordinateTransform(config).pdfToViewport([NaN, 0])).toThrow("InvalidCoordinates");
  });
});

class MockRenderer implements Renderer<Uint8Array, string> {
  private state: RendererDocumentState | null = null;
  readonly invalidated: PageBounds[] = [];
  async open(_document: Uint8Array, state: RendererDocumentState): Promise<ContractResult<void>> { this.state = state; return { ok: true, value: undefined }; }
  currentState() { return this.state; }
  async update(state: RendererDocumentState, _delta: DocumentDelta): Promise<ContractResult<void>> { this.state = state; return { ok: true, value: undefined }; }
  async renderViewport(_viewport: ViewportRequest): Promise<ContractResult<string>> { return { ok: true, value: "mock frame" }; }
  hitTest(_pageIndex: number, _pdfPoint: readonly [number, number]) { return { ok: true as const, value: [] }; }
  invalidate(bounds: readonly PageBounds[]) { this.invalidated.push(...bounds); return { ok: true as const, value: undefined }; }
}

it("supports a renderer adapter without wiring it into PDF.js", async () => {
  const renderer = new MockRenderer();
  await renderer.open(new Uint8Array(), { document_id: "doc", revision: "0" });
  await renderer.update({ document_id: "doc", revision: "1" }, emptyDelta);
  const bounds: PageBounds = { page_index: 0, rect: [0, 0, 10, 10] };
  renderer.invalidate([bounds]);
  expect(renderer.currentState()?.revision).toBe("1");
  expect(renderer.invalidated).toEqual([bounds]);
  const transform = createCoordinateTransform({ media_box: [0, 0, 100, 100], rotation: 0, scale: 1, user_unit: 1, device_pixel_ratio: 1 });
  expect(await renderer.renderViewport({ page_index: 0, transform, viewport_bounds: [0, 0, 100, 100] })).toEqual({ ok: true, value: "mock frame" });
  expect(renderer.hitTest(0, [1, 1])).toEqual({ ok: true, value: [] });
});

// The qualification command supplies temporary Node bindings; never import production WASM assets.
it.runIf(Boolean(process.env.STARPDF_CONTRACTS_WASM))("round-trips TypeScript → WASM serde → Rust → TypeScript → Rust exactly", () => {
  const wasm: { contract_round_trip_star_node_id(value: string): string; contract_round_trip_render_node_id(value: string): string } =
    createRequire(import.meta.url)(process.env.STARPDF_CONTRACTS_WASM!);
  const node = sampleNode();
  expect(parseStarNodeId(wasm.contract_round_trip_star_node_id(node))).toBe(node);
  expect(wasm.contract_round_trip_star_node_id(parseStarNodeId(wasm.contract_round_trip_star_node_id(node)))).toBe(node);
  const render = createRenderNodeId(node, vector.primitive);
  expect(parseRenderNodeId(wasm.contract_round_trip_render_node_id(render))).toBe(render);
  expect(wasm.contract_round_trip_render_node_id(parseRenderNodeId(wasm.contract_round_trip_render_node_id(render)))).toBe(render);
  expect(() => wasm.contract_round_trip_star_node_id("sn1/64/01/0/70")).toThrow();
});
