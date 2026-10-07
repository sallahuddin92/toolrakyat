import { describe, expect, it } from "vitest";
import { PDF_PALETTE_COMMANDS, searchPdfPalette } from "./palette";

describe("SmartPDF command palette", () => {
  it("has each requested command exactly once", () => {
    expect(PDF_PALETTE_COMMANDS.map((command) => command.id)).toEqual([
      "add-text", "add-image", "search", "delete", "duplicate", "rotate",
      "extract", "properties", "bookmarks", "attachments", "export",
    ]);
  });

  it("ranks exact semantic matches before keyword matches", () => {
    expect(searchPdfPalette("image")[0].id).toBe("add-image");
    expect(searchPdfPalette("save")[0].id).toBe("export");
    expect(searchPdfPalette("document info")[0].id).toBe("properties");
    expect(searchPdfPalette("nonexistent")).toEqual([]);
  });
});
