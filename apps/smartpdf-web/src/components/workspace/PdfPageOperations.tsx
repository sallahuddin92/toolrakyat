"use client";

import {
  ArrowLeft,
  ArrowRight,
  Copy,
  FileInput,
  FileOutput,
  FolderInput,
  Loader2,
  Plus,
  Scissors,
  Trash2,
} from "lucide-react";

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
  onMerge?: () => void;
  onMergeFiles?: (files: Uint8Array[]) => void;
  onImport?: () => void;
  onSplit?: () => void;
  mutationEnabled?: boolean;
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
  onMergeFiles,
  onImport,
  onSplit,
  mutationEnabled = true,
}: PdfPageOperationsProps) {
  const isMulti = selectedCount > 1;

  return (
    <div
      className="flex min-h-11 shrink-0 items-center gap-1.5 overflow-x-auto border-b border-slate-200 bg-white px-4 py-1.5"
      data-testid="pdf-page-operations"
      aria-label="Page operations"
    >
      <span className="mr-1 whitespace-nowrap text-xs font-medium text-slate-500">
        Page {currentPage} / {pageCount} {selectedCount > 0 ? `(${selectedCount} sel)` : ""}
      </span>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onMoveLeft}
        disabled={!mutationEnabled || isProcessing || currentPage <= 1}
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
        disabled={!mutationEnabled || isProcessing || currentPage >= pageCount}
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
        disabled={!mutationEnabled || isProcessing}
        data-testid="page-duplicate"
      >
        <Copy className="size-3.5" />
        {isMulti ? `Duplicate (${selectedCount})` : "Duplicate"}
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onInsertBlank}
        disabled={!mutationEnabled || isProcessing}
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
        disabled={!mutationEnabled || isProcessing || pageCount <= 0}
        data-testid="page-extract"
      >
        <FileOutput className="size-3.5" />
        Extract {selectedCount > 1 ? `(${selectedCount})` : ""}
      </Button>
      {onMergeFiles ? (
        <label className="inline-flex cursor-pointer items-center">
          <input
            type="file"
            accept="application/pdf"
            multiple
            className="sr-only"
            disabled={!mutationEnabled || isProcessing}
            onChange={async (e) => {
              const files = e.target.files;
              if (files && files.length > 0) {
                const additions = await Promise.all(
                  Array.from(files).map(async (file) => new Uint8Array(await file.arrayBuffer())),
                );
                onMergeFiles(additions);
              }
              e.target.value = "";
            }}
          />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={!mutationEnabled || isProcessing}
            data-testid="page-merge"
            asChild
          >
            <span>
              <FileInput className="size-3.5" />
              Add PDF
            </span>
          </Button>
        </label>
      ) : (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onMerge}
          disabled={!mutationEnabled || isProcessing}
          data-testid="page-merge"
        >
          <FileInput className="size-3.5" />
          Add PDF
        </Button>
      )}
      {onImport && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onImport}
          disabled={!mutationEnabled || isProcessing}
          data-testid="page-import"
        >
          <FolderInput className="size-3.5" />
          Import pages
        </Button>
      )}
      {onSplit && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onSplit}
          disabled={!mutationEnabled || isProcessing}
          data-testid="page-split"
        >
          <Scissors className="size-3.5" />
          Split PDF
        </Button>
      )}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onDelete}
        disabled={!mutationEnabled || isProcessing || pageCount <= 1 || (isMulti && selectedCount >= pageCount)}
        className="text-red-600 hover:text-red-700"
        data-testid="page-delete"
      >
        <Trash2 className="size-3.5" />
        {isMulti ? `Delete (${selectedCount})` : "Delete"}
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
