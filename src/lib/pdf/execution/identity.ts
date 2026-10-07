import { ContractError } from "./refusal";
import type { PdfObjectRef, RenderNodeId, StarNodeId } from "./contracts";

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });
const U64_MAX = BigInt("18446744073709551615");

function invalid(): never { throw new ContractError("InvalidIdentity", "expected a canonical nonempty v1 identity"); }
function hexText(value: string): string {
  const bytes = encoder.encode(value);
  if (!value || decoder.decode(bytes) !== value) return invalid();
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
function unhexText(value: string): string {
  if (!/^(?:[0-9a-f]{2})+$/.test(value)) return invalid();
  try { return decoder.decode(Uint8Array.from(value.match(/../g)!, (pair) => parseInt(pair, 16))); }
  catch { return invalid(); }
}
export function createStarNodeId(document: string, object: PdfObjectRef, occurrence: string): StarNodeId {
  if (!/^[1-9][0-9]*$/.test(object.number) || BigInt(object.number) > U64_MAX ||
      !Number.isInteger(object.generation) || object.generation < 0 || object.generation > 65535) return invalid();
  return `sn1/${hexText(document)}/${object.number}/${object.generation}/${hexText(occurrence)}` as StarNodeId;
}
export function parseStarNodeId(value: string): StarNodeId {
  const parts = value.split("/");
  if (parts.length !== 5 || parts[0] !== "sn1") return invalid();
  const id = createStarNodeId(unhexText(parts[1]), { number: parts[2], generation: Number(parts[3]) }, unhexText(parts[4]));
  if (value !== id) return invalid();
  return id;
}
export function starNodeParts(value: StarNodeId): { document: string; object: PdfObjectRef; occurrence: string } {
  const parts = parseStarNodeId(value).split("/");
  return { document: unhexText(parts[1]), object: { number: parts[2], generation: Number(parts[3]) }, occurrence: unhexText(parts[4]) };
}
export function createRenderNodeId(node: StarNodeId, primitive: string): RenderNodeId {
  return `rn1/${hexText(parseStarNodeId(node))}/${hexText(primitive)}` as RenderNodeId;
}
export function parseRenderNodeId(value: string): RenderNodeId {
  const parts = value.split("/");
  if (parts.length !== 3 || parts[0] !== "rn1") return invalid();
  const id = createRenderNodeId(parseStarNodeId(unhexText(parts[1])), unhexText(parts[2]));
  if (id !== value) return invalid();
  return id;
}
export function renderStarNode(value: RenderNodeId): StarNodeId {
  return parseStarNodeId(unhexText(parseRenderNodeId(value).split("/")[1]));
}
