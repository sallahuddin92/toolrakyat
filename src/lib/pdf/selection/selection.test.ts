import { describe, expect, it } from "vitest";
import {
  HIT_TEST_PRIORITY,
  resolveSelectionAfterMutation,
  type SmartPdfSelection,
} from "./types";
import { convertPdfRectToPixels, isPointInRect } from "./geometry";
import type {
  StarPdfTextSpan,
  StarPdfImageInfo,
  StarPdfVectorGraphicInfo,
} from "../starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "../pdf-types";

describe("SmartPDF Selection Model & Geometry", () => {
  it("codifies the strict hit-testing priority order", () => {
    expect(HIT_TEST_PRIORITY).toEqual([
      "form",
      "annotation",
      "text",
      "image",
      "vector",
    ]);
  });

  describe("convertPdfRectToPixels", () => {
    const page = { width: 612, height: 792, rotation: 0 };

    it("converts 0deg rotation accurately from PDF bottom-left to viewport top-left", () => {
      // PDF rect: x=100, y=700 (near top), w=200, h=50
      const pdfRect = { x: 100, y: 700, width: 200, height: 50 };
      const pixelRect = convertPdfRectToPixels(pdfRect, page, 1.0, 0);

      expect(pixelRect.left).toBe(100);
      expect(pixelRect.top).toBe(792 - (700 + 50)); // 42
      expect(pixelRect.width).toBe(200);
      expect(pixelRect.height).toBe(50);
    });

    it("handles zoom scale accurately", () => {
      const pdfRect = { x: 50, y: 700, width: 100, height: 40 };
      const pixelRect = convertPdfRectToPixels(pdfRect, page, 1.5, 0);

      expect(pixelRect.left).toBe(75);
      expect(pixelRect.top).toBe((792 - 740) * 1.5); // 52 * 1.5 = 78
      expect(pixelRect.width).toBe(150);
      expect(pixelRect.height).toBe(60);
    });

    it("handles 90deg, 180deg, 270deg page rotations", () => {
      const pdfRect = { x: 100, y: 200, width: 50, height: 30 };

      const r90 = convertPdfRectToPixels(pdfRect, page, 1.0, 90);
      expect(r90.left).toBe(200);
      expect(r90.top).toBe(100);
      expect(r90.width).toBe(30);
      expect(r90.height).toBe(50);

      const r180 = convertPdfRectToPixels(pdfRect, page, 1.0, 180);
      expect(r180.left).toBe(612 - (100 + 50));
      expect(r180.top).toBe(200);

      const r270 = convertPdfRectToPixels(pdfRect, page, 1.0, 270);
      expect(r270.left).toBe(792 - (200 + 30));
      expect(r270.top).toBe(612 - (100 + 50));
    });
  });

  describe("isPointInRect", () => {
    const rect = { x: 10, y: 20, width: 100, height: 50 };

    it("identifies points inside and outside", () => {
      expect(isPointInRect(15, 25, rect)).toBe(true);
      expect(isPointInRect(10, 20, rect)).toBe(true);
      expect(isPointInRect(110, 70, rect)).toBe(true);
      expect(isPointInRect(9, 25, rect)).toBe(false);
      expect(isPointInRect(50, 75, rect)).toBe(false);
    });
  });

  describe("resolveSelectionAfterMutation", () => {
    const textSpan: StarPdfTextSpan = {
      span_id: "span-1",
      page_index: 0,
      text: "Invoice #101",
      x: 50,
      y: 700,
      width: 120,
      height: 14,
      rotation: 0,
      font_name: "Helvetica",
      font_size: 12,
      confidence: 1.0,
      stream_index: 0,
      instruction_index: 0,
      operand_index: 0,
      operator_name: "Tj",
      is_editable: true,
      editability_code: "EDITABLE_NATIVE_TEXT",
    };

    const imageInfo: StarPdfImageInfo = {
      image_id: "img-1",
      page_index: 0,
      resource_name: "Im0",
      stream_index: 0,
      instruction_index: 0,
      width: 200,
      height: 100,
      bits_per_component: 8,
      color_space: "DeviceRGB",
      filter: "DCTDecode",
      transform: [1, 0, 0, 1, 0, 0],
      rect: [50, 500, 200, 100],
      is_nested_form: false,
      is_shared: false,
    };

    const graphicInfo: StarPdfVectorGraphicInfo = {
      graphic_id: "vec-1",
      page_index: 0,
      stream_index: 0,
      start_instruction_index: 0,
      end_instruction_index: 1,
      graphic_type: "Path",
      bounds: [30, 300, 150, 80],
      local_bounds: [30, 300, 150, 80],
      transform: [1, 0, 0, 1, 0, 0],
      stroke_color_rgb: [0, 0, 0],
      fill_color_rgb: [0.8, 0.8, 0.8],
      line_width: 2,
      is_stroked: true,
      is_filled: true,
      is_shared: false,
      is_editable: true,
      editability_code: "EDITABLE_GRAPHIC",
    };

    const formField: AcroFormField = {
      name: "customer_name",
      type: "text",
      value: "Jane Doe",
      originalValue: "Jane Doe",
      isReadOnly: false,
      isRequired: false,
      rect: { x: 50, y: 600, width: 200, height: 24 },
    };

    const annotation: PdfMarkupAnnotation = {
      id: "annot-0-1",
      subtype: "FreeText",
      contents: "Approved",
      rect: { x: 100, y: 100, width: 80, height: 20 },
      pageIndex: 0,
    };

    it("preserves valid text selection with refreshed data", () => {
      const prev: SmartPdfSelection = {
        type: "text",
        id: "span-1",
        pageIndex: 0,
        data: textSpan,
        pdfRect: { x: 50, y: 700, width: 120, height: 14 },
      };

      const updatedSpan = { ...textSpan, text: "Invoice #102" };
      const resolved = resolveSelectionAfterMutation(prev, 0, [updatedSpan], [], [], [], []);
      expect(resolved).not.toBeNull();
      expect(resolved?.type).toBe("text");
      expect((resolved?.data as StarPdfTextSpan).text).toBe("Invoice #102");
    });

    it("clears text selection when span is no longer on page", () => {
      const prev: SmartPdfSelection = {
        type: "text",
        id: "span-deleted",
        pageIndex: 0,
        data: textSpan,
        pdfRect: { x: 50, y: 700, width: 120, height: 14 },
      };

      const resolved = resolveSelectionAfterMutation(prev, 0, [textSpan], [], [], [], []);
      expect(resolved).toBeNull();
    });

    it("clears selection when page index changes", () => {
      const prev: SmartPdfSelection = {
        type: "image",
        id: "img-1",
        pageIndex: 0,
        data: imageInfo,
        pdfRect: { x: 50, y: 500, width: 200, height: 100 },
      };

      const resolved = resolveSelectionAfterMutation(prev, 1, [], [imageInfo], [], [], []);
      expect(resolved).toBeNull();
    });

    it("preserves valid image selection", () => {
      const prev: SmartPdfSelection = {
        type: "image",
        id: "img-1",
        pageIndex: 0,
        data: imageInfo,
        pdfRect: { x: 50, y: 500, width: 200, height: 100 },
      };

      const resolved = resolveSelectionAfterMutation(prev, 0, [], [imageInfo], [], [], []);
      expect(resolved).not.toBeNull();
      expect(resolved?.id).toBe("img-1");
    });

    it("preserves valid vector graphic selection", () => {
      const prev: SmartPdfSelection = {
        type: "vector",
        id: "vec-1",
        pageIndex: 0,
        data: graphicInfo,
        pdfRect: { x: 30, y: 300, width: 150, height: 80 },
      };

      const resolved = resolveSelectionAfterMutation(prev, 0, [], [], [graphicInfo], [], []);
      expect(resolved).not.toBeNull();
      expect(resolved?.id).toBe("vec-1");
    });

    it("preserves valid form selection", () => {
      const prev: SmartPdfSelection = {
        type: "form",
        id: "customer_name",
        pageIndex: 0,
        data: formField,
      };

      const resolved = resolveSelectionAfterMutation(prev, 0, [], [], [], [formField], []);
      expect(resolved).not.toBeNull();
      expect(resolved?.id).toBe("customer_name");
    });

    it("preserves valid annotation selection", () => {
      const prev: SmartPdfSelection = {
        type: "annotation",
        id: "annot-0-1",
        pageIndex: 0,
        data: annotation,
        pdfRect: annotation.rect,
      };

      const resolved = resolveSelectionAfterMutation(prev, 0, [], [], [], [], [annotation]);
      expect(resolved).not.toBeNull();
      expect(resolved?.id).toBe("annot-0-1");
    });
  });
});
