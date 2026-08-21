/**
 * StarPDF WASM & Worker TypeScript Type Definitions
 */

export type StarPdfRecoveryStatus =
  | "NONE"
  | "XREF_RECOVERED"
  | "STREAM_LENGTH_RECONCILED"
  | "OPTIONAL_ENTRY_DEFAULTED"
  | "PRODUCER_COMPATIBILITY_PATH"
  | "UNSUPPORTED_STRUCTURE"
  | "MALFORMED_DOCUMENT";

export interface StarPdfDocumentInfo {
  page_count: number;
  pdf_version: string;
  is_valid: boolean;
  recovery_status?: StarPdfRecoveryStatus;
  recovery_events?: string[];
}

export interface StarPdfSecurityInfo {
  signature_state:
    | "UNSIGNED"
    | "SIGNED_PRESENT"
    | "SIGNED_WITH_BYTE_RANGE"
    | "SIGNED_STRUCTURE_MALFORMED";
  signature_count: number;
  byte_range_count: number;
  encryption_state:
    | "NOT_ENCRYPTED"
    | "STANDARD_SECURITY_DETECTED"
    | "PUBLIC_KEY_SECURITY_DETECTED"
    | "UNSUPPORTED_ENCRYPTION"
    | "MALFORMED_ENCRYPTION_DICTIONARY";
  encryption_filter?: string;
  encryption_subfilter?: string;
  permission_raw?: number;
  permission_printing?: boolean;
  permission_modification?: boolean;
  permission_copying?: boolean;
  permission_annotation_and_forms?: boolean;
  mutation_allowed: boolean;
  mutation_reason_code?: string;
  signed_mutation_state:
    | "NOT_APPLICABLE"
    | "SIGNED_BYTES_PRESERVED"
    | "POST_SIGNATURE_REVISION_ADDED"
    | "SIGNATURE_VALIDITY_UNKNOWN"
    | "MUTATION_REFUSED";
  cryptographic_verification: "NOT_PERFORMED";
  document_id_valid: boolean;
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
  span_id: string;
  stream_index: number;
  instruction_index: number;
  operand_index: number;
  operator_name: string;
  is_editable: boolean;
  editability_code:
    | "EDITABLE_NATIVE_TEXT"
    | "UNSUPPORTED_FONT_ENCODING"
    | "UNSUPPORTED_COMPLEX_SCRIPT"
    | "UNSUPPORTED_VERTICAL_WRITING"
    | "INLINE_IMAGE_OR_OVERLAY"
    | "FORM_OR_ANNOTATION_TEXT"
    | "READ_ONLY_SECURITY_RESTRICTED"
    | string;
  refusal_reason?: string;
}

