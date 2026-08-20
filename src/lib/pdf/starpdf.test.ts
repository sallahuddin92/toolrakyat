import { describe, expect, it } from "vitest";
import fs from "fs";
import path from "path";
import { StarPdfClient } from "./starpdf-client";

function loadTestAsset(filename: string): Uint8Array {
  const assetPath = path.resolve(process.cwd(), "test-assets", filename);
  const buffer = fs.readFileSync(assetPath);
  return new Uint8Array(buffer);
}

describe("StarPDF v0.6 WASM Client Runtime & Native Mutation", () => {
  it("retrieves engine version 0.6.0", async () => {
    const version = await StarPdfClient.getVersion();
    expect(version).toBe("0.6.0");
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

  it("extracts AcroForm fields and annotations from real form fixture", async () => {
    const bytes = loadTestAsset("smartpdf-form.pdf");
    const doc = await StarPdfClient.open(bytes);

    const fields = await doc.getFormFields();
    expect(fields.length).toBe(3);

    const textField = fields.find((f) => f.name === "full_name");
    expect(textField).toBeDefined();
    expect(textField?.field_type).toBe("text");
    expect(textField?.widgets.length).toBe(1);

    const checkField = fields.find((f) => f.name === "agree");
    expect(checkField).toBeDefined();
    expect(checkField?.field_type).toBe("checkbox");

    const annotations = await doc.getAnnotations(0);
    expect(annotations.length).toBeGreaterThanOrEqual(3);

    await doc.close();
  });

  it("performs native incremental mutation and exports valid roundtrip PDF", async () => {
    const bytes = loadTestAsset("smartpdf-form.pdf");
    const doc = await StarPdfClient.open(bytes);

    const fields = await doc.getFormFields();
    const textField = fields.find((f) => f.name === "full_name")!;
    const checkField = fields.find((f) => f.name === "agree")!;

    await doc.setTextField(textField.object_num, textField.object_gen, "Ahmad Albab");
    await doc.setCheckbox(checkField.object_num, checkField.object_gen, true);

    const mutatedBytes = await doc.exportIncremental();
    expect(mutatedBytes.length).toBeGreaterThan(bytes.length);

    await doc.close();

    // Reopen mutated document with StarPDF WASM client
    const reopenedDoc = await StarPdfClient.open(mutatedBytes);
    expect(await reopenedDoc.getPageCount()).toBe(1);
    expect(await reopenedDoc.validate()).toBe(true);

    const reopenedFields = await reopenedDoc.getFormFields();
    const updatedTextField = reopenedFields.find((f) => f.name === "full_name");
    expect(updatedTextField?.value).toBe("Ahmad Albab");

    const updatedCheckField = reopenedFields.find((f) => f.name === "agree");
    expect(updatedCheckField?.value).toBe("true");

    await reopenedDoc.close();
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
