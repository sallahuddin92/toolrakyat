"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import type { PDFDocumentProxy, RenderTask } from "@/lib/pdf/pdfjs-init";
import { cn } from "@/lib/utils";
import { Copy, FileOutput, Trash2, GripVertical } from "lucide-react";

interface ThumbnailItemProps {
  pdfDocument: PDFDocumentProxy | null;
  pageNumber: number;
  isActive: boolean;
  isSelected: boolean;
  isDragging: boolean;
  dropPosition: "before" | "after" | null;
  onClick: (e: React.MouseEvent) => void;
  onToggleSelection: () => void;
  onDragStart: (e: React.DragEvent, pageNumber: number) => void;
  onDragOver: (e: React.DragEvent, pageNumber: number) => void;
  onDragLeave: (e: React.DragEvent, pageNumber: number) => void;
  onDrop: (e: React.DragEvent, pageNumber: number) => void;
  onDragEnd: (e: React.DragEvent) => void;
}

function ThumbnailItem({
  pdfDocument,
  pageNumber,
  isActive,
  isSelected,
  isDragging,
  dropPosition,
  onClick,
  onToggleSelection,
  onDragStart,
  onDragOver,
  onDragLeave,
  onDrop,
  onDragEnd,
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
    <div
      className={cn(
        "relative group transition-transform",
        isDragging && "opacity-40 scale-95",
      )}
      draggable
      onDragStart={(e) => onDragStart(e, pageNumber)}
      onDragOver={(e) => onDragOver(e, pageNumber)}
      onDragLeave={(e) => onDragLeave(e, pageNumber)}
      onDrop={(e) => onDrop(e, pageNumber)}
      onDragEnd={onDragEnd}
      data-testid={`thumbnail-item-${pageNumber}`}
    >
      {/* Drop Insertion Line Before */}
      {dropPosition === "before" && (
        <div
          className="absolute -top-1.5 left-0 right-0 h-1 bg-sky-500 rounded-full z-20 shadow-xs ring-2 ring-sky-300 pointer-events-none animate-pulse"
          data-testid="drop-indicator-before"
        />
      )}

      <button
        type="button"
        onClick={onClick}
        className={cn(
          "group flex w-full flex-col items-center gap-1.5 rounded-xl p-2 text-center text-xs transition-all focus:outline-none focus:ring-2 focus:ring-sky-500 cursor-pointer",
          isActive
            ? "bg-sky-50 font-semibold text-sky-700 ring-1 ring-sky-200"
            : isSelected
              ? "bg-slate-100 font-medium text-slate-900"
              : "text-slate-500 hover:bg-slate-100/80 hover:text-slate-900",
        )}
        aria-label={`Page ${pageNumber}`}
        aria-current={isActive ? "page" : undefined}
      >
        <div
          className={cn(
            "flex min-h-[140px] w-full items-center justify-center overflow-hidden rounded-lg border-2 bg-white shadow-xs transition-all relative",
            isActive
              ? "border-sky-500 ring-2 ring-sky-500/20 shadow-sm"
              : isSelected
                ? "border-sky-400 ring-1 ring-sky-300"
                : "border-slate-200 group-hover:border-slate-300",
          )}
        >
          {pdfDocument ? (
            <canvas ref={canvasRef} className="block h-auto max-w-full" />
          ) : (
            <div className="h-32 w-full animate-pulse bg-slate-100" />
          )}

          {/* Drag Handle Overlay icon on hover */}
          <div className="absolute top-1.5 left-1.5 opacity-0 group-hover:opacity-60 transition-opacity text-slate-400">
            <GripVertical className="size-3.5" />
          </div>
        </div>
        <div className="flex items-center justify-between w-full px-1">
          <span className={cn(isSelected && "text-sky-700 font-semibold")}>Page {pageNumber}</span>
          {isSelected && (
            <span className="size-1.5 rounded-full bg-sky-600" />
          )}
        </div>
      </button>

      {/* Page Selection Checkbox */}
      <label className="absolute right-3 top-3 flex size-6 cursor-pointer items-center justify-center rounded-md border border-slate-300 bg-white/95 shadow-xs hover:border-slate-400 transition-colors z-10">
        <input
          type="checkbox"
          checked={isSelected}
          onChange={onToggleSelection}
          className="size-3.5 accent-sky-600 cursor-pointer"
          aria-label={`Toggle extraction selection ${pageNumber}`}
        />
      </label>

      {/* Drop Insertion Line After */}
      {dropPosition === "after" && (
        <div
          className="absolute -bottom-1.5 left-0 right-0 h-1 bg-sky-500 rounded-full z-20 shadow-xs ring-2 ring-sky-300 pointer-events-none animate-pulse"
          data-testid="drop-indicator-after"
        />
      )}
    </div>
  );
}

