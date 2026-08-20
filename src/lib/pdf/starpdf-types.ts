/**
 * StarPDF WASM & Worker TypeScript Type Definitions
 */

export interface StarPdfDocumentInfo {
  page_count: number;
  pdf_version: string;
  is_valid: boolean;
}

export interface StarPdfTextSpan {
  page_index: number;
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
  font_name: string;
  font_size: number;
  confidence: number;
}

export interface StarPdfPageText {
  page_index: number;
  plain_text: string;
  spans: StarPdfTextSpan[];
}

export interface StarPdfSearchBoundingBox {
  page_index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
}

export interface StarPdfSearchResult {
  page_index: number;
  matched_text: string;
  start_span_index: number;
  end_span_index: number;
  boxes: StarPdfSearchBoundingBox[];
  confidence: number;
}

export interface StarPdfSearchOptions {
  caseSensitive?: boolean;
}

/**
 * Worker Protocol Request Messages
 */
export type StarPdfWorkerRequest =
  | { type: "init"; id: string; wasmUrl?: string }
  | { type: "open"; id: string; buffer: ArrayBuffer }
  | { type: "info"; id: string; handle: number }
  | { type: "pageCount"; id: string; handle: number }
  | { type: "extractPage"; id: string; handle: number; pageIndex: number }
  | { type: "extractAll"; id: string; handle: number }
  | { type: "search"; id: string; handle: number; query: string; caseSensitive: boolean }
  | { type: "validate"; id: string; handle: number }
  | { type: "close"; id: string; handle: number }
  | { type: "createMinimal"; id: string; text: string };

/**
 * Worker Protocol Response Messages
 */
export type StarPdfWorkerResponse =
  | { type: "init"; id: string; success: true; version: string }
  | { type: "open"; id: string; success: true; handle: number }
  | { type: "info"; id: string; success: true; info: StarPdfDocumentInfo }
  | { type: "pageCount"; id: string; success: true; pageCount: number }
  | { type: "extractPage"; id: string; success: true; pageText: StarPdfPageText }
  | { type: "extractAll"; id: string; success: true; pages: StarPdfPageText[] }
  | { type: "search"; id: string; success: true; results: StarPdfSearchResult[] }
  | { type: "validate"; id: string; success: true; isValid: boolean }
  | { type: "close"; id: string; success: true }
  | { type: "createMinimal"; id: string; success: true; bytes: Uint8Array }
  | { type: "error"; id: string; success: false; error: string; code?: string };
