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

function loadComplexFixture(filename: string): Uint8Array {
  const fixturePath = path.resolve(
    process.cwd(),
    "engine/starpdf/tests/fixtures/v0_11_complex",
    filename,
  );
  return new Uint8Array(fs.readFileSync(fixturePath));
}

describe("StarPDF v0.12 WASM Client Runtime & Preservation Engine", () => {
  it("retrieves engine version 0.12.1", async () => {
    const version = await StarPdfClient.getVersion();
    expect(version).toBe("0.12.1");
  });

  it("reports signed document metadata without claiming verification", async () => {
    const doc = await StarPdfClient.open(loadComplexFixture("synthetic-signed-valid.pdf"));
    const security = await doc.getSecurityInfo();
    expect(security.signature_state).toBe("SIGNED_WITH_BYTE_RANGE");
    expect(security.signature_count).toBe(1);
    expect(security.cryptographic_verification).toBe("NOT_PERFORMED");
    expect(security.mutation_allowed).toBe(true);
    await doc.close();
  });

  it("detects encrypted metadata and refuses mutation explicitly", async () => {
    const doc = await StarPdfClient.open(loadComplexFixture("synthetic-encrypted-standard.pdf"));
    const security = await doc.getSecurityInfo();
    expect(security.encryption_state).toBe("STANDARD_SECURITY_DETECTED");
    expect(security.mutation_allowed).toBe(false);
    await doc.addAnnotation(0, {
      subtype: "Square",
      rect: [10, 10, 20, 20],
    });
    await expect(doc.exportIncremental()).rejects.toThrow(
      "ENCRYPTED_DOCUMENT_MUTATION_UNSUPPORTED",
    );
    await doc.close();
  });

  it("leaves ordinary PDFs unrestricted", async () => {
    const doc = await StarPdfClient.open(await StarPdfClient.createMinimalPdf("ordinary"));
    const security = await doc.getSecurityInfo();
    expect(security.signature_state).toBe("UNSIGNED");
    expect(security.encryption_state).toBe("NOT_ENCRYPTED");
    expect(security.mutation_allowed).toBe(true);
    await doc.close();
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

    await doc.setTextField(textField.object_num, textField.object_gen, "Ahmad Albab v0.9");
    await doc.setCheckbox(checkField.object_num, checkField.object_gen, true);

    const mutatedBytes = await doc.exportIncremental();
    expect(mutatedBytes.length).toBeGreaterThan(bytes.length);
    expect(mutatedBytes.slice(0, bytes.length)).toEqual(bytes);
    expect(await doc.getGlyphMappingQuality()).toBe("EXACT");

    await doc.close();

    // Reopen mutated document with StarPDF WASM client
    const reopenedDoc = await StarPdfClient.open(mutatedBytes);
    expect(await reopenedDoc.getPageCount()).toBe(1);
    expect(await reopenedDoc.validate()).toBe(true);

    const reopenedFields = await reopenedDoc.getFormFields();
    const updatedTextField = reopenedFields.find((f) => f.name === "full_name");
    expect(updatedTextField?.value).toBe("Ahmad Albab v0.9");

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
    expect(referenceText.getText()).toBe("Ahmad Albab v0.9");
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
    expect(await step1Doc.getAppearanceStatus()).toBe("AP_REGENERATED");
    expect(await step1Doc.getGlyphMappingQuality()).toBe("NOT_APPLICABLE");
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

  it("keeps WinAnsi FreeText fast and re-plans adaptive multilingual annotation edits", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("FreeText script policy");
    const doc = await StarPdfClient.open(bytes);
    const supported = "Café di Kuala Lumpur";
    await doc.addAnnotation(0, {
      subtype: "FreeText",
      rect: [50, 100, 280, 160],
      contents: supported,
      font_size: 14,
      color: [0, 0, 0],
    });
    const output = await doc.exportIncremental();
    const winAnsiAppearance = Buffer.from("436166E9206469204B75616C61204C756D707572");
    expect(Buffer.from(output).includes(winAnsiAppearance)).toBe(true);
    await doc.close();

    let reopened = await StarPdfClient.open(output);
    let [annotation] = await reopened.getAnnotations(0);
    expect(annotation.contents).toBe(supported);
    for (const value of ["توليس جاوي ڤ چ ڠ ڽ", "中文測試", "Latin العربية 中文"]) {
      await reopened.updateAnnotation(annotation.object_num, annotation.object_gen, {
        contents: value,
      });
      const adaptiveOutput = await reopened.exportIncremental();
      expect(Buffer.from(adaptiveOutput).includes(Buffer.from("/Subtype /Type0"))).toBe(true);
      expect(Buffer.from(adaptiveOutput).includes(Buffer.from("/Encoding /Identity-H"))).toBe(true);
      expect(Buffer.from(adaptiveOutput).includes(Buffer.from("/ToUnicode"))).toBe(true);
      await reopened.close();

      reopened = await StarPdfClient.open(adaptiveOutput);
      [annotation] = await reopened.getAnnotations(0);
      expect(annotation.contents).toBe(value);
    }

    const finalLatin = "Résumé accepted after multilingual edits";
    await reopened.updateAnnotation(annotation.object_num, annotation.object_gen, {
      contents: finalLatin,
    });
    const finalOutput = await reopened.exportIncremental();
    await reopened.close();

    const finalDoc = await StarPdfClient.open(finalOutput);
    expect((await finalDoc.getAnnotations(0))[0].contents).toBe(finalLatin);
    await finalDoc.close();
  });

  it("creates and reopens a Line annotation with supported endings and appearance status", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("Line WASM Test");
    const doc = await StarPdfClient.open(bytes);
    await doc.addAnnotation(0, {
      subtype: "Line",
      rect: [40, 80, 260, 160],
      line_points: [50, 90, 250, 150],
      line_endings: ["OpenArrow", "ClosedArrow"],
      contents: "Measured line",
      color: [0.1, 0.2, 0.8],
      fill_color: [0.9, 0.2, 0.2],
      border_width: 3,
    });
    const output = await doc.exportIncremental();
    expect(await doc.getAppearanceStatus()).toBe("AP_REGENERATED");
    await doc.close();

    const reopened = await StarPdfClient.open(output);
    const annotations = await reopened.getAnnotations(0);
    const line = annotations.find((annotation) => annotation.subtype === "Line");
    expect(line?.line_points).toEqual([50, 90, 250, 150]);
    expect(line?.line_endings).toEqual(["OpenArrow", "ClosedArrow"]);
    expect(line?.border_width).toBe(3);
    expect(line?.interior_color).toEqual([0.9, 0.2, 0.2]);
    await reopened.close();
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

  it("performs typed page operations and keeps generated outputs reopenable", async () => {
    const original = loadTestAsset("multi-page.test.pdf");
    const doc = await StarPdfClient.open(original);

    const moved = await doc.movePage(0, 1);
    expect(moved.slice(0, original.length)).toEqual(original);
    expect(await doc.getPageCount()).toBe(2);

    const duplicated = await doc.duplicatePage(0, 1);
    expect(duplicated.length).toBeGreaterThan(0);
    expect(await doc.getPageCount()).toBe(3);

    const blankInserted = await doc.insertBlankPage(1, 612, 792, 90);
    expect(blankInserted.slice(0, duplicated.length)).toEqual(duplicated);
    expect(await doc.getPageCount()).toBe(4);

    const deleted = await doc.deletePage(1);
    expect(deleted.slice(0, blankInserted.length)).toEqual(blankInserted);
    expect(await doc.getPageCount()).toBe(3);

    const extracted = await doc.extractPages([2, 0, 2]);
    const extractedDoc = await StarPdfClient.open(extracted);
    expect(await extractedDoc.getPageCount()).toBe(3);
    expect(await extractedDoc.validate()).toBe(true);
    expect(await doc.getPageCount()).toBe(3);

    await extractedDoc.close();
    await doc.close();
  });

  it("refuses a page operation while field mutations are pending", async () => {
    const doc = await StarPdfClient.open(loadTestAsset("smartpdf-form.pdf"));
    const field = (await doc.getFormFields()).find((candidate) => candidate.name === "full_name")!;
    await doc.setTextField(field.object_num, field.object_gen, "Pending page operation");
    await expect(doc.insertBlankPage(1, 612, 792)).rejects.toThrow("pending");
    await doc.close();
  });

  it("merges, imports, and splits documents through synchronized WASM bindings", async () => {
    const first = loadTestAsset("multi-page.test.pdf");
    const second = await StarPdfClient.createMinimalPdf("SECOND-DOCUMENT");
    const merged = await StarPdfClient.mergeDocuments([first, second], [
      { documentIndex: 1, pageIndex: 0 },
      { documentIndex: 0, pageIndex: 1 },
      { documentIndex: 1, pageIndex: 0 },
    ]);
    const mergedDoc = await StarPdfClient.open(merged);
    expect(await mergedDoc.getPageCount()).toBe(3);
    expect((await mergedDoc.extractPageText(0)).plain_text).toContain("SECOND-DOCUMENT");

    const parts = await mergedDoc.splitDocument([
      { start: 0, endExclusive: 1 },
      { start: 1, endExclusive: 3 },
    ]);
    expect(parts).toHaveLength(2);
    const firstPart = await StarPdfClient.open(parts[0]);
    const secondPart = await StarPdfClient.open(parts[1]);
    expect(await firstPart.getPageCount()).toBe(1);
    expect(await secondPart.getPageCount()).toBe(2);

    const imported = await firstPart.insertImportedPage(second, 0, 1);
    const importedDoc = await StarPdfClient.open(imported);
    expect(await importedDoc.getPageCount()).toBe(2);

    await importedDoc.close();
    await secondPart.close();
    await firstPart.close();
    await mergedDoc.close();
  });

  it("handles malformed PDF safely without crash", async () => {
    const bytes = loadTestAsset("invalid.pdf");
    await expect(StarPdfClient.open(bytes)).rejects.toThrow();
  });

  it("extracts text spans with structural identity and editability status", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("StarPDF Native Text");
    const doc = await StarPdfClient.open(bytes);

    const pageText = await doc.extractPageText(0);
    expect(pageText.spans.length).toBe(1);
    const span = pageText.spans[0];
    expect(span.text).toBe("StarPDF Native Text");
    expect(span.span_id).toMatch(/^p0_s\d+_i\d+_o\d+$/);
    expect(span.is_editable).toBe(true);
    expect(span.editability_code).toBe("EDITABLE_NATIVE_TEXT");

    const editability = await doc.getTextEditability(0, span.span_id);
    expect(editability.span_id).toBe(span.span_id);
    expect(editability.is_editable).toBe(true);

    await doc.close();
  });

  it("replaces existing text natively and verifies roundtrip search", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("Original Alpha Text");
    const doc = await StarPdfClient.open(bytes);

    const pageText = await doc.extractPageText(0);
    const span = pageText.spans[0];
    expect(span.text).toBe("Original Alpha Text");

    const result = await doc.replaceText(0, span.span_id, "Replaced Omega Text");
    expect(result.success).toBe(true);
    expect(["EXACT_FIT", "FIT_WITHIN_ORIGINAL_BOX", "WIDTH_CHANGED"]).toContain(result.layout_result);

    const updatedBytes = await doc.exportIncremental();
    expect(updatedBytes.length).toBeGreaterThan(bytes.length);

    // Reopen modified PDF
    const reopened = await StarPdfClient.open(updatedBytes);
    const reopenedText = await reopened.extractPageText(0);
    expect(reopenedText.plain_text).toContain("Replaced Omega Text");
    expect(reopenedText.plain_text).not.toContain("Original Alpha Text");

    const searchHits = await reopened.search("Omega", { caseSensitive: false });
    expect(searchHits.length).toBe(1);
    expect(searchHits[0].matched_text).toBe("Omega");

    await reopened.close();
    await doc.close();
  });

  it("shapes, embeds, and reopens Arabic replacement text", async () => {
    const bytes = await StarPdfClient.createMinimalPdf("Ascii Title");
    const doc = await StarPdfClient.open(bytes);

    const pageText = await doc.extractPageText(0);
    const span = pageText.spans[0];

    const replacement = "العربية";
    const result = await doc.replaceText(0, span.span_id, replacement);
    expect(result.success).toBe(true);

    const exported = await doc.exportIncremental();
    const reopened = await StarPdfClient.open(exported);
    const reopenedText = await reopened.extractPageText(0);
    expect(reopenedText.plain_text).toContain(replacement);
    const hits = await reopened.search(replacement, { caseSensitive: true });
    expect(hits).toHaveLength(1);

    await reopened.close();
    await doc.close();
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

  it("adds, enumerates, replaces and removes images natively", async () => {
    const minimalBytes = await StarPdfClient.createMinimalPdf("Image Test Document");
    const doc = await StarPdfClient.open(minimalBytes);

    // Initial enumeration
    let images = await doc.enumerateImages(0);
    expect(images.length).toBe(0);

    // Create a minimal 2x2 JPEG
    const testJpeg1 = new Uint8Array([
      0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00,
      0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00,
      ...new Array(64).fill(16),
      0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x02, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00,
      0x03, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01,
      0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
      0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00,
      0x3F, 0x00, 0x10, 0x20, 0x30, 0x7F, 0xFF, 0xD9,
    ]);

    const addRes = await doc.addImage(0, testJpeg1, 50, 100, 120, 80);
    expect(addRes.success).toBe(true);

    const docBytesWithImg = await doc.exportIncremental();
    await doc.close();

    // Reopen and verify image enumerated
    const doc2 = await StarPdfClient.open(docBytesWithImg);
    images = await doc2.enumerateImages(0);
    expect(images.length).toBe(1);
    expect(images[0].width).toBe(2);
    expect(images[0].height).toBe(2);
    expect(images[0].color_space).toBe("DeviceRGB");

    // Replace the image
    const testJpeg2 = new Uint8Array([
      0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00,
      0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00,
      ...new Array(64).fill(16),
      0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x02, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00,
      0x03, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01,
      0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
      0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00,
      0x3F, 0x00, 0xFF, 0x00, 0x00, 0x7F, 0xFF, 0xD9,
    ]);

    const replaceRes = await doc2.replaceImage(0, images[0].image_id, testJpeg2, true);
    expect(replaceRes.success).toBe(true);

    const docBytesReplaced = await doc2.exportIncremental();
    await doc2.close();

    // Reopen and remove image
    const doc3 = await StarPdfClient.open(docBytesReplaced);
    const images3 = await doc3.enumerateImages(0);
    expect(images3.length).toBe(1);

    const removeRes = await doc3.removeImage(0, images3[0].image_id);
    expect(removeRes.success).toBe(true);

    const docBytesRemoved = await doc3.exportIncremental();
    await doc3.close();

    const doc4 = await StarPdfClient.open(docBytesRemoved);
    const images4 = await doc4.enumerateImages(0);
    expect(images4.length).toBe(0);
    await doc4.close();
  });

  it("supports bounded vector graphic operations (add, enumerate, update, delete)", async () => {
    const minimalBytes = await StarPdfClient.createMinimalPdf("Vector Test Doc");
    const doc = await StarPdfClient.open(minimalBytes);

    // Initial state: no vector shapes
    const initialGraphics = await doc.enumerateGraphics(0);
    expect(initialGraphics.length).toBe(0);

    // 1. Add Rectangle
    const addRectRes = await doc.addRectangle({
      page_index: 0,
      x: 50,
      y: 50,
      width: 120,
      height: 60,
      stroke_color_rgb: [0, 0, 1],
      fill_color_rgb: [0.8, 0.9, 1],
      line_width: 2.0,
      is_stroked: true,
      is_filled: true,
    });
    expect(addRectRes.success).toBe(true);

    // 2. Add Line
    const addLineRes = await doc.addLine({
      page_index: 0,
      x1: 100,
      y1: 200,
      x2: 300,
      y2: 200,
      stroke_color_rgb: [1, 0, 0],
      line_width: 3.0,
    });
    expect(addLineRes.success).toBe(true);

    const docWithVectors = await doc.exportIncremental();
    await doc.close();

    // 3. Reopen and verify enumeration
    const doc2 = await StarPdfClient.open(docWithVectors);
    const graphics = await doc2.enumerateGraphics(0);
    expect(graphics.length).toBe(2);

    const rect = graphics.find((g) => g.graphic_type === "Rectangle");
    expect(rect).toBeDefined();
    expect(rect?.is_stroked).toBe(true);
    expect(rect?.is_filled).toBe(true);
    expect(rect?.is_editable).toBe(true);

    const line = graphics.find((g) => g.graphic_type === "Line");
    expect(line).toBeDefined();
    expect(line?.line_width).toBe(3.0);

    // 4. Update the rectangle's line width and stroke color
    const updateRes = await doc2.updateGraphic({
      page_index: 0,
      graphic_id: rect!.graphic_id,
      line_width: 5.0,
      stroke_color_rgb: [0, 1, 0],
      clone_if_shared: true,
    });
    expect(updateRes.success).toBe(true);

    const docAfterUpdate = await doc2.exportIncremental();
    await doc2.close();

    // 5. Reopen and delete the line
    const doc3 = await StarPdfClient.open(docAfterUpdate);
    const graphics3 = await doc3.enumerateGraphics(0);
    const line3 = graphics3.find((g) => g.graphic_type === "Line");
    expect(line3).toBeDefined();

    const deleteRes = await doc3.deleteGraphic({
      page_index: 0,
      graphic_id: line3!.graphic_id,
      clone_if_shared: true,
    });
    expect(deleteRes.success).toBe(true);

    const docFinalBytes = await doc3.exportIncremental();
    await doc3.close();

    // 6. Final verification
    const doc4 = await StarPdfClient.open(docFinalBytes);
    const finalGraphics = await doc4.enumerateGraphics(0);
    expect(finalGraphics.length).toBe(1);
    expect(finalGraphics[0].graphic_type).toBe("Rectangle");
    expect(finalGraphics[0].line_width).toBe(5.0);
    await doc4.close();
  });

  it("qualifies large multi-page document operations and 20-cycle retention", async () => {
    // Generate a deterministic 30-page PDF in TypeScript
    const generateMultiPagePdf = (numPages: number): Uint8Array => {
      let pdf = "%PDF-1.7\n%StarPDF\n";
      const offsets: number[] = [0];

      const o1 = pdf.length;
      offsets.push(o1);
      pdf += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

      const o2 = pdf.length;
      offsets.push(o2);
      let kids = "";
      for (let i = 0; i < numPages; i++) {
        kids += `${3 + i * 2} 0 R `;
      }
      pdf += `2 0 obj\n<< /Type /Pages /Kids [${kids}] /Count ${numPages} >>\nendobj\n`;

      const fontObjNum = 3 + numPages * 2;
      for (let i = 0; i < numPages; i++) {
        const pageObjNum = 3 + i * 2;
        const contentObjNum = 4 + i * 2;

        offsets.push(pdf.length);
        pdf += `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents ${contentObjNum} 0 R /Resources << /Font << /F1 ${fontObjNum} 0 R >> >> >>\nendobj\n`;

        let streamContent = "BT\n/F1 12 Tf\n";
        for (let l = 0; l < 10; l++) {
          const y = 700 - l * 20;
          streamContent += `50 ${y} Td (Page ${i + 1} Line ${l + 1}: StarPDF qualification payload #${(i * 10 + l) % 1000}) Tj\n`;
        }
        streamContent += "ET\n";

        offsets.push(pdf.length);
        pdf += `${contentObjNum} 0 obj\n<< /Length ${streamContent.length} >>\nstream\n${streamContent}\nendstream\nendobj\n`;
      }

      offsets.push(pdf.length);
      pdf += `${fontObjNum} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n`;

      const xrefOffset = pdf.length;
      pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
      for (let i = 1; i < offsets.length; i++) {
        pdf += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
      }
      pdf += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

      return new TextEncoder().encode(pdf);
    };

    const doc30Bytes = generateMultiPagePdf(30);
    const doc = await StarPdfClient.open(doc30Bytes);
    const count = await doc.getPageCount();
    expect(count).toBe(30);

    // 1. Text extraction across pages
    const p15Text = await doc.extractPageText(14);
    expect(p15Text.plain_text).toContain("Page 15 Line 1");
    expect(p15Text.spans.length).toBe(10);

    // 2. Full Search
    const searchHits = await doc.search("qualification", { caseSensitive: false });
    expect(searchHits.length).toBe(300);

    // 3. Vector addition on page 15
    const addRectRes = await doc.addRectangle({
      page_index: 14,
      x: 100,
      y: 100,
      width: 150,
      height: 80,
      stroke_color_rgb: [1, 0, 0],
      fill_color_rgb: [0.9, 0.9, 1],
      line_width: 2.0,
      is_stroked: true,
      is_filled: true,
    });
    expect(addRectRes.success).toBe(true);

    const docMutated = await doc.exportIncremental();
    await doc.close();

    // 4. Verify reopen and shape presence
    const reopenedDoc = await StarPdfClient.open(docMutated);
    const p15Shapes = await reopenedDoc.enumerateGraphics(14);
    expect(p15Shapes.length).toBe(1);
    expect(p15Shapes[0].graphic_type).toBe("Rectangle");

    // 5. 20-cycle repeated open/edit/save/close test
    let currentBytes = doc30Bytes;
    for (let c = 1; c <= 20; c++) {
      const cycleDoc = await StarPdfClient.open(currentBytes);
      const text = await cycleDoc.extractPageText(0);
      const span = text.spans[0];
      await cycleDoc.replaceText(0, span.span_id, `CYCLE_${c}_MUTATION`);
      currentBytes = await cycleDoc.exportIncremental();
      await cycleDoc.close();
      expect(cycleDoc.isClosed).toBe(true);
    }

    const finalDoc = await StarPdfClient.open(currentBytes);
    const finalP0Text = await finalDoc.extractPageText(0);
    expect(finalP0Text.plain_text).toContain("CYCLE_20_MUTATION");
    await finalDoc.close();
  });

  it("v0.17: rejects stale document handles after close", async () => {
    const doc = await StarPdfClient.open(await StarPdfClient.createMinimalPdf("Stale Handle Test"));
    expect(doc.isClosed).toBe(false);
    await doc.close();
    expect(doc.isClosed).toBe(true);

    await expect(doc.getPageCount()).rejects.toThrow(/closed/i);
    await expect(doc.extractPageText(0)).rejects.toThrow(/closed/i);
    await expect(doc.search("test")).rejects.toThrow(/closed/i);
    await expect(doc.exportIncremental()).rejects.toThrow(/closed/i);
  });

  it("v0.17: qualifies 100-page document load, search, extraction, and page extraction in client", async () => {
    // Generate deterministic 100-page PDF
    let pdf = "%PDF-1.7\n%StarPDF\n";
    const offsets: number[] = [0];
    const numPages = 100;

    offsets.push(pdf.length);
    pdf += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

    offsets.push(pdf.length);
    let kids = "";
    for (let i = 0; i < numPages; i++) {
      kids += `${3 + i * 2} 0 R `;
    }
    pdf += `2 0 obj\n<< /Type /Pages /Kids [${kids}] /Count ${numPages} >>\nendobj\n`;

    const fontObjNum = 3 + numPages * 2;
    for (let i = 0; i < numPages; i++) {
      const pageObjNum = 3 + i * 2;
      const contentObjNum = 4 + i * 2;

      offsets.push(pdf.length);
      pdf += `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents ${contentObjNum} 0 R /Resources << /Font << /F1 ${fontObjNum} 0 R >> >> >>\nendobj\n`;

      let streamContent = "BT\n/F1 11 Tf\n";
      for (let l = 0; l < 5; l++) {
        const y = 720 - l * 20;
        streamContent += `50 ${y} Td (Page ${i + 1} Line ${l + 1}: StarPDF v0.17 qualification benchmark text) Tj\n`;
      }
      streamContent += "ET\n";

      offsets.push(pdf.length);
      pdf += `${contentObjNum} 0 obj\n<< /Length ${streamContent.length} >>\nstream\n${streamContent}\nendstream\nendobj\n`;
    }

    offsets.push(pdf.length);
    pdf += `${fontObjNum} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n`;

    const xrefOffset = pdf.length;
    pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
    for (let i = 1; i < offsets.length; i++) {
      pdf += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    pdf += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

    const doc100Bytes = new TextEncoder().encode(pdf);
    const doc = await StarPdfClient.open(doc100Bytes);

    const count = await doc.getPageCount();
    expect(count).toBe(100);

    const p50Text = await doc.extractPageText(49);
    expect(p50Text.plain_text).toContain("Page 50 Line 1");
    expect(p50Text.spans.length).toBe(5);

    const searchHits = await doc.search("benchmark", { caseSensitive: false });
    expect(searchHits.length).toBe(500);

    // Extract pages 0, 25, 50, 75, 99
    const extractedBytes = await doc.extractPages([0, 25, 50, 75, 99]);
    expect(extractedBytes.length).toBeGreaterThan(100);

    const extractedDoc = await StarPdfClient.open(extractedBytes);
    expect(await extractedDoc.getPageCount()).toBe(5);
    await extractedDoc.close();

    await doc.close();
  });

  it("v0.18: opens multi-producer documents and exposes recovery information", async () => {
    // Generate valid producer PDF
    let pdf = "%PDF-1.7\n%StarPDF\n";
    const offsets: number[] = [0];

    offsets.push(pdf.length);
    pdf += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

    offsets.push(pdf.length);
    pdf += "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";

    offsets.push(pdf.length);
    pdf += "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n";

    const content = "BT\n/F1 12 Tf\n50 700 Td (Produced by Skia/PDF m120 Google Chrome) Tj\nET\n";
    offsets.push(pdf.length);
    pdf += `4 0 obj\n<< /Length ${content.length} >>\nstream\n${content}endstream\nendobj\n`;

    offsets.push(pdf.length);
    pdf += "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";

    const xrefOffset = pdf.length;
    pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
    for (let i = 1; i < offsets.length; i++) {
      pdf += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    pdf += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

    const doc = await StarPdfClient.open(new TextEncoder().encode(pdf));
    const info = await doc.getInfo();
    expect(info.page_count).toBe(1);
    expect(info.is_valid).toBe(true);

    const text = await doc.extractPageText(0);
    expect(text.plain_text).toContain("Skia/PDF");

    await doc.close();
  });

  it("v0.18: recovers PDF with preceding UTF-8 BOM and logs compatibility event", async () => {
    let pdf = "%PDF-1.7\n%StarPDF\n";
    const offsets: number[] = [0];

    offsets.push(pdf.length);
    pdf += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

    offsets.push(pdf.length);
    pdf += "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";

    offsets.push(pdf.length);
    pdf += "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n";

    const content = "BT\n/F1 12 Tf\n50 700 Td (BOM Recovered PDF) Tj\nET\n";
    offsets.push(pdf.length);
    pdf += `4 0 obj\n<< /Length ${content.length} >>\nstream\n${content}endstream\nendobj\n`;

    offsets.push(pdf.length);
    pdf += "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";

    const xrefOffset = pdf.length;
    pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
    for (let i = 1; i < offsets.length; i++) {
      pdf += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    pdf += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

    // Prepend UTF-8 BOM (\xEF\xBB\xBF)
    const bomBytes = new Uint8Array([0xef, 0xbb, 0xbf]);
    const pdfBytes = new TextEncoder().encode(pdf);
    const fullBytes = new Uint8Array(bomBytes.length + pdfBytes.length);
    fullBytes.set(bomBytes, 0);
    fullBytes.set(pdfBytes, bomBytes.length);

    const doc = await StarPdfClient.open(fullBytes);
    const info = await doc.getInfo();
    expect(info.page_count).toBe(1);
    expect(info.is_valid).toBe(true);

    const text = await doc.extractPageText(0);
    expect(text.plain_text).toContain("BOM Recovered PDF");

    await doc.close();
  });

  it("exposes bounded malformed Prev recovery across open, document, and security metadata", async () => {
    const original = await StarPdfClient.createMinimalPdf("Recovery metadata");
    const decoder = new TextDecoder();
    const encoder = new TextEncoder();
    const source = decoder.decode(original);
    const rootLine = "  /Root 1 0 R\n";
    const insertion = source.lastIndexOf(rootLine) + rootLine.length;
    const malformed = encoder.encode(
      `${source.slice(0, insertion)}  /Prev 9999999999\n${source.slice(insertion)}`,
    );

    const document = await StarPdfClient.open(malformed);
    expect(document.xrefStatus).toBe("RECOVERED_MALFORMED_PREV");
    await expect(document.getInfo()).resolves.toMatchObject({
      xref_status: "RECOVERED_MALFORMED_PREV",
    });
    await expect(document.getSecurityInfo()).resolves.toMatchObject({
      xref_status: "RECOVERED_MALFORMED_PREV",
    });
    await document.close();
  });

  it("exposes typed UNRECOVERABLE status when open cannot prove a coherent graph", async () => {
    const original = await StarPdfClient.createMinimalPdf("Unrecoverable metadata");
    const decoder = new TextDecoder();
    const encoder = new TextEncoder();
    const source = decoder.decode(original);
    const marker = "startxref\n";
    const valueStart = source.lastIndexOf(marker) + marker.length;
    const valueEnd = source.indexOf("\n", valueStart);
    const malformed = encoder.encode(
      `${source.slice(0, valueStart)}${"0".repeat(valueEnd - valueStart)}${source.slice(valueEnd)}`,
    );

    await expect(StarPdfClient.open(malformed)).rejects.toMatchObject({
      xref_status: "UNRECOVERABLE",
      message: "StarPDF could not establish a coherent document graph for native editing.",
    });
  });
});
