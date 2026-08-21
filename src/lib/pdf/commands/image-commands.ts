import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";

export class ReplaceImageCommand implements SmartPdfCommand {
  readonly id = "image.replace";
  readonly label = "Replace image";
  readonly isMutating = true;

  constructor(
    public readonly imageId: string,
    public readonly fileOrBytes: File | Uint8Array,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    let imageBytes: Uint8Array;
    if (this.fileOrBytes instanceof Uint8Array) {
      imageBytes = this.fileOrBytes;
    } else {
      const buffer = await this.fileOrBytes.arrayBuffer();
      imageBytes = new Uint8Array(buffer);
    }

    const pageIndex = currentPage - 1;
    await starPdfDoc.replaceImage(pageIndex, this.imageId, imageBytes, true);
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Image replaced successfully.",
    };
  }
}

export class RemoveImageCommand implements SmartPdfCommand {
  readonly id = "image.remove";
  readonly label = "Remove image";
  readonly isMutating = true;

  constructor(public readonly imageId: string) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const pageIndex = currentPage - 1;
    await starPdfDoc.removeImage(pageIndex, this.imageId);
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      clearSelection: true,
      message: "Image removed from page.",
    };
  }
}
