/**
 * Form Affordance Detection Types
 * Pure geometric presentation & interaction hints for flat and scanned PDFs.
 * (No OCR / No semantic label guessing)
 */

export type FlatFormCandidateType = "checkbox" | "radio" | "text-region";

export interface FlatFormCandidate {
  id: string;
  type: FlatFormCandidateType;
  pageIndex: number;
  pdfRect: [number, number, number, number]; // [x1, y1, x2, y2] in canonical PDF points
  confidence: "HIGH" | "MEDIUM" | "LOW";
  source: "acroform" | "vector" | "raster";
}

export interface DetectorOptions {
  minCheckboxSize?: number; // in PDF points (default: 8)
  maxCheckboxSize?: number; // in PDF points (default: 32)
  minRadioDiameter?: number; // in PDF points (default: 8)
  maxRadioDiameter?: number; // in PDF points (default: 28)
  minTextRegionWidth?: number; // in PDF points (default: 40)
  maxTextRegionWidth?: number; // in PDF points (default: 500)
  minTextRegionHeight?: number; // in PDF points (default: 10)
}

