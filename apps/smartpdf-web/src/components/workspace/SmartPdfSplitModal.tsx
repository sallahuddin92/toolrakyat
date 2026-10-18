"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Scissors, X } from "lucide-react";
import toast from "react-hot-toast";

interface SmartPdfSplitModalProps {
  isOpen: boolean;
  totalPageCount: number;
  selectedPages: ReadonlySet<number>;
  onClose: () => void;
  onExtractSelected: () => Promise<void>;
  onSplitRanges: (ranges: { start: number; endExclusive: number }[]) => Promise<void>;
}

export function SmartPdfSplitModal({
  isOpen,
  totalPageCount,
  selectedPages,
  onClose,
  onExtractSelected,
  onSplitRanges,
}: SmartPdfSplitModalProps) {
  const [splitMode, setSplitMode] = useState<"selected" | "ranges" | "single-pages">(
    selectedPages.size > 0 ? "selected" : "ranges",
  );
  const [rangeInput, setRangeInput] = useState<string>("1-2, 3-4");
  const [isProcessing, setIsProcessing] = useState<boolean>(false);

  if (!isOpen) return null;

  function parseRanges(text: string, maxPages: number): { start: number; endExclusive: number }[] {
    const parts = text.split(",");
    const result: { start: number; endExclusive: number }[] = [];

    for (const part of parts) {
      const trimmed = part.trim();
      if (!trimmed) continue;
      if (trimmed.includes("-")) {
        const [startStr, endStr] = trimmed.split("-");
        const start = parseInt(startStr, 10);
        const end = parseInt(endStr, 10);
        if (!isNaN(start) && !isNaN(end)) {
          const s = Math.max(1, Math.min(start, end));
          const e = Math.min(maxPages, Math.max(start, end));
          if (s <= e) {
            result.push({ start: s - 1, endExclusive: e });
          }
        }
      } else {
        const p = parseInt(trimmed, 10);
        if (!isNaN(p) && p >= 1 && p <= maxPages) {
          result.push({ start: p - 1, endExclusive: p });
        }
      }
    }
    return result;
  }

  async function handleConfirm() {
    setIsProcessing(true);
    try {
      if (splitMode === "selected") {
        if (selectedPages.size === 0) {
          toast.error("No pages currently selected in the organizer.");
          return;
        }
        await onExtractSelected();
        onClose();
      } else if (splitMode === "ranges") {
        const parsed = parseRanges(rangeInput, totalPageCount);
        if (parsed.length === 0) {
          toast.error("Please enter valid page ranges (e.g. 1-2, 3-4).");
          return;
        }
        await onSplitRanges(parsed);
        onClose();
      } else if (splitMode === "single-pages") {
        const allSingleRanges = Array.from({ length: totalPageCount }, (_, i) => ({
          start: i,
          endExclusive: i + 1,
        }));
        await onSplitRanges(allSingleRanges);
        onClose();
      }
    } catch (e: unknown) {
      toast.error(e instanceof Error ? e.message : "Failed to split document.");
    } finally {
      setIsProcessing(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs animate-in fade-in duration-100"
      role="dialog"
      aria-modal="true"
      aria-labelledby="split-modal-title"
      data-testid="smartpdf-split-modal"
    >
      <div className="w-full max-w-md bg-white rounded-2xl border border-slate-200 shadow-2xl overflow-hidden p-6 space-y-4 animate-in zoom-in-95 duration-100">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2.5">
            <div className="size-9 rounded-lg bg-sky-50 text-sky-600 flex items-center justify-center">
              <Scissors className="size-5" />
            </div>
            <div>
              <h3 id="split-modal-title" className="text-base font-semibold text-slate-900">
                Split / Extract Document
              </h3>
              <p className="text-xs text-slate-500">
                Extract selected pages or split into multiple separate PDF files.
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition-colors"
            aria-label="Close"
          >
            <X className="size-4" />
          </button>
        </div>

        {/* Split Options */}
        <div className="space-y-2 pt-1">
          {selectedPages.size > 0 && (
            <label className="flex items-start gap-3 p-3 border rounded-xl cursor-pointer hover:bg-slate-50 transition-colors">
              <input
                type="radio"
                name="splitMode"
                value="selected"
                checked={splitMode === "selected"}
                onChange={() => setSplitMode("selected")}
                className="mt-0.5 accent-sky-600"
              />
              <div className="space-y-0.5">
                <p className="text-xs font-semibold text-slate-900">
                  Extract {selectedPages.size} Selected Page(s)
                </p>
                <p className="text-[11px] text-slate-500">
                  Exports only pages {Array.from(selectedPages).sort((a, b) => a - b).join(", ")} into a new PDF.
                </p>
              </div>
            </label>
          )}

          <label className="flex items-start gap-3 p-3 border rounded-xl cursor-pointer hover:bg-slate-50 transition-colors">
            <input
              type="radio"
              name="splitMode"
              value="ranges"
              checked={splitMode === "ranges"}
              onChange={() => setSplitMode("ranges")}
              className="mt-0.5 accent-sky-600"
            />
            <div className="space-y-1.5 flex-1">
              <p className="text-xs font-semibold text-slate-900">Split by Page Ranges</p>
              <p className="text-[11px] text-slate-500">
                Separate document by custom ranges (Total: {totalPageCount} pages).
              </p>
              {splitMode === "ranges" && (
                <input
                  type="text"
                  value={rangeInput}
                  onChange={(e) => setRangeInput(e.target.value)}
                  placeholder="e.g. 1-2, 3-4"
                  className="w-full px-3 py-1.5 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                  data-testid="split-range-input"
                />
              )}
            </div>
          </label>

          <label className="flex items-start gap-3 p-3 border rounded-xl cursor-pointer hover:bg-slate-50 transition-colors">
            <input
              type="radio"
              name="splitMode"
              value="single-pages"
              checked={splitMode === "single-pages"}
              onChange={() => setSplitMode("single-pages")}
              className="mt-0.5 accent-sky-600"
            />
            <div className="space-y-0.5">
              <p className="text-xs font-semibold text-slate-900">Split Every Page</p>
              <p className="text-[11px] text-slate-500">
                Extract every individual page into its own 1-page PDF.
              </p>
            </div>
          </label>
        </div>

        {/* Modal Actions */}
        <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-100">
          <Button type="button" variant="outline" size="sm" onClick={onClose} disabled={isProcessing}>
            Cancel
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={() => void handleConfirm()}
            disabled={isProcessing}
            className="bg-sky-600 hover:bg-sky-700 text-white"
            data-testid="confirm-split-btn"
          >
            {isProcessing ? "Processing…" : "Extract / Split"}
          </Button>
        </div>
      </div>
    </div>
  );
}
