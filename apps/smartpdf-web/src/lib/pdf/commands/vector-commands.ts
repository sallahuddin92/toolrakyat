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

    await starPdfDoc.updateGraphic({
      clone_if_shared: true,
      ...this.input,
    });
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

export class AddRectangleCommand implements SmartPdfCommand {
  readonly id = "vector.add_rectangle";
  readonly label = "Add rectangle";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly x: number,
    public readonly y: number,
    public readonly width: number,
    public readonly height: number,
    public readonly strokeColorRgb?: [number, number, number],
    public readonly fillColorRgb?: [number, number, number],
    public readonly lineWidth = 1,
    public readonly isStroked = true,
    public readonly isFilled = false,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.addRectangle({
      page_index: this.pageIndex,
      x: this.x,
      y: this.y,
      width: this.width,
      height: this.height,
      stroke_color_rgb: this.strokeColorRgb,
      fill_color_rgb: this.fillColorRgb,
      line_width: this.lineWidth,
      is_stroked: this.isStroked,
      is_filled: this.isFilled,
    });
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Rectangle added to page.",
    };
  }
}

export class AddLineCommand implements SmartPdfCommand {
  readonly id = "vector.add_line";
  readonly label = "Add line";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly x1: number,
    public readonly y1: number,
    public readonly x2: number,
    public readonly y2: number,
    public readonly strokeColorRgb?: [number, number, number],
    public readonly lineWidth = 1,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.addLine({
      page_index: this.pageIndex,
      x1: this.x1,
      y1: this.y1,
      x2: this.x2,
      y2: this.y2,
      stroke_color_rgb: this.strokeColorRgb,
      line_width: this.lineWidth,
    });
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Line added to page.",
    };
  }
}

