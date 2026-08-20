import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { PDFDocument } from "pdf-lib";
import {
  inspectPdfDocument,
  updateAcroFormFields,
  validateExportedPdf,
  exportPdfDocument,
  generateExportFilename,
} from "./pdf-engine";
import { PdfParseError } from "./pdf-errors";

function loadTestAsset(filename: string): Uint8Array {
  const filePath = path.join(process.cwd(), "test-assets", filename);
  return new Uint8Array(fs.readFileSync(filePath));
}

describe("SmartPDF Core Engine (pdf-engine)", () => {
  describe("inspectPdfDocument", () => {
    it("loads a valid single-page document and reports correct metadata and pages", async () => {
      const bytes = loadTestAsset("edit-test.pdf");
      const result = await inspectPdfDocument(bytes, "edit-test.pdf", bytes.byteLength);

      expect(result.metadata.filename).toBe("edit-test.pdf");
      expect(result.metadata.pageCount).toBe(1);
      expect(result.pages.length).toBe(1);
      expect(result.pages[0]?.pageNumber).toBe(1);
      expect(result.pages[0]?.width).toBeGreaterThan(0);
      expect(result.pages[0]?.height).toBeGreaterThan(0);
      expect(result.fields.length).toBe(0);
    });

    it("loads a multi-page document and reports all pages", async () => {
      const bytes = loadTestAsset("multi-page.test.pdf");
      const result = await inspectPdfDocument(bytes, "multi-page.test.pdf", bytes.byteLength);

      expect(result.metadata.pageCount).toBe(2);
      expect(result.pages.length).toBe(2);
      expect(result.pages[0]?.pageNumber).toBe(1);
      expect(result.pages[1]?.pageNumber).toBe(2);
    });

    it("detects AcroForm fields, types, and values correctly", async () => {
      const bytes = loadTestAsset("smartpdf-form.pdf");
      const result = await inspectPdfDocument(bytes, "smartpdf-form.pdf", bytes.byteLength);

      expect(result.metadata.formFieldCount).toBe(3);
      expect(result.fields.length).toBe(3);

      const textField = result.fields.find((f) => f.name === "full_name");
      expect(textField).toBeDefined();
      expect(textField?.type).toBe("text");

      const checkboxField = result.fields.find((f) => f.name === "agree");
      expect(checkboxField).toBeDefined();
      expect(checkboxField?.type).toBe("checkbox");

      const radioField = result.fields.find((f) => f.name === "gender");
      expect(radioField).toBeDefined();
      expect(radioField?.type).toBe("radio");
    });

    it("throws a PdfParseError on corrupted or invalid PDF data", async () => {
      const bytes = loadTestAsset("invalid.pdf");
      await expect(
        inspectPdfDocument(bytes, "invalid.pdf", bytes.byteLength),
      ).rejects.toThrow(PdfParseError);
    });
  });

  describe("updateAcroFormFields & export", () => {
    it("updates text field and exports editable PDF preserving interactive form", async () => {
      const bytes = loadTestAsset("smartpdf-form.pdf");
      const updatedBytes = await updateAcroFormFields(
        bytes,
        {
          full_name: "Ahmad Albab",
          agree: true,
        },
        "editable",
      );

      // Verify the exported bytes load properly
      expect(updatedBytes.byteLength).toBeGreaterThan(0);
      const reloadedDoc = await PDFDocument.load(updatedBytes);
      expect(reloadedDoc.getPageCount()).toBe(1);

      const form = reloadedDoc.getForm();
      expect(form).toBeDefined();

      const textField = form.getTextField("full_name");
      expect(textField.getText()).toBe("Ahmad Albab");

      const checkBox = form.getCheckBox("agree");
      expect(checkBox.isChecked()).toBe(true);
    });

    it("flattens form fields in flattened mode", async () => {
      const bytes = loadTestAsset("smartpdf-form.pdf");
      const flattenedBytes = await updateAcroFormFields(
        bytes,
        {
          full_name: "Siti Nurhaliza",
          agree: false,
        },
        "flattened",
      );

      expect(flattenedBytes.byteLength).toBeGreaterThan(0);
      const reloadedDoc = await PDFDocument.load(flattenedBytes);
      expect(reloadedDoc.getPageCount()).toBe(1);

      // In a flattened PDF, form fields are converted to visual page content
      const form = reloadedDoc.getForm();
      const fields = form.getFields();
      expect(fields.length).toBe(0);
    });

    it("validates exported PDF structurally", async () => {
      const bytes = loadTestAsset("smartpdf-form.pdf");
      const isValid = await validateExportedPdf(bytes, 1);
      expect(isValid).toBe(true);
    });

    it("generates correct export filenames for modes", () => {
      expect(generateExportFilename("application.pdf", "editable")).toBe("application-edited.pdf");
      expect(generateExportFilename("invoice.pdf", "flattened")).toBe("invoice-flattened.pdf");
    });

    it("orchestrates full export pipeline successfully", async () => {
      const bytes = loadTestAsset("smartpdf-form.pdf");
      const result = await exportPdfDocument(
        bytes,
        "test-doc.pdf",
        { full_name: "John Doe" },
        "editable",
        1,
      );

      expect(result.filename).toBe("test-doc-edited.pdf");
      expect(result.mode).toBe("editable");
      expect(result.validated).toBe(true);
      expect(result.pdfBytes.byteLength).toBeGreaterThan(0);
    });
  });
});
