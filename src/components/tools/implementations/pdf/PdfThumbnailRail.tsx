"use client";

import { useEffect, useRef, useState } from "react";
import type { PDFDocumentProxy } from "@/lib/pdf/pdfjs-init";
import { cn } from "@/lib/utils";

interface ThumbnailItemProps {
  pdfDocument: PDFDocumentProxy;
  pageNumber: number;
  isActive: boolean;
  onClick: () => void;
}

function ThumbnailItem({ pdfDocument, pageNumber, isActive, onClick }: ThumbnailItemProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [rendered, setRendered] = useState(false);

  useEffect(() => {
    let isCancelled = false;

    async function renderThumbnail() {
      if (!canvasRef.current || rendered) return;

      try {
        const page = await pdfDocument.getPage(pageNumber);
        if (isCancelled || !canvasRef.current) return;

        // Scale to fit ~120px width thumbnail
        const unscaledViewport = page.getViewport({ scale: 1.0 });
        const targetWidth = 120;
        const scale = targetWidth / unscaledViewport.width;
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

        await renderTask.promise;
        if (!isCancelled) {
          setRendered(true);
        }
      } catch {
        // Silently handle cancelled renders
      }
    }

    void renderThumbnail();

    return () => {
      isCancelled = true;
    };
  }, [pdfDocument, pageNumber, rendered]);

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "group flex flex-col items-center gap-1.5 p-2 rounded-xl text-xs transition-all w-full text-center focus:outline-none focus:ring-2 focus:ring-sky-500",
        isActive
          ? "bg-sky-50 text-sky-700 font-semibold"
          : "hover:bg-slate-100 text-slate-500 hover:text-slate-900",
      )}
      aria-label={`Page ${pageNumber}`}
      aria-current={isActive ? "page" : undefined}
    >
      <div
        className={cn(
          "w-full rounded-lg overflow-hidden border-2 bg-white shadow-xs transition-all flex items-center justify-center min-h-[140px]",
          isActive
            ? "border-sky-500 ring-2 ring-sky-500/20"
            : "border-slate-200 group-hover:border-slate-300",
        )}
      >
        <canvas ref={canvasRef} className="max-w-full h-auto block" />
      </div>
      <span>Page {pageNumber}</span>
    </button>
  );
}

interface PdfThumbnailRailProps {
  pdfDocument: PDFDocumentProxy | null;
  pageCount: number;
  currentPage: number;
  onPageSelect: (pageNumber: number) => void;
  className?: string;
}

export function PdfThumbnailRail({
  pdfDocument,
  pageCount,
  currentPage,
  onPageSelect,
  className = "",
}: PdfThumbnailRailProps) {
  if (!pdfDocument || pageCount === 0) return null;

  return (
    <aside
      className={cn(
        "w-44 shrink-0 border-r border-slate-200 bg-slate-50/80 p-3 overflow-y-auto max-h-[750px] space-y-2",
        className,
      )}
      aria-label="Document page thumbnails"
    >
      <div className="text-[11px] font-semibold uppercase tracking-wider text-slate-400 px-2 pb-1">
        Pages ({pageCount})
      </div>
      <div className="space-y-3">
        {Array.from({ length: pageCount }, (_, i) => i + 1).map((pageNum) => (
          <ThumbnailItem
            key={pageNum}
            pdfDocument={pdfDocument}
            pageNumber={pageNum}
            isActive={currentPage === pageNum}
            onClick={() => onPageSelect(pageNum)}
          />
        ))}
      </div>
    </aside>
  );
}
