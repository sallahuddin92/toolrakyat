import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import type { StarPdfReplaceTextResult } from "../starpdf-types";

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


