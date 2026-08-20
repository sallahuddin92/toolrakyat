"use client";

import { ArrowLeft, ArrowRight, Copy, FileInput, FileOutput, Loader2, Plus, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";

interface PdfPageOperationsProps {
  currentPage: number;
  pageCount: number;
  selectedCount: number;
  isProcessing: boolean;
  onDelete: () => void;
  onMoveLeft: () => void;
  onMoveRight: () => void;
  onDuplicate: () => void;
  onInsertBlank: () => void;
  onExtract: () => void;
  onMerge: () => void;
}

export function PdfPageOperations({
  currentPage,
  pageCount,
  selectedCount,
  isProcessing,
  onDelete,
  onMoveLeft,
  onMoveRight,
  onDuplicate,
  onInsertBlank,
  onExtract,
  onMerge,
}: PdfPageOperationsProps) {
  return (
    <div
      className="flex min-h-11 shrink-0 items-center gap-1.5 overflow-x-auto border-b border-slate-200 bg-white px-4 py-1.5"
      data-testid="pdf-page-operations"
      aria-label="Page operations"
    >
      <span className="mr-1 whitespace-nowrap text-xs font-medium text-slate-500">
        Page {currentPage}
      </span>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onMoveLeft}
        disabled={isProcessing || currentPage <= 1}
        data-testid="page-move-left"
      >
        <ArrowLeft className="size-3.5" />
        Move left
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onMoveRight}
        disabled={isProcessing || currentPage >= pageCount}
        data-testid="page-move-right"
      >
        <ArrowRight className="size-3.5" />
        Move right
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onDuplicate}
        disabled={isProcessing}
        data-testid="page-duplicate"
      >
        <Copy className="size-3.5" />
        Duplicate
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onInsertBlank}
        disabled={isProcessing}
        data-testid="page-insert-blank"
      >
        <Plus className="size-3.5" />
        Blank page
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onExtract}
        disabled={isProcessing || selectedCount === 0}
        data-testid="page-extract"
      >
        <FileOutput className="size-3.5" />
        Extract {selectedCount}
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onMerge}
        disabled={isProcessing}
        data-testid="page-merge"
      >
        <FileInput className="size-3.5" />
        Add PDF
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onDelete}
        disabled={isProcessing || pageCount <= 1}
        className="text-red-600 hover:text-red-700"
        data-testid="page-delete"
      >
        <Trash2 className="size-3.5" />
        Delete
      </Button>
      {isProcessing ? (
        <span className="ml-auto flex items-center gap-1.5 whitespace-nowrap text-xs text-sky-700" role="status">
          <Loader2 className="size-3.5 animate-spin" />
          Processing in StarPDF worker…
        </span>
      ) : null}
    </div>
  );
}
