"use client";

import { type PdfDocumentMetadata } from "@/lib/pdf/pdf-types";
import { formatBytes } from "@/lib/tools/format";
import { Button } from "@/components/ui/button";
import {
  Info,
  ShieldCheck,
  X,
} from "lucide-react";

interface PdfDocumentInfoProps {
  metadata: PdfDocumentMetadata;
  isOpen: boolean;
  onClose: () => void;
}

export function PdfDocumentInfo({ metadata, isOpen, onClose }: PdfDocumentInfoProps) {
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/50 backdrop-blur-xs p-4 animate-in fade-in duration-150"
      role="dialog"
      aria-modal="true"
      aria-labelledby="doc-info-title"
    >
      <div className="w-full max-w-md rounded-2xl bg-white shadow-xl border border-slate-200 overflow-hidden space-y-4 p-6">
        <div className="flex items-center justify-between border-b border-slate-100 pb-3">
          <h3 id="doc-info-title" className="text-base font-semibold text-slate-900 flex items-center gap-2">
            <Info className="size-4 text-sky-600" />
            Document Properties
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
            <span className="col-span-2 text-slate-900">{metadata.formFieldCount} field(s)</span>
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
