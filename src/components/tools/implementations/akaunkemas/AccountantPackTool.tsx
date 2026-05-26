"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { Download, FileSpreadsheet, FileText, Loader2, Package } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import { ToolError } from "@/components/tools/ToolError";
import type { Transaction, Receipt, AccountantPackInput } from "@/lib/akaunkemas/types";
import { parseBankCsv } from "@/lib/akaunkemas/csv-parser";
import { computeMonthlySummary } from "@/lib/akaunkemas/summary";
import type { ToolDefinition } from "@/lib/tools/types";

const MYR = new Intl.NumberFormat("ms-MY", { style: "currency", currency: "MYR" });

function formatCurrency(n: number): string {
  return MYR.format(Number.isFinite(n) ? n : 0);
}

function downloadBlob(content: Uint8Array | string | Blob, filename: string, mime: string) {
  const blob =
    content instanceof Blob
      ? content
      : typeof content === "string"
        ? new Blob([content], { type: mime })
        : new Blob([content as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 250);
}

/** Validate that a File has a .csv extension (case-insensitive). */
function isValidCsvFile(f: File): boolean {
  return f.name.toLowerCase().endsWith(".csv");
}

/** Validate that a File has a .json extension (case-insensitive). */
function isValidJsonFile(f: File): boolean {
  return f.name.toLowerCase().endsWith(".json");
}

export function AccountantPackTool(_props: { tool: ToolDefinition }) {
  // Transaction (bank CSV) state
  const csvFileInputRef = useRef<HTMLInputElement>(null);
  const [csvFile, setCsvFile] = useState<File | null>(null);
  const [csvParsing, setCsvParsing] = useState(false);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [csvDragOver, setCsvDragOver] = useState(false);

  // Receipt (JSON) state
  const receiptFileInputRef = useRef<HTMLInputElement>(null);
  const [receiptFile, setReceiptFile] = useState<File | null>(null);
  const [receiptParsing, setReceiptParsing] = useState(false);
  const [receipts, setReceipts] = useState<Receipt[]>([]);
  const [receiptDragOver, setReceiptDragOver] = useState(false);

  // Notes state
  const [notes, setNotes] = useState("");

  // Generation state
  const [generating, setGenerating] = useState(false);
  const [zipBlob, setZipBlob] = useState<Blob | null>(null);
  const [error, setError] = useState<string | null>(null);

  const summary = useMemo(() => computeMonthlySummary(transactions), [transactions]);

  /** Parse uploaded bank CSV file */
  const handleCsvFile = useCallback(
    async (f: File) => {
      if (!isValidCsvFile(f)) {
        setError("Invalid file type. Please upload a .csv file for transactions.");
        return;
      }

      if (transactions.length > 0 && !window.confirm(
        "Uploading a new CSV will replace all current transactions. Continue?"
      )) {
        return;
      }

      setCsvFile(f);
      setError(null);
      setCsvParsing(true);

      try {
        const csvText = await f.text();
        const parsed = parseBankCsv(csvText);
        if (parsed.transactions.length === 0 && parsed.errors.length > 0) {
          setError(parsed.errors.join("\n"));
        }
        setTransactions(parsed.transactions);
        // Clear previously generated ZIP when data changes
        setZipBlob(null);
      } catch (e) {
        const msg = e instanceof Error ? e.message : "Failed to parse CSV file.";
        setError(msg);
      } finally {
        setCsvParsing(false);
      }
    },
    [transactions.length],
  );

  /** Parse uploaded receipt JSON file */
  const handleReceiptFile = useCallback(
    async (f: File) => {
      if (!isValidJsonFile(f)) {
        setError("Invalid file type. Please upload a .json file for receipts.");
        return;
      }

      if (receipts.length > 0 && !window.confirm(
        "Uploading a new receipt file will replace all current receipts. Continue?"
      )) {
        return;
      }

      setReceiptFile(f);
      setError(null);
      setReceiptParsing(true);

      try {
        const jsonText = await f.text();
        const data = JSON.parse(jsonText);

        // Support both { receipts: [...] } and direct array [...]
        const parsedReceipts: Receipt[] = Array.isArray(data)
          ? data
          : Array.isArray(data.receipts)
            ? data.receipts
            : [];

        setReceipts(parsedReceipts);
        // Clear previously generated ZIP when data changes
        setZipBlob(null);
      } catch (e) {
        const msg = e instanceof Error ? e.message : "Failed to parse receipt JSON file.";
        setError(msg);
      } finally {
        setReceiptParsing(false);
      }
    },
    [receipts.length],
  );

  /** Validate and handle CSV file from drop or input */
  const validateCsvFile = useCallback(
    (f: File | undefined) => {
      if (!f) return;
      if (!isValidCsvFile(f)) {
        setError("Invalid file type. Please upload a .csv file for transactions.");
        return;
      }
      handleCsvFile(f);
    },
    [handleCsvFile],
  );

  /** Validate and handle receipt JSON file from drop or input */
  const validateReceiptFile = useCallback(
    (f: File | undefined) => {
      if (!f) return;
      if (!isValidJsonFile(f)) {
        setError("Invalid file type. Please upload a .json file for receipts.");
        return;
      }
      handleReceiptFile(f);
    },
    [handleReceiptFile],
  );

  /** Generate the accountant pack ZIP */
  const handleGenerate = useCallback(async () => {
    setError(null);
    setGenerating(true);
    setZipBlob(null);

    try {
      const { generateAccountantPackZip } = await import("@/lib/akaunkemas/accountant-pack");

      const input: AccountantPackInput = {
        transactions,
        receipts,
        notes: notes.trim() || undefined,
      };

      const blob = await generateAccountantPackZip(input);
      setZipBlob(blob);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to generate accountant pack.";
      setError(msg);
    } finally {
      setGenerating(false);
    }
  }, [transactions, receipts, notes]);

  /** Download the generated ZIP */
  const handleDownloadZip = useCallback(() => {
    if (!zipBlob) return;
    downloadBlob(zipBlob, "akaunkemas-accountant-pack.zip", "application/zip");
  }, [zipBlob]);

  const canGenerate = transactions.length > 0;

  return (
    <div className="space-y-6">
      {error ? (
        <ToolError
          message={error}
          onRetry={() => {
            setError(null);
            if (csvFile) handleCsvFile(csvFile);
          }}
        />
      ) : null}

      {/* Bank CSV Upload */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">1. Upload Bank CSV (Transactions)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div
            className={`rounded-2xl border border-dashed bg-white p-6 text-center transition-colors ${
              csvDragOver ? "border-sky-400 bg-sky-50/50" : "border-slate-200"
            }`}
            onDragOver={(e) => {
              e.preventDefault();
              setCsvDragOver(true);
            }}
            onDragLeave={() => setCsvDragOver(false)}
            onDrop={(e) => {
              e.preventDefault();
              setCsvDragOver(false);
              const f = e.dataTransfer?.files?.[0];
              validateCsvFile(f);
            }}
          >
            <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-900 ring-1 ring-slate-200">
              <FileSpreadsheet className="size-5" aria-hidden="true" />
            </div>
            <div className="mt-3 text-sm font-medium text-slate-900">
              {csvFile ? csvFile.name : "Drop your bank CSV here, or click to browse"}
            </div>
            <div className="mt-1 text-xs text-slate-500">
              Required: .csv &bull; Parsed client-side in your browser
            </div>
            <div className="mt-4 flex justify-center">
              <input
                ref={csvFileInputRef}
                type="file"
                accept=".csv,text/csv"
                className="sr-only"
                aria-label="Select a bank CSV file"
                onChange={(e) => {
                  const f = e.target?.files?.[0];
                  validateCsvFile(f);
                }}
              />
              <Button
                type="button"
                className="rounded-2xl"
                onClick={() => {
                  if (csvFileInputRef.current) csvFileInputRef.current.value = "";
                  csvFileInputRef.current?.click();
                }}
              >
                Choose CSV file
              </Button>
            </div>
          </div>
          {csvParsing ? (
            <div className="flex items-center gap-2 text-sm text-slate-600">
              <Loader2 className="size-4 animate-spin" aria-hidden="true" />
              Parsing CSV...
            </div>
          ) : null}
          {transactions.length > 0 && !csvParsing ? (
            <div className="text-xs text-slate-500">
              Loaded {transactions.length} transaction(s).
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Receipt JSON Upload */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">2. Upload Receipt JSON (Optional)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div
            className={`rounded-2xl border border-dashed bg-white p-6 text-center transition-colors ${
              receiptDragOver ? "border-sky-400 bg-sky-50/50" : "border-slate-200"
            }`}
            onDragOver={(e) => {
              e.preventDefault();
              setReceiptDragOver(true);
            }}
            onDragLeave={() => setReceiptDragOver(false)}
            onDrop={(e) => {
              e.preventDefault();
              setReceiptDragOver(false);
              const f = e.dataTransfer?.files?.[0];
              validateReceiptFile(f);
            }}
          >
            <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-900 ring-1 ring-slate-200">
              <FileText className="size-5" aria-hidden="true" />
            </div>
            <div className="mt-3 text-sm font-medium text-slate-900">
              {receiptFile ? receiptFile.name : "Drop your receipt JSON here, or click to browse"}
            </div>
            <div className="mt-1 text-xs text-slate-500">
              Optional: .json with {"{ receipts: [...] }"} structure
            </div>
            <div className="mt-4 flex justify-center">
              <input
                ref={receiptFileInputRef}
                type="file"
                accept=".json,application/json"
                className="sr-only"
                aria-label="Select a receipt JSON file"
                onChange={(e) => {
                  const f = e.target?.files?.[0];
                  validateReceiptFile(f);
                }}
              />
              <Button
                type="button"
                variant="secondary"
                className="rounded-2xl"
                onClick={() => {
                  if (receiptFileInputRef.current) receiptFileInputRef.current.value = "";
                  receiptFileInputRef.current?.click();
                }}
              >
                Choose JSON file
              </Button>
            </div>
          </div>
          {receiptParsing ? (
            <div className="flex items-center gap-2 text-sm text-slate-600">
              <Loader2 className="size-4 animate-spin" aria-hidden="true" />
              Parsing receipt JSON...
            </div>
          ) : null}
          {receipts.length > 0 && !receiptParsing ? (
            <div className="text-xs text-slate-500">
              Loaded {receipts.length} receipt(s).
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Notes */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">3. Notes for Accountant (Optional)</CardTitle>
        </CardHeader>
        <CardContent>
          <Label htmlFor="accountant-notes" className="text-xs text-slate-500">
            Any additional notes or instructions for your accountant.
          </Label>
          <Textarea
            id="accountant-notes"
            className="mt-2 rounded-xl"
            rows={4}
            placeholder="E.g., Please review category assignments for Q1 transactions. The rent payment in January was for two months."
            value={notes}
            onChange={(e) => {
              setNotes(e.target.value);
              setZipBlob(null);
            }}
          />
        </CardContent>
      </Card>

      {/* Summary + Generate */}
      {transactions.length > 0 ? (
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle className="text-base">Summary</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
              <div>
                <div className="text-xs text-slate-500">Total Income</div>
                <div className="text-lg font-semibold text-green-700 tabular-nums">
                  {formatCurrency(summary.totalIncome)}
                </div>
              </div>
              <div>
                <div className="text-xs text-slate-500">Total Expenses</div>
                <div className="text-lg font-semibold text-red-600 tabular-nums">
                  {formatCurrency(summary.totalExpense)}
                </div>
              </div>
              <div>
                <div className="text-xs text-slate-500">Net Cashflow</div>
                <div
                  className={`text-lg font-semibold tabular-nums ${
                    summary.netCashflow >= 0 ? "text-green-700" : "text-red-600"
                  }`}
                >
                  {formatCurrency(summary.netCashflow)}
                </div>
              </div>
              <div>
                <div className="text-xs text-slate-500">Transactions</div>
                <div className="text-lg font-semibold tabular-nums">
                  {summary.transactionCount}
                </div>
              </div>
            </div>

            <Separator />

            <div className="space-y-2">
              <div className="text-xs font-medium text-slate-500">Pack Contents Preview</div>
              <div className="grid gap-1 text-sm">
                <div className="flex items-center justify-between">
                  <span className="text-slate-600">Transactions</span>
                  <span className="tabular-nums font-medium">{transactions.length}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-slate-600">Receipts</span>
                  <span className="tabular-nums font-medium">{receipts.length}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-slate-600">Notes</span>
                  <span className="tabular-nums font-medium">
                    {notes.trim() ? "Included" : "Not included"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-slate-600">Files in pack</span>
                  <span className="tabular-nums font-medium">
                    3{notes.trim() ? " + notes" : ""}{receipts.length > 0 ? " + receipts" : ""} + PDF
                  </span>
                </div>
              </div>
            </div>

            <Separator />

            <div className="flex flex-wrap gap-3">
              <Button
                type="button"
                className="rounded-2xl"
                onClick={handleGenerate}
                disabled={generating || !canGenerate}
              >
                {generating ? (
                  <>
                    <Loader2 className="mr-2 size-4 animate-spin" aria-hidden="true" />
                    Generating Pack...
                  </>
                ) : (
                  <>
                    <Package className="mr-2 size-4" aria-hidden="true" />
                    Generate Accountant Pack
                  </>
                )}
              </Button>

              {zipBlob ? (
                <Button
                  type="button"
                  variant="secondary"
                  className="rounded-2xl"
                  onClick={handleDownloadZip}
                >
                  <Download className="mr-2 size-4" aria-hidden="true" />
                  Download ZIP
                </Button>
              ) : null}
            </div>

            {zipBlob ? (
              <div className="text-xs text-slate-500">
                ZIP ready: akaunkemas-accountant-pack.zip (
                {(zipBlob.size / 1024).toFixed(1)} KB)
              </div>
            ) : null}
          </CardContent>
        </Card>
      ) : (
        <Card className="rounded-2xl">
          <CardContent className="py-8 text-center">
            <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-400 ring-1 ring-slate-200">
              <Package className="size-5" aria-hidden="true" />
            </div>
            <div className="mt-3 text-sm font-medium text-slate-500">
              Upload a bank CSV to generate your accountant pack.
            </div>
            <div className="mt-1 text-xs text-slate-400">
              The pack includes cleaned CSV, receipt list, monthly summary JSON, PDF, and optional notes.
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
