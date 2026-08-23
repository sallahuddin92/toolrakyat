import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import type { StarPdfUpdateAnnotationInput } from "../starpdf-types";

interface ExtendedStarPdfDoc {
  getAnnotations?: (pageIndex: number) => Promise<Array<{ object_num: number; object_gen: number; contents?: string }>>;
  updateAnnotation?: (
    objectNum: number,
    objectGen: number,
    input: StarPdfUpdateAnnotationInput,
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

export class AddSquareAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.add_square";
  readonly label = "Add rectangle";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly rect: [number, number, number, number],
    public readonly strokeColor: [number, number, number] = [0, 0, 0],
    public readonly fillColor?: [number, number, number],
    public readonly borderWidth = 1.5,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "Square",
      rect: this.rect,
      color: this.strokeColor,
      fill_color: this.fillColor,
      border_width: this.borderWidth,
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Rectangle added.",
    };
  }
}

export class AddCircleAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.add_circle";
  readonly label = "Add circle";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly rect: [number, number, number, number],
    public readonly strokeColor: [number, number, number] = [0, 0, 0],
    public readonly fillColor?: [number, number, number],
    public readonly borderWidth = 1.5,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "Circle",
      rect: this.rect,
      color: this.strokeColor,
      fill_color: this.fillColor,
      border_width: this.borderWidth,
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Circle added.",
    };
  }
}

export class AddHighlightAnnotationCommand implements SmartPdfCommand {
  readonly id = "annotation.add_highlight";
  readonly label = "Add highlight";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number,
    public readonly rect: [number, number, number, number],
    public readonly quadPoints: number[],
    public readonly color: [number, number, number] = [1, 1, 0],
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "Highlight",
      rect: this.rect,
      quad_points: this.quadPoints,
      color: this.color,
    });

    const updatedBytes = await starPdfDoc.exportIncremental();

    return {
      bytes: updatedBytes,
      message: "Highlight added.",
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

async function resolveAnnotationTarget(
  annotId: string,
  pageIndex: number,
  starPdfDoc?: unknown,
  inspectionResult?: SmartPdfCommandContext["inspectionResult"],
): Promise<{ targetNum?: number; targetGen: number }> {
  let targetNum: number | undefined;
  let targetGen = 0;

  if (inspectionResult) {
    const match = inspectionResult.annotations.find(
      (a) =>
        a.id === annotId ||
        (a.objectNumber !== undefined &&
          `annot-obj-${a.objectNumber}-${a.generationNumber ?? 0}` === annotId),
    );
    if (match && match.objectNumber !== undefined) {
      return { targetNum: match.objectNumber, targetGen: match.generationNumber ?? 0 };
    }
  }

  const objMatch = annotId.match(/annot-obj-(\d+)-(\d+)/);
  if (objMatch) {
    return { targetNum: parseInt(objMatch[1], 10), targetGen: parseInt(objMatch[2], 10) };
  }

  if (starPdfDoc) {
    const extendedDoc = starPdfDoc as unknown as ExtendedStarPdfDoc;
    if (typeof extendedDoc.getAnnotations === "function") {
      try {
        const annots = await extendedDoc.getAnnotations(pageIndex);
        const indexMatch = annotId.match(/annot-\d+-(\d+)/);
        if (indexMatch) {
          const idx = parseInt(indexMatch[1], 10);
          if (annots[idx]) {
            return { targetNum: annots[idx].object_num, targetGen: annots[idx].object_gen };
          }
        }
        const match = annots.find(
          (a) =>
            `${a.object_num}` === annotId ||
            `annot-${pageIndex}-${a.object_num}` === annotId ||
            a.contents === annotId,
        );
        if (match) {
          return { targetNum: match.object_num, targetGen: match.object_gen };
        }
        if (annots.length > 0) {
          return { targetNum: annots[0].object_num, targetGen: annots[0].object_gen };
        }
      } catch {
        // Fallback below
      }
    }
  }

  const digits = annotId.match(/\d+/g);
  if (digits && digits.length > 0) {
    targetNum = parseInt(digits[digits.length - (digits.length > 1 ? 2 : 1)], 10);
    targetGen = digits.length > 1 ? parseInt(digits[digits.length - 1], 10) : 0;
  }

  return { targetNum, targetGen };
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
      const resolved = await resolveAnnotationTarget(
        this.annotationId,
        this.pageIndex,
        starPdfDoc,
        inspectionResult,
      );
      targetNum = resolved.targetNum;
      targetGen = resolved.targetGen;
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

export class UpdateAnnotationPropertiesCommand implements SmartPdfCommand {
  readonly id = "annotation.update_properties";
  readonly label = "Style annotation";
  readonly isMutating = true;

  constructor(
    public readonly annotId: string,
    public readonly properties: StarPdfUpdateAnnotationInput,
    public readonly pageIndex?: number,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, inspectionResult } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const { targetNum, targetGen } = await resolveAnnotationTarget(
      this.annotId,
      this.pageIndex ?? 0,
      starPdfDoc,
      inspectionResult,
    );

    if (targetNum !== undefined) {
      await starPdfDoc.updateAnnotation(targetNum, targetGen, this.properties);
      const updatedBytes = await starPdfDoc.exportIncremental();
      return {
        bytes: updatedBytes,
        message: "Annotation styled.",
      };
    }

    return {
      message: "Annotation updated.",
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
    public readonly pageIndex?: number,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, inspectionResult } = context;

    const { targetNum, targetGen } = await resolveAnnotationTarget(
      this.annotId,
      this.pageIndex ?? 0,
      starPdfDoc,
      inspectionResult,
    );

    if (starPdfDoc && targetNum !== undefined) {
      await starPdfDoc.updateAnnotation(targetNum, targetGen, { contents: this.value });
      const updatedBytes = await starPdfDoc.exportIncremental();
      return {
        bytes: updatedBytes,
        annotationValues: {
          ...context.annotationValues,
          [this.annotId]: this.value,
        },
        message: `Annotation updated.`,
      };
    }

    return {
      annotationValues: {
        ...context.annotationValues,
        [this.annotId]: this.value,
      },
      message: `Annotation updated.`,
    };
  }
}

export class UpdateAnnotationRectCommand implements SmartPdfCommand {
  readonly id = "annotation.update_rect";
  readonly label = "Move/resize annotation";
  readonly isMutating = true;

  constructor(
    public readonly annotId: string,
    public readonly rect: [number, number, number, number],
    public readonly pageNumber?: number,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { starPdfDoc, inspectionResult } = context;
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const { targetNum, targetGen } = await resolveAnnotationTarget(
      this.annotId,
      (this.pageNumber ?? 1) - 1,
      starPdfDoc,
      inspectionResult,
    );

    if (targetNum !== undefined) {
      await starPdfDoc.updateAnnotation(targetNum, targetGen, { rect: this.rect });
      const updatedBytes = await starPdfDoc.exportIncremental();
      return {
        bytes: updatedBytes,
        message: "Annotation moved/resized.",
      };
    }

    return {
      message: "Annotation position updated.",
    };
  }
}
