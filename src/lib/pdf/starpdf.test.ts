import { describe, expect, it } from "vitest";
import fs from "fs";
import path from "path";
import { PDFCheckBox, PDFDocument, PDFTextField } from "pdf-lib";
import { StarPdfClient } from "./starpdf-client";

function loadTestAsset(filename: string): Uint8Array {
  const assetPath = path.resolve(process.cwd(), "test-assets", filename);
  const buffer = fs.readFileSync(assetPath);
  return new Uint8Array(buffer);
}

describe("StarPDF v0.7 WASM Client Runtime & Appearance Engine", () => {
  it("retrieves engine version 0.7.0", async () => {
    const version = await StarPdfClient.getVersion();
    expect(version).toBe("0.7.0");
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

  it("performs native incremental mutation and exports valid roundtrip PDF with regenerated appearances", async () => {
    const bytes = loadTestAsset("smartpdf-form.pdf");
    const doc = await StarPdfClient.open(bytes);

    const fields = await doc.getFormFields();
    const textField = fields.find((f) => f.name === "full_name")!;
    const checkField = fields.find((f) => f.name === "agree")!;

    await doc.setTextField(textField.object_num, textField.object_gen, "Ahmad Albab v0.7");
    await doc.setCheckbox(checkField.object_num, checkField.object_gen, true);

    const mutatedBytes = await doc.exportIncremental();
    expect(mutatedBytes.length).toBeGreaterThan(bytes.length);
    expect(mutatedBytes.slice(0, bytes.length)).toEqual(bytes);

    await doc.close();

    // Reopen mutated document with StarPDF WASM client
    const reopenedDoc = await StarPdfClient.open(mutatedBytes);
    expect(await reopenedDoc.getPageCount()).toBe(1);
    expect(await reopenedDoc.validate()).toBe(true);

    const reopenedFields = await reopenedDoc.getFormFields();
    const updatedTextField = reopenedFields.find((f) => f.name === "full_name");
    expect(updatedTextField?.value).toBe("Ahmad Albab v0.7");

    const updatedCheckField = reopenedFields.find((f) => f.name === "agree");
    expect(updatedCheckField?.value).toBe("true");

    // Differential semantic validation against pdf-lib.
    const referenceDoc = await PDFDocument.load(mutatedBytes, {
      updateMetadata: false,
    });
    const referenceForm = referenceDoc.getForm();
    const referenceText = referenceForm.getTextField("full_name");
    const referenceCheckbox = referenceForm.getCheckBox("agree");
    expect(referenceText).toBeInstanceOf(PDFTextField);
    expect(referenceText.getText()).toBe("Ahmad Albab v0.7");
    expect(referenceCheckbox).toBeInstanceOf(PDFCheckBox);
    expect(referenceCheckbox.isChecked()).toBe(true);

    await reopenedDoc.close();
  });

  it("adds, updates, and removes annotations natively with valid roundtrip", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("Annotation WASM Test");
    const doc = await StarPdfClient.open(bytes);

    // 1. Add FreeText annotation
    await doc.addAnnotation(0, {
      subtype: "FreeText",
      rect: [50, 100, 250, 160],
      contents: "Approved via WASM",
      font_size: 14,
      color: [0, 0, 0],
    });

    // 2. Add Square annotation
    await doc.addAnnotation(0, {
      subtype: "Square",
      rect: [100, 200, 300, 300],
      color: [1, 0, 0],
      fill_color: [0.9, 0.9, 0.9],
      border_width: 2,
    });

    const step1Bytes = await doc.exportIncremental();
    await doc.close();

    // Reopen and inspect added annotations
    const step1Doc = await StarPdfClient.open(step1Bytes);
    const annots1 = await step1Doc.getAnnotations(0);
    expect(annots1.length).toBe(2);
    expect(annots1[0].subtype).toBe("FreeText");
    expect(annots1[0].contents).toBe("Approved via WASM");
    expect(annots1[1].subtype).toBe("Square");

    // 3. Update the FreeText annotation
    const freeTextRef = annots1[0];
    await step1Doc.updateAnnotation(freeTextRef.object_num, freeTextRef.object_gen, {
      contents: "Revised WASM Approval",
    });

    const step2Bytes = await step1Doc.exportIncremental();
    await step1Doc.close();

    const step2Doc = await StarPdfClient.open(step2Bytes);
    const annots2 = await step2Doc.getAnnotations(0);
    expect(annots2.length).toBe(2);
    expect(annots2[0].contents).toBe("Revised WASM Approval");

    // 4. Remove the Square annotation
    const squareRef = annots2[1];
    await step2Doc.removeAnnotation(0, squareRef.object_num, squareRef.object_gen);

    const step3Bytes = await step2Doc.exportIncremental();
    await step2Doc.close();

    const step3Doc = await StarPdfClient.open(step3Bytes);
    const annots3 = await step3Doc.getAnnotations(0);
    expect(annots3.length).toBe(1);
    expect(annots3[0].subtype).toBe("FreeText");

    await step3Doc.close();
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
