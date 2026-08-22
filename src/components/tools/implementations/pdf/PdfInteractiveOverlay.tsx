"use client";

import { useState, useRef, useCallback, useMemo, useEffect } from "react";

import type {
  StarPdfImageInfo,
  StarPdfTextSpan,
  StarPdfVectorGraphicInfo,
} from "@/lib/pdf/starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "@/lib/pdf/pdf-types";
import {
  type SmartPdfSelection,
  type PixelRect,
  type PdfRect,
  convertPdfRectToPixels,
  convertPixelsToPdfPoint,
  convertPixelsToPdfRect,
  convertPdfPointToPixels,
} from "@/lib/pdf/selection";
import { cn } from "@/lib/utils";
import type { EditorMode, FillAndSignTool } from "./PdfToolbar";
import { Check, X } from "lucide-react";
import { computeAutoCenteredMark, type FlatFormCandidate } from "@/lib/pdf/detection";
import { groupTextSpans } from "@/lib/pdf/grouping";

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
  candidates?: FlatFormCandidate[];
  formAssistEnabled?: boolean;
  selectedItem: SmartPdfSelection;
  onSelectItem: (item: SmartPdfSelection) => void;
  mode?: EditorMode;
  fillAndSignTool?: FillAndSignTool;
  onPlaceFreeText?: (pageIndex: number, pdfX: number, pdfY: number, text: string) => void;
  onPlaceCheck?: (pageIndex: number, pdfX: number, pdfY: number) => void;
  onPlaceCross?: (pageIndex: number, pdfX: number, pdfY: number) => void;
  onPlaceSignature?: (pageIndex: number, pdfX: number, pdfY: number) => void;
  onPlaceDrawing?: (
    pageIndex: number,
    inkList: [number, number][][],
    rect: [number, number, number, number],
  ) => void;
  onTransformImage?: (pageIndex: number, imageId: string, pdfRect: PdfRect) => void;
  onTransformVector?: (
    pageIndex: number,
    graphicId: string,
    pdfRect: PdfRect,
    linePoints?: { x1: number; y1: number; x2: number; y2: number },
  ) => void;
}

interface DragTransformState {
  type: "move" | "resize" | "endpoint1" | "endpoint2";
  handle?: "nw" | "ne" | "se" | "sw";
  startPointerX: number;
  startPointerY: number;
  startRect: PixelRect;
  currentRect: PixelRect;
  startLine?: { x1: number; y1: number; x2: number; y2: number };
  currentLine?: { x1: number; y1: number; x2: number; y2: number };
}


interface InlineTextState {
  pixelX: number;
  pixelY: number;
  pdfX: number;
  pdfY: number;
  text: string;
}

