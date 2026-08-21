"use client";

import { useEffect, useRef } from "react";
import type { PDFDocumentProxy, RenderTask } from "@/lib/pdf/pdfjs-init";
import { cn } from "@/lib/utils";

interface ThumbnailItemProps {
  pdfDocument: PDFDocumentProxy;
  pageNumber: number;
  isActive: boolean;
  onClick: () => void;
  isSelected: boolean;
  onToggleSelection: () => void;
}

function ThumbnailItem({
  pdfDocument,
  pageNumber,
  isActive,
  onClick,
  isSelected,
  onToggleSelection,
}: ThumbnailItemProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const currentTaskRef = useRef<RenderTask | null>(null);

  useEffect(() => {
    let isCancelled = false;

    async function renderThumbnail() {
      if (!canvasRef.current || !pdfDocument) return;

      if (pageNumber < 1 || (pdfDocument.numPages && pageNumber > pdfDocument.numPages)) {
        return;
      }

      try {
        if (currentTaskRef.current) {
          currentTaskRef.current.cancel();
          currentTaskRef.current = null;
        }

        const page = await pdfDocument.getPage(pageNumber);
        if (isCancelled || !canvasRef.current) return;

        // Scale to fit ~120px width thumbnail
        const unscaledViewport = page.getViewport({ scale: 1.0 });
        const targetWidth = 120;
        const scale = targetWidth / (unscaledViewport.width || 1);
        const viewport = page.getViewport({ scale });

        const canvas = canvasRef.current;
        const context = canvas.getContext("2d", { alpha: false });
        if (!context) return;

        canvas.width = viewport.width;
        canvas.height = viewport.height;

        const renderTask = page.render({
          canvasContext: context,
          viewport: viewport,
          canvas: canvas,
        });
        currentTaskRef.current = renderTask;

        await renderTask.promise;
      } catch {
        // Silently handle cancelled renders
      }
    }

    void renderThumbnail();

    return () => {
      isCancelled = true;
      if (currentTaskRef.current) {
        currentTaskRef.current.cancel();
        currentTaskRef.current = null;
      }
    };
  }, [pdfDocument, pageNumber]);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={onClick}
        className={cn(
          "group flex w-full flex-col items-center gap-1.5 rounded-xl p-2 text-center text-xs transition-all focus:outline-none focus:ring-2 focus:ring-sky-500",
          isActive
            ? "bg-sky-50 font-semibold text-sky-700"
            : "text-slate-500 hover:bg-slate-100 hover:text-slate-900",
        )}
        aria-label={`Page ${pageNumber}`}
        aria-current={isActive ? "page" : undefined}
      >
        <div
          className={cn(
            "flex min-h-[140px] w-full items-center justify-center overflow-hidden rounded-lg border-2 bg-white shadow-xs transition-all",
            isActive
              ? "border-sky-500 ring-2 ring-sky-500/20"
              : "border-slate-200 group-hover:border-slate-300",
          )}
        >
          <canvas ref={canvasRef} className="block h-auto max-w-full" />
        </div>
        <span>Page {pageNumber}</span>
      </button>
      <label className="absolute right-3 top-3 flex size-6 cursor-pointer items-center justify-center rounded-md border border-slate-300 bg-white/95 shadow-sm">
        <input
          type="checkbox"
          checked={isSelected}
          onChange={onToggleSelection}
          className="size-3.5 accent-sky-600"
          aria-label={`Toggle extraction selection ${pageNumber}`}
        />
      </label>
    </div>
  );
}

interface PdfThumbnailRailProps {
  pdfDocument: PDFDocumentProxy | null;
  pageCount: number;
  currentPage: number;
  onPageSelect: (pageNumber: number) => void;
  selectedPages: ReadonlySet<number>;
  onToggleSelection: (pageNumber: number) => void;
  className?: string;
}

export function PdfThumbnailRail({
  pdfDocument,
  pageCount,
  currentPage,
  onPageSelect,
  selectedPages,
  onToggleSelection,
  className = "",
}: PdfThumbnailRailProps) {
  if (!pdfDocument || pageCount === 0) return null;

  return (
    <aside
      className={cn(
        "w-44 shrink-0 border-r border-slate-200 bg-slate-50/90 p-3 overflow-y-auto h-full space-y-2 select-none",
        className,
      )}
      aria-label="Document page thumbnails"
      data-testid="pdf-thumbnail-rail"
    >
      <div className="text-[11px] font-semibold uppercase tracking-wider text-slate-400 px-2 pb-1 flex items-center justify-between">
        <span>Pages ({pageCount})</span>
      </div>
      <div className="space-y-3">
        {Array.from({ length: pageCount }, (_, i) => i + 1).map((pageNum) => (
          <ThumbnailItem
            key={`${pdfDocument.numPages}-p${pageNum}`}
            pdfDocument={pdfDocument}
            pageNumber={pageNum}
            isActive={currentPage === pageNum}
            onClick={() => onPageSelect(pageNum)}
            isSelected={selectedPages.has(pageNum)}
            onToggleSelection={() => onToggleSelection(pageNum)}
          />
        ))}
      </div>
    </aside>
  );
}
