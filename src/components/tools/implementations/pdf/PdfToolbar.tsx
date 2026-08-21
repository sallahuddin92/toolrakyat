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
  Undo2,
  Redo2,
  FilePlus2,
  PanelLeftClose,
  PanelLeftOpen,
  ShieldCheck,
} from "lucide-react";
import { type ExportMode } from "@/lib/pdf/pdf-types";

interface PdfToolbarProps {
  filename: string;
  currentPage: number;
  pageCount: number;
  scale: number;
  isExporting: boolean;
  isModified?: boolean;
  canUndo?: boolean;
  canRedo?: boolean;
  onUndo?: () => void;
  onRedo?: () => void;
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
  onMergeClick?: () => void;
  isThumbnailsOpen?: boolean;
  onToggleThumbnails?: () => void;
}

export function PdfToolbar({
  filename,
  currentPage,
  pageCount,
  scale,
  isExporting,
  isModified = false,
  canUndo = false,
  canRedo = false,
  onUndo,
  onRedo,
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
  onMergeClick,
  isThumbnailsOpen = true,
  onToggleThumbnails,
}: PdfToolbarProps) {
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  const [showSearchInput, setShowSearchInput] = useState(false);
  const zoomPercentage = Math.round(scale * 100);

  return (
    <header className="h-14 border-b border-slate-200 bg-white px-3 sm:px-4 flex items-center justify-between gap-2 sm:gap-3 shrink-0 select-none">
      {/* Left: Rail toggle / Open / Filename / Modified badge / Undo / Redo */}
      <div className="flex items-center gap-1.5 sm:gap-2 shrink-0">
        {onToggleThumbnails && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onToggleThumbnails}
            className="size-8 text-slate-600 rounded-lg shrink-0 flex items-center justify-center"
            title={isThumbnailsOpen ? "Hide Page Thumbnails" : "Show Page Thumbnails"}
            aria-label={isThumbnailsOpen ? "Hide page thumbnails" : "Show page thumbnails"}
            data-testid="toolbar-toggle-thumbnails-btn"
          >
            {isThumbnailsOpen ? <PanelLeftClose className="size-4" /> : <PanelLeftOpen className="size-4" />}
          </Button>
        )}

        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onOpenNewFile}
          className="text-xs text-slate-600 gap-1.5 h-8 rounded-lg shrink-0"
          title="Open another PDF document"
          data-testid="toolbar-open-file-btn"
        >
          <FolderOpen className="size-3.5" />
          <span className="hidden sm:inline">Open</span>
        </Button>

        {onMergeClick && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onMergeClick}
            className="text-xs text-slate-600 gap-1 h-8 rounded-lg shrink-0 hidden lg:inline-flex"
            title="Add and merge another PDF"
            data-testid="toolbar-merge-btn"
          >
            <FilePlus2 className="size-3.5 text-slate-500" />
            <span>Add PDF</span>
          </Button>
        )}

        <span className="text-slate-300 hidden sm:inline">|</span>

        {/* History Undo / Redo */}
        <div className="flex items-center gap-0.5 shrink-0">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onUndo}
            disabled={!canUndo}
            className="size-7 text-slate-600 disabled:text-slate-300 rounded-md shrink-0"
            title="Undo (Ctrl+Z / Cmd+Z)"
            aria-label="Undo"
            data-testid="toolbar-undo-btn"
          >
            <Undo2 className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onRedo}
            disabled={!canRedo}
            className="size-7 text-slate-600 disabled:text-slate-300 rounded-md shrink-0"
            title="Redo (Ctrl+Shift+Z / Cmd+Shift+Z)"
            aria-label="Redo"
            data-testid="toolbar-redo-btn"
          >
            <Redo2 className="size-3.5" />
          </Button>
        </div>

        <span className="text-slate-300">|</span>

        <div className="flex items-center gap-1.5 min-w-0">
          <span className="text-xs font-semibold text-slate-800 truncate max-w-[80px] sm:max-w-[120px] lg:max-w-[160px]" title={filename}>
            {filename}
          </span>
          {isModified && (
            <span
              className="inline-block w-2.5 h-2.5 rounded-full bg-amber-500 shrink-0"
              title="Document has unsaved modifications"
              data-testid="document-modified-dot"
            />
          )}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onShowInfo}
            className="size-6 text-slate-400 hover:text-slate-600 rounded-md shrink-0"
            title="Document Properties & Diagnostics"
            aria-label="Document properties"
            data-testid="toolbar-info-btn"
          >
            <Info className="size-3.5" />
          </Button>
        </div>
      </div>

      {/* Center: Page Navigation, Zoom Controls & Search */}
      <div className="flex items-center gap-1 sm:gap-2.5">
        {/* Page Nav */}
        <div className="flex items-center gap-0.5 sm:gap-1 bg-slate-50 p-0.5 rounded-lg border border-slate-200">
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

          <span className="text-xs text-slate-700 font-medium px-1 sm:px-1.5 tabular-nums">
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
                  className="bg-transparent text-xs text-slate-800 placeholder-slate-400 focus:outline-none w-24 sm:w-36 h-6"
                  autoFocus
                  data-testid="search-query-input"
                />
                {searchResultCount > 0 && (
                  <span className="text-[10px] text-slate-500 font-medium whitespace-nowrap pl-1" data-testid="search-results-count">
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
                      aria-label="Previous hit"
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
                      aria-label="Next hit"
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
                  aria-label="Close search"
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
                data-testid="toolbar-search-toggle-btn"
              >
                <Search className="size-3.5" />
              </Button>
            )}
          </div>
        )}
      </div>

      {/* Right: Privacy badge + Export Menu */}
      <div className="flex items-center gap-2">
        {/* Compact Local Processing Privacy Badge */}
        <div
          className="hidden xl:flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-emerald-50 text-emerald-800 border border-emerald-200 text-xs font-medium cursor-help"
          title="All PDF processing happens locally in your browser via StarPDF WebAssembly. 0 bytes uploaded to servers."
          data-testid="privacy-local-badge"
        >
          <ShieldCheck className="size-3.5 text-emerald-600 shrink-0" />
          <span>Local processing</span>
        </div>

        <div className="relative">
          <div className="flex items-center rounded-xl bg-sky-600 text-white shadow-xs">
            <Button
              type="button"
              onClick={() => void onExport("editable")}
              disabled={isExporting}
              className="h-8 rounded-l-xl rounded-r-none bg-transparent hover:bg-sky-700 text-white px-2.5 sm:px-3 text-xs gap-1.5 border-r border-sky-500 font-medium"
              data-testid="toolbar-export-btn"
            >
              {isExporting ? (
                <>
                  <Loader2 className="size-3.5 animate-spin" />
                  <span className="hidden sm:inline">Exporting...</span>
                </>
              ) : (
                <>
                  <Download className="size-3.5" />
                  <span className="hidden sm:inline">Export Editable</span>
                </>
              )}
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              disabled={isExporting}
              onClick={() => setExportMenuOpen(!exportMenuOpen)}
              className="size-8 rounded-l-none rounded-r-xl bg-transparent hover:bg-sky-700 text-white"
              title="More export options"
              aria-label="More export options"
              aria-expanded={exportMenuOpen}
            >
              <ChevronDown className="size-3.5" />
            </Button>
          </div>

          {exportMenuOpen && (
            <>
              <div
                className="fixed inset-0 z-40"
                onClick={() => setExportMenuOpen(false)}
              />
              <div className="absolute right-0 top-full mt-1.5 w-60 rounded-xl border border-slate-200 bg-white p-1.5 shadow-xl z-50 animate-in fade-in duration-100">
                <button
                  type="button"
                  onClick={() => {
                    setExportMenuOpen(false);
                    void onExport("editable");
                  }}
                  className="flex w-full items-start gap-2.5 rounded-lg p-2 text-left hover:bg-slate-50 transition-colors"
                  data-testid="export-editable-option"
                >
                  <Check className="size-4 text-sky-600 mt-0.5 shrink-0" />
                  <div>
                    <div className="text-xs font-semibold text-slate-800">
                      Export Editable PDF
                    </div>
                    <div className="text-[11px] text-slate-500">
                      Preserves interactive AcroForms, markup annotations, and native text
                    </div>
                  </div>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setExportMenuOpen(false);
                    void onExport("flattened");
                  }}
                  className="flex w-full items-start gap-2.5 rounded-lg p-2 text-left hover:bg-slate-50 transition-colors mt-0.5"
                  data-testid="export-flattened-option"
                >
                  <Download className="size-4 text-slate-500 mt-0.5 shrink-0" />
                  <div>
                    <div className="text-xs font-semibold text-slate-800">
                      Export Flattened PDF
                    </div>
                    <div className="text-[11px] text-slate-500">
                      Locks and embeds all form values and annotations permanently
                    </div>
                  </div>
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </header>
  );
}
