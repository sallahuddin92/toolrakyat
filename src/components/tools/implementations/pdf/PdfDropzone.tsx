"use client";

import { useState, useCallback } from "react";
import { useDropzone } from "react-dropzone";
import { FileUp, FileText, AlertCircle, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { validateUploadedFile } from "@/lib/tools/file-validation";

interface PdfDropzoneProps {
  onFileSelect: (bytes: Uint8Array, filename: string, fileSize: number) => Promise<void>;
  isLoading?: boolean;
  loadingMessage?: string;
  error?: string | null;
}

export function PdfDropzone({
  onFileSelect,
  isLoading = false,
  loadingMessage = "Analyzing PDF document...",
  error = null,
}: PdfDropzoneProps) {
  const [localError, setLocalError] = useState<string | null>(null);

  const handleFile = useCallback(
    async (file: File) => {
      setLocalError(null);

      // Validate file size and type
      const validation = await validateUploadedFile({
        file,
        allowedTypes: ["application/pdf"],
        maxSizeMB: 50,
      });

      if (!validation.ok) {
        setLocalError(validation.error || "Please select a valid PDF file under 50MB.");
        return;
      }

      try {
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);

        // Check PDF header magic bytes (%PDF-)
        const header = new TextDecoder().decode(bytes.slice(0, 5));
        if (!header.startsWith("%PDF-")) {
          setLocalError("The selected file does not appear to be a valid PDF document.");
          return;
        }

        await onFileSelect(bytes, file.name, file.size);
      } catch (err: unknown) {
        setLocalError(
          err instanceof Error
            ? err.message
            : "Failed to read the selected PDF file.",
        );
      }
    },
    [onFileSelect],
  );

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop: (acceptedFiles) => {
      if (acceptedFiles.length > 0 && acceptedFiles[0]) {
        void handleFile(acceptedFiles[0]);
      }
    },
    accept: {
      "application/pdf": [".pdf"],
    },
    maxFiles: 1,
    multiple: false,
    disabled: isLoading,
  });

  const displayError = error || localError;

  return (
    <div className="w-full max-w-2xl mx-auto space-y-4">
      <Card
        {...getRootProps()}
        className={`border-2 border-dashed rounded-3xl transition-all cursor-pointer text-center p-8 sm:p-12 ${
          isDragActive
            ? "border-sky-500 bg-sky-50/50"
            : "border-slate-300 hover:border-slate-400 bg-slate-50/50 hover:bg-slate-50"
        } ${isLoading ? "pointer-events-none opacity-80" : ""}`}
      >
        <CardContent className="flex flex-col items-center justify-center p-0 space-y-4">
          <input {...getInputProps()} />

          <div className="size-16 sm:size-20 rounded-2xl bg-white shadow-xs border border-slate-200 flex items-center justify-center text-sky-600">
            {isLoading ? (
              <Loader2 className="size-8 sm:size-10 animate-spin text-sky-600" />
            ) : (
              <FileUp className="size-8 sm:size-10 text-sky-600" />
            )}
          </div>

          <div className="space-y-1">
            <h3 className="text-lg sm:text-xl font-semibold text-slate-900">
              {isLoading
                ? loadingMessage
                : isDragActive
                  ? "Drop your PDF here"
                  : "Upload a PDF Document"}
            </h3>
            <p className="text-sm text-slate-500 max-w-sm">
              {isLoading
                ? "Parsing page structures and detecting form fields..."
                : "Drag & drop a PDF file here, or click to browse from your device."}
            </p>
          </div>

          {!isLoading && (
            <div className="pt-2 flex flex-col sm:flex-row items-center gap-3">
              <Button type="button" className="rounded-xl px-6">
                <FileText className="size-4 mr-2" />
                Select PDF File
              </Button>
            </div>
          )}

          <div className="pt-2 flex items-center gap-2 text-xs text-slate-400">
            <span>Max size: 50MB</span>
            <span>•</span>
            <span>All processing is 100% private in your browser</span>
          </div>
        </CardContent>
      </Card>

      {displayError && (
        <div
          className="flex items-start gap-3 p-4 rounded-2xl bg-red-50 border border-red-200 text-red-700 text-sm"
          data-testid="starpdf-document-error"
        >
          <AlertCircle className="size-5 shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="font-medium">Unable to open document</p>
            <p className="text-xs text-red-600 mt-0.5">{displayError}</p>
          </div>
        </div>
      )}
    </div>
  );
}
