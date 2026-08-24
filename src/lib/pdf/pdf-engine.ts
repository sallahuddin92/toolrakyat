import {
  PDFDocument,
  PDFTextField,
  PDFCheckBox,
  PDFRadioGroup,
  PDFDropdown,
  PDFOptionList,
  PDFName,
  PDFHexString,
  PDFNumber,
  PDFDict,
  PDFRef,
} from "pdf-lib";

import {
  type AcroFormField,
  type AcroFormFieldType,
  type DocumentInspectionResult,
  type ExportMode,
  type ExportResult,
  type PdfDocumentMetadata,
  type PdfMarkupAnnotation,
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

  const annotations: PdfMarkupAnnotation[] = [];
  for (let i = 0; i < pageCount; i++) {
    const page = doc.getPage(i);
    const annots = page.node.Annots();
    if (annots) {
      for (let j = 0; j < annots.size(); j++) {
        try {
          const annotRef = annots.get(j);
          const annotDict = doc.context.lookup(annotRef);
          if (!annotDict || !(annotDict instanceof PDFDict)) continue;
          const subtypeRaw = annotDict.get(PDFName.of("Subtype"))?.toString()?.replace(/^\//, "") || "Unknown";
          // Exclude AcroForm widget annotations (which are handled in fields)
          if (subtypeRaw === "Widget") continue;

          let contents = "";
          const contentsEntry = annotDict.get(PDFName.of("Contents"));
          if (contentsEntry && typeof (contentsEntry as unknown as { decodeText?: () => string }).decodeText === "function") {
            contents = (contentsEntry as unknown as { decodeText: () => string }).decodeText();
          } else if (contentsEntry && typeof (contentsEntry as unknown as { value?: string }).value === "string") {
            contents = (contentsEntry as unknown as { value: string }).value;
          }

          let rect = { x: 0, y: 0, width: 50, height: 50 };
          const rectEntry = annotDict.get(PDFName.of("Rect"));
          if (rectEntry && (rectEntry as unknown as { asArray?: () => unknown[] }).asArray) {
            const arr = (rectEntry as unknown as { asArray: () => unknown[] }).asArray();
            if (arr.length >= 4) {
              const x1 = (arr[0] as PDFNumber).asNumber();
              const y1 = (arr[1] as PDFNumber).asNumber();
              const x2 = (arr[2] as PDFNumber).asNumber();
              const y2 = (arr[3] as PDFNumber).asNumber();
              rect = {
                x: Math.min(x1, x2),
                y: Math.min(y1, y2),
                width: Math.abs(x2 - x1),
                height: Math.abs(y2 - y1),
              };
            }
          }

          let author: string | undefined;
          const tEntry = annotDict.get(PDFName.of("T"));
          if (tEntry && typeof (tEntry as unknown as { decodeText?: () => string }).decodeText === "function") {
            author = (tEntry as unknown as { decodeText: () => string }).decodeText();
          }

          let fontFamily: PdfMarkupAnnotation["fontFamily"];
          let fontSize: number | undefined;
          let isBold: boolean | undefined;
          let isItalic: boolean | undefined;
          let textColor: [number, number, number] | undefined;
          if (subtypeRaw === "FreeText") {
            const daEntry = annotDict.get(PDFName.of("DA"));
            const da =
              daEntry &&
              typeof (daEntry as unknown as { decodeText?: () => string }).decodeText === "function"
                ? (daEntry as unknown as { decodeText: () => string }).decodeText()
                : daEntry?.toString().replace(/^\(|\)$/g, "") ?? "";
            const fontMatch = /\/([^\s]+)\s+([+-]?(?:\d+\.?\d*|\.\d+))\s+Tf/.exec(da);
            const fontName = fontMatch?.[1] ?? "Helvetica";
            const lowerFont = fontName.toLowerCase();
            fontFamily = lowerFont.includes("times")
              ? "Serif"
              : lowerFont.includes("courier")
                ? "Monospace"
                : "SansSerif";
            fontSize = fontMatch ? Number(fontMatch[2]) : 12;
            isBold = lowerFont.includes("bold");
            isItalic = lowerFont.includes("italic") || lowerFont.includes("oblique");
            const rgbMatch = /([+-]?(?:\d+\.?\d*|\.\d+))\s+([+-]?(?:\d+\.?\d*|\.\d+))\s+([+-]?(?:\d+\.?\d*|\.\d+))\s+rg/.exec(
              da,
            );
            const grayMatch = /([+-]?(?:\d+\.?\d*|\.\d+))\s+g(?:\s|$)/.exec(da);
            textColor = rgbMatch
              ? [Number(rgbMatch[1]), Number(rgbMatch[2]), Number(rgbMatch[3])]
              : grayMatch
                ? [Number(grayMatch[1]), Number(grayMatch[1]), Number(grayMatch[1])]
                : [0, 0, 0];
          }

          let objectNumber: number | undefined;
          let generationNumber: number | undefined;
          if (annotRef instanceof PDFRef) {
            objectNumber = annotRef.objectNumber;
            generationNumber = annotRef.generationNumber;
          } else if (annotRef && typeof annotRef === "object" && "objectNumber" in annotRef) {
            objectNumber = Number((annotRef as { objectNumber: unknown }).objectNumber);
            generationNumber = Number((annotRef as { generationNumber?: unknown }).generationNumber ?? 0);
          }

          annotations.push({
            id:
              objectNumber !== undefined
                ? `annot-obj-${objectNumber}-${generationNumber ?? 0}`
                : `annot-inline-${i}-${j}`,
            subtype: subtypeRaw,
            contents,
            rect,
            pageIndex: i,
            author,
            objectNumber,
            generationNumber,
            fontFamily,
            fontSize,
            isBold,
            isItalic,
            textColor,
          });
        } catch {
          // Ignore malformed individual annotation
        }
      }
    }
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
    annotations,
  };
}

/**
 * Updates AcroForm field values and markup annotations in the source PDF and produces an exported Uint8Array.
 * If mode is 'flattened', interactive widgets are converted to visual page content.
 */
export async function updateAcroFormFields(
  sourceBytes: Uint8Array,
  fieldValues: Record<string, string | boolean | string[]>,
  mode: ExportMode,
  annotationValues: Record<string, string> = {},
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
    // 1. Update Markup Annotations if any modified
    if (Object.keys(annotationValues).length > 0) {
      for (let i = 0; i < doc.getPageCount(); i++) {
        const page = doc.getPage(i);
        const annots = page.node.Annots();
        if (annots) {
          for (let j = 0; j < annots.size(); j++) {
            const annotRef = annots.get(j);
            let objectNumber: number | undefined;
            let generationNumber: number | undefined;
            if (annotRef instanceof PDFRef) {
              objectNumber = annotRef.objectNumber;
              generationNumber = annotRef.generationNumber;
            } else if (annotRef && typeof annotRef === "object" && "objectNumber" in annotRef) {
              objectNumber = Number((annotRef as { objectNumber: unknown }).objectNumber);
              generationNumber = Number((annotRef as { generationNumber?: unknown }).generationNumber ?? 0);
            }

            const idxId = `annot-${i}-${j}`;
            const objId = objectNumber !== undefined ? `annot-obj-${objectNumber}-${generationNumber ?? 0}` : null;
            const targetVal =
              (objId && annotationValues[objId] !== undefined)
                ? annotationValues[objId]
                : annotationValues[idxId];

            if (targetVal !== undefined) {
              try {
                const annotDict = doc.context.lookup(annotRef);
                if (annotDict && annotDict instanceof PDFDict) {
                  annotDict.set(PDFName.of("Contents"), PDFHexString.fromText(targetVal));
                }
              } catch {
                // Ignore failure on specific annotation
              }
            }
          }
        }
      }
    }


    // 2. Update AcroForm Fields
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
  annotationValues: Record<string, string> = {},
): Promise<ExportResult> {
  const pdfBytes = await updateAcroFormFields(sourceBytes, fieldValues, mode, annotationValues);
  const validated = await validateExportedPdf(pdfBytes, expectedPageCount);
  const filename = generateExportFilename(originalFilename, mode);

  return {
    pdfBytes,
    filename,
    mode,
    validated,
  };
}
