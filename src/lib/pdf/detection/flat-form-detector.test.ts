import { describe, it, expect } from "vitest";
import {
  detectFlatFormCandidates,
  calculateIoU,
  deduplicateCandidates,
  computeAutoCenteredMark,
} from "./flat-form-detector";
import type { AcroFormField } from "@/lib/pdf/pdf-types";
import type { StarPdfVectorGraphicInfo } from "@/lib/pdf/starpdf-types";
import type { FlatFormCandidate } from "./types";

describe("Flat-Form Affordance Detector", () => {
  describe("calculateIoU", () => {
    it("returns 1.0 for identical rectangles", () => {
      const rect: [number, number, number, number] = [10, 10, 50, 50];
      expect(calculateIoU(rect, rect)).toBeCloseTo(1.0);
    });

    it("returns 0.0 for disjoint rectangles", () => {
      const rectA: [number, number, number, number] = [10, 10, 20, 20];
      const rectB: [number, number, number, number] = [30, 30, 40, 40];
      expect(calculateIoU(rectA, rectB)).toBe(0);
    });

    it("calculates partial overlap correctly", () => {
      const rectA: [number, number, number, number] = [0, 0, 10, 10]; // area 100
      const rectB: [number, number, number, number] = [5, 0, 15, 10]; // area 100, inter 50, union 150
      expect(calculateIoU(rectA, rectB)).toBeCloseTo(50 / 150);
    });
  });

  describe("deduplicateCandidates", () => {
    it("suppresses duplicate nested/overlapping candidates prioritizing AcroForm > Vector > Raster", () => {
      const candidates: FlatFormCandidate[] = [
        {
          id: "cand-raster-1",
          type: "checkbox",
          pageIndex: 0,
          pdfRect: [100, 200, 114, 214],
          confidence: "MEDIUM",
          source: "raster",
        },
        {
          id: "cand-vec-1",
          type: "checkbox",
          pageIndex: 0,
          pdfRect: [100, 200, 115, 215],
          confidence: "HIGH",
          source: "vector",
        },
      ];

      const deduped = deduplicateCandidates(candidates);
      expect(deduped).toHaveLength(1);
      expect(deduped[0].id).toBe("cand-vec-1");
      expect(deduped[0].source).toBe("vector");
    });
  });

  describe("detectFlatFormCandidates", () => {
    it("Level 1: Identifies native AcroForm checkbox and text fields", () => {
      const acroFields: AcroFormField[] = [
        {
          name: "AgreeTerms",
          type: "checkbox",
          value: false,
          originalValue: false,
          isReadOnly: false,
          isRequired: false,
          rect: { x: 50, y: 700, width: 14, height: 14 },
          pageNumber: 1,
        },
        {
          name: "FullName",
          type: "text",
          value: "",
          originalValue: "",
          isReadOnly: false,
          isRequired: false,
          rect: { x: 50, y: 650, width: 200, height: 20 },
          pageNumber: 1,
        },
      ];

      const candidates = detectFlatFormCandidates({
        pageIndex: 0,
        pageWidth: 612,
        pageHeight: 792,
        acroFields,
      });

      expect(candidates).toHaveLength(2);
      expect(candidates[0].type).toBe("checkbox");
      expect(candidates[0].source).toBe("acroform");
      expect(candidates[1].type).toBe("text-region");
      expect(candidates[1].source).toBe("acroform");
    });

    it("Level 2: Identifies native vector geometry (checkboxes, radio circles, text lines)", () => {
      const createMockVector = (
        id: string,
        type: string,
        bounds: [number, number, number, number],
      ): StarPdfVectorGraphicInfo => ({
        graphic_id: id,
        page_index: 0,
        stream_index: 0,
        start_instruction_index: 0,
        end_instruction_index: 1,
        graphic_type: type,
        bounds,
        local_bounds: bounds,
        transform: [1, 0, 0, 1, 0, 0],
        line_width: 1,
        is_stroked: true,
        is_filled: false,
        is_shared: false,
        is_editable: true,
        editability_code: "OK",
      });

      const vectorGraphics: StarPdfVectorGraphicInfo[] = [
        createMockVector("vec-cb-1", "Rectangle", [100, 500, 14, 14]), // 14x14 pt square -> checkbox
        createMockVector("vec-rad-1", "Circle", [100, 450, 12, 12]), // 12x12 pt circular ring -> radio
        createMockVector("vec-line-1", "Line", [100, 400, 200, 2]), // 200pt wide horizontal line -> text region
        createMockVector("vec-table-cell", "Rectangle", [50, 100, 500, 250]), // Large table cell -> rejected from checkbox
      ];

      const candidates = detectFlatFormCandidates({
        pageIndex: 0,
        pageWidth: 612,
        pageHeight: 792,
        vectorGraphics,
      });


      expect(candidates.find((c) => c.id === "cand-vec-cb-vec-cb-1")?.type).toBe("checkbox");
      expect(candidates.find((c) => c.id === "cand-vec-rad-vec-rad-1")?.type).toBe("radio");
      expect(candidates.find((c) => c.id === "cand-vec-line-vec-line-1")?.type).toBe("text-region");
      expect(candidates.find((c) => c.id.includes("vec-table-cell"))).toBeUndefined();
    });


    it("Level 3: Analyzes raster pixel bitmap for small empty squares (checkboxes)", () => {
      // Create synthetic 612x792 ImageData with one 14x14 black square box at (100, 100) with white center
      const imgW = 612;
      const imgH = 792;
      const data = new Uint8ClampedArray(imgW * imgH * 4);
      data.fill(255); // all white background

      const boxX = 100;
      const boxY = 100;
      const boxSize = 14;

      // Draw box border (black pixels)
      for (let x = boxX; x < boxX + boxSize; x++) {
        for (const y of [boxY, boxY + boxSize - 1]) {
          const idx = (y * imgW + x) * 4;
          data[idx] = 0;
          data[idx + 1] = 0;
          data[idx + 2] = 0;
          data[idx + 3] = 255;
        }
      }
      for (let y = boxY; y < boxY + boxSize; y++) {
        for (const x of [boxX, boxX + boxSize - 1]) {
          const idx = (y * imgW + x) * 4;
          data[idx] = 0;
          data[idx + 1] = 0;
          data[idx + 2] = 0;
          data[idx + 3] = 255;
        }
      }

      const mockImageData = {
        width: imgW,
        height: imgH,
        data,
        colorSpace: "srgb" as PredefinedColorSpace,
      } as ImageData;

      const candidates = detectFlatFormCandidates({
        pageIndex: 0,
        pageWidth: 612,
        pageHeight: 792,
        imageData: mockImageData,
      });


      expect(candidates.length).toBeGreaterThanOrEqual(1);
      const cb = candidates.find((c) => c.type === "checkbox");
      expect(cb).toBeDefined();
      expect(cb?.source).toBe("raster");
    });
  });

  describe("computeAutoCenteredMark", () => {
    it("computes clean centered coordinates for checkmark inside candidate rectangle", () => {
      const candidateRect: [number, number, number, number] = [100, 200, 116, 216]; // 16x16 square
      const mark = computeAutoCenteredMark(candidateRect, "check");

      expect(mark.fontSize).toBeGreaterThanOrEqual(12);
      expect(mark.x + mark.width / 2).toBeCloseTo(108);
      expect(mark.y + mark.height / 2).toBeCloseTo(208);
    });

    it("computes centered coordinates for radio mark", () => {
      const candidateRect: [number, number, number, number] = [50, 50, 70, 70]; // 20x20 circle box
      const mark = computeAutoCenteredMark(candidateRect, "radio");

      expect(mark.x + mark.width / 2).toBeCloseTo(60);
      expect(mark.y + mark.height / 2).toBeCloseTo(60);
    });
  });
});
