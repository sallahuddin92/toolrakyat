import type {
  StarPdfTextSpan,
  StarPdfImageInfo,
  StarPdfVectorGraphicInfo,
} from "../starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "../pdf-types";
import type { HumanTextGroup } from "../grouping/types";

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
  pageIndex: number;
  data: StarPdfTextSpan;
  group?: HumanTextGroup;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}


export interface ImageSelection {
  type: "image";
  id: string; // image_id
  pageIndex: number;
  data: StarPdfImageInfo;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}

export interface VectorSelection {
  type: "vector";
  id: string; // graphic_id
  pageIndex: number;
  data: StarPdfVectorGraphicInfo;
  pdfRect: PdfRect;
  bounds?: PixelRect;
}

export interface FormSelection {
  type: "form";
  id: string; // field.name
  pageIndex: number;
  data: AcroFormField;
  pdfRect?: PdfRect;
  bounds?: PixelRect;
}

export interface AnnotationSelection {
  type: "annotation";
  id: string; // annotation.id
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
  "form",
  "annotation",
  "text",
  "image",
  "vector",
] as const;

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
        (a) => a.id === previousSelection.id && a.pageIndex === currentPageIndex,
      );
      if (!match) return null;
      return {
        ...previousSelection,
        data: match,
        pdfRect: { ...match.rect },
      };
    }
    default:
      return null;
  }
}
