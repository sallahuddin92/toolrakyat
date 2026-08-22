import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import { convertToJpegBytes } from "../image-utils";

export class AddImageCommand implements SmartPdfCommand {
  readonly id = "image.add";
  readonly label = "Insert signature image";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly x: number,
    public readonly y: number,
    public readonly width: number,
    public readonly height: number,
    public readonly fileOrBytes: File | Uint8Array,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const { bytes } = await convertToJpegBytes(this.fileOrBytes);

    await starPdfDoc.addImage(
      this.pageIndex,
      bytes,
      this.x,
      this.y,
      this.width,
      this.height,
    );
    const updatedBytes = await starPdfDoc.exportIncremental();


    return {
      bytes: updatedBytes,
      message: "Signature image inserted.",
    };
  }
}

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

    const { bytes } = await convertToJpegBytes(this.fileOrBytes);

    const pageIndex = currentPage - 1;
    await starPdfDoc.replaceImage(pageIndex, this.imageId, bytes, true);
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
