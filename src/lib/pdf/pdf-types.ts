export type AcroFormFieldType =
  | "text"
  | "checkbox"
  | "radio"
  | "dropdown"
  | "optionList"
  | "unsupported";

export interface AcroFormField {
  name: string;
  type: AcroFormFieldType;
  value: string | boolean | string[];
  originalValue: string | boolean | string[];
  options?: string[];
  isReadOnly: boolean;
  isRequired: boolean;
  rect?: { x: number; y: number; width: number; height: number };
}

export interface PdfDocumentMetadata {
  filename: string;
  fileSize: number;
  pageCount: number;
  title?: string;
  author?: string;
  subject?: string;
  producer?: string;
  creationDate?: Date;
  modificationDate?: Date;
  formFieldCount: number;
}

export interface PdfPageInfo {
  pageNumber: number; // 1-indexed
  width: number;
  height: number;
  rotation: number;
}

export interface DocumentInspectionResult {
  metadata: PdfDocumentMetadata;
  pages: PdfPageInfo[];
  fields: AcroFormField[];
}

export type ExportMode = "editable" | "flattened";

export interface ExportResult {
  pdfBytes: Uint8Array;
  filename: string;
  mode: ExportMode;
  validated: boolean;
}
