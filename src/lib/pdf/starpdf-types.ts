/**
 * StarPDF Engine TypeScript Interface Definitions
 *
 * Future-facing TypeScript contracts and boundary declarations for StarPDF native/WASM engine.
 * SmartPDF currently uses the in-browser PDF.js + pdf-lib architecture; StarPDF will eventually
 * replace low-level operations as the core native document authority.
 */

export type StarPdfObjectId = number;
export type StarPdfGeneration = number;

export interface StarPdfObjectRef {
  number: StarPdfObjectId;
  generation: StarPdfGeneration;
}

export type StarPdfPrimitive =
  | { type: "null" }
  | { type: "boolean"; value: boolean }
  | { type: "integer"; value: number }
  | { type: "real"; value: number }
  | { type: "name"; value: string }
  | { type: "string"; value: string | Uint8Array }
  | { type: "array"; items: StarPdfPrimitive[] }
  | { type: "dictionary"; entries: Record<string, StarPdfPrimitive> }
  | { type: "reference"; target: StarPdfObjectRef }
  | {
      type: "stream";
      dictionary: Record<string, StarPdfPrimitive>;
      data: Uint8Array;
    };

export interface StarPdfMetrics {
  objectsKnown: number;
  objectsResolved: number;
  cacheHits: number;
  bytesParsed: number;
}

export interface StarPdfDocumentHandle {
  version: string;
  pageCount: number;
  trailer: Record<string, StarPdfPrimitive>;
  metrics: StarPdfMetrics;
}
