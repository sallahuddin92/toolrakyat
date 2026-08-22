import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";

interface ExtendedStarPdfDoc {
  getAnnotations?: (pageIndex: number) => Promise<Array<{ object_num: number; object_gen: number; contents?: string }>>;
  updateAnnotation?: (
    objectNum: number,
    objectGen: number,
    input: { contents?: string },
  ) => Promise<void>;
}


export class AddFreeTextCommand implements SmartPdfCommand {
  readonly id = "annotation.add_freetext";
  readonly label = "Add text annotation";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly x: number,
    public readonly y: number,
    public readonly text: string,
    public readonly fontSize = 12,
    public readonly color: [number, number, number] = [0, 0, 0],
    public readonly width?: number,
    public readonly height?: number,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    // Estimate box width & height if not specified
    const boxWidth = this.width ?? Math.max(60, this.text.length * (this.fontSize * 0.65) + 16);
    const boxHeight = this.height ?? Math.max(18, this.fontSize * 1.3);

    const rect: [number, number, number, number] = [
      this.x,
      this.y,
      this.x + boxWidth,
      this.y + boxHeight,
    ];

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "FreeText",
      rect,
      contents: this.text,
      font_size: this.fontSize,
      color: this.color,
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: `Text "${this.text}" added.`,
    };
  }
}

export class AddCheckMarkCommand implements SmartPdfCommand {
  readonly id = "annotation.add_check";
  readonly label = "Add checkmark";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly x: number,
    public readonly y: number,
    public readonly size = 16,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const rect: [number, number, number, number] = [
      this.x,
      this.y,
      this.x + this.size,
      this.y + this.size,
    ];

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "FreeText",
      rect,
      contents: "✓",
      font_size: this.size,
      color: [0, 0.5, 0],
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Checkmark added.",
    };
  }
}

export class AddCrossMarkCommand implements SmartPdfCommand {
  readonly id = "annotation.add_cross";
  readonly label = "Add crossmark";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly x: number,
    public readonly y: number,
    public readonly size = 16,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const rect: [number, number, number, number] = [
      this.x,
      this.y,
      this.x + this.size,
      this.y + this.size,
    ];

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "FreeText",
      rect,
      contents: "✕",
      font_size: this.size,
      color: [0.8, 0, 0],
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Cross mark added.",
    };
  }
}

export class AddInkAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.add_ink";
  readonly label = "Add freehand drawing";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly inkList: [number, number][][],
    public readonly rect: [number, number, number, number],
    public readonly color: [number, number, number] = [0, 0, 0.8],
    public readonly borderWidth = 2,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "Ink",
      rect: this.rect,
      ink_list: this.inkList,
      color: this.color,
      border_width: this.borderWidth,
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Drawing added.",
    };
  }
}

export class DeleteAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.delete";
  readonly label = "Delete annotation";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly annotationId?: string,
    public readonly objectNum?: number,
    public readonly objectGen = 0,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, inspectionResult } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    let targetNum = this.objectNum;
    let targetGen = this.objectGen;

    if (targetNum === undefined && this.annotationId) {
      // 1. First check inspectionResult for exact match with objectNumber
      if (inspectionResult) {
        const match = inspectionResult.annotations.find(
          (a) =>
            a.id === this.annotationId ||
            (a.objectNumber !== undefined &&
              `annot-obj-${a.objectNumber}-${a.generationNumber ?? 0}` === this.annotationId),
        );
        if (match && match.objectNumber !== undefined) {
          targetNum = match.objectNumber;
          targetGen = match.generationNumber ?? 0;
        }
      }

      // 2. Check if annotationId contains object numbers, e.g. "annot-obj-42-0"
      if (targetNum === undefined) {
        const parts = this.annotationId.match(/\d+/g);
        if (parts && parts.length > 0) {
          targetNum = parseInt(parts[parts.length - (parts.length > 1 ? 2 : 1)], 10);
          targetGen = parts.length > 1 ? parseInt(parts[parts.length - 1], 10) : 0;
        } else {
          const extendedDoc = starPdfDoc as unknown as ExtendedStarPdfDoc;
          if (typeof extendedDoc.getAnnotations === "function") {
            const annots = await extendedDoc.getAnnotations(this.pageIndex);
            const match = annots.find(
              (a) =>
                `${a.object_num}` === this.annotationId ||
                `annot-${this.pageIndex}-${a.object_num}` === this.annotationId ||
                a.contents === this.annotationId,
            );
            if (match) {
              targetNum = match.object_num;
              targetGen = match.object_gen;
            } else if (annots.length > 0) {
              targetNum = annots[0].object_num;
              targetGen = annots[0].object_gen;
            }
          }
        }
      }
    }

    if (targetNum !== undefined) {
      await starPdfDoc.removeAnnotation(this.pageIndex, targetNum, targetGen);
    }

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      clearSelection: true,
      message: "Annotation deleted.",
    };
  }
}

export class UpdateAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.update";
  readonly label = "Update annotation";
  readonly isMutating = true;

  constructor(
    public readonly annotId: string,
    public readonly value: string,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, inspectionResult } = context;

    let targetNum: number | undefined;
    let targetGen = 0;

    if (inspectionResult) {
      const match = inspectionResult.annotations.find(
        (a) =>
          a.id === this.annotId ||
          (a.objectNumber !== undefined &&
            `annot-obj-${a.objectNumber}-${a.generationNumber ?? 0}` === this.annotId),
      );
      if (match && match.objectNumber !== undefined) {
        targetNum = match.objectNumber;
        targetGen = match.generationNumber ?? 0;
      }
    }

    const extendedDoc = starPdfDoc as unknown as ExtendedStarPdfDoc;
    if (extendedDoc && typeof extendedDoc.updateAnnotation === "function" && targetNum !== undefined) {
      await extendedDoc.updateAnnotation(targetNum, targetGen, { contents: this.value });
      const updatedBytes = await starPdfDoc!.exportIncremental();
      return {
        bytes: updatedBytes,
        annotationValues: {
          [this.annotId]: this.value,
          ...(targetNum !== undefined ? { [`annot-obj-${targetNum}-${targetGen}`]: this.value } : {}),
        },
        message: `Annotation updated.`,
      };
    }

    return {
      annotationValues: {
        [this.annotId]: this.value,
        ...(targetNum !== undefined ? { [`annot-obj-${targetNum}-${targetGen}`]: this.value } : {}),
      },
      message: `Annotation updated.`,
    };
  }
}