export interface StarPdfReplaceTextResult {
  success: boolean;
  layout_result:
    | "EXACT_FIT"
    | "FIT_WITHIN_ORIGINAL_BOX"
    | "WIDTH_CHANGED"
    | "UNSUPPORTED_LAYOUT"
    | string;
  modified_object_count: number;
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
  has_normal_appearance: boolean;
  has_rollover_appearance: boolean;
  has_down_appearance: boolean;
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
  graph_classification:
    | "CANONICAL_FIELD"
    | "MULTI_WIDGET_FIELD"
    | "ORPHAN_WIDGET"
    | "AMBIGUOUS_WIDGET_GROUP"
    | "MALFORMED_FIELD_GRAPH";
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
  uri?: string;
  has_normal_appearance: boolean;
  has_rollover_appearance: boolean;
  has_down_appearance: boolean;
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
export interface StarPdfImageInfo {
  image_id: string;
  page_index: number;
  stream_index: number;
  instruction_index: number;
  resource_name: string;
  width: number;
  height: number;
  color_space: string;
  bits_per_component: number;
  filter?: string;
  transform: [number, number, number, number, number, number];
  rect: [number, number, number, number];
  is_nested_form: boolean;
  is_shared: boolean;
}

export interface StarPdfImageMutationResult {
  success: boolean;
  modified_object_count: number;
}

export interface StarPdfVectorGraphicInfo {
  graphic_id: string;
  page_index: number;
  stream_index: number;
  start_instruction_index: number;
  end_instruction_index: number;
  graphic_type: "Rectangle" | "Line" | "Path" | string;
  bounds: [number, number, number, number];
  local_bounds: [number, number, number, number];
  transform: [number, number, number, number, number, number];
  stroke_color_rgb?: [number, number, number];
  fill_color_rgb?: [number, number, number];
  stroke_color_hex?: string;
  fill_color_hex?: string;
  line_width: number;
  is_stroked: boolean;
  is_filled: boolean;
  is_shared: boolean;
  is_editable: boolean;
  editability_code: string;
  refusal_reason?: string;
  rect_x?: number;
  rect_y?: number;
  rect_w?: number;
  rect_h?: number;
  line_x1?: number;
  line_y1?: number;
  line_x2?: number;
  line_y2?: number;
}

export interface StarPdfVectorMutationResult {
  success: boolean;
  modified_object_count: number;
}

export interface StarPdfUpdateVectorGraphicInput {
  page_index: number;
  graphic_id: string;
  rect_x?: number;
  rect_y?: number;
  rect_w?: number;
  rect_h?: number;
  line_x1?: number;
  line_y1?: number;
  line_x2?: number;
  line_y2?: number;
  stroke_color_rgb?: [number, number, number];
  fill_color_rgb?: [number, number, number];
  line_width?: number;
  is_stroked?: boolean;
  is_filled?: boolean;
  clone_if_shared?: boolean;
}

export interface StarPdfAddRectangleInput {
  page_index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  stroke_color_rgb?: [number, number, number];
  fill_color_rgb?: [number, number, number];
  line_width: number;
  is_stroked: boolean;
  is_filled: boolean;
}

export interface StarPdfAddLineInput {
  page_index: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  stroke_color_rgb?: [number, number, number];
  line_width: number;
}

export interface StarPdfDeleteVectorGraphicInput {
  page_index: number;
  graphic_id: string;
  clone_if_shared?: boolean;
}

export type StarPdfWorkerRequest =
  | { type: "init"; id: string; wasmUrl?: string }
  | { type: "open"; id: string; buffer: ArrayBuffer }
  | { type: "info"; id: string; handle: number }
  | { type: "securityInfo"; id: string; handle: number }
  | { type: "pageCount"; id: string; handle: number }
  | { type: "extractPage"; id: string; handle: number; pageIndex: number }
  | { type: "extractAll"; id: string; handle: number }
  | { type: "search"; id: string; handle: number; query: string; caseSensitive?: boolean }
  | { type: "validate"; id: string; handle: number }
  | { type: "getFormFields"; id: string; handle: number }
  | { type: "getAnnotations"; id: string; handle: number; pageIndex: number }
  | { type: "setTextField"; id: string; handle: number; objNum: number; objGen: number; value: string }
  | { type: "setCheckbox"; id: string; handle: number; objNum: number; objGen: number; checked: boolean }
  | { type: "setRadio"; id: string; handle: number; parentNum: number; parentGen: number; widgetNum: number; widgetGen: number; onState: string }
  | { type: "setChoice"; id: string; handle: number; objNum: number; objGen: number; value: string }
  | { type: "setChoiceValues"; id: string; handle: number; objNum: number; objGen: number; values: string[] }
  | { type: "addAnnotation"; id: string; handle: number; pageIndex: number; annotation: Record<string, unknown> }
  | { type: "updateAnnotation"; id: string; handle: number; objNum: number; objGen: number; update: Record<string, unknown> }
  | { type: "removeAnnotation"; id: string; handle: number; pageIndex: number; objNum: number; objGen: number }
  | { type: "replaceText"; id: string; handle: number; pageIndex: number; spanId: string; newText: string }
  | { type: "getTextEditability"; id: string; handle: number; pageIndex: number; spanId: string }
  | { type: "enumerateImages"; id: string; handle: number; pageIndex: number }
  | { type: "replaceImage"; id: string; handle: number; pageIndex: number; imageId: string; newImageBytes: Uint8Array; cloneIfShared?: boolean }
  | { type: "addImage"; id: string; handle: number; pageIndex: number; imageBytes: Uint8Array; x: number; y: number; width: number; height: number }
  | { type: "removeImage"; id: string; handle: number; pageIndex: number; imageId: string }
  | { type: "enumerateGraphics"; id: string; handle: number; pageIndex: number }
  | { type: "updateGraphic"; id: string; handle: number; input: StarPdfUpdateVectorGraphicInput }
  | { type: "addRectangle"; id: string; handle: number; input: StarPdfAddRectangleInput }
  | { type: "addLine"; id: string; handle: number; input: StarPdfAddLineInput }
  | { type: "deleteGraphic"; id: string; handle: number; input: StarPdfDeleteVectorGraphicInput }
  | { type: "exportIncremental"; id: string; handle: number }
  | { type: "deletePage"; id: string; handle: number; pageIndex: number }
  | { type: "movePage"; id: string; handle: number; fromIndex: number; toIndex: number }
  | { type: "duplicatePage"; id: string; handle: number; pageIndex: number; destinationIndex: number }
  | { type: "insertBlankPage"; id: string; handle: number; pageIndex: number; width: number; height: number; rotation: 0 | 90 | 180 | 270 }
  | { type: "extractPages"; id: string; handle: number; pageIndices: number[] }
  | { type: "insertImportedPage"; id: string; handle: number; buffer: ArrayBuffer; importedPageIndex: number; destinationIndex: number }
  | { type: "mergeDocuments"; id: string; buffers: ArrayBuffer[]; pageSources?: { documentIndex: number; pageIndex: number }[] }
  | { type: "splitDocument"; id: string; handle: number; ranges: { start: number; endExclusive: number }[] }
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
  | { type: "securityInfo"; id: string; success: true; securityInfo: StarPdfSecurityInfo }
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
  | { type: "replaceText"; id: string; success: true; result: StarPdfReplaceTextResult }
  | { type: "getTextEditability"; id: string; success: true; span: StarPdfTextSpan }
  | { type: "enumerateImages"; id: string; success: true; images: StarPdfImageInfo[] }
  | { type: "replaceImage"; id: string; success: true; result: StarPdfImageMutationResult }
  | { type: "addImage"; id: string; success: true; result: StarPdfImageMutationResult }
  | { type: "removeImage"; id: string; success: true; result: StarPdfImageMutationResult }
  | { type: "enumerateGraphics"; id: string; success: true; graphics: StarPdfVectorGraphicInfo[] }
  | { type: "updateGraphic"; id: string; success: true; result: StarPdfVectorMutationResult }
  | { type: "addRectangle"; id: string; success: true; result: StarPdfVectorMutationResult }
  | { type: "addLine"; id: string; success: true; result: StarPdfVectorMutationResult }
  | { type: "deleteGraphic"; id: string; success: true; result: StarPdfVectorMutationResult }
  | { type: "exportIncremental"; id: string; success: true; bytes: Uint8Array }
  | { type: "deletePage"; id: string; success: true; bytes: Uint8Array }
  | { type: "movePage"; id: string; success: true; bytes: Uint8Array }
  | { type: "duplicatePage"; id: string; success: true; bytes: Uint8Array }
  | { type: "insertBlankPage"; id: string; success: true; bytes: Uint8Array }
  | { type: "extractPages"; id: string; success: true; bytes: Uint8Array }
  | { type: "insertImportedPage"; id: string; success: true; bytes: Uint8Array }
  | { type: "mergeDocuments"; id: string; success: true; bytes: Uint8Array }
  | { type: "splitDocument"; id: string; success: true; outputs: Uint8Array[] }
  | { type: "getAppearanceStatus"; id: string; success: true; status: "AP_REGENERATED" | "AP_PRESERVED" | "AP_NOT_REQUIRED" | "AP_UNSUPPORTED" }
  | { type: "getGlyphMappingQuality"; id: string; success: true; quality: "EXACT" | "FALLBACK" | "UNREPRESENTABLE" | "NOT_APPLICABLE" }
  | { type: "close"; id: string; success: true }
  | { type: "createMinimal"; id: string; success: true; bytes: Uint8Array }
  | {
      type: "error";
      id: string;
      success: false;
      error: string;
      code: "INVALID_HANDLE" | "RESOURCE_LIMIT" | "UNSUPPORTED" | "SIGNED_DOCUMENT" | "ENCRYPTED_DOCUMENT" | "INVALID_PDF" | "ENGINE_ERROR";
    };
