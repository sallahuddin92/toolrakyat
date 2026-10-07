import { ContractError } from "./refusal";
export type HashDomain = "Font" | "Image" | "TextRun" | "Annotation" | "PdfObject";
export interface SemanticHashInput { readonly domain: HashDomain; readonly fields: Readonly<Record<string, Uint8Array>> }
const domains: Record<HashDomain, number> = { Font: 1, Image: 2, TextRun: 3, Annotation: 4, PdfObject: 5 };
const encoder = new TextEncoder();
function u64(value: number): Uint8Array {
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigUint64(0, BigInt(value), false);
  return bytes;
}
export function canonicalHashBytes(input: SemanticHashInput): Uint8Array {
  const names = Object.keys(input.fields).sort();
  if (!(input.domain in domains) || names.some((name) => !name || Array.from(name).some((c) => c.charCodeAt(0) > 127))) {
    throw new ContractError("InvalidHashInput", "field names must be nonempty ASCII");
  }
  const chunks = [encoder.encode("StarPDF.semantic.v1\0"), Uint8Array.of(domains[input.domain]), u64(names.length)];
  for (const name of names) {
    const key = encoder.encode(name), value = input.fields[name];
    chunks.push(u64(key.length), key, u64(value.length), value);
  }
  const result = new Uint8Array(chunks.reduce((size, chunk) => size + chunk.length, 0));
  let offset = 0;
  for (const chunk of chunks) { result.set(chunk, offset); offset += chunk.length; }
  return result;
}
export async function semanticHash(input: SemanticHashInput): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", Uint8Array.from(canonicalHashBytes(input)).buffer);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}
export function semanticNumberBytes(value: number): Uint8Array {
  if (!Number.isFinite(value)) throw new ContractError("InvalidHashInput", "semantic numbers must be finite");
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setFloat64(0, value === 0 ? 0 : value, false);
  return bytes;
}
