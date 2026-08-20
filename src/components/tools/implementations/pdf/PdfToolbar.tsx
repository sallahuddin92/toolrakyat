"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  ChevronLeft,
  ChevronRight,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Minimize2,
  Download,
  Info,
  FolderOpen,
  ChevronDown,
  Loader2,
  Check,
  Search,
} from "lucide-react";
import { type ExportMode } from "@/lib/pdf/pdf-types";

interface PdfToolbarProps {
  filename: string;
  currentPage: number;
  pageCount: number;
  scale: number;
  isExporting: boolean;
  searchQuery?: string;
  searchResultCount?: number;
  activeSearchIndex?: number;
  onSearchChange?: (query: string) => void;
  onNextSearchResult?: () => void;
  onPrevSearchResult?: () => void;
  onPageChange: (pageNumber: number) => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFitWidth: () => void;
  onFitPage: () => void;
  onExport: (mode: ExportMode) => Promise<void>;
  onShowInfo: () => void;
  onOpenNewFile: () => void;
}

export function PdfToolbar({
  filename,
  currentPage,
  pageCount,
  scale,
  isExporting,
  searchQuery = "",
  searchResultCount = 0,
  activeSearchIndex = 0,
  onSearchChange,
  onNextSearchResult,
  onPrevSearchResult,
  onPageChange,
  onZoomIn,
  onZoomOut,
  onFitWidth,
  onFitPage,
  onExport,
  onShowInfo,
  onOpenNewFile,
}: PdfToolbarProps) {
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  const [showSearchInput, setShowSearchInput] = useState(false);
  const zoomPercentage = Math.round(scale * 100);

  return (
    <header className="h-14 border-b border-slate-200 bg-white px-4 flex items-center justify-between gap-3 shrink-0 select-none">
      {/* Left: Filename & File Switcher */}
      <div className="flex items-center gap-2 min-w-0 max-w-[280px] sm:max-w-xs">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onOpenNewFile}
          className="text-xs text-slate-600 gap-1.5 h-8 rounded-lg shrink-0"
          title="Open another PDF document"
        >
          <FolderOpen className="size-3.5" />
          <span className="hidden sm:inline">Open</span>
        </Button>
        <span className="text-slate-300">|</span>
        <div className="flex items-center gap-1.5 min-w-0">
          <span className="text-xs font-semibold text-slate-800 truncate" title={filename}>
            {filename}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onShowInfo}
            className="size-6 text-slate-400 hover:text-slate-600 rounded-md shrink-0"
            title="Document Properties"
          >
            <Info className="size-3.5" />
          </Button>
        </div>
      </div>

      {/* Center: Page Navigation, Zoom Controls & Search */}
      <div className="flex items-center gap-1 sm:gap-3">
        {/* Page Nav */}
        <div className="flex items-center gap-1 bg-slate-50 p-0.5 rounded-lg border border-slate-200">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => onPageChange(currentPage - 1)}
            disabled={currentPage <= 1}
            className="size-7 text-slate-600 rounded-md"
            title="Previous Page"
            aria-label="Previous Page"
          >
            <ChevronLeft className="size-3.5" />
          </Button>

          <span className="text-xs text-slate-700 font-medium px-1.5 tabular-nums">
            {currentPage} / {pageCount}
          </span>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => onPageChange(currentPage + 1)}
            disabled={currentPage >= pageCount}
            className="size-7 text-slate-600 rounded-md"
            title="Next Page"
            aria-label="Next Page"
          >
            <ChevronRight className="size-3.5" />
          </Button>
        </div>

        {/* Zoom Controls */}
        <div className="hidden md:flex items-center gap-1 bg-slate-50 p-0.5 rounded-lg border border-slate-200">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onZoomOut}
            disabled={scale <= 0.25}
            className="size-7 text-slate-600 rounded-md"
            title="Zoom Out"
            aria-label="Zoom Out"
          >
            <ZoomOut className="size-3.5" />
          </Button>

          <span className="text-xs text-slate-700 font-medium px-1.5 w-12 text-center tabular-nums">
            {zoomPercentage}%
          </span>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onZoomIn}
            disabled={scale >= 4.0}
            className="size-7 text-slate-600 rounded-md"
            title="Zoom In"
            aria-label="Zoom In"
          >
            <ZoomIn className="size-3.5" />
          </Button>
        </div>

        {/* Fit Width / Fit Page */}
        <div className="hidden lg:flex items-center gap-1">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onFitWidth}
            className="size-8 text-slate-500 hover:text-slate-900 rounded-lg"
            title="Fit Width"
            aria-label="Fit Width"
          >
            <Maximize2 className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onFitPage}
            className="size-8 text-slate-500 hover:text-slate-900 rounded-lg"
            title="Fit Page"
            aria-label="Fit Page"
          >
            <Minimize2 className="size-3.5" />
          </Button>
        </div>

        {/* StarPDF Native Search Input */}
        {onSearchChange && (
          <div className="flex items-center gap-1">
            {showSearchInput ? (
              <div className="flex items-center gap-1 bg-slate-50 border border-slate-200 rounded-lg px-2 py-0.5 animate-in fade-in duration-100">
                <Search className="size-3 text-slate-400 shrink-0" />
                <input
                  type="text"
                  placeholder="Search in document..."
                  value={searchQuery}
                  onChange={(e) => onSearchChange(e.target.value)}
                  className="bg-transparent text-xs text-slate-800 placeholder-slate-400 focus:outline-none w-28 sm:w-36 h-6"
                  autoFocus
                />
                {searchResultCount > 0 && (
                  <span className="text-[10px] text-slate-500 font-medium whitespace-nowrap pl-1">
                    {activeSearchIndex + 1}/{searchResultCount}
                  </span>
                )}
                {searchResultCount > 0 && onPrevSearchResult && onNextSearchResult && (
                  <div className="flex items-center gap-0.5 pl-1">
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={onPrevSearchResult}
                      className="size-5 rounded text-slate-500"
                      title="Previous hit"
                    >
                      <ChevronLeft className="size-3" />
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={onNextSearchResult}
                      className="size-5 rounded text-slate-500"
                      title="Next hit"
                    >
                      <ChevronRight className="size-3" />
                    </Button>
                  </div>
                )}
                <button
                  type="button"
                  onClick={() => {
                    setShowSearchInput(false);
                    onSearchChange("");
                  }}
                  className="text-slate-400 hover:text-slate-600 text-xs px-1"
                >
                  ✕
                </button>
              </div>
            ) : (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => setShowSearchInput(true)}
                className="size-8 text-slate-500 hover:text-slate-900 rounded-lg"
                title="Search text (StarPDF WASM)"
                aria-label="Search text"
              >
                <Search className="size-3.5" />
              </Button>
            )}
          </div>
        )}
      </div>

      {/* Right: Export Menu */}
      <div className="relative">
        <div className="flex items-center rounded-xl bg-sky-600 text-white shadow-xs">
          <Button
            type="button"
            onClick={() => void onExport("editable")}
            disabled={isExporting}
            className="h-8 rounded-l-xl rounded-r-none bg-sky-600 hover:bg-sky-700 text-white text-xs px-3 font-medium border-r border-sky-500 gap-1.5"
          >
            {isExporting ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <Download className="size-3.5" />
            )}
            <span>Export Editable</span>
          </Button>
          <Button
            type="button"
            onClick={() => setExportMenuOpen((prev) => !prev)}
            disabled={isExporting}
            className="h-8 px-1.5 rounded-l-none rounded-r-xl bg-sky-600 hover:bg-sky-700 text-white"
            title="More export options"
            aria-label="More export options"
          >
            <ChevronDown className="size-3.5" />
          </Button>
        </div>

        {/* Dropdown Menu */}
        {exportMenuOpen && (
          <>
            <div className="fixed inset-0 z-40" onClick={() => setExportMenuOpen(false)} />
            <div className="absolute right-0 mt-1.5 w-60 rounded-xl bg-white shadow-xl border border-slate-200 py-1 z-50 text-xs animate-in fade-in zoom-in-95 duration-100">
              <button
                type="button"
                onClick={() => {
                  setExportMenuOpen(false);
                  void onExport("editable");
                }}
                className="w-full text-left px-3 py-2.5 hover:bg-slate-50 flex flex-col gap-0.5 text-slate-800"
              >
                <span className="font-semibold flex items-center justify-between">
                  Export Editable PDF
                  <Check className="size-3 text-sky-600" />
                </span>
                <span className="text-[11px] text-slate-500">
                  Preserves interactive AcroForm fields for future editing.
                </span>
              </button>

              <div className="my-1 border-t border-slate-100" />

              <button
                type="button"
                onClick={() => {
                  setExportMenuOpen(false);
                  void onExport("flattened");
                }}
                className="w-full text-left px-3 py-2.5 hover:bg-slate-50 flex flex-col gap-0.5 text-slate-800"
              >
                <span className="font-semibold">Export Flattened PDF</span>
                <span className="text-[11px] text-slate-500">
                  Burns form values into static page content; locks widgets.
                </span>
              </button>
            </div>
          </>
        )}
      </div>
    </header>
  );
}
