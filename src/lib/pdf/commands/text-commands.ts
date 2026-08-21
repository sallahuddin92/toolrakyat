import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";

export class ReplaceTextCommand implements SmartPdfCommand {
  readonly id = "text.replace";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly spanId: string,
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
    const result = await starPdfDoc.replaceText(pageIndex, this.spanId, this.newText);
    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: `Text updated (${result.layout_result}). Native content stream modified.`,
    };
  }
}