export interface PdfThumbnailRailProps {
  pdfDocument: PDFDocumentProxy | null;
  pageCount: number;
  currentPage: number;
  onPageSelect: (pageNumber: number) => void;
  selectedPages: ReadonlySet<number>;
  onToggleSelection: (pageNumber: number) => void;
  onSelectRange?: (startPage: number, endPage: number) => void;
  onSelectAllPages?: () => void;
  onClearSelectedPages?: () => void;
  onReorderPages?: (movingPages: number[], targetPage: number, placeBefore: boolean) => void;
  onDuplicateSelected?: () => void;
  onDeleteSelected?: () => void;
  onExtractSelected?: () => void;
  onInsertBlankBefore?: (pageNumber: number) => void;
  onInsertBlankAfter?: (pageNumber: number) => void;
  className?: string;
}

export function PdfThumbnailRail({
  pdfDocument,
  pageCount,
  currentPage,
  onPageSelect,
  selectedPages,
  onToggleSelection,
  onSelectRange,
  onSelectAllPages,
  onClearSelectedPages,
  onReorderPages,
  onDuplicateSelected,
  onDeleteSelected,
  onExtractSelected,
  className = "",
}: PdfThumbnailRailProps) {
  const railRef = useRef<HTMLElement | null>(null);
  const [explicitAnchorPage, setExplicitAnchorPage] = useState<number | null>(null);
  const anchorPage = explicitAnchorPage ?? currentPage;
  const [draggingPages, setDraggingPages] = useState<number[] | null>(null);
  const [dropTarget, setDropTarget] = useState<{ pageNumber: number; position: "before" | "after" } | null>(null);

  // Cancel drag on Escape key
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        setDraggingPages(null);
        setDropTarget(null);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleThumbnailClick = useCallback(
    (e: React.MouseEvent, pageNum: number) => {
      if (e.shiftKey && onSelectRange) {
        // Shift range selection from anchorPage to pageNum
        onSelectRange(anchorPage, pageNum);
        onPageSelect(pageNum);
      } else if (e.metaKey || e.ctrlKey) {
        // Cmd/Ctrl toggle
        onToggleSelection(pageNum);
        setExplicitAnchorPage(pageNum);
        onPageSelect(pageNum);
      } else {
        // Plain single click: navigate to page
        onPageSelect(pageNum);
        setExplicitAnchorPage(pageNum);
      }
    },
    [anchorPage, onPageSelect, onSelectRange, onToggleSelection],
  );

  const handleDragStart = useCallback(
    (e: React.DragEvent, pageNum: number) => {
      const moving = selectedPages.has(pageNum) && selectedPages.size > 1
        ? Array.from(selectedPages).sort((a, b) => a - b)
        : [pageNum];

      setDraggingPages(moving);
      e.dataTransfer.setData("text/plain", JSON.stringify(moving));
      e.dataTransfer.effectAllowed = "move";
    },
    [selectedPages],
  );

  const handleDragOver = useCallback(
    (e: React.DragEvent, pageNum: number) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";

      // Auto-scroll container when dragging near top/bottom edges
      if (railRef.current) {
        const railRect = railRef.current.getBoundingClientRect();
        const topEdgeDist = e.clientY - railRect.top;
        const bottomEdgeDist = railRect.bottom - e.clientY;

        if (topEdgeDist < 40) {
          railRef.current.scrollTop -= 12;
        } else if (bottomEdgeDist < 40) {
          railRef.current.scrollTop += 12;
        }
      }

      const targetElem = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const midY = targetElem.top + targetElem.height / 2;
      const position: "before" | "after" = e.clientY < midY ? "before" : "after";

      setDropTarget({ pageNumber: pageNum, position });
    },
    [],
  );

  const handleDragLeave = useCallback(() => {
    // Optional clear on leaving element
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent, pageNum: number) => {
      e.preventDefault();
      if (!draggingPages || draggingPages.length === 0 || !onReorderPages) {
        setDraggingPages(null);
        setDropTarget(null);
        return;
      }

      const targetElem = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const midY = targetElem.top + targetElem.height / 2;
      const placeBefore = e.clientY < midY;

      onReorderPages(draggingPages, pageNum, placeBefore);
      setDraggingPages(null);
      setDropTarget(null);
    },
    [draggingPages, onReorderPages],
  );

  const handleDragEnd = useCallback(() => {
    setDraggingPages(null);
    setDropTarget(null);
  }, []);

  if (pageCount === 0) return null;

  const hasSelection = selectedPages.size > 0;

  return (
    <aside
      ref={railRef}
      className={cn(
        "w-48 shrink-0 border-r border-slate-200 bg-slate-50/90 p-3 overflow-y-auto h-full space-y-2 select-none flex flex-col",
        className,
      )}
      aria-label="Document page thumbnails"
      data-testid="pdf-thumbnail-rail"
    >
      {/* Header & Stats */}
      <div className="space-y-1.5 pb-1">
        <div className="text-[11px] font-semibold uppercase tracking-wider text-slate-500 px-1 flex items-center justify-between">
          <span>Pages ({pageCount})</span>
          {hasSelection && (
            <span className="text-sky-700 bg-sky-100 px-1.5 py-0.5 rounded-full font-bold text-[10px]" data-testid="selected-pages-count">
              {selectedPages.size} sel
            </span>
          )}
        </div>

        {/* Multi-Selection Action Toolbar */}
        <div className="flex items-center justify-between gap-1 px-1 text-xs">
          {hasSelection ? (
            <>
              <button
                type="button"
                onClick={onClearSelectedPages}
                className="text-[11px] text-slate-500 hover:text-slate-800 font-medium hover:underline cursor-pointer"
              >
                Clear
              </button>
              <div className="flex items-center gap-1">
                {onDuplicateSelected && (
                  <button
                    type="button"
                    onClick={onDuplicateSelected}
                    title="Duplicate selected page(s)"
                    className="p-1 rounded-md text-slate-600 hover:bg-slate-200 hover:text-slate-900 cursor-pointer"
                    data-testid="rail-duplicate-selected"
                  >
                    <Copy className="size-3.5" />
                  </button>
                )}
                {onExtractSelected && (
                  <button
                    type="button"
                    onClick={onExtractSelected}
                    title="Extract selected page(s)"
                    className="p-1 rounded-md text-slate-600 hover:bg-slate-200 hover:text-slate-900 cursor-pointer"
                    data-testid="rail-extract-selected"
                  >
                    <FileOutput className="size-3.5" />
                  </button>
                )}
                {onDeleteSelected && (
                  <button
                    type="button"
                    onClick={onDeleteSelected}
                    disabled={selectedPages.size >= pageCount}
                    title="Delete selected page(s)"
                    className={cn(
                      "p-1 rounded-md text-red-600 hover:bg-red-100 hover:text-red-700 cursor-pointer",
                      selectedPages.size >= pageCount && "opacity-40 cursor-not-allowed",
                    )}
                    data-testid="rail-delete-selected"
                  >
                    <Trash2 className="size-3.5" />
                  </button>
                )}
              </div>
            </>
          ) : (
            onSelectAllPages && (
              <button
                type="button"
                onClick={onSelectAllPages}
                className="text-[11px] text-slate-500 hover:text-slate-800 font-medium hover:underline cursor-pointer"
              >
                Select all
              </button>
            )
          )}
        </div>
      </div>

      {/* Thumbnails List */}
      <div className="space-y-3 flex-1 pb-4">
        {Array.from({ length: pageCount }, (_, i) => i + 1).map((pageNum) => {
          const isDraggingThis = draggingPages ? draggingPages.includes(pageNum) : false;
          const dropPos = dropTarget && dropTarget.pageNumber === pageNum ? dropTarget.position : null;

          return (
            <ThumbnailItem
              key={`thumb-p${pageNum}-of-${pageCount}`}
              pdfDocument={pdfDocument}
              pageNumber={pageNum}
              isActive={currentPage === pageNum}
              isSelected={selectedPages.has(pageNum)}
              isDragging={isDraggingThis}
              dropPosition={dropPos}
              onClick={(e) => handleThumbnailClick(e, pageNum)}
              onToggleSelection={() => onToggleSelection(pageNum)}
              onDragStart={handleDragStart}
              onDragOver={handleDragOver}
              onDragLeave={handleDragLeave}
              onDrop={handleDrop}
              onDragEnd={handleDragEnd}
            />
          );
        })}
      </div>
    </aside>
  );
}
