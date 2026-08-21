import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import type { StarPdfUpdateVectorGraphicInput } from "../starpdf-types";

export class UpdateVectorCommand implements SmartPdfCommand {
  readonly id = "vector.update";
  readonly label = "Update vector shape";
  readonly isMutating = true;

  constructor(public readonly input: StarPdfUpdateVectorGraphicInput) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.updateGraphic(this.input);
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Vector shape updated successfully.",
    };
  }
}

export class DeleteVectorCommand implements SmartPdfCommand {
  readonly id = "vector.delete";
  readonly label = "Delete shape";
  readonly isMutating = true;

  constructor(public readonly graphicId: string) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const pageIndex = currentPage - 1;
    await starPdfDoc.deleteGraphic({
      page_index: pageIndex,
      graphic_id: this.graphicId,
      clone_if_shared: true,
    });
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      clearSelection: true,
      message: "Vector shape removed from page.",
    };
  }
}
