"use client";

import { useEffect, useRef, useState } from "react";
import type { PDFDocumentProxy, RenderTask } from "@/lib/pdf/pdfjs-init";
import { Loader2 } from "lucide-react";
import { PdfInteractiveOverlay } from "./PdfInteractiveOverlay";
import type { SmartPdfSelection } from "@/lib/pdf/selection";
import type {
  StarPdfImageInfo,
  StarPdfTextSpan,
  StarPdfVectorGraphicInfo,
} from "@/lib/pdf/starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "@/lib/pdf/pdf-types";
import type { EditorMode, FillAndSignTool } from "./PdfToolbar";
import type { FlatFormCandidate } from "@/lib/pdf/detection";

interface PdfPageCanvasProps {
  pdfDocument: PDFDocumentProxy | null;
  pageNumber: number;
  scale: number;
  rotation?: number;
  pageWidth?: number;
  pageHeight?: number;
  textSpans?: StarPdfTextSpan[];
  images?: StarPdfImageInfo[];
  graphics?: StarPdfVectorGraphicInfo[];
  fields?: AcroFormField[];
  annotations?: PdfMarkupAnnotation[];
  candidates?: FlatFormCandidate[];
  formAssistEnabled?: boolean;
  selectedItem?: SmartPdfSelection;
  onSelectItem?: (item: SmartPdfSelection) => void;
  className?: string;
  onPageRendered?: (pageNumber: number, width: number, height: number) => void;
  onCanvasRendered?: (canvas: HTMLCanvasElement) => void;
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
  onTransformImage?: (pageIndex: number, imageId: string, pdfRect: import("@/lib/pdf/selection").PdfRect) => void;
  onTransformVector?: (
    pageIndex: number,
    graphicId: string,
    pdfRect: import("@/lib/pdf/selection").PdfRect,
    linePoints?: { x1: number; y1: number; x2: number; y2: number },
  ) => void;
  onMoveText?: (
    pageIndex: number,
    spanId: string | string[],
    dx: number,
    dy: number,
  ) => void;
  onTransformAnnotation?: (
    pageIndex: number,
    annotId: string,
    pdfRect: import("@/lib/pdf/selection").PdfRect,
  ) => void;
}


