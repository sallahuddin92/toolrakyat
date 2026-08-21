import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";

export class SetFormFieldValueCommand implements SmartPdfCommand {
  readonly id = "form.set-value";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly fieldName: string,
    public readonly value: string | boolean | string[],
  ) {
    this.label = `Edit field "${fieldName}"`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const updatedValues = {
      ...context.fieldValues,
      [this.fieldName]: this.value,
    };

    return {
      fieldValues: updatedValues,
      message: `Field "${this.fieldName}" updated.`,
    };
  }
}
