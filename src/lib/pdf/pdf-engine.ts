import {
  PDFDocument,
  PDFTextField,
  PDFCheckBox,
  PDFRadioGroup,
  PDFDropdown,
  PDFOptionList,
} from "pdf-lib";
import {
  type AcroFormField,
  type AcroFormFieldType,
  type DocumentInspectionResult,
  type ExportMode,
  type ExportResult,
  type PdfDocumentMetadata,
  type PdfPageInfo,
} from "./pdf-types";
import {
  PdfEncryptedError,
  PdfExportError,
  PdfParseError,
  PdfValidationError,
} from "./pdf-errors";

/**
 * Inspects a PDF document and extracts metadata, page information, and AcroForm fields.
 */
export async function inspectPdfDocument(
  bytes: Uint8Array,
  filename: string,
  fileSize: number,
): Promise<DocumentInspectionResult> {
  let doc: PDFDocument;
  try {
    doc = await PDFDocument.load(bytes, {
      ignoreEncryption: false,
      updateMetadata: false,
    });
  } catch (err: unknown) {
    const errorMsg = err instanceof Error ? err.message : String(err);
    if (
      errorMsg.includes("encrypted") ||
      errorMsg.includes("password") ||
      errorMsg.includes("Password")
    ) {
      throw new PdfEncryptedError();
    }
    throw new PdfParseError(`Failed to parse PDF document: ${errorMsg}`);
  }

  const pageCount = doc.getPageCount();
  const pages: PdfPageInfo[] = [];

  for (let i = 0; i < pageCount; i++) {
    const page = doc.getPage(i);
    const { width, height } = page.getSize();
    const rotation = page.getRotation().angle;
    pages.push({
      pageNumber: i + 1,
      width,
      height,
      rotation,
    });
  }

  const fields: AcroFormField[] = [];
  try {
    const form = doc.getForm();
    const rawFields = form.getFields();

    for (const field of rawFields) {
      const name = field.getName();
      let type: AcroFormFieldType = "unsupported";
      let value: string | boolean | string[] = "";
      let options: string[] | undefined;

      const isReadOnly = field.isReadOnly();
      const isRequired = field.isRequired();

      if (field instanceof PDFTextField) {
        type = "text";
        value = field.getText() ?? "";
      } else if (field instanceof PDFCheckBox) {
        type = "checkbox";
        value = field.isChecked();
      } else if (field instanceof PDFRadioGroup) {
        type = "radio";
        options = field.getOptions();
        value = field.getSelected() ?? "";
      } else if (field instanceof PDFDropdown) {
        type = "dropdown";
        options = field.getOptions();
        const selected = field.getSelected();
        value = selected && selected.length > 0 ? selected[0]! : "";
      } else if (field instanceof PDFOptionList) {
        type = "optionList";
        options = field.getOptions();
        value = field.getSelected() ?? [];
      }

      let rect: { x: number; y: number; width: number; height: number } | undefined;
      try {
        const widgets = field.acroField.getWidgets();
        if (widgets.length > 0) {
          const r = widgets[0].getRectangle();
          rect = { x: r.x, y: r.y, width: r.width, height: r.height };
        }
      } catch {
        // Optional widget rectangle
      }

      fields.push({
        name,
        type,
        value,
        originalValue: Array.isArray(value) ? [...value] : value,
        options,
        isReadOnly,
        isRequired,
        rect,
      });
    }
  } catch {
    // If the PDF does not have an interactive AcroForm dict, doc.getForm() may throw or return empty.
    // In that case, fields remain empty.
  }

  const metadata: PdfDocumentMetadata = {
    filename,
    fileSize,
    pageCount,
    title: doc.getTitle() || undefined,
    author: doc.getAuthor() || undefined,
    subject: doc.getSubject() || undefined,
    producer: doc.getProducer() || undefined,
    creationDate: doc.getCreationDate() || undefined,
    modificationDate: doc.getModificationDate() || undefined,
    formFieldCount: fields.length,
  };

  return {
    metadata,
    pages,
    fields,
  };
}

