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

export interface StarPdfChoiceOption {
  export_value: string;
  display_value: string;
}

export interface StarPdfWidget {
  object_num: number;
  object_gen: number;
  page_index?: number;
  rect: [number, number, number, number];
  appearance_state?: string;
  normal_appearance_states: string[];
  is_checked: boolean;
}

export interface StarPdfFormField {
  object_num: number;
  object_gen: number;
  parent_num?: number;
  parent_gen?: number;
  field_type: "text" | "checkbox" | "radio" | "button" | "combobox" | "listbox" | "signature" | string;
  name: string;
  alternate_name?: string;
  mapping_name?: string;
  value: string;
  is_read_only: boolean;
  is_required: boolean;
  max_len?: number;
  is_comb: boolean;
  options: StarPdfChoiceOption[];
  selected_indices: number[];
  widgets: StarPdfWidget[];
}

export interface StarPdfAnnotation {
  object_num: number;
  object_gen: number;
  page_index: number;
  subtype: string;
  rect: [number, number, number, number];
  contents?: string;
  name?: string;
  appearance_state?: string;
  color?: number[];
  interior_color?: number[];
  border_width?: number;
  line_points?: [number, number, number, number];
  line_endings?: StarPdfAddAnnotationInput["line_endings"];
  quad_points: number[];
  ink_list: [number, number][][];
}

export interface StarPdfAddAnnotationInput {
  subtype: "FreeText" | "Square" | "Circle" | "Line" | "Highlight" | "Underline" | "StrikeOut" | "Ink" | "Link";
  rect: [number, number, number, number];
  contents?: string;
  font_size?: number;
  color?: number[];
  fill_color?: number[];
  border_width?: number;
  quad_points?: number[];
  line_points?: [number, number, number, number];
  line_endings?: ["None" | "Square" | "Circle" | "Diamond" | "OpenArrow" | "ClosedArrow", "None" | "Square" | "Circle" | "Diamond" | "OpenArrow" | "ClosedArrow"];
  ink_list?: [number, number][][];
  uri?: string;
}

export interface StarPdfUpdateAnnotationInput {
  rect?: [number, number, number, number];
  contents?: string;
  color?: number[];
  fill_color?: number[];
  border_width?: number;
  line_points?: [number, number, number, number];
  line_endings?: StarPdfAddAnnotationInput["line_endings"];
  quad_points?: number[];
  ink_list?: [number, number][][];
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
  | { type: "getFormFields"; id: string; handle: number }
  | { type: "getAnnotations"; id: string; handle: number; pageIndex: number }
  | { type: "setTextField"; id: string; handle: number; objectNum: number; objectGen: number; value: string }
  | { type: "setCheckbox"; id: string; handle: number; objectNum: number; objectGen: number; checked: boolean }
  | {
      type: "setRadio";
      id: string;
      handle: number;
      parentNum: number;
      parentGen: number;
      widgetNum: number;
      widgetGen: number;
      onState: string;
    }
  | { type: "setChoice"; id: string; handle: number; objectNum: number; objectGen: number; value: string }
  | { type: "setChoiceValues"; id: string; handle: number; objectNum: number; objectGen: number; values: string[] }
  | { type: "addAnnotation"; id: string; handle: number; pageIndex: number; input: StarPdfAddAnnotationInput }
  | { type: "updateAnnotation"; id: string; handle: number; objectNum: number; objectGen: number; input: StarPdfUpdateAnnotationInput }
  | { type: "removeAnnotation"; id: string; handle: number; pageIndex: number; objectNum: number; objectGen: number }
  | { type: "exportIncremental"; id: string; handle: number }
  | { type: "getAppearanceStatus"; id: string; handle: number }
  | { type: "getGlyphMappingQuality"; id: string; handle: number }
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
  | { type: "getFormFields"; id: string; success: true; fields: StarPdfFormField[] }
  | { type: "getAnnotations"; id: string; success: true; annotations: StarPdfAnnotation[] }
  | { type: "setTextField"; id: string; success: true }
  | { type: "setCheckbox"; id: string; success: true }
  | { type: "setRadio"; id: string; success: true }
  | { type: "setChoice"; id: string; success: true }
  | { type: "setChoiceValues"; id: string; success: true }
  | { type: "addAnnotation"; id: string; success: true }
  | { type: "updateAnnotation"; id: string; success: true }
  | { type: "removeAnnotation"; id: string; success: true }
  | { type: "exportIncremental"; id: string; success: true; bytes: Uint8Array }
  | { type: "getAppearanceStatus"; id: string; success: true; status: "AP_REGENERATED" | "AP_PRESERVED" | "AP_NOT_REQUIRED" | "AP_UNSUPPORTED" }
  | { type: "getGlyphMappingQuality"; id: string; success: true; quality: "EXACT" | "FALLBACK" | "UNREPRESENTABLE" | "NOT_APPLICABLE" }
  | { type: "close"; id: string; success: true }
  | { type: "createMinimal"; id: string; success: true; bytes: Uint8Array }
  | {
      type: "error";
      id: string;
      success: false;
      error: string;
      code: "INVALID_HANDLE" | "RESOURCE_LIMIT" | "UNSUPPORTED" | "INVALID_PDF" | "ENGINE_ERROR";
    };
