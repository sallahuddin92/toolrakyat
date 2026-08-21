"use client";

import { useMemo } from "react";
import type {
  StarPdfImageInfo,
  StarPdfTextSpan,
  StarPdfVectorGraphicInfo,
} from "@/lib/pdf/starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "@/lib/pdf/pdf-types";
import type { SelectedItem } from "./PdfContextualToolbar";
import { cn } from "@/lib/utils";

interface PdfInteractiveOverlayProps {
  pageWidth: number;
  pageHeight: number;
  pageNumber?: number;
  scale: number;
  rotation?: number;
  textSpans?: StarPdfTextSpan[];
  images?: StarPdfImageInfo[];
  graphics?: StarPdfVectorGraphicInfo[];
  fields?: AcroFormField[];
  annotations?: PdfMarkupAnnotation[];
  selectedItem: SelectedItem | null;
  onSelectItem: (item: SelectedItem | null) => void;
}

export function PdfInteractiveOverlay({
  pageWidth,
  pageHeight,
  pageNumber = 1,
  scale,
  rotation = 0,
  textSpans = [],
  images = [],
  graphics = [],
  fields = [],
  annotations = [],
  selectedItem,
  onSelectItem,
}: PdfInteractiveOverlayProps) {
  // Helper to convert PDF coordinate box to canvas CSS pixel box
  const convertPdfRectToPixels = useMemo(() => {
    return (x: number, y: number, width: number, height: number) => {
      const rot = ((rotation % 360) + 360) % 360;
      let left = 0;
      let top = 0;
      let w = width * scale;
      let h = height * scale;

      if (rot === 0) {
        left = x * scale;
        top = (pageHeight - (y + height)) * scale;
      } else if (rot === 90) {
        left = y * scale;
        top = x * scale;
        w = height * scale;
        h = width * scale;
      } else if (rot === 180) {
        left = (pageWidth - (x + width)) * scale;
        top = y * scale;
      } else if (rot === 270) {
        left = (pageHeight - (y + height)) * scale;
        top = (pageWidth - (x + width)) * scale;
        w = height * scale;
        h = width * scale;
      }

      return {
        left: Math.max(0, left),
        top: Math.max(0, top),
        width: Math.max(4, w),
        height: Math.max(4, h),
      };
    };
  }, [pageWidth, pageHeight, scale, rotation]);

  return (
    <div
      className="absolute inset-0 z-10 pointer-events-none select-none overflow-hidden"
      data-testid="pdf-interactive-overlay"
    >
      {/* 1. VECTOR GRAPHICS (Z-10) */}
      {graphics.map((g) => {
        const isSelected = selectedItem?.type === "vector" && selectedItem.id === g.graphic_id;
        const [x, y, w, h] = g.bounds || [0, 0, 100, 100];
        const rect = convertPdfRectToPixels(x, y, w, h);

        return (
          <div
            key={g.graphic_id}
            onClick={(e) => {
              e.stopPropagation();
              onSelectItem({
                type: "vector",
                id: g.graphic_id,
                data: g,
                bounds: rect,
              });
            }}
            style={{
              left: `${rect.left}px`,
              top: `${rect.top}px`,
              width: `${rect.width}px`,
              height: `${rect.height}px`,
            }}
            className={cn(
              "absolute pointer-events-auto cursor-pointer rounded-xs transition-colors",
              isSelected
                ? "ring-2 ring-indigo-500 bg-indigo-400/20 shadow-xs z-40"
                : "hover:bg-indigo-400/15 hover:ring-1 hover:ring-indigo-300/60 z-10",
            )}
            title={`Vector shape (${g.graphic_type || "Path"}) - Click to edit color & width`}
            data-testid={`canvas-graphic-${g.graphic_id}`}
          />
        );
      })}

      {/* 2. IMAGES (Z-15) */}
      {images.map((img) => {
        const isSelected = selectedItem?.type === "image" && selectedItem.id === img.image_id;
        const [x, y, w, h] = img.rect || [0, 0, img.width, img.height];
        const rect = convertPdfRectToPixels(x, y, w || img.width, h || img.height);

        return (
          <div
            key={img.image_id}
            onClick={(e) => {
              e.stopPropagation();
              onSelectItem({
                type: "image",
                id: img.image_id,
                data: img,
                bounds: rect,
              });
            }}
            style={{
              left: `${rect.left}px`,
              top: `${rect.top}px`,
              width: `${rect.width}px`,
              height: `${rect.height}px`,
            }}
            className={cn(
              "absolute pointer-events-auto cursor-pointer rounded-xs transition-colors",
              isSelected
                ? "ring-2 ring-emerald-500 bg-emerald-400/20 shadow-xs z-40"
                : "hover:bg-emerald-400/15 hover:ring-1 hover:ring-emerald-300/60 z-15",
            )}
            title={`Image (${Math.round(img.width)}×${Math.round(img.height)} pt) - Click to replace or remove`}
            data-testid={`canvas-image-${img.image_id}`}
          />
        );
      })}

      {/* 3. TEXT SPANS (Z-20) */}
      {textSpans.map((span) => {
        const isSelected = selectedItem?.type === "text" && selectedItem.id === span.span_id;
        const rect = convertPdfRectToPixels(span.x, span.y, span.width, span.height);

        return (
          <div
            key={span.span_id}
            onClick={(e) => {
              e.stopPropagation();
              onSelectItem({
                type: "text",
                id: span.span_id,
                data: span,
                bounds: rect,
              });
            }}
            style={{
              left: `${rect.left}px`,
              top: `${rect.top}px`,
              width: `${rect.width}px`,
              height: `${rect.height}px`,
            }}
            className={cn(
              "absolute pointer-events-auto cursor-pointer rounded-xs transition-colors",
              isSelected
                ? "ring-2 ring-sky-500 bg-sky-400/20 shadow-xs z-40"
                : "hover:bg-sky-400/15 hover:ring-1 hover:ring-sky-300/60 z-20",
              !span.is_editable && "cursor-not-allowed opacity-80",
            )}
            title={
              span.is_editable
                ? `Text: "${span.text}" (Click to edit)`
                : `Read-only text: ${span.refusal_reason || "Font encoding not rewritable"}`
            }
            data-testid={`canvas-text-span-${span.span_id}`}
          />
        );
      })}

      {/* 4. MARKUP ANNOTATIONS (Z-25) */}
      {annotations.map((annot) => {
        if (annot.pageIndex !== (pageNumber - 1)) return null;
        const isSelected = selectedItem?.type === "annotation" && selectedItem.id === annot.id;
        const rect = convertPdfRectToPixels(annot.rect.x, annot.rect.y, annot.rect.width, annot.rect.height);

        return (
          <div
            key={annot.id}
            onClick={(e) => {
              e.stopPropagation();
              onSelectItem({
                type: "annotation",
                id: annot.id,
                data: annot,
                bounds: rect,
              });
            }}
            style={{
              left: `${rect.left}px`,
              top: `${rect.top}px`,
              width: `${rect.width}px`,
              height: `${rect.height}px`,
            }}
            className={cn(
              "absolute pointer-events-auto cursor-pointer rounded-xs transition-colors",
              isSelected
                ? "ring-2 ring-purple-500 bg-purple-400/20 shadow-xs z-40"
                : "hover:bg-purple-400/15 hover:ring-1 hover:ring-purple-300/60 z-25",
            )}
            title={`Markup annotation (${annot.subtype}): ${annot.contents || annot.subtype}`}
            data-testid={`canvas-annotation-${annot.id}`}
          />
        );
      })}

      {/* 5. FORM FIELD / WIDGET ANNOTATIONS (Z-30) */}
      {fields.map((field) => {
        if (!field.rect) return null;
        const isSelected = selectedItem?.type === "form" && selectedItem.id === field.name;
        const rect = convertPdfRectToPixels(field.rect.x, field.rect.y, field.rect.width, field.rect.height);

        return (
          <div
            key={`field-${field.name}`}
            onClick={(e) => {
              e.stopPropagation();
              onSelectItem({
                type: "form",
                id: field.name,
                data: field,
                bounds: rect,
              });
            }}
            style={{
              left: `${rect.left}px`,
              top: `${rect.top}px`,
              width: `${rect.width}px`,
              height: `${rect.height}px`,
            }}
            className={cn(
              "absolute pointer-events-auto cursor-pointer rounded-xs transition-colors",
              isSelected
                ? "ring-2 ring-amber-500 bg-amber-400/20 shadow-xs z-40"
                : "hover:bg-amber-400/15 hover:ring-1 hover:ring-amber-300/60 z-30",
              field.isReadOnly && "cursor-not-allowed opacity-80",
            )}
            title={`Form field (${field.type}): ${field.name}${field.isReadOnly ? " (Read-only)" : ""}`}
            data-testid={`canvas-field-${field.name}`}
          />
        );
      })}
    </div>
  );
}
