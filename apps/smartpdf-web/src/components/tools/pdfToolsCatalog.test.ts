import { describe, it, expect } from "vitest";
import {
  SMARTPDF_TOOLS,
  PDF_WORKFLOW_CATEGORIES,
} from "./pdfToolsCatalog";

describe("SmartPDF Tools Catalog", () => {
  it("defines all 5 canonical professional workflow categories", () => {
    const categoryIds = PDF_WORKFLOW_CATEGORIES.map((c) => c.id);
    expect(categoryIds).toContain("edit");
    expect(categoryIds).toContain("pages");
    expect(categoryIds).toContain("forms");
    expect(categoryIds).toContain("annotate");
    expect(categoryIds).toContain("security");
    expect(categoryIds.length).toBe(5);
  });

  it("ensures every tool has valid required metadata", () => {
    expect(SMARTPDF_TOOLS.length).toBeGreaterThanOrEqual(15);

    for (const tool of SMARTPDF_TOOLS) {
      expect(tool.id).toBeTruthy();
      expect(tool.title).toBeTruthy();
      expect(tool.description).toBeTruthy();
      expect(tool.category).toBeTruthy();
      expect(tool.categoryLabel).toBeTruthy();
      expect(tool.iconName).toBeTruthy();
      expect(["open_mode", "open_dialog", "custom"]).toContain(tool.actionType);
    }
  });

  it("ensures every workflow category contains at least two tools", () => {
    for (const cat of PDF_WORKFLOW_CATEGORIES) {
      const toolsInCat = SMARTPDF_TOOLS.filter((t) => t.category === cat.id);
      expect(toolsInCat.length).toBeGreaterThanOrEqual(2);
    }
  });

  it("assigns valid recommended modes to mode-based tools", () => {
    const modeTools = SMARTPDF_TOOLS.filter((t) => t.actionType === "open_mode");
    for (const tool of modeTools) {
      expect(tool.recommendedMode).toBeDefined();
      expect(["SELECT", "TEXT", "FILL_AND_SIGN", "ANNOTATE", "PAGES"]).toContain(
        tool.recommendedMode,
      );
    }
  });
});
