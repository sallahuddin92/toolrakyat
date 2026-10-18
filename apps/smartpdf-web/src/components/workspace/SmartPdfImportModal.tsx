"use client";

import { useState, useRef } from "react";
import { StarPdfClient } from "@/lib/pdf/starpdf-client";
import { Button } from "@/components/ui/button";
import { AlertTriangle, FileInput, Lock, Upload, X } from "lucide-react";
import toast from "react-hot-toast";

interface SmartPdfImportModalProps {
  isOpen: boolean;
  currentPage: number;
  totalPageCount: number;
  onClose: () => void;
  onImportPages: (
    importedBytes: Uint8Array,
    selectedPageIndices: number[], // 0-indexed
    position: "start" | "end" | "before" | "after",
    sourceFilename: string,
  ) => Promise<void>;
}

export function SmartPdfImportModal({
  isOpen,
  currentPage,
  totalPageCount,
  onClose,
  onImportPages,
}: SmartPdfImportModalProps) {
  const [file, setFile] = useState<File | null>(null);
  const [fileBytes, setFileBytes] = useState<Uint8Array | null>(null);
  const [pageCount, setPageCount] = useState<number>(0);
  const [isEncrypted, setIsEncrypted] = useState<boolean>(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [isValidating, setIsValidating] = useState<boolean>(false);
  const [importMode, setImportMode] = useState<"all" | "custom">("all");
  const [pageRangeText, setPageRangeText] = useState<string>("");
  const [position, setPosition] = useState<"start" | "end" | "before" | "after">("after");
  const [isImporting, setIsImporting] = useState<boolean>(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  if (!isOpen) return null;

  async function handleFileSelect(selectedFile: File) {
    setFile(selectedFile);
    setValidationError(null);
    setIsEncrypted(false);
    setIsValidating(true);

    try {
      const buffer = await selectedFile.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      setFileBytes(bytes);

      // Validate locally via StarPDF client
      const handle = await StarPdfClient.open(bytes);
      const secInfo = await handle.getSecurityInfo();

      if (secInfo.encryption_state !== "NOT_ENCRYPTED") {
        setIsEncrypted(true);
        setValidationError("This document is password-protected or encrypted and cannot be imported.");
        await handle.close();
        return;
      }

      const pages = await handle.getPageCount();
      setPageCount(pages);
      await handle.close();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Failed to open PDF document.";
      setValidationError(msg);
    } finally {
      setIsValidating(false);
    }
  }

  function parsePageRange(text: string, maxPages: number): number[] {
    if (!text.trim()) return Array.from({ length: maxPages }, (_, i) => i);
    const indices: Set<number> = new Set();
    const parts = text.split(",");

    for (const part of parts) {
      const trimmed = part.trim();
      if (!trimmed) continue;
      if (trimmed.includes("-")) {
        const [startStr, endStr] = trimmed.split("-");
        const start = parseInt(startStr, 10);
        const end = parseInt(endStr, 10);
        if (!isNaN(start) && !isNaN(end)) {
          const min = Math.max(1, Math.min(start, end));
          const max = Math.min(maxPages, Math.max(start, end));
          for (let p = min; p <= max; p++) {
            indices.add(p - 1);
          }
        }
      } else {
        const p = parseInt(trimmed, 10);
        if (!isNaN(p) && p >= 1 && p <= maxPages) {
          indices.add(p - 1);
        }
      }
    }
    return Array.from(indices).sort((a, b) => a - b);
  }

  async function handleConfirm() {
    if (!fileBytes || !file || isEncrypted) return;

    const selectedIndices =
      importMode === "all"
        ? Array.from({ length: pageCount }, (_, i) => i)
        : parsePageRange(pageRangeText, pageCount);

    if (selectedIndices.length === 0) {
      toast.error("Please specify at least one valid page to import.");
      return;
    }

    setIsImporting(true);
    try {
      await onImportPages(fileBytes, selectedIndices, position, file.name);
      onClose();
    } catch (e: unknown) {
      toast.error(e instanceof Error ? e.message : "Failed to import pages.");
    } finally {
      setIsImporting(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs animate-in fade-in duration-100"
      role="dialog"
      aria-modal="true"
      aria-labelledby="import-modal-title"
      data-testid="smartpdf-import-modal"
    >
      <div className="w-full max-w-lg bg-white rounded-2xl border border-slate-200 shadow-2xl overflow-hidden p-6 space-y-4 animate-in zoom-in-95 duration-100">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2.5">
            <div className="size-9 rounded-lg bg-sky-50 text-sky-600 flex items-center justify-center">
              <FileInput className="size-5" />
            </div>
            <div>
              <h3 id="import-modal-title" className="text-base font-semibold text-slate-900">
                Import Pages / Merge PDF
              </h3>
              <p className="text-xs text-slate-500">
                Insert pages from another PDF into your current document.
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

        {/* File Drop / Select Area */}
        {!file ? (
          <div
            onClick={() => fileInputRef.current?.click()}
            className="border-2 border-dashed border-slate-300 hover:border-sky-500 hover:bg-sky-50/50 rounded-xl p-6 text-center cursor-pointer transition-colors space-y-2"
          >
            <Upload className="size-8 mx-auto text-slate-400" />
            <p className="text-sm font-medium text-slate-700">
              Click to select or drop a PDF document
            </p>
            <p className="text-xs text-slate-400">Processed locally in your browser</p>
          </div>
        ) : (
          <div className="border border-slate-200 rounded-xl p-3.5 bg-slate-50 flex items-center justify-between">
            <div className="space-y-0.5 min-w-0">
              <p className="text-sm font-semibold text-slate-900 truncate">{file.name}</p>
              <p className="text-xs text-slate-500">
                {isValidating
                  ? "Inspecting document…"
                  : isEncrypted
                    ? "Encrypted PDF"
                    : `${pageCount} page(s) • ${(file.size / 1024).toFixed(1)} KB`}
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                setFile(null);
                setFileBytes(null);
                setPageCount(0);
                setIsEncrypted(false);
                setValidationError(null);
              }}
            >
              Change
            </Button>
          </div>
        )}

        <input
          ref={fileInputRef}
          type="file"
          accept="application/pdf"
          className="hidden"
          onChange={(e) => {
            if (e.target.files && e.target.files.length > 0) {
              void handleFileSelect(e.target.files[0]);
            }
          }}
        />

        {/* Encrypted / Error Notice */}
        {isEncrypted && (
          <div className="bg-red-50 border border-red-200 rounded-xl p-3 flex items-start gap-2.5 text-xs text-red-700" data-testid="import-encrypted-refusal">
            <Lock className="size-4 shrink-0 text-red-600 mt-0.5" />
            <p>{validationError || "Password-protected PDFs cannot be imported."}</p>
          </div>
        )}

        {validationError && !isEncrypted && (
          <div className="bg-red-50 border border-red-200 rounded-xl p-3 flex items-start gap-2.5 text-xs text-red-700">
            <AlertTriangle className="size-4 shrink-0 text-red-600 mt-0.5" />
            <p>{validationError}</p>
          </div>
        )}

        {/* Import Configuration Options */}
        {file && !isEncrypted && !validationError && (
          <div className="space-y-3 pt-1">
            {/* Pages Selection */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Pages to Import</label>
              <div className="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={() => setImportMode("all")}
                  className={`px-3 py-2 text-xs font-medium rounded-lg border text-left transition-colors cursor-pointer ${
                    importMode === "all"
                      ? "border-sky-500 bg-sky-50 text-sky-700 font-semibold"
                      : "border-slate-200 hover:bg-slate-50 text-slate-600"
                  }`}
                >
                  All pages (1 – {pageCount})
                </button>
                <button
                  type="button"
                  onClick={() => setImportMode("custom")}
                  className={`px-3 py-2 text-xs font-medium rounded-lg border text-left transition-colors cursor-pointer ${
                    importMode === "custom"
                      ? "border-sky-500 bg-sky-50 text-sky-700 font-semibold"
                      : "border-slate-200 hover:bg-slate-50 text-slate-600"
                  }`}
                >
                  Custom selection
                </button>
              </div>

              {importMode === "custom" && (
                <input
                  type="text"
                  placeholder="e.g. 1, 3-5, 8"
                  value={pageRangeText}
                  onChange={(e) => setPageRangeText(e.target.value)}
                  className="w-full mt-1.5 px-3 py-1.5 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                  data-testid="import-page-range-input"
                />
              )}
            </div>

            {/* Destination Position */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Insert Location</label>
              <select
                value={position}
                onChange={(e) => setPosition(e.target.value as "start" | "end" | "before" | "after")}
                className="w-full px-3 py-1.5 text-xs border border-slate-300 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="import-position-select"
              >
                <option value="after">After active page (Page {currentPage})</option>
                <option value="before">Before active page (Page {currentPage})</option>
                <option value="start">At the beginning (Before Page 1)</option>
                <option value="end">At the end (After Page {totalPageCount})</option>
              </select>
            </div>
          </div>
        )}

        {/* Modal Actions */}
        <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-100">
          <Button type="button" variant="outline" size="sm" onClick={onClose} disabled={isImporting}>
            Cancel
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={() => void handleConfirm()}
            disabled={!fileBytes || isEncrypted || isValidating || isImporting}
            className="bg-sky-600 hover:bg-sky-700 text-white"
            data-testid="confirm-import-btn"
          >
            {isImporting ? "Importing…" : "Import Pages"}
          </Button>
        </div>
      </div>
    </div>
  );
}
