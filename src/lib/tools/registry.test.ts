import { describe, it, expect } from "vitest";
import {
  tools,
  getToolByCategoryAndSlug,
  getToolsByCategory,
  getPopularTools,
  getCategoryLabel,
} from "./registry";

describe("Tool Registry", () => {
  it("contains valid tool definitions", () => {
    expect(tools.length).toBeGreaterThan(0);
    for (const tool of tools) {
      expect(tool.id).toBeTruthy();
      expect(tool.name).toBeTruthy();
      expect(tool.slug).toBeTruthy();
      expect(tool.categoryId).toBeTruthy();
      expect(tool.route).toBeTruthy();
    }
  });

  it("finds text word counter by category and slug", () => {
    const tool = getToolByCategoryAndSlug("text", "word-counter");
    expect(tool).toBeDefined();
    expect(tool?.id).toBe("text-word-counter");
    expect(tool?.isImplemented).toBe(true);
  });

  it("returns undefined for non-existent category or slug", () => {
    expect(getToolByCategoryAndSlug("text", "non-existent-slug")).toBeUndefined();
    expect(getToolByCategoryAndSlug("pdf", "non-existent-slug")).toBeUndefined();
  });

  it("filters tools by category", () => {
    const pdfTools = getToolsByCategory("pdf");
    expect(pdfTools.length).toBeGreaterThan(0);
    expect(pdfTools.every((t) => t.categoryId === "pdf")).toBe(true);
  });

  it("returns popular tools", () => {
    const popular = getPopularTools(5);
    expect(popular.length).toBeLessThanOrEqual(5);
    expect(popular.every((t) => t.isPopular)).toBe(true);
  });

  it("resolves category labels", () => {
    expect(getCategoryLabel("pdf")).toBe("PDF");
    expect(getCategoryLabel("text")).toBe("Text");
    expect(getCategoryLabel("akaunkemas")).toBe("AkaunKemas");
  });
});
