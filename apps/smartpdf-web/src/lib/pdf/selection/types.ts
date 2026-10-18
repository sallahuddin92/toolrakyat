import type {
  StarPdfTextSpan,
  StarPdfImageInfo,
  StarPdfVectorGraphicInfo,
} from "../starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "../pdf-types";
import type { HumanTextGroup } from "../grouping/types";
import type { StarNodeId } from "../execution/contracts";
import { createStarNodeId, parseStarNodeId } from "../execution/identity";

export type SelectionType = "text" | "image" | "vector" | "form" | "annotation";

export interface PdfRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PixelRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface TextSelection {
  type: "text";
  id: string; // span_id
  nodeId?: StarNodeId;
  pageIndex: number;
  data: StarPdfTextSpan;
  group?: HumanTextGroup;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}


export interface ImageSelection {
  type: "image";
  id: string; // image_id
  nodeId?: StarNodeId;
  pageIndex: number;
  data: StarPdfImageInfo;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}

export interface VectorSelection {
  type: "vector";
  id: string; // graphic_id
  nodeId?: StarNodeId;
  pageIndex: number;
  data: StarPdfVectorGraphicInfo;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}

export interface FormSelection {
  type: "form";
  id: string; // field.name
  nodeId?: StarNodeId;
  pageIndex: number;
  data: AcroFormField;
  pdfRect?: PdfRect;
  bounds?: PixelRect;
}

export interface AnnotationSelection {
  type: "annotation";
  id: string; // annotation.id
  nodeId?: StarNodeId;
  pageIndex: number;
  data: PdfMarkupAnnotation;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}

export type SmartPdfSelection =
  | TextSelection
  | ImageSelection
  | VectorSelection
  | FormSelection
  | AnnotationSelection
  | null;

/**
 * Codified Hit-Testing Priority Order (highest precedence first)
 * When objects overlap spatially on the canvas, user selection resolves according to this order.
 */
export const HIT_TEST_PRIORITY: readonly SelectionType[] = [
  "annotation",
  "form",
  "text",
  "image",
  "vector",
] as const;

/** Prefer the native contract identity when it is present. */
export function selectionNodeId(selection: NonNullable<SmartPdfSelection>, documentKey: string): StarNodeId | null {
  if (selection.nodeId) return selection.nodeId;
  const native = (selection.data as { star_node_id?: unknown }).star_node_id;
  if (typeof native === "string") {
    try { return parseStarNodeId(native); } catch { return null; }
  }
  if (selection.type === "annotation" && selection.data.objectNumber !== undefined &&
      selection.data.objectNumber > 0 &&
      selection.data.generationNumber !== undefined) {
    return createStarNodeId(documentKey, {
      number: String(selection.data.objectNumber),
      generation: selection.data.generationNumber,
    }, `annotation:page:${selection.pageIndex}`);
  }
  return null;
}

export function orderedHitCandidates(
  candidates: readonly NonNullable<SmartPdfSelection>[],
  selected: SmartPdfSelection,
): NonNullable<SmartPdfSelection>[] {
  const rank = (item: NonNullable<SmartPdfSelection>) =>
    item.type === "annotation" && selected?.type === "annotation" && selected.id === item.id
      ? -1 : HIT_TEST_PRIORITY.indexOf(item.type);
  return [...candidates].sort((a, b) => rank(a) - rank(b) ||
    a.pageIndex - b.pageIndex || a.id.localeCompare(b.id));
}

export function cycleHitCandidate(
  candidates: readonly NonNullable<SmartPdfSelection>[],
  selected: SmartPdfSelection,
): NonNullable<SmartPdfSelection> | null {
  const ordered = orderedHitCandidates(candidates, selected);
  if (!ordered.length) return null;
  const current = ordered.findIndex((item) => item.type === selected?.type && item.id === selected.id);
  return ordered[(current + 1) % ordered.length];
}

/**
 * Resolves whether a selection remains valid after a mutation or document refresh.
 * Returns the refreshed selection with updated data if still present, or null if deleted/invalid.
 */
export function resolveSelectionAfterMutation(
  previousSelection: SmartPdfSelection,
  currentPageIndex: number,
  pageTextSpans: StarPdfTextSpan[] = [],
  pageImages: StarPdfImageInfo[] = [],
  pageGraphics: StarPdfVectorGraphicInfo[] = [],
  formFields: AcroFormField[] = [],
  annotations: PdfMarkupAnnotation[] = [],
): SmartPdfSelection {
  if (!previousSelection) return null;
  if (previousSelection.pageIndex !== currentPageIndex) return null;

  switch (previousSelection.type) {
    case "text": {
      const match = pageTextSpans.find(
        (s) =>
          s.span_id === previousSelection.id ||
          (previousSelection.group && s.span_id === previousSelection.group.primarySpanId),
      );
      if (!match) return null;
      return {
        ...previousSelection,
        id: previousSelection.id,
        data: match,
        pdfRect: { x: match.x, y: match.y, width: match.width, height: match.height },
      };
    }


    case "image": {
      const match = pageImages.find((img) => img.image_id === previousSelection.id);
      if (!match) return null;
      const [x, y, w, h] = match.rect || [0, 0, match.width, match.height];
      return {
        ...previousSelection,
        data: match,
        pdfRect: { x, y, width: w || match.width, height: h || match.height },
      };
    }
    case "vector": {
      const match = pageGraphics.find((g) => g.graphic_id === previousSelection.id);
      if (!match) return null;
      const [x, y, w, h] = match.bounds || [0, 0, 100, 100];
      return {
        ...previousSelection,
        data: match,
        pdfRect: { x, y, width: w, height: h },
      };
    }
    case "form": {
      const match = formFields.find((f) => f.name === previousSelection.id);
      if (!match) return null;
      return {
        ...previousSelection,
        data: match,
        pdfRect: match.rect ? { ...match.rect } : undefined,
      };
    }
    case "annotation": {
      const match = annotations.find(
        (a) => a.pageIndex === currentPageIndex &&
          (a.id === previousSelection.id ||
            (previousSelection.data.objectNumber !== undefined &&
              a.objectNumber === previousSelection.data.objectNumber &&
              a.generationNumber === previousSelection.data.generationNumber)),
      );
      if (!match) return null;
      return {
        ...previousSelection,
        id: match.id,
        data: match,
        pdfRect: { ...match.rect },
      };
    }
    default:
      return null;
  }
}
