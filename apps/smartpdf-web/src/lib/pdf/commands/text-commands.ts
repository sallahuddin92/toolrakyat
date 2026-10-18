import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import type { StarPdfReplaceTextResult, StarPdfTextStylePatch } from "../starpdf-types";

export class ApplyTextStyleCommand implements SmartPdfCommand {
  readonly id = "text.apply_style";
  readonly label = "Format text";
  readonly isMutating = true;

  constructor(
    public readonly spanId: string | string[],
    public readonly text: string,
    public readonly patch: StarPdfTextStylePatch,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }
    const targets = Array.isArray(this.spanId) ? this.spanId : [this.spanId];
    if (targets.length !== 1) {
      throw new Error(
        "TEXT_STYLE_GROUP_NOT_ISOLATABLE: select one native text run to apply formatting safely.",
      );
    }
    const result = await starPdfDoc.applyTextStyle(
      currentPage - 1,
      targets[0],
      this.text,
      this.patch,
    );
    const updatedBytes = await starPdfDoc.exportIncremental();
    return {
      bytes: updatedBytes,
      message: `Text formatting applied (${result.layout_result}).`,
    };
  }
}

export class ReplaceTextCommand implements SmartPdfCommand {
  readonly id = "text.replace";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly spanId: string | string[],
    public readonly newText: string,
  ) {
    this.label = `Edit text "${newText}"`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const pageIndex = currentPage - 1;
    let result: StarPdfReplaceTextResult;
    if (Array.isArray(this.spanId)) {
      if (this.spanId.length === 1) {
        result = await starPdfDoc.replaceText(pageIndex, this.spanId[0], this.newText);
      } else {
        result = await (starPdfDoc.replaceTextGroup
          ? starPdfDoc.replaceTextGroup(pageIndex, this.spanId, this.newText)
          : starPdfDoc.replaceText(pageIndex, this.spanId[0], this.newText));
      }
    } else {
      result = await starPdfDoc.replaceText(pageIndex, this.spanId, this.newText);
    }
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: `Text updated (${result.layout_result}). Native content stream modified.`,
    };
  }
}

export class DeleteTextCommand implements SmartPdfCommand {
  readonly id = "text.delete";
  readonly label = "Delete native text";
  readonly isMutating = true;

  constructor(public readonly spanId: string | string[]) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const pageIndex = currentPage - 1;
    let result: StarPdfReplaceTextResult;
    if (Array.isArray(this.spanId)) {
      if (this.spanId.length === 1) {
        result = await starPdfDoc.replaceText(pageIndex, this.spanId[0], "");
      } else {
        result = await (starPdfDoc.replaceTextGroup
          ? starPdfDoc.replaceTextGroup(pageIndex, this.spanId, "")
          : starPdfDoc.replaceText(pageIndex, this.spanId[0], ""));
      }
    } else {
      result = await starPdfDoc.replaceText(pageIndex, this.spanId, "");
    }
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: `Text removed (${result.layout_result}). Native content stream modified.`,
    };
  }
}

export class MoveTextCommand implements SmartPdfCommand {
  readonly id = "text.move";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly spanId: string | string[],
    public readonly dx: number,
    public readonly dy: number,
    public readonly pageNumber?: number,
  ) {
    this.label = `Move text by (${dx.toFixed(1)}, ${dy.toFixed(1)}) pt`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, currentPage } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const pageIndex = (this.pageNumber ?? currentPage) - 1;
    if (Array.isArray(this.spanId)) {
      if (this.spanId.length === 1) {
        await starPdfDoc.moveText(pageIndex, this.spanId[0], this.dx, this.dy);
      } else {
        await (starPdfDoc.moveTextGroup
          ? starPdfDoc.moveTextGroup(pageIndex, this.spanId, this.dx, this.dy)
          : starPdfDoc.moveText(pageIndex, this.spanId[0], this.dx, this.dy));
      }
    } else {
      await starPdfDoc.moveText(pageIndex, this.spanId, this.dx, this.dy);
    }
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: `Text moved (${this.dx > 0 ? "+" : ""}${this.dx.toFixed(1)}, ${this.dy > 0 ? "+" : ""}${this.dy.toFixed(1)} pt). Native content stream modified.`,
    };
  }
}


