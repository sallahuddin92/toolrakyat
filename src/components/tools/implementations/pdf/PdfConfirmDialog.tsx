"use client";

import { Button } from "@/components/ui/button";
import { AlertTriangle, X } from "lucide-react";

interface PdfConfirmDialogProps {
  isOpen: boolean;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  isDestructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function PdfConfirmDialog({
  isOpen,
  title,
  description,
  confirmLabel = "Continue",
  cancelLabel = "Cancel",
  isDestructive = false,
  onConfirm,
  onCancel,
}: PdfConfirmDialogProps) {
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs animate-in fade-in duration-100"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
      data-testid="pdf-confirm-dialog"
    >
      <div
        className="w-full max-w-md bg-white rounded-2xl border border-slate-200 shadow-2xl overflow-hidden p-6 space-y-4 animate-in zoom-in-95 duration-100"
        onKeyDown={(e) => {
          if (e.key === "Escape") onCancel();
          if (e.key === "Enter") onConfirm();
        }}
      >
        <div className="flex items-start gap-3.5">
          <div
            className={`size-10 rounded-xl flex items-center justify-center shrink-0 ${
              isDestructive ? "bg-red-50 text-red-600" : "bg-amber-50 text-amber-600"
            }`}
          >
            <AlertTriangle className="size-5" />
          </div>
          <div className="space-y-1 min-w-0 flex-1">
            <h3 id="confirm-dialog-title" className="text-base font-semibold text-slate-900">
              {title}
            </h3>
            <p className="text-xs text-slate-500 leading-relaxed">{description}</p>
          </div>
          <button
            type="button"
            onClick={onCancel}
            className="text-slate-400 hover:text-slate-600 rounded-lg p-1"
            aria-label="Close"
          >
            <X className="size-4" />
          </button>
        </div>

        <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-100">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onCancel}
            className="text-xs h-9 px-4 rounded-xl"
            data-testid="confirm-dialog-cancel-btn"
          >
            {cancelLabel}
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={onConfirm}
            className={`text-xs h-9 px-4 rounded-xl text-white ${
              isDestructive
                ? "bg-red-600 hover:bg-red-700"
                : "bg-sky-600 hover:bg-sky-700"
            }`}
            data-testid="confirm-dialog-confirm-btn"
            autoFocus
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
