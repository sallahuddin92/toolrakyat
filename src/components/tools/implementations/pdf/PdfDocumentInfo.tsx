"use client";

import { useState } from "react";
import { type PdfDocumentMetadata } from "@/lib/pdf/pdf-types";
import { formatBytes } from "@/lib/tools/format";
import { Button } from "@/components/ui/button";
import {
  Info,
  ShieldCheck,
  X,
  ChevronDown,
  ChevronRight,
  Code2,
} from "lucide-react";

interface PdfDocumentInfoProps {
  metadata: PdfDocumentMetadata;
  isOpen: boolean;
  onClose: () => void;
}

export function PdfDocumentInfo({ metadata, isOpen, onClose }: PdfDocumentInfoProps) {
  const [showDiagnostics, setShowDiagnostics] = useState(false);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/50 backdrop-blur-xs p-4 animate-in fade-in duration-150"
      role="dialog"
      aria-modal="true"
      aria-labelledby="doc-info-title"
      data-testid="doc-info-modal"
    >
      <div className="w-full max-w-md rounded-2xl bg-white shadow-xl border border-slate-200 overflow-hidden space-y-4 p-6">
        <div className="flex items-center justify-between border-b border-slate-100 pb-3">
          <h3 id="doc-info-title" className="text-base font-semibold text-slate-900 flex items-center gap-2">
            <Info className="size-4 text-sky-600" />
            Document Properties & Diagnostics
          </h3>
          <Button variant="ghost" size="icon" onClick={onClose} className="size-8 rounded-lg text-slate-500">
            <X className="size-4" />
          </Button>
        </div>

        <div className="space-y-3 text-xs">
          <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
            <span className="text-slate-500 font-medium">Filename</span>
            <span className="col-span-2 text-slate-900 font-mono truncate" title={metadata.filename}>
              {metadata.filename}
            </span>
          </div>

          <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
            <span className="text-slate-500 font-medium">File Size</span>
            <span className="col-span-2 text-slate-900">{formatBytes(metadata.fileSize)}</span>
          </div>

          <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
            <span className="text-slate-500 font-medium">Page Count</span>
            <span className="col-span-2 text-slate-900">{metadata.pageCount} page(s)</span>
          </div>

          <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
            <span className="text-slate-500 font-medium">Form Fields</span>
            <span className="col-span-2 text-slate-900">{metadata.formFieldCount} interactive field(s)</span>
          </div>

          {metadata.title && (
            <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
              <span className="text-slate-500 font-medium">Title</span>
              <span className="col-span-2 text-slate-900">{metadata.title}</span>
            </div>
          )}

          {metadata.author && (
            <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
              <span className="text-slate-500 font-medium">Author</span>
              <span className="col-span-2 text-slate-900">{metadata.author}</span>
            </div>
          )}

          {metadata.subject && (
            <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
              <span className="text-slate-500 font-medium">Subject</span>
              <span className="col-span-2 text-slate-900">{metadata.subject}</span>
            </div>
          )}

          {metadata.producer && (
            <div className="grid grid-cols-3 gap-2 py-1 border-b border-slate-50">
              <span className="text-slate-500 font-medium">Producer</span>
              <span className="col-span-2 text-slate-900 truncate" title={metadata.producer}>
                {metadata.producer}
              </span>
            </div>
          )}
        </div>

        {/* Optional Document Diagnostics Section */}
        <div className="border border-slate-200 rounded-xl overflow-hidden">
          <button
            type="button"
            onClick={() => setShowDiagnostics(!showDiagnostics)}
            className="w-full flex items-center justify-between p-2.5 bg-slate-50 hover:bg-slate-100 text-xs font-medium text-slate-700 transition-colors"
            data-testid="toggle-diagnostics-btn"
          >
            <span className="flex items-center gap-1.5">
              <Code2 className="size-3.5 text-slate-500" />
              Document Diagnostics
            </span>
            {showDiagnostics ? <ChevronDown className="size-3.5 text-slate-400" /> : <ChevronRight className="size-3.5 text-slate-400" />}
          </button>
          {showDiagnostics && (
            <div className="p-3 text-[11px] bg-slate-900 text-slate-200 font-mono space-y-1.5">
              <div>Page count: {metadata.pageCount}</div>
              <div>Form field count: {metadata.formFieldCount}</div>
              <div>File size: {formatBytes(metadata.fileSize)}</div>
              {metadata.creationDate && <div>Created: {metadata.creationDate.toISOString()}</div>}
              {metadata.modificationDate && <div>Modified: {metadata.modificationDate.toISOString()}</div>}
              <div className="text-slate-400 text-[10px] pt-1">StarPDF WASM Direct-Manipulation Engine v0.20</div>
            </div>
          )}
        </div>

        <div className="p-3 rounded-xl bg-slate-50 border border-slate-200/80 flex items-start gap-2.5 text-[11px] text-slate-600">
          <ShieldCheck className="size-4 text-emerald-600 shrink-0 mt-0.5" />
          <span>This file is loaded and processed entirely inside your web browser. No data was sent to any server.</span>
        </div>

        <div className="pt-2 flex justify-end">
          <Button type="button" onClick={onClose} size="sm" className="rounded-xl">
            Done
          </Button>
        </div>
      </div>
    </div>
  );
}
