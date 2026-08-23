import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";

export class SetFormFieldValueCommand implements SmartPdfCommand {
  readonly id = "form.set-value";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly fieldName: string,
    public readonly value: string | boolean | string[],
    public readonly widgetNum?: number,
    public readonly widgetGen = 0,
  ) {
    this.label = `Edit field "${fieldName}"`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;

    const updatedValues = {
      ...context.fieldValues,
      [this.fieldName]: this.value,
    };

    if (starPdfDoc) {
      const fields = await starPdfDoc.getFormFields();
      const field = fields.find((f) => f.name === this.fieldName);

      if (field) {
        if (field.is_read_only) {
          throw new Error(`READ_ONLY_FIELD: Field "${this.fieldName}" is read-only and cannot be mutated.`);
        }

        if (field.field_type === "signature") {
          throw new Error(`SIGNATURE_FIELD_READ_ONLY: Signature field "${this.fieldName}" cannot be mutated directly.`);
        }

        const normType = field.field_type.toLowerCase();

        if (normType === "checkbox") {
          await starPdfDoc.setCheckbox(field.object_num, field.object_gen, Boolean(this.value));
        } else if (normType === "radio") {
          const parentNum = field.parent_num ?? field.object_num;
          const parentGen = field.parent_gen ?? field.object_gen;

          let targetWidget = field.widgets.find((w) =>
            this.widgetNum !== undefined ? w.object_num === this.widgetNum : false,
          );

          if (!targetWidget) {
            targetWidget =
              field.widgets.find(
                (w) =>
                  w.appearance_state === String(this.value) ||
                  w.normal_appearance_states.includes(String(this.value)),
              ) || field.widgets[0];
          }

          if (targetWidget) {
            const onState =
              targetWidget.normal_appearance_states.find((s) => s !== "Off") ||
              String(this.value);
            await starPdfDoc.setRadio(
              parentNum,
              parentGen,
              targetWidget.object_num,
              targetWidget.object_gen,
              onState,
            );
          }
        } else if (
          normType === "combobox" ||
          normType === "listbox" ||
          normType === "dropdown" ||
          normType === "optionlist" ||
          normType === "choice"
        ) {
          if (Array.isArray(this.value)) {
            await starPdfDoc.setChoiceValues(field.object_num, field.object_gen, this.value);
          } else {
            await starPdfDoc.setChoice(field.object_num, field.object_gen, String(this.value));
          }
        } else {
          // Default text field
          await starPdfDoc.setTextField(field.object_num, field.object_gen, String(this.value));
        }

        const updatedBytes = await starPdfDoc.exportIncremental();

        return {
          bytes: updatedBytes,
          fieldValues: updatedValues,
          message: `Field "${this.fieldName}" updated.`,
        };
      }
    }

    return {
      fieldValues: updatedValues,
      message: `Field "${this.fieldName}" updated.`,
    };
  }
}
