import { describe, it, expect, vi } from "vitest";
import {
  AddFreeTextCommand,
  AddCheckMarkCommand,
  AddCrossMarkCommand,
  AddInkAnnotationCommand,
  DeleteAnnotationCommand,
  AddImageCommand,
} from "./index";
import type { SmartPdfCommandContext } from "./types";
import { convertPixelsToPdfPoint, convertPdfPointToPixels } from "../selection/geometry";

type MockStarPdfDoc = NonNullable<SmartPdfCommandContext["starPdfDoc"]>;

function createMockContext(mockDoc: Partial<MockStarPdfDoc>): SmartPdfCommandContext {
  return {
    sourceBytes: new Uint8Array(10),
    filename: "test.pdf",
    currentPage: 1,
    pageCount: 1,
    selection: null,
    fieldValues: {},
    annotationValues: {},
    inspectionResult: null,
    starPdfDoc: mockDoc as unknown as MockStarPdfDoc,
  };
}

describe("Phase 3A Creation Commands", () => {
  it("AddFreeTextCommand invokes starPdfDoc.addAnnotation with FreeText subtype", async () => {
    const mockExportIncremental = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const mockAddAnnotation = vi.fn().mockResolvedValue(42);

    const context = createMockContext({
      addAnnotation: mockAddAnnotation,
      exportIncremental: mockExportIncremental,
    });

    const cmd = new AddFreeTextCommand(0, 100, 200, "Patient Name: Jane Doe", 14, [0, 0, 0], 200, 18);
    expect(cmd.id).toBe("annotation.add_freetext");
    expect(cmd.isMutating).toBe(true);

    const result = await cmd.execute(context);
    expect(mockAddAnnotation).toHaveBeenCalledWith(0, {
      subtype: "FreeText",
      rect: [100, 200, 300, 218],
      contents: "Patient Name: Jane Doe",
      font_size: 14,
      color: [0, 0, 0],
    });
    expect(result.bytes).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("AddCheckMarkCommand adds checkmark annotation", async () => {
    const mockExportIncremental = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const mockAddAnnotation = vi.fn().mockResolvedValue(43);

    const context = createMockContext({
      addAnnotation: mockAddAnnotation,
      exportIncremental: mockExportIncremental,
    });

    const cmd = new AddCheckMarkCommand(0, 50, 60, 16);
    expect(cmd.id).toBe("annotation.add_check");

    const result = await cmd.execute(context);
    expect(mockAddAnnotation).toHaveBeenCalledWith(0, {
      subtype: "FreeText",
      rect: [50, 60, 66, 76],
      contents: "✓",
      font_size: 16,
      color: [0, 0.5, 0],
    });
    expect(result.bytes).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("AddCrossMarkCommand adds crossmark annotation", async () => {
    const mockExportIncremental = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const mockAddAnnotation = vi.fn().mockResolvedValue(44);

    const context = createMockContext({
      addAnnotation: mockAddAnnotation,
      exportIncremental: mockExportIncremental,
    });

    const cmd = new AddCrossMarkCommand(0, 70, 80, 16);
    expect(cmd.id).toBe("annotation.add_cross");

    const result = await cmd.execute(context);
    expect(mockAddAnnotation).toHaveBeenCalledWith(0, {
      subtype: "FreeText",
      rect: [70, 80, 86, 96],
      contents: "✕",
      font_size: 16,
      color: [0.8, 0, 0],
    });
    expect(result.bytes).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("AddInkAnnotationCommand adds freehand drawing Ink annotation", async () => {
    const mockExportIncremental = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const mockAddAnnotation = vi.fn().mockResolvedValue(45);

    const context = createMockContext({
      addAnnotation: mockAddAnnotation,
      exportIncremental: mockExportIncremental,
    });

    const stroke: [number, number][] = [
      [10, 20],
      [15, 25],
      [20, 30],
    ];
    const rect: [number, number, number, number] = [10, 20, 20, 30];

    const cmd = new AddInkAnnotationCommand(0, [stroke], rect, [0, 0, 0.8], 2);
    expect(cmd.id).toBe("annotation.add_ink");

    const result = await cmd.execute(context);
    expect(mockAddAnnotation).toHaveBeenCalledWith(0, {
      subtype: "Ink",
      rect,
      ink_list: [stroke],
      color: [0, 0, 0.8],
      border_width: 2,
    });
    expect(result.bytes).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("DeleteAnnotationCommand parses object numbers and removes annotation", async () => {
    const mockExportIncremental = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const mockRemoveAnnotation = vi.fn().mockResolvedValue(undefined);

    const context = createMockContext({
      removeAnnotation: mockRemoveAnnotation,
      exportIncremental: mockExportIncremental,
    });

    const cmd = new DeleteAnnotationCommand(0, "annot-42-0");
    expect(cmd.id).toBe("annotation.delete");

    const result = await cmd.execute(context);
    expect(mockRemoveAnnotation).toHaveBeenCalledWith(0, 42, 0);
    expect(result.clearSelection).toBe(true);
  });

  it("AddImageCommand inserts signature stamp", async () => {
    const mockExportIncremental = vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3]));
    const mockAddImage = vi.fn().mockResolvedValue(undefined);

    const context = createMockContext({
      addImage: mockAddImage,
      exportIncremental: mockExportIncremental,
    });

    // 1x1 JPEG minimal bytes
    const dummyJpeg = new Uint8Array([0xff, 0xd8, 0xff, 0xd9]);
    const cmd = new AddImageCommand(0, 150, 250, 120, 40, dummyJpeg);
    expect(cmd.id).toBe("image.add");

    const result = await cmd.execute(context);
    expect(mockAddImage).toHaveBeenCalledWith(
      0,
      expect.any(Uint8Array),
      150,
      250,
      120,
      40,
    );
    expect(result.bytes).toEqual(new Uint8Array([1, 2, 3]));
  });
});

describe("Phase 3A Coordinate Round-Trip Mapping", () => {
  it("converts pixel point to PDF user space and back identically", () => {
    const pageDims = { width: 612, height: 792, rotation: 0 };
    const scale = 1.5;

    const screenX = 300;
    const screenY = 450;

    const pdfPt = convertPixelsToPdfPoint(screenX, screenY, pageDims, scale, 0);
    const roundTripScreen = convertPdfPointToPixels(pdfPt.x, pdfPt.y, pageDims, scale, 0);

    expect(Math.round(roundTripScreen.x)).toBe(screenX);
    expect(Math.round(roundTripScreen.y)).toBe(screenY);
  });

  it("handles 90-degree rotated pages accurately", () => {
    const pageDims = { width: 612, height: 792, rotation: 90 };
    const scale = 1.2;

    const screenX = 200;
    const screenY = 300;

    const pdfPt = convertPixelsToPdfPoint(screenX, screenY, pageDims, scale, 90);
    const roundTrip = convertPdfPointToPixels(pdfPt.x, pdfPt.y, pageDims, scale, 90);

    expect(Math.round(roundTrip.x)).toBe(screenX);
    expect(Math.round(roundTrip.y)).toBe(screenY);
  });
});