/**
 * Updates AcroForm field values in the source PDF and produces an exported Uint8Array.
 * If mode is 'flattened', interactive widgets are converted to visual page content.
 */
export async function updateAcroFormFields(
  sourceBytes: Uint8Array,
  fieldValues: Record<string, string | boolean | string[]>,
  mode: ExportMode,
): Promise<Uint8Array> {
  let doc: PDFDocument;
  try {
    doc = await PDFDocument.load(sourceBytes, {
      ignoreEncryption: true,
      updateMetadata: false,
    });
  } catch (err: unknown) {
    throw new PdfExportError(
      `Failed to load source PDF for export: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  try {
    let form = null;
    try {
      form = doc.getForm();
    } catch {
      // Document has no AcroForm dictionary; exporting non-form PDF
      form = null;
    }

    if (form) {
      for (const [name, val] of Object.entries(fieldValues)) {
        try {
          const field = form.getFieldMaybe(name);
          if (!field) continue;

          if (field instanceof PDFTextField && typeof val === "string") {
            field.setText(val);
          } else if (field instanceof PDFCheckBox && typeof val === "boolean") {
            if (val) {
              field.check();
            } else {
              field.uncheck();
            }
          } else if (field instanceof PDFRadioGroup && typeof val === "string") {
            if (val && field.getOptions().includes(val)) {
              field.select(val);
            } else if (!val) {
              field.clear();
            }
          } else if (field instanceof PDFDropdown && typeof val === "string") {
            if (val && field.getOptions().includes(val)) {
              field.select(val);
            }
          } else if (field instanceof PDFOptionList && Array.isArray(val)) {
            for (const item of val) {
              if (field.getOptions().includes(item)) {
                field.select(item);
              }
            }
          }
        } catch (fieldError) {
          console.warn(`Failed to update field "${name}":`, fieldError);
        }
      }

      if (mode === "flattened") {
        try {
          form.flatten();
        } catch (flattenErr) {
          console.warn("Form flatten warning:", flattenErr);
        }
      }
    }

    return await doc.save();
  } catch (exportErr: unknown) {
    throw new PdfExportError(
      `Failed to generate exported PDF: ${exportErr instanceof Error ? exportErr.message : String(exportErr)}`,
    );
  }
}

/**
 * Validates that an exported PDF document is structurally sound and readable.
 */
export async function validateExportedPdf(
  exportedBytes: Uint8Array,
  expectedPageCount: number,
): Promise<boolean> {
  try {
    const doc = await PDFDocument.load(exportedBytes, {
      ignoreEncryption: false,
    });
    if (doc.getPageCount() !== expectedPageCount) {
      throw new Error(
        `Page count mismatch: expected ${expectedPageCount}, got ${doc.getPageCount()}`,
      );
    }
    return true;
  } catch (err: unknown) {
    throw new PdfValidationError(
      `Export validation failed: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}

/**
 * Generates an export filename based on original filename and mode.
 */
export function generateExportFilename(
  originalFilename: string,
  mode: ExportMode,
): string {
  const baseName = originalFilename.replace(/\.[^/.]+$/, "");
  const suffix = mode === "editable" ? "-edited.pdf" : "-flattened.pdf";
  return `${baseName}${suffix}`;
}

/**
 * High-level orchestration function to export, validate, and return the export result.
 */
export async function exportPdfDocument(
  sourceBytes: Uint8Array,
  originalFilename: string,
  fieldValues: Record<string, string | boolean | string[]>,
  mode: ExportMode,
  expectedPageCount: number,
): Promise<ExportResult> {
  const pdfBytes = await updateAcroFormFields(sourceBytes, fieldValues, mode);
  const validated = await validateExportedPdf(pdfBytes, expectedPageCount);
  const filename = generateExportFilename(originalFilename, mode);

  return {
    pdfBytes,
    filename,
    mode,
    validated,
  };
}