export function PdfPageCanvas({
  pdfDocument,
  pageNumber,
  scale,
  rotation = 0,
  pageWidth = 612,
  pageHeight = 792,
  textSpans = [],
  images = [],
  graphics = [],
  fields = [],
  annotations = [],
  candidates = [],
  formAssistEnabled = true,
  selectedItem = null,
  onSelectItem,
  className = "",
  onPageRendered,
  onCanvasRendered,
  mode = "SELECT",
  fillAndSignTool = "text",
  onPlaceFreeText,
  onPlaceCheck,
  onPlaceCross,
  onPlaceSignature,
  onPlaceDrawing,
  onTransformImage,
  onTransformVector,
  onMoveText,
  onTransformAnnotation,
}: PdfPageCanvasProps) {



  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [isRendering, setIsRendering] = useState<boolean>(false);
  const [renderCompleted, setRenderCompleted] = useState<boolean>(false);
  const [renderError, setRenderError] = useState<string | null>(null);
  const currentRenderTask = useRef<RenderTask | null>(null);

  useEffect(() => {
    let isCancelled = false;

    async function renderPage() {
      if (!pdfDocument || !canvasRef.current) {
        setIsRendering(false);
        setRenderCompleted(false);
        return;
      }

      // Guard against out-of-bounds page requests during page count changes
      if (pageNumber < 1 || (pdfDocument.numPages && pageNumber > pdfDocument.numPages)) {
        setIsRendering(false);
        setRenderCompleted(false);
        return;
      }

      try {
        setIsRendering(true);
        setRenderCompleted(false);
        setRenderError(null);

        // Cancel previous task if still in flight
        if (currentRenderTask.current) {
          currentRenderTask.current.cancel();
          currentRenderTask.current = null;
        }

        const page = await pdfDocument.getPage(pageNumber);
        if (isCancelled || !canvasRef.current) {
          setIsRendering(false);
          return;
        }

        const viewport = page.getViewport({ scale, rotation });
        const canvas = canvasRef.current;
        const context = canvas.getContext("2d", { alpha: false });
        if (!context) {
          setIsRendering(false);
          return;
        }

        const dpr = typeof window !== "undefined" ? Math.min(window.devicePixelRatio || 1, 2) : 1;

        // Set display dimensions (CSS pixels)
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;

        // Set buffer dimensions (actual pixel buffer for sharp rendering on Retina displays)
        canvas.width = Math.floor(viewport.width * dpr);
        canvas.height = Math.floor(viewport.height * dpr);

        context.setTransform(dpr, 0, 0, dpr, 0, 0);

        const renderContext = {
          canvasContext: context,
          viewport: viewport,
          canvas: canvas,
        };

        const renderTask = page.render(renderContext);
        currentRenderTask.current = renderTask;

        await renderTask.promise;

        if (!isCancelled) {
          setIsRendering(false);
          setRenderCompleted(true);
          onPageRendered?.(pageNumber, viewport.width, viewport.height);
          if (canvas) {
            onCanvasRendered?.(canvas);
          }
        }
      } catch (err: unknown) {
        if (!isCancelled) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          // Ignore intentional cancellation when zooming or switching pages quickly
          if (!errorMsg.includes("cancelled") && !errorMsg.includes("RenderingCancelledException")) {
            console.error("PDF page render error:", err);
            setRenderError("Failed to render page.");
          }
          setIsRendering(false);
        }
      } finally {
        if (isCancelled) {
          setIsRendering(false);
        }
      }
    }

    void renderPage();

    return () => {
      isCancelled = true;
      if (currentRenderTask.current) {
        currentRenderTask.current.cancel();
        currentRenderTask.current = null;
      }
      setIsRendering(false);
    };
  }, [pdfDocument, pageNumber, scale, rotation, onPageRendered, onCanvasRendered]);

  return (
    <div className={`relative inline-block shadow-lg rounded-md bg-white overflow-hidden ${className}`}>
      <canvas
        ref={canvasRef}
        data-rendered={renderCompleted && !isRendering ? "true" : "false"}
        className="block transition-all"
      />
      {/* Interactive Selection & Creation Overlay */}
      {onSelectItem && !isRendering && (
        <PdfInteractiveOverlay
          pageWidth={pageWidth}
          pageHeight={pageHeight}
          scale={scale}
          rotation={rotation}
          textSpans={textSpans}
          images={images}
          graphics={graphics}
          fields={fields}
          annotations={annotations}
          candidates={candidates}
          formAssistEnabled={formAssistEnabled}
          pageNumber={pageNumber}
          selectedItem={selectedItem}
          onSelectItem={onSelectItem}
          mode={mode}
          fillAndSignTool={fillAndSignTool}
          onPlaceFreeText={onPlaceFreeText}
          onPlaceCheck={onPlaceCheck}
          onPlaceCross={onPlaceCross}
          onPlaceSignature={onPlaceSignature}
          onPlaceDrawing={onPlaceDrawing}
          onTransformImage={onTransformImage}
          onTransformVector={onTransformVector}
          onMoveText={onMoveText}
          onTransformAnnotation={onTransformAnnotation}
        />


      )}

      {isRendering && (
        <div className="absolute inset-0 bg-white/60 backdrop-blur-xs flex items-center justify-center pointer-events-none">
          <Loader2 className="size-6 animate-spin text-sky-600" />
        </div>
      )}
      {renderError && (
        <div className="absolute inset-0 bg-red-50 flex items-center justify-center p-4 text-xs text-red-600 text-center">
          {renderError}
        </div>
      )}
    </div>
  );
}
