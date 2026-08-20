import { describe, expect, it } from "vitest";
import fs from "fs";
import path from "path";
import { StarPdfClient } from "./starpdf-client";

function loadTestAsset(filename: string): Uint8Array {
  const assetPath = path.resolve(process.cwd(), "test-assets", filename);
  const buffer = fs.readFileSync(assetPath);
  return new Uint8Array(buffer);
}

describe("StarPDF v0.5 WASM Client Runtime", () => {
  it("retrieves engine version", async () => {
    const version = await StarPdfClient.getVersion();
    expect(version).toBe("0.5.0");
  });

  it("creates minimal PDF and opens it", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("StarPDF WASM Test Document");
    expect(bytes.length).toBeGreaterThan(100);

    const doc = await StarPdfClient.open(bytes);
    const count = await doc.getPageCount();
    expect(count).toBe(1);

    const isValid = await doc.validate();
    expect(isValid).toBe(true);

    const pageText = await doc.extractPageText(0);
    expect(pageText.plain_text).toContain("StarPDF WASM Test Document");

    const searchHits = await doc.search("Test", { caseSensitive: false });
    expect(searchHits.length).toBe(1);
    expect(searchHits[0].matched_text).toBe("Test");
    expect(searchHits[0].boxes.length).toBe(1);

    await doc.close();
    expect(doc.isClosed).toBe(true);
    await expect(doc.getPageCount()).rejects.toThrow();
  });

  it("opens multi-page document and extracts text across all pages", async () => {
    const bytes = loadTestAsset("multi-page.test.pdf");
    const doc = await StarPdfClient.open(bytes);

    const count = await doc.getPageCount();
    expect(count).toBe(2);

    const allPages = await doc.extractAllText();
    expect(allPages.length).toBe(2);

    await doc.close();
  });

  it("opens form fixture and searches form labels", async () => {
    const bytes = loadTestAsset("smartpdf-form.pdf");
    const doc = await StarPdfClient.open(bytes);

    const count = await doc.getPageCount();
    expect(count).toBe(1);

    const info = await doc.getInfo();
    expect(info.is_valid).toBe(true);

    const pageText = await doc.extractPageText(0);
    expect(pageText.spans.length).toBeGreaterThan(0);

    await doc.close();
  });

  it("handles malformed PDF safely without crash", async () => {
    const bytes = loadTestAsset("invalid.pdf");
    await expect(StarPdfClient.open(bytes)).rejects.toThrow();
  });

  it("handles scanned PDF returning zero text without crash", async () => {
    const bytes = loadTestAsset("scanned-test.pdf");
    const doc = await StarPdfClient.open(bytes);

    const count = await doc.getPageCount();
    expect(count).toBe(1);

    const pageText = await doc.extractPageText(0);
    expect(pageText.spans.length).toBe(0);

    const hits = await doc.search("Anything", { caseSensitive: false });
    expect(hits.length).toBe(0);

    await doc.close();
  });
});
