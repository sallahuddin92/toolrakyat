import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import { mergeStarPdfDocuments } from "../starpdf-page-worker-client";
import { exportPdfDocument } from "../pdf-engine";
import type { ExportMode } from "../pdf-types";

export class MergeDocumentsCommand implements SmartPdfCommand {
  readonly id = "document.merge";
  readonly label: string;
  readonly isMutating = true;

  constructor(public readonly additionalPdfs: Uint8Array[]) {
    this.label = `Merged ${additionalPdfs.length} PDF(s)`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.additionalPdfs.length === 0) {
      throw new Error("No additional PDF documents provided to merge.");
    }

    const output = await mergeStarPdfDocuments([sourceBytes, ...this.additionalPdfs]);

    return {
      bytes: output,
      nextPage: 1,
      clearSelection: true,
      message: `Added and merged ${this.additionalPdfs.length} PDF document(s).`,
    };
  }
}

export class ExportDocumentCommand implements SmartPdfCommand {
  readonly id = "document.export";
  readonly label: string;
  readonly isMutating = false; // Export generates a download and does not mutate the in-memory document

  constructor(public readonly mode: ExportMode) {
    this.label = mode === "editable" ? "Export Editable PDF" : "Export Flattened PDF";
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, filename, fieldValues, annotationValues, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No document loaded to export.");
    }

    const result = await exportPdfDocument(
      sourceBytes,
      filename,
      fieldValues,
      this.mode,
      pageCount,
      annotationValues,
    );

    return {
      download: {
        filename: result.filename,
        bytes: result.pdfBytes,
      },
      message: `Exported ${result.filename} successfully.`,
    };
  }
}
