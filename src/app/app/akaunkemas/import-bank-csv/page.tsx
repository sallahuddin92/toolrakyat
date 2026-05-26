"use client";

import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Upload, FileText, CheckCircle, AlertCircle, ArrowLeft } from "lucide-react";
import Link from "next/link";
import { importTransactionsCsv } from "../transactions/actions";
import { parseBankCsv } from "@/lib/akaunkemas/csv-parser";
import type { DetectedColumns } from "@/lib/akaunkemas/types";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PreviewRow {
  date: string;
  description: string;
  debit: number;
  credit: number;
  amount: number;
  balance: number | null;
}

interface ImportResult {
  success: boolean;
  imported?: number;
  skipped?: number;
  errors?: string[];
  error?: string;
}

// ---------------------------------------------------------------------------
// CSV Import Page
// ---------------------------------------------------------------------------

export default function ImportBankCsvPage() {
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<PreviewRow[]>([]);
  const [detectedColumns, setDetectedColumns] = useState<DetectedColumns | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);

  // Handle file selection and parse preview
  const handleFile = useCallback(async (selectedFile: File) => {
    setFile(selectedFile);
    setParseError(null);
    setResult(null);

    try {
      const text = await selectedFile.text();
      const parseResult = parseBankCsv(text);

      if (parseResult.transactions.length === 0) {
        setParseError(
          parseResult.errors.length > 0
            ? parseResult.errors.join("; ")
            : "No transactions found in this CSV file.",
        );
        setPreview([]);
        setDetectedColumns(null);
        return;
      }

      setDetectedColumns(parseResult.detectedColumns);
      setPreview(
        parseResult.transactions.slice(0, 10).map((tx) => ({
          date: tx.date,
          description: tx.description,
          debit: tx.debit,
          credit: tx.credit,
          amount: tx.amount,
          balance: tx.balance,
        })),
      );

      if (parseResult.errors.length > 0) {
        setParseError(parseResult.errors.join("; "));
      }
    } catch (e) {
      setParseError(e instanceof Error ? e.message : "Failed to parse CSV");
    }
  }, []);

  // Handle import
  const handleImport = useCallback(async () => {
    if (!file) return;
    setImporting(true);

    const formData = new FormData();
    formData.set("file", file);
    const res = await importTransactionsCsv(formData);
    setResult(res);
    setImporting(false);
  }, [file]);

  // Handle drag & drop
  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const droppedFile = e.dataTransfer.files[0];
      if (droppedFile) handleFile(droppedFile);
    },
    [handleFile],
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const selected = e.target.files?.[0];
      if (selected) handleFile(selected);
    },
    [handleFile],
  );

  function formatCurrency(n: number): string {
    return new Intl.NumberFormat("en-MY", {
      style: "currency",
      currency: "MYR",
      minimumFractionDigits: 2,
    }).format(n);
  }

  return (
    <div className="space-y-6 max-w-4xl">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Link href="/app/akaunkemas/transactions">
          <Button variant="ghost" size="icon">
            <ArrowLeft className="size-4" />
          </Button>
        </Link>
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Import Bank CSV</h1>
          <p className="text-sm text-slate-500">
            Upload a bank statement CSV to import transactions.
          </p>
        </div>
      </div>

      {/* Upload zone */}
      {!file && (
        <Card className="rounded-2xl border-dashed">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-sky-50">
              <Upload className="size-8 text-sky-500" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">
                Drop your bank CSV file here
              </p>
              <p className="text-xs text-slate-500 mt-1">
                Supports Maybank, CIMB, Public Bank, and other Malaysian banks
              </p>
            </div>
            <label
              className="cursor-pointer"
              onDragOver={(e) => e.preventDefault()}
              onDrop={handleDrop}
            >
              <input
                type="file"
                accept=".csv"
                className="hidden"
                onChange={handleChange}
              />
              <Button variant="outline" asChild>
                <span>Browse Files</span>
              </Button>
            </label>
          </CardContent>
        </Card>
      )}

      {/* Preview */}
      {file && !result && (
        <>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <FileText className="size-5 text-slate-400" />
              <div>
                <p className="text-sm font-medium text-slate-900">{file.name}</p>
                <p className="text-xs text-slate-500">
                  {(file.size / 1024).toFixed(1)} KB &middot; {preview.length} row
                  {preview.length !== 1 ? "s" : ""} previewed
                </p>
              </div>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setFile(null);
                setPreview([]);
                setParseError(null);
              }}
            >
              Change file
            </Button>
          </div>

          {parseError && (
            <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
              <AlertCircle className="size-4 mt-0.5 shrink-0" />
              {parseError}
            </div>
          )}

          {detectedColumns && (
            <div className="flex flex-wrap gap-2 text-xs text-slate-500">
              <span>Detected columns:</span>
              {detectedColumns.dateCol && (
                <Badge variant="secondary" className="text-[10px]">
                  Date: {detectedColumns.dateCol}
                </Badge>
              )}
              {detectedColumns.descCol && (
                <Badge variant="secondary" className="text-[10px]">
                  Desc: {detectedColumns.descCol}
                </Badge>
              )}
              {detectedColumns.debitCol && (
                <Badge variant="secondary" className="text-[10px]">
                  Debit: {detectedColumns.debitCol}
                </Badge>
              )}
              {detectedColumns.creditCol && (
                <Badge variant="secondary" className="text-[10px]">
                  Credit: {detectedColumns.creditCol}
                </Badge>
              )}
              {detectedColumns.balanceCol && (
                <Badge variant="secondary" className="text-[10px]">
                  Balance: {detectedColumns.balanceCol}
                </Badge>
              )}
            </div>
          )}

          {/* Preview table */}
          {preview.length > 0 && (
            <Card className="rounded-2xl overflow-hidden">
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Preview (first {Math.min(preview.length, 10)} rows)</CardTitle>
              </CardHeader>
              <CardContent className="p-0">
                <div className="grid grid-cols-12 gap-2 border-b border-slate-200 bg-slate-50 px-4 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                  <div className="col-span-2">Date</div>
                  <div className="col-span-4">Description</div>
                  <div className="col-span-1 text-right">Debit</div>
                  <div className="col-span-1 text-right">Credit</div>
                  <div className="col-span-2 text-right">Amount</div>
                  <div className="col-span-2 text-right">Balance</div>
                </div>
                <div className="divide-y divide-slate-50">
                  {preview.map((row, i) => (
                    <div
                      key={i}
                      className="grid grid-cols-12 gap-2 px-4 py-2 text-xs"
                    >
                      <div className="col-span-2 text-slate-500">{row.date}</div>
                      <div className="col-span-4 truncate text-slate-900">
                        {row.description}
                      </div>
                      <div className="col-span-1 text-right tabular-nums text-red-600">
                        {row.debit > 0 ? formatCurrency(row.debit) : ""}
                      </div>
                      <div className="col-span-1 text-right tabular-nums text-green-600">
                        {row.credit > 0 ? formatCurrency(row.credit) : ""}
                      </div>
                      <div
                        className={cn(
                          "col-span-2 text-right tabular-nums",
                          row.amount < 0 ? "text-red-600" : "text-green-700",
                        )}
                      >
                        {formatCurrency(row.amount)}
                      </div>
                      <div className="col-span-2 text-right tabular-nums text-slate-500">
                        {row.balance != null ? formatCurrency(row.balance) : ""}
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          <div className="flex justify-end gap-3">
            <Button
              variant="outline"
              onClick={() => {
                setFile(null);
                setPreview([]);
                setParseError(null);
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleImport}
              disabled={importing || preview.length === 0}
              className="gap-2"
            >
              {importing ? (
                <>Importing...</>
              ) : (
                <>
                  <Upload className="size-4" />
                  Import {preview.length > 10 ? `${preview.length}+` : preview.length} Transactions
                </>
              )}
            </Button>
          </div>
        </>
      )}

      {/* Result */}
      {result && (
        <Card className="rounded-2xl">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            {result.success ? (
              <>
                <div className="flex size-16 items-center justify-center rounded-2xl bg-green-50">
                  <CheckCircle className="size-8 text-green-500" />
                </div>
                <div className="text-center">
                  <p className="text-sm font-medium text-slate-900">Import complete</p>
                  <p className="text-xs text-slate-500 mt-1">
                    {result.imported} imported, {result.skipped} skipped (duplicates)
                  </p>
                </div>
                <div className="flex gap-3">
                  <Link href="/app/akaunkemas/transactions">
                    <Button>View Transactions</Button>
                  </Link>
                  <Button
                    variant="outline"
                    onClick={() => {
                      setFile(null);
                      setPreview([]);
                      setResult(null);
                      setParseError(null);
                    }}
                  >
                    Import Another File
                  </Button>
                </div>
              </>
            ) : (
              <>
                <div className="flex size-16 items-center justify-center rounded-2xl bg-red-50">
                  <AlertCircle className="size-8 text-red-500" />
                </div>
                <div className="text-center">
                  <p className="text-sm font-medium text-slate-900">Import failed</p>
                  <p className="text-xs text-red-600 mt-1">{result.error}</p>
                </div>
                <Button
                  variant="outline"
                  onClick={() => {
                    setResult(null);
                  }}
                >
                  Try Again
                </Button>
              </>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