interface CandidatePopupState {
  candidate: FlatFormCandidate;
  pixelX: number;
  pixelY: number;
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
  candidates = [],
  formAssistEnabled = true,
  selectedItem,
  onSelectItem,
  mode = "SELECT",
  fillAndSignTool = "text",
  onPlaceFreeText,
  onPlaceCheck,
  onPlaceCross,
  onPlaceSignature,
  onPlaceDrawing,
  onTransformImage,
  onTransformVector,
}: PdfInteractiveOverlayProps) {

  const pageDims = useMemo(() => ({ width: pageWidth, height: pageHeight, rotation }), [pageWidth, pageHeight, rotation]);
  const pageIdx = pageNumber - 1;
  const humanTextGroups = useMemo(() => groupTextSpans(textSpans), [textSpans]);

  const overlayRef = useRef<HTMLDivElement | null>(null);


  // Inline placement state for text tool
  const [inlineText, setInlineText] = useState<InlineTextState | null>(null);

  // Active candidate action popup state
  const [activeCandidatePopup, setActiveCandidatePopup] = useState<CandidatePopupState | null>(null);

  // Freehand drawing state
  const [isDrawing, setIsDrawing] = useState(false);
  const [currentStroke, setCurrentStroke] = useState<{ x: number; y: number }[]>([]);

  // Direct Manipulation Drag/Transform State
  const [dragTransform, setDragTransform] = useState<DragTransformState | null>(null);

  const startDrag = useCallback(
    (
      e: React.PointerEvent,
      type: "move" | "resize" | "endpoint1" | "endpoint2",
      handle?: "nw" | "ne" | "se" | "sw",
      startLine?: { x1: number; y1: number; x2: number; y2: number },
    ) => {
      e.stopPropagation();
      e.preventDefault();
      if (!selectedItem || !selectedItem.bounds) return;

      const startRect = selectedItem.bounds;
      setDragTransform({
        type,
        handle,
        startPointerX: e.clientX,
        startPointerY: e.clientY,
        startRect: { ...startRect },
        currentRect: { ...startRect },
        startLine: startLine ? { ...startLine } : undefined,
        currentLine: startLine ? { ...startLine } : undefined,
      });
    },
    [selectedItem],
  );

  useEffect(() => {
    if (!dragTransform) return;

    const onPointerMove = (e: PointerEvent) => {
      const deltaX = e.clientX - dragTransform.startPointerX;
      const deltaY = e.clientY - dragTransform.startPointerY;
      const { startRect, type, handle, startLine } = dragTransform;

      if (type === "move") {
        const currentRect: PixelRect = {
          left: Math.max(0, startRect.left + deltaX),
          top: Math.max(0, startRect.top + deltaY),
          width: startRect.width,
          height: startRect.height,
        };
        let currentLine = undefined;
        if (startLine) {
          currentLine = {
            x1: startLine.x1 + deltaX,
            y1: startLine.y1 + deltaY,
            x2: startLine.x2 + deltaX,
            y2: startLine.y2 + deltaY,
          };
        }
        setDragTransform((prev) => (prev ? { ...prev, currentRect, currentLine } : null));
      } else if (type === "resize") {
        let left = startRect.left;
        let top = startRect.top;
        let width = startRect.width;
        let height = startRect.height;

        if (handle === "se") {
          width = Math.max(10, startRect.width + deltaX);
          height = Math.max(10, startRect.height + deltaY);
        } else if (handle === "sw") {
          const newW = Math.max(10, startRect.width - deltaX);
          left = startRect.left + (startRect.width - newW);
          width = newW;
          height = Math.max(10, startRect.height + deltaY);
        } else if (handle === "ne") {
          width = Math.max(10, startRect.width + deltaX);
          const newH = Math.max(10, startRect.height - deltaY);
          top = startRect.top + (startRect.height - newH);
          height = newH;
        } else if (handle === "nw") {
          const newW = Math.max(10, startRect.width - deltaX);
          const newH = Math.max(10, startRect.height - deltaY);
          left = startRect.left + (startRect.width - newW);
          top = startRect.top + (startRect.height - newH);
          width = newW;
          height = newH;
        }

        setDragTransform((prev) =>
          prev ? { ...prev, currentRect: { left, top, width, height } } : null,
        );
      } else if (type === "endpoint1" && startLine) {
        setDragTransform((prev) =>
          prev
            ? {
                ...prev,
                currentLine: {
                  x1: startLine.x1 + deltaX,
                  y1: startLine.y1 + deltaY,
                  x2: startLine.x2,
                  y2: startLine.y2,
                },
              }
            : null,
        );
      } else if (type === "endpoint2" && startLine) {
        setDragTransform((prev) =>
          prev
            ? {
                ...prev,
                currentLine: {
                  x1: startLine.x1,
                  y1: startLine.y1,
                  x2: startLine.x2 + deltaX,
                  y2: startLine.y2 + deltaY,
                },
              }
            : null,
        );
      }
    };

    const onPointerUp = () => {
      const { startRect, currentRect, currentLine } = dragTransform;
      setDragTransform(null);

      const deltaX = Math.abs(currentRect.left - startRect.left);
      const deltaY = Math.abs(currentRect.top - startRect.top);
      const deltaW = Math.abs(currentRect.width - startRect.width);
      const deltaH = Math.abs(currentRect.height - startRect.height);

      if (deltaX <= 1 && deltaY <= 1 && deltaW <= 1 && deltaH <= 1 && !currentLine) {
        return;
      }

      if (selectedItem?.type === "image") {
        const newPdfRect = convertPixelsToPdfRect(currentRect, pageDims, scale, rotation);
        onTransformImage?.(pageIdx, selectedItem.id, newPdfRect);
      } else if (selectedItem?.type === "vector") {
        if (currentLine) {
          const pt1 = convertPixelsToPdfPoint(
            currentLine.x1,
            currentLine.y1,
            pageDims,
            scale,
            rotation,
          );
          const pt2 = convertPixelsToPdfPoint(
            currentLine.x2,
            currentLine.y2,
            pageDims,
            scale,
            rotation,
          );
          onTransformVector?.(
            pageIdx,
            selectedItem.id,
            { x: pt1.x, y: pt1.y, width: 0, height: 0 },
            { x1: pt1.x, y1: pt1.y, x2: pt2.x, y2: pt2.y },
          );
        } else {
          const newPdfRect = convertPixelsToPdfRect(currentRect, pageDims, scale, rotation);
          onTransformVector?.(pageIdx, selectedItem.id, newPdfRect);
        }
      }
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setDragTransform(null);
      }
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [
    dragTransform,
    selectedItem,
    pageDims,
    scale,
    rotation,
    pageIdx,
    onTransformImage,
    onTransformVector,
  ]);


  const isFillAndSign = mode === "FILL_AND_SIGN";


  const handleOverlayClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (!isFillAndSign || !overlayRef.current) return;
      const rect = overlayRef.current.getBoundingClientRect();
      const pixelX = e.clientX - rect.left;
      const pixelY = e.clientY - rect.top;
      const pdfPt = convertPixelsToPdfPoint(pixelX, pixelY, pageDims, scale, rotation);

      if (fillAndSignTool === "text") {
        setInlineText({
          pixelX,
          pixelY,
          pdfX: pdfPt.x,
          pdfY: Math.max(0, pdfPt.y - 14),
          text: "",
        });
      } else if (fillAndSignTool === "check") {
        onPlaceCheck?.(pageIdx, pdfPt.x, Math.max(0, pdfPt.y - 16));
      } else if (fillAndSignTool === "cross") {
        onPlaceCross?.(pageIdx, pdfPt.x, Math.max(0, pdfPt.y - 16));
      } else if (fillAndSignTool === "signature") {
        onPlaceSignature?.(pageIdx, pdfPt.x, Math.max(0, pdfPt.y - 40));
      }
    },
    [isFillAndSign, fillAndSignTool, pageDims, scale, rotation, pageIdx, onPlaceCheck, onPlaceCross, onPlaceSignature],
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isFillAndSign || fillAndSignTool !== "draw" || !overlayRef.current) return;
      const rect = overlayRef.current.getBoundingClientRect();
      const pt = { x: e.clientX - rect.left, y: e.clientY - rect.top };
      setIsDrawing(true);
      setCurrentStroke([pt]);
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [isFillAndSign, fillAndSignTool],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isDrawing || !overlayRef.current) return;
      const rect = overlayRef.current.getBoundingClientRect();
      const pt = { x: e.clientX - rect.left, y: e.clientY - rect.top };
      setCurrentStroke((prev) => [...prev, pt]);
    },
    [isDrawing],
  );

  const handlePointerUp = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isDrawing) return;
      setIsDrawing(false);
      try {
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
      } catch {
        // Ignored
      }

      if (currentStroke.length > 1) {
        // Convert screen stroke to PDF coordinates
        const pdfStroke: [number, number][] = currentStroke.map((pt) => {
          const pdfPt = convertPixelsToPdfPoint(pt.x, pt.y, pageDims, scale, rotation);
          return [pdfPt.x, pdfPt.y];
        });

        let minX = Infinity;
        let minY = Infinity;
        let maxX = -Infinity;
        let maxY = -Infinity;

        for (const [px, py] of pdfStroke) {
          if (px < minX) minX = px;
          if (py < minY) minY = py;
          if (px > maxX) maxX = px;
          if (py > maxY) maxY = py;
        }

        // Add padding
        minX = Math.max(0, minX - 5);
        minY = Math.max(0, minY - 5);
        maxX = Math.min(pageWidth, maxX + 5);
        maxY = Math.min(pageHeight, maxY + 5);

        onPlaceDrawing?.(pageIdx, [pdfStroke], [minX, minY, maxX, maxY]);
      }
      setCurrentStroke([]);
    },
    [isDrawing, currentStroke, pageDims, scale, rotation, pageIdx, pageWidth, pageHeight, onPlaceDrawing],
  );

  const handleCommitInlineText = () => {
    if (!inlineText || !inlineText.text.trim()) {
      setInlineText(null);
      return;
    }
    onPlaceFreeText?.(pageIdx, inlineText.pdfX, inlineText.pdfY, inlineText.text.trim());
    setInlineText(null);
  };

  return (
    <div
      ref={overlayRef}
      onClick={isFillAndSign ? handleOverlayClick : undefined}
      onPointerDown={isFillAndSign && fillAndSignTool === "draw" ? handlePointerDown : undefined}
      onPointerMove={isFillAndSign && fillAndSignTool === "draw" ? handlePointerMove : undefined}
      onPointerUp={isFillAndSign && fillAndSignTool === "draw" ? handlePointerUp : undefined}
      className={cn(
        "absolute inset-0 z-10 select-none overflow-hidden",
        isFillAndSign ? "pointer-events-auto cursor-crosshair" : "pointer-events-none",
      )}
      data-testid="pdf-interactive-overlay"
    >
      {/* 0. FORM ASSIST CANDIDATES (Z-15) */}
      {formAssistEnabled &&
        candidates.map((cand) => {
          if (cand.pageIndex !== pageIdx) return null;
          const [x1, y1, x2, y2] = cand.pdfRect;
          const pdfRect = {
            x: Math.min(x1, x2),
            y: Math.min(y1, y2),
            width: Math.abs(x2 - x1),
            height: Math.abs(y2 - y1),
          };
          const rect = convertPdfRectToPixels(pdfRect, pageDims, scale, rotation);

          return (
            <div
              key={cand.id}
              onClick={(e) => {
                e.stopPropagation();
                setActiveCandidatePopup({
                  candidate: cand,
                  pixelX: rect.left + rect.width / 2,
                  pixelY: rect.top,
                });
              }}
              style={{
                left: `${rect.left}px`,
                top: `${rect.top}px`,
                width: `${rect.width}px`,
                height: `${rect.height}px`,
              }}
              className={cn(
                "absolute pointer-events-auto cursor-pointer rounded-xs transition-colors z-25",
                "border border-dashed border-sky-400/60 hover:border-sky-500 hover:bg-sky-400/20 hover:ring-1 hover:ring-sky-400/70",
              )}
              title={`Form Assist (${cand.type}): Click for quick actions`}
              data-testid={`canvas-candidate-${cand.id}`}
            />
          );
        })}


      {/* 1. VECTOR GRAPHICS (Z-10) */}
      {!isFillAndSign &&
        graphics.map((g) => {
          const isSelected = selectedItem?.type === "vector" && selectedItem.id === g.graphic_id;
          const [minX, minY, maxX, maxY] = g.bounds || [0, 0, 100, 100];
          const width = Math.max(1, maxX - minX);
          const height = Math.max(1, maxY - minY);
          const pdfRect = { x: minX, y: minY, width, height };
          const rect = convertPdfRectToPixels(pdfRect, pageDims, scale, rotation);

          return (
            <div
              key={g.graphic_id}
              onClick={(e) => {
                e.stopPropagation();
                onSelectItem({
                  type: "vector",
                  id: g.graphic_id,
                  pageIndex: pageIdx,
                  data: g,
                  pdfRect,
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
      {!isFillAndSign &&
        images.map((img) => {
          const isSelected = selectedItem?.type === "image" && selectedItem.id === img.image_id;
          const [minX, minY, maxX, maxY] = img.rect || [0, 0, img.width, img.height];
          const width = maxX > minX ? maxX - minX : img.width;
          const height = maxY > minY ? maxY - minY : img.height;
          const pdfRect = { x: minX, y: minY, width, height };
          const rect = convertPdfRectToPixels(pdfRect, pageDims, scale, rotation);

          return (
            <div
              key={img.image_id}
              onClick={(e) => {
                e.stopPropagation();
                onSelectItem({
                  type: "image",
                  id: img.image_id,
                  pageIndex: pageIdx,
                  data: img,
                  pdfRect,
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
                  : "hover:bg-emerald-400/15 hover:ring-1 hover:ring-emerald-300/60 z-12",
              )}
              title={`Image (${Math.round(img.width)}×${Math.round(img.height)} pt) - Click to replace or remove`}
              data-testid={`canvas-image-${img.image_id}`}
            />
          );
        })}


      {/* 3. HUMAN-SCALE TEXT GROUPS (Z-30) */}
      {!isFillAndSign &&
        humanTextGroups.map((group) => {
          const isSelected =
            selectedItem?.type === "text" &&
            (selectedItem.id === group.id ||
              selectedItem.id === group.primarySpanId ||
              selectedItem.group?.id === group.id);
          const pdfRect = { x: group.x, y: group.y, width: group.width, height: group.height };
          const rect = convertPdfRectToPixels(pdfRect, pageDims, scale, rotation);

          return (
            <div
              key={group.id}
              onClick={(e) => {
                e.stopPropagation();
                onSelectItem({
                  type: "text",
                  id: group.primarySpanId,
                  pageIndex: pageIdx,
                  data: group.sourceSpans[0],
                  group,
                  pdfRect,
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
                  ? "ring-2 ring-sky-500 bg-sky-400/20 shadow-xs z-50"
                  : "hover:bg-sky-400/15 hover:ring-1 hover:ring-sky-300/60 z-30",
                !group.is_editable && "cursor-default opacity-80",
              )}
              title={
                group.is_editable
                  ? `Text: "${group.text}" (Click to edit)`
                  : `Read-only text: ${group.refusal_reason || "Cannot edit in place"}`
              }
              data-testid={`canvas-text-span-${group.primarySpanId}`}
              data-group-id={group.id}
            />
          );
        })}


      {/* 4. MARKUP ANNOTATIONS (Z-25) */}
      {!isFillAndSign &&
        annotations.map((annot) => {
          if (annot.pageIndex !== pageNumber - 1) return null;
          const isSelected = selectedItem?.type === "annotation" && selectedItem.id === annot.id;
          const pdfRect = {
            x: annot.rect.x,
            y: annot.rect.y,
            width: annot.rect.width,
            height: annot.rect.height,
          };
          const rect = convertPdfRectToPixels(pdfRect, pageDims, scale, rotation);

          return (
            <div
              key={annot.id}
              onClick={(e) => {
                e.stopPropagation();
                onSelectItem({
                  type: "annotation",
                  id: annot.id,
                  pageIndex: pageIdx,
                  data: annot,
                  pdfRect,
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
      {!isFillAndSign &&
        fields.map((field) => {
          if (!field.rect) return null;
          const isSelected = selectedItem?.type === "form" && selectedItem.id === field.name;
          const pdfRect = {
            x: field.rect.x,
            y: field.rect.y,
            width: field.rect.width,
            height: field.rect.height,
          };
          const rect = convertPdfRectToPixels(pdfRect, pageDims, scale, rotation);

          return (
            <div
              key={`field-${field.name}`}
              onClick={(e) => {
                e.stopPropagation();
                onSelectItem({
                  type: "form",
                  id: field.name,
                  pageIndex: pageIdx,
                  data: field,
                  pdfRect,
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

      {/* INLINE TEXT PLACEMENT INPUT */}
      {inlineText && (
        <div
          style={{ left: `${inlineText.pixelX}px`, top: `${inlineText.pixelY}px` }}
          onClick={(e) => e.stopPropagation()}
          className="absolute z-50 flex items-center gap-1 bg-white p-1 rounded-md shadow-lg border border-sky-400 -translate-y-1/2"
        >
          <input
            type="text"
            value={inlineText.text}
            onChange={(e) => setInlineText({ ...inlineText, text: e.target.value })}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCommitInlineText();
              if (e.key === "Escape") setInlineText(null);
            }}
            placeholder="Type text..."
            className="text-xs px-1.5 py-0.5 border border-slate-300 rounded focus:outline-none focus:ring-1 focus:ring-sky-500 w-36"
            autoFocus
            data-testid="inline-text-placement-input"
          />
          <button
            type="button"
            onClick={handleCommitInlineText}
            className="p-1 bg-sky-600 text-white rounded hover:bg-sky-700"
            title="Place text"
            data-testid="inline-text-placement-commit-btn"
          >
            <Check className="size-3" />
          </button>
          <button
            type="button"
            onClick={() => setInlineText(null)}
            className="p-1 text-slate-400 hover:text-slate-600"
            title="Cancel"
          >
            <X className="size-3" />
          </button>
        </div>
      )}

      {/* CANDIDATE CONTEXTUAL ACTION POPUP */}
      {activeCandidatePopup && (() => {
        const { candidate, pixelX, pixelY } = activeCandidatePopup;
        return (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              left: `${pixelX}px`,
              top: `${Math.max(10, pixelY - 44)}px`,
            }}
            className="absolute -translate-x-1/2 pointer-events-auto z-50 bg-slate-900/95 text-white backdrop-blur-md rounded-lg shadow-xl px-2 py-1.5 flex items-center gap-1.5 border border-slate-700 select-none"
            data-testid="candidate-action-popup"
          >

            {candidate.type === "checkbox" && (
              <>
                <button
                  type="button"
                  onClick={() => {
                    const mark = computeAutoCenteredMark(candidate.pdfRect, "check");
                    onPlaceCheck?.(pageIdx, mark.x, mark.y);
                    setActiveCandidatePopup(null);
                  }}
                  className="px-2 py-1 text-xs font-medium rounded hover:bg-slate-800 text-emerald-400 flex items-center gap-1"
                  data-testid="candidate-action-check"
                >
                  <Check className="size-3.5" />
                  <span>Check</span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const mark = computeAutoCenteredMark(candidate.pdfRect, "cross");
                    onPlaceCross?.(pageIdx, mark.x, mark.y);
                    setActiveCandidatePopup(null);
                  }}
                  className="px-2 py-1 text-xs font-medium rounded hover:bg-slate-800 text-rose-400 flex items-center gap-1"
                  data-testid="candidate-action-cross"
                >
                  <X className="size-3.5" />
                  <span>Cross</span>
                </button>
              </>
            )}

            {candidate.type === "radio" && (
              <>
                <button
                  type="button"
                  onClick={() => {
                    const mark = computeAutoCenteredMark(candidate.pdfRect, "radio");
                    onPlaceCheck?.(pageIdx, mark.x, mark.y);
                    setActiveCandidatePopup(null);
                  }}
                  className="px-2 py-1 text-xs font-medium rounded hover:bg-slate-800 text-emerald-400 flex items-center gap-1"
                  data-testid="candidate-action-select"
                >
                  <span>● Select</span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const mark = computeAutoCenteredMark(candidate.pdfRect, "check");
                    onPlaceCheck?.(pageIdx, mark.x, mark.y);
                    setActiveCandidatePopup(null);
                  }}
                  className="px-2 py-1 text-xs font-medium rounded hover:bg-slate-800 text-sky-400 flex items-center gap-1"
                  data-testid="candidate-action-check"
                >
                  <Check className="size-3.5" />
                  <span>Check</span>
                </button>
              </>
            )}

            {candidate.type === "text-region" && (
              <>
                <button
                  type="button"
                  onClick={() => {
                    const mark = computeAutoCenteredMark(candidate.pdfRect, "text");
                    const [px1, py1] = [candidate.pdfRect[0], candidate.pdfRect[1]];
                    const scr = convertPdfRectToPixels(
                      { x: px1, y: py1, width: 20, height: 20 },
                      pageDims,
                      scale,
                      rotation,
                    );
                    setInlineText({
                      pixelX: scr.left,
                      pixelY: scr.top,
                      pdfX: mark.x,
                      pdfY: mark.y,
                      text: "",
                    });
                    setActiveCandidatePopup(null);
                  }}
                  className="px-2 py-1 text-xs font-medium rounded hover:bg-slate-800 text-sky-300 flex items-center gap-1"
                  data-testid="candidate-action-text"
                >
                  <span>Add Text</span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const mark = computeAutoCenteredMark(candidate.pdfRect, "text");
                    onPlaceSignature?.(pageIdx, mark.x, mark.y);
                    setActiveCandidatePopup(null);
                  }}
                  className="px-2 py-1 text-xs font-medium rounded hover:bg-slate-800 text-purple-300 flex items-center gap-1"
                  data-testid="candidate-action-signature"
                >
                  <span>Signature</span>
                </button>
              </>
            )}

            <button
              type="button"
              onClick={() => setActiveCandidatePopup(null)}
              className="px-1.5 py-1 text-xs text-slate-400 hover:text-white rounded hover:bg-slate-800"
              title="Close"
            >
              <X className="size-3" />
            </button>
          </div>
        );
      })()}

      {/* LIVE DRAWING SVG PREVIEW */}
      {currentStroke.length > 1 && (
        <svg className="absolute inset-0 pointer-events-none z-50 w-full h-full">
          <polyline
            points={currentStroke.map((p) => `${p.x},${p.y}`).join(" ")}
            fill="none"
            stroke="#1d4ed8"
            strokeWidth={3}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      )}

      {/* DIRECT MANIPULATION TRANSFORM BOX FOR SELECTED IMAGE / VECTOR */}
      {!isFillAndSign &&
        selectedItem &&
        (selectedItem.type === "image" || selectedItem.type === "vector") &&
        selectedItem.pageIndex === pageIdx &&
        (() => {
          const isVectorLine =
            selectedItem.type === "vector" &&
            (selectedItem.data as StarPdfVectorGraphicInfo)?.graphic_type === "Line";

          if (isVectorLine) {
            const g = selectedItem.data as StarPdfVectorGraphicInfo;
            const [lx1, ly1, lx2, ly2] = [
              g.line_x1 ?? g.bounds[0],
              g.line_y1 ?? g.bounds[1],
              g.line_x2 ?? g.bounds[0] + g.bounds[2],
              g.line_y2 ?? g.bounds[1] + g.bounds[3],
            ];
            const p1 = convertPdfPointToPixels(lx1, ly1, pageDims, scale, rotation);
            const p2 = convertPdfPointToPixels(lx2, ly2, pageDims, scale, rotation);
            const curP1 = dragTransform?.currentLine
              ? { x: dragTransform.currentLine.x1, y: dragTransform.currentLine.y1 }
              : p1;
            const curP2 = dragTransform?.currentLine
              ? { x: dragTransform.currentLine.x2, y: dragTransform.currentLine.y2 }
              : p2;

            return (
              <div className="absolute inset-0 pointer-events-none z-50">
                {/* SVG line preview during drag */}
                {dragTransform?.currentLine && (
                  <svg className="absolute inset-0 pointer-events-none w-full h-full">
                    <line
                      x1={curP1.x}
                      y1={curP1.y}
                      x2={curP2.x}
                      y2={curP2.y}
                      stroke="#4f46e5"
                      strokeWidth={2}
                      strokeDasharray="4 4"
                    />
                  </svg>
                )}
                {/* Endpoint 1 handle */}
                <div
                  onPointerDown={(e) =>
                    startDrag(e, "endpoint1", undefined, {
                      x1: p1.x,
                      y1: p1.y,
                      x2: p2.x,
                      y2: p2.y,
                    })
                  }
                  style={{ left: `${curP1.x - 6}px`, top: `${curP1.y - 6}px` }}
                  className="absolute size-3 bg-white border-2 border-indigo-600 rounded-full pointer-events-auto cursor-crosshair shadow-sm hover:scale-125 transition-transform"
                  title="Drag start endpoint"
                  data-testid="vector-line-endpoint-1"
                />
                {/* Endpoint 2 handle */}
                <div
                  onPointerDown={(e) =>
                    startDrag(e, "endpoint2", undefined, {
                      x1: p1.x,
                      y1: p1.y,
                      x2: p2.x,
                      y2: p2.y,
                    })
                  }
                  style={{ left: `${curP2.x - 6}px`, top: `${curP2.y - 6}px` }}
                  className="absolute size-3 bg-white border-2 border-indigo-600 rounded-full pointer-events-auto cursor-crosshair shadow-sm hover:scale-125 transition-transform"
                  title="Drag end endpoint"
                  data-testid="vector-line-endpoint-2"
                />
              </div>
            );
          }

          const activeRect = dragTransform?.currentRect || selectedItem.bounds;
          if (!activeRect) return null;

          const ringColor =
            selectedItem.type === "image"
              ? "ring-emerald-500 bg-emerald-400/10"
              : "ring-indigo-500 bg-indigo-400/10";
          const borderColor =
            selectedItem.type === "image" ? "border-emerald-600" : "border-indigo-600";

          return (
            <div
              style={{
                left: `${activeRect.left}px`,
                top: `${activeRect.top}px`,
                width: `${activeRect.width}px`,
                height: `${activeRect.height}px`,
              }}
              className={cn(
                "absolute pointer-events-auto ring-2 z-50 rounded-xs shadow-xs select-none",
                ringColor,
              )}
              data-testid="direct-manipulation-transform-box"
            >
              {/* Move handle (interior area) */}
              <div
                onPointerDown={(e) => startDrag(e, "move")}
                className="absolute inset-0 cursor-move"
                title="Drag to move"
                data-testid="direct-manipulation-move-handle"
              />
              {/* Corner resize handles */}
              <div
                onPointerDown={(e) => startDrag(e, "resize", "nw")}
                className={cn(
                  "absolute -top-1.5 -left-1.5 size-3 bg-white border-2 rounded-xs cursor-nwse-resize shadow-sm hover:scale-125 transition-transform",
                  borderColor,
                )}
                title="Resize top-left"
                data-testid="direct-manipulation-handle-nw"
              />
              <div
                onPointerDown={(e) => startDrag(e, "resize", "ne")}
                className={cn(
                  "absolute -top-1.5 -right-1.5 size-3 bg-white border-2 rounded-xs cursor-nesw-resize shadow-sm hover:scale-125 transition-transform",
                  borderColor,
                )}
                title="Resize top-right"
                data-testid="direct-manipulation-handle-ne"
              />
              <div
                onPointerDown={(e) => startDrag(e, "resize", "se")}
                className={cn(
                  "absolute -bottom-1.5 -right-1.5 size-3 bg-white border-2 rounded-xs cursor-nwse-resize shadow-sm hover:scale-125 transition-transform",
                  borderColor,
                )}
                title="Resize bottom-right"
                data-testid="direct-manipulation-handle-se"
              />
              <div
                onPointerDown={(e) => startDrag(e, "resize", "sw")}
                className={cn(
                  "absolute -bottom-1.5 -left-1.5 size-3 bg-white border-2 rounded-xs cursor-nesw-resize shadow-sm hover:scale-125 transition-transform",
                  borderColor,
                )}
                title="Resize bottom-left"
                data-testid="direct-manipulation-handle-sw"
              />
            </div>
          );
        })()}

    </div>
  );
}


