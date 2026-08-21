import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";

export class UpdateAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.update";
  readonly label = "Edit annotation";
  readonly isMutating = true;

  constructor(
    public readonly annotationId: string,
    public readonly value: string,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const updatedValues = {
      ...context.annotationValues,
      [this.annotationId]: this.value,
    };

    return {
      annotationValues: updatedValues,
      message: "Annotation updated.",
    };
  }
}
