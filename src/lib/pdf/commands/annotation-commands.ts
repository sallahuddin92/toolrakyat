import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import type { PdfMarkupAnnotation } from "../pdf-types";
import type { StarPdfAnnotation, StarPdfUpdateAnnotationInput } from "../starpdf-types";

interface ExtendedStarPdfDoc {
  getAnnotations?: (pageIndex: number) => Promise<StarPdfAnnotation[]>;
  updateAnnotation?: (
    objectNum: number,
    objectGen: number,
    input: StarPdfUpdateAnnotationInput,
  ) => Promise<void>;
}

const canonicalAnnotationId = (objectNum: number, objectGen: number) =>
  `annot-obj-${objectNum}-${objectGen}`;

function annotationSelection(annotation: StarPdfAnnotation): NonNullable<SmartPdfCommandResult["nextSelection"]> {
  const [x1, y1, x2, y2] = annotation.rect;
  const data: PdfMarkupAnnotation = {
    id: canonicalAnnotationId(annotation.object_num, annotation.object_gen),
    subtype: annotation.subtype,
    contents: annotation.contents ?? "",
    rect: {
      x: Math.min(x1, x2),
      y: Math.min(y1, y2),
      width: Math.abs(x2 - x1),
      height: Math.abs(y2 - y1),
    },
    pageIndex: annotation.page_index,
    objectNumber: annotation.object_num,
    generationNumber: annotation.object_gen,
    fontFamily: annotation.font_family,
    fontSize: annotation.font_size,
    isBold: annotation.bold,
    isItalic: annotation.italic,
    textColor: annotation.text_color,
    isUnderlined: annotation.underline,
    isStruckThrough: annotation.strikethrough,
    highlightColor: annotation.highlight_color,
  };
  return {
    type: "annotation",
    id: data.id,
    pageIndex: data.pageIndex,
    data,
    pdfRect: { ...data.rect },
  };
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

    const before = await starPdfDoc.getAnnotations(this.pageIndex);
    const beforeIdentities = new Set(
      before.map((annotation) => canonicalAnnotationId(annotation.object_num, annotation.object_gen)),
    );

    await starPdfDoc.addAnnotation(this.pageIndex, {
      subtype: "FreeText",
      rect,
      contents: this.text,
      font_size: this.fontSize,
      color: this.color,
    });

    const updatedBytes = await starPdfDoc.exportIncremental();
    const created = (await starPdfDoc.getAnnotations(this.pageIndex)).filter(
      (annotation) =>
        !beforeIdentities.has(canonicalAnnotationId(annotation.object_num, annotation.object_gen)),
    );
    if (created.length !== 1) {
      throw new Error(
        `ANNOTATION_CREATE_IDENTITY_UNRESOLVED: expected one new annotation, found ${created.length}`,
      );
    }
    const createdSelection = annotationSelection(created[0]);

    return {
      bytes: updatedBytes,
      nextSelection: createdSelection,
      annotationValues: {
        ...context.annotationValues,
        [createdSelection.id]: this.text,
      },
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

export async function resolveAnnotationTarget(
  annotId: string,
  pageIndex: number,
  starPdfDoc?: unknown,
  inspectionResult?: SmartPdfCommandContext["inspectionResult"],
): Promise<{ targetNum: number; targetGen: number }> {
  const inspectionMatches = (inspectionResult?.annotations ?? []).filter(
    (annotation) => annotation.id === annotId && annotation.pageIndex === pageIndex,
  );
  if (inspectionMatches.length > 1) {
    throw new Error(`ANNOTATION_TARGET_AMBIGUOUS: multiple annotations have identity ${annotId}`);
  }

  let targetNum = inspectionMatches[0]?.objectNumber;
  let targetGen = inspectionMatches[0]?.generationNumber ?? 0;
  if (targetNum === undefined) {
    const objectMatch = /^annot-obj-(\d+)-(\d+)$/.exec(annotId);
    if (!objectMatch) {
      throw new Error(
        `ANNOTATION_TARGET_UNRESOLVED: ${annotId} has no authoritative indirect object identity`,
      );
    }
    targetNum = Number.parseInt(objectMatch[1], 10);
    targetGen = Number.parseInt(objectMatch[2], 10);
  }

  const extendedDoc = starPdfDoc as ExtendedStarPdfDoc | undefined;
  if (typeof extendedDoc?.getAnnotations === "function") {
    const matches = (await extendedDoc.getAnnotations(pageIndex)).filter(
      (annotation) =>
        annotation.object_num === targetNum && annotation.object_gen === targetGen,
    );
    if (matches.length !== 1) {
      const code = matches.length > 1 ? "ANNOTATION_TARGET_AMBIGUOUS" : "ANNOTATION_TARGET_UNRESOLVED";
      throw new Error(`${code}: ${annotId} is not uniquely associated with page ${pageIndex + 1}`);
    }
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

    if (targetNum === undefined) {
      throw new Error("ANNOTATION_TARGET_UNRESOLVED: delete requires an indirect object identity");
    }
    await starPdfDoc.removeAnnotation(this.pageIndex, targetNum, targetGen);

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

    const currentSelection = context.selection;
    const hasTextStyleUpdate =
      this.properties.font_family !== undefined ||
      this.properties.font_size !== undefined ||
      this.properties.bold !== undefined ||
      this.properties.italic !== undefined ||
      this.properties.text_color !== undefined;
    const hasTextAppearanceUpdate =
      hasTextStyleUpdate ||
      this.properties.underline !== undefined ||
      this.properties.strikethrough !== undefined ||
      this.properties.highlight_enabled !== undefined ||
      this.properties.highlight_color !== undefined;
    const selectedContents =
      currentSelection?.type === "annotation" && currentSelection.id === this.annotId
        ? currentSelection.data.contents
        : inspectionResult?.annotations.find((annotation) => annotation.id === this.annotId)
            ?.contents;
    await starPdfDoc.updateAnnotation(
      targetNum,
      targetGen,
      this.properties,
      hasTextAppearanceUpdate ? selectedContents : undefined,
    );
    const updatedBytes = await starPdfDoc.exportIncremental();
    const refreshedAnnotation = (await starPdfDoc.getAnnotations(this.pageIndex ?? 0)).find(
      (annotation) => annotation.object_num === targetNum && annotation.object_gen === targetGen,
    );
    if (!refreshedAnnotation) {
      throw new Error(
        "ANNOTATION_STYLE_READBACK_FAILED: styled annotation identity was not returned by StarPDF",
      );
    }
    return {
      bytes: updatedBytes,
      nextSelection:
        currentSelection?.type === "annotation" && currentSelection.id === this.annotId
          ? annotationSelection(refreshedAnnotation)
          : undefined,
      message: "Annotation styled.",
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
    if (!starPdfDoc) {
      throw new Error("No active StarPDF document handle available.");
    }

    const { targetNum, targetGen } = await resolveAnnotationTarget(
      this.annotId,
      this.pageIndex ?? 0,
      starPdfDoc,
      inspectionResult,
    );

    await starPdfDoc.updateAnnotation(targetNum, targetGen, { contents: this.value });
    const updatedBytes = await starPdfDoc.exportIncremental();
    const currentSelection = context.selection;
    return {
      bytes: updatedBytes,
      annotationValues: {
        ...context.annotationValues,
        [this.annotId]: this.value,
      },
      nextSelection:
        currentSelection?.type === "annotation" && currentSelection.id === this.annotId
          ? {
              ...currentSelection,
              data: { ...currentSelection.data, contents: this.value },
            }
          : undefined,
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

    await starPdfDoc.updateAnnotation(targetNum, targetGen, { rect: this.rect });
    const updatedBytes = await starPdfDoc.exportIncremental();
    return {
      bytes: updatedBytes,
      message: "Annotation moved/resized.",
    };
  }
}
