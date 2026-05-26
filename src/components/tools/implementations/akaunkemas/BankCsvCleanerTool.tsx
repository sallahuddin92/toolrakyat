"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { Download, FileSpreadsheet, FileText, Loader2, Table } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { ToolError } from "@/components/tools/ToolError";
import type { CategorySlug, ParseResult, Transaction } from "@/lib/akaunkemas/types";
import { CATEGORIES, getCategoryLabel } from "@/lib/akaunkemas/categories";
import { parseBankCsv, validateParseResult } from "@/lib/akaunkemas/csv-parser";
import { computeMonthlySummary } from "@/lib/akaunkemas/summary";
import { exportCleanedCsv } from "@/lib/akaunkemas/export-cleaned-csv";
import type { ToolDefinition } from "@/lib/tools/types";

const CLIENT_MAX_BYTES = 5 * 1024 * 1024; // 5MB client-side threshold

function downloadBlob(content: Uint8Array | string, filename: string, mime: string) {
  const blob =
    typeof content === "string"
      ? new Blob([content], { type: mime })
      : new Blob([content as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 250);
}

const MYR = new Intl.NumberFormat("ms-MY", { style: "currency", currency: "MYR" });

function formatCurrency(n: number): string {
  return MYR.format(Number.isFinite(n) ? n : 0);
}

/** Validate that a File has a .csv extension (case-insensitive). */
function isValidCsvFile(f: File): boolean {
  return f.name.toLowerCase().endsWith(".csv");
}

export function BankCsvCleanerTool(_props: { tool: ToolDefinition }) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [parsing, setParsing] = useState(false);
  const [result, setResult] = useState<ParseResult | null>(null);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [serverFallback, setServerFallback] = useState(false);

  // Business header state for PDF
  const [showBusinessHeader, setShowBusinessHeader] = useState(false);
  const [bizName, setBizName] = useState("");
  const [bizReg, setBizReg] = useState("");
  const [bizAddress, setBizAddress] = useState("");
  const [bizPhone, setBizPhone] = useState("");
  const [bizEmail, setBizEmail] = useState("");
  const [bizPreparedBy, setBizPreparedBy] = useState("");

  // PDF generation state
  const [pdfGenerating, setPdfGenerating] = useState(false);

  const summary = useMemo(() => computeMonthlySummary(transactions), [transactions]);

  /** Client-side parse */
  const parseClientSide = useCallback(async (csvText: string) => {
    const parsed = parseBankCsv(csvText);
    const issues = validateParseResult(parsed);
    const allErrors = [...parsed.errors, ...issues];
    setResult(parsed);
    setTransactions(parsed.transactions);
    // Only treat as a blocking error when zero transactions were parsed.
    // Warnings alongside usable transactions don't block the UI.
    setError(parsed.transactions.length === 0 && allErrors.length ? allErrors.join("\n") : null);
  }, []);

  /** Server-side fallback for large files */
  const parseServerSide = useCallback(async (csvText: string) => {
    try {
      const res = await fetch("/api/akaunkemas/parse-csv", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ csvText }),
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error((data as { error?: string }).error ?? "Server parse failed.");
      }
      const data = (await res.json()) as {
        success: boolean;
        transactions: Transaction[];
        detectedColumns: ParseResult["detectedColumns"];
        errors: string[];
        rawRowCount: number;
      };
      setResult({
        transactions: data.transactions,
        detectedColumns: data.detectedColumns,
        errors: data.errors,
        rawRowCount: data.rawRowCount,
      });
      setTransactions(data.transactions);
      // Only treat as a blocking error when zero transactions were parsed.
      setError(data.transactions.length === 0 && data.errors.length ? data.errors.join("\n") : null);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Server parse failed.";
      setError(msg);
    }
  }, []);

  /** Handle file upload */
  const handleFile = useCallback(
    async (f: File) => {
      // Validate file type before processing
      if (!isValidCsvFile(f)) {
        setError("Invalid file type. Please upload a .csv file.");
        return;
      }

      // Warn before replacing existing parsed data
      if (transactions.length > 0 && !window.confirm(
        "Uploading a new file will replace all current transactions and category assignments. Continue?"
      )) {
        return;
      }

      setFile(f);
      setError(null);
      setResult(null);
      setTransactions([]);
      setServerFallback(false);
      setParsing(true);

      try {
        const csvText = await f.text();

        if (f.size <= CLIENT_MAX_BYTES) {
          // Client-side
          await parseClientSide(csvText);
        } else {
          // Server-side fallback
          setServerFallback(true);
          await parseServerSide(csvText);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : "Failed to read file.";
        setError(msg);
      } finally {
        setParsing(false);
      }
    },
    [parseClientSide, parseServerSide, transactions.length],
  );

  /** Update category for a single transaction */
  const updateCategory = useCallback((txId: number, category: CategorySlug) => {
    setTransactions((prev) =>
      prev.map((tx) => (tx.id === txId ? { ...tx, category } : tx)),
    );
  }, []);

  /** Export cleaned CSV */
  const handleExportCsv = useCallback(() => {
    const csv = exportCleanedCsv(transactions);
    downloadBlob(csv, "akaunkemas-cleaned.csv", "text/csv");
  }, [transactions]);

  /** Export summary JSON */
  const handleExportJson = useCallback(() => {
    const json = JSON.stringify(
      {
        summary: {
          totalIncome: summary.totalIncome,
          totalExpense: summary.totalExpense,
          netCashflow: summary.netCashflow,
          transactionCount: summary.transactionCount,
        },
        categoryBreakdown: summary.categorySummaries.map((cs) => ({
          category: getCategoryLabel(cs.category),
          slug: cs.category,
          total: cs.total,
          count: cs.count,
        })),
      },
      null,
      2,
    );
    downloadBlob(json, "akaunkemas-summary.json", "application/json");
  }, [summary]);

  /** Export PDF */
  const handleExportPdf = useCallback(async () => {
    setPdfGenerating(true);
    try {
      const { generateMonthlySummaryPdf } = await import("@/lib/akaunkemas/pdf-export");
      const periodStart = transactions.length > 0
        ? transactions.reduce((a, b) => (a.date < b.date ? a : b)).date
        : "";
      const periodEnd = transactions.length > 0
        ? transactions.reduce((a, b) => (a.date > b.date ? a : b)).date
        : "";

      const pdfBytes = await generateMonthlySummaryPdf({
        summary,
        periodStart,
        periodEnd,
        businessHeader:
          showBusinessHeader && bizName
            ? {
                name: bizName,
                registrationNumber: bizReg || undefined,
                address: bizAddress || undefined,
                phone: bizPhone || undefined,
                email: bizEmail || undefined,
                preparedBy: bizPreparedBy || undefined,
              }
            : undefined,
      });

      downloadBlob(pdfBytes, "akaunkemas-monthly-summary.pdf", "application/pdf");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "PDF generation failed.";
      setError(msg);
    } finally {
      setPdfGenerating(false);
    }
  }, [summary, transactions, showBusinessHeader, bizName, bizReg, bizAddress, bizPhone, bizEmail, bizPreparedBy]);

  /** Validate and process a file from the drop event or file input. */
  const validateAndHandleFile = useCallback(
    (f: File | undefined) => {
      if (!f) return;
      if (!isValidCsvFile(f)) {
        setError("Invalid file type. Please upload a .csv file.");
        return;
      }
      handleFile(f);
    },
    [handleFile],
  );

  return (
    <div className="space-y-6">
      {error ? (
        <ToolError
          message={error}
          onRetry={() => {
            setError(null);
            if (file) handleFile(file);
          }}
        />
      ) : null}

      {/* Upload */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">Upload Bank CSV</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div
            className={`rounded-2xl border border-dashed bg-white p-6 text-center transition-colors ${
              dragOver ? "border-sky-400 bg-sky-50/50" : "border-slate-200"
            }`}
            onDragOver={(e) => {
              e.preventDefault();
              setDragOver(true);
            }}
            onDragLeave={() => setDragOver(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDragOver(false);
              const f = e.dataTransfer?.files?.[0];
              validateAndHandleFile(f);
            }}
          >
            <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-900 ring-1 ring-slate-200">
              <FileSpreadsheet className="size-5" aria-hidden="true" />
            </div>
            <div className="mt-3 text-sm font-medium text-slate-900">
              {file ? file.name : "Drop your bank CSV here, or click to browse"}
            </div>
            <div className="mt-1 text-xs text-slate-500">
              Accepted: .csv &bull; Max 25MB &bull; Files under 5MB processed locally
            </div>
            <div className="mt-4 flex justify-center">
              <input
                ref={fileInputRef}
                type="file"
                accept=".csv,text/csv"
                className="sr-only"
                aria-label="Select a bank CSV file to upload"
                onChange={(e) => {
                  const f = e.target?.files?.[0];
                  validateAndHandleFile(f);
                }}
              />
              <Button
                type="button"
                className="rounded-2xl"
                onClick={() => {
                  // Reset so the same file can be re-selected.
                  if (fileInputRef.current) fileInputRef.current.value = "";
                  fileInputRef.current?.click();
                }}
              >
                Choose CSV file
              </Button>
            </div>
          </div>
          {parsing ? (
            <div className="flex items-center gap-2 text-sm text-slate-600">
              <Loader2 className="size-4 animate-spin" aria-hidden="true" />
              {serverFallback
                ? "File too large for local processing. Parsing on server..."
                : "Parsing CSV..."}
            </div>
          ) : null}
          {result && !parsing ? (
            <div className="text-xs text-slate-500">
              Detected {result.transactions.length} transactions from {result.rawRowCount} rows.
              {serverFallback ? " (Server-side)" : " (Client-side)"}
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Transactions Table + Category Selection */}
      {transactions.length > 0 ? (
        <>
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">
                Transactions ({transactions.length})
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left text-xs font-medium text-slate-500">
                      <th className="py-2 pr-3">#</th>
                      <th className="py-2 pr-3">Date</th>
                      <th className="py-2 pr-3">Description</th>
                      <th className="py-2 pr-3 text-right">Debit</th>
                      <th className="py-2 pr-3 text-right">Credit</th>
                      <th className="py-2 pr-3 text-right">Amount</th>
                      <th className="py-2 pr-3">Category</th>
                    </tr>
                  </thead>
                  <tbody>
                    {transactions.slice(0, 100).map((tx) => (
                      <tr key={tx.id} className="border-b last:border-0">
                        <td className="py-2 pr-3 text-slate-400">{tx.id}</td>
                        <td className="py-2 pr-3 whitespace-nowrap">{tx.date}</td>
                        <td className="py-2 pr-3 max-w-[200px] truncate">{tx.description}</td>
                        <td className="py-2 pr-3 text-right tabular-nums">
                          {tx.debit > 0 ? tx.debit.toFixed(2) : "-"}
                        </td>
                        <td className="py-2 pr-3 text-right tabular-nums">
                          {tx.credit > 0 ? tx.credit.toFixed(2) : "-"}
                        </td>
                        <td
                          className={`py-2 pr-3 text-right tabular-nums ${
                            tx.amount < 0 ? "text-red-600" : "text-green-700"
                          }`}
                        >
                          {tx.amount.toFixed(2)}
                        </td>
                        <td className="py-2">
                          <Select
                            value={tx.category}
                            onValueChange={(v) => updateCategory(tx.id, v as CategorySlug)}
                          >
                            <SelectTrigger className="h-8 w-[180px] rounded-xl text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {CATEGORIES.map((cat) => (
                                <SelectItem key={cat.slug} value={cat.slug}>
                                  {cat.label}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {transactions.length > 100 ? (
                  <p className="mt-2 text-xs text-slate-500">
                    Showing first 100 of {transactions.length} transactions. Export still includes all parsed transactions.
                  </p>
                ) : null}
              </div>
            </CardContent>
          </Card>

          {/* Summary */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Summary</CardTitle>
            </CardHeader>
            <CardContent>
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

              {summary.categorySummaries.length > 0 ? (
                <div className="mt-4">
                  <Separator className="my-3" />
                  <div className="text-xs font-medium text-slate-500 mb-2">
                    Category Breakdown
                  </div>
                  <div className="space-y-1">
                    {summary.categorySummaries.map((cs) => (
                      <div
                        key={cs.category}
                        className="flex items-center justify-between text-sm"
                      >
                        <span className="text-slate-700">
                          {getCategoryLabel(cs.category)}
                          <span className="ml-1 text-xs text-slate-400">({cs.count})</span>
                        </span>
                        <span className="tabular-nums font-medium">
                          {formatCurrency(cs.total)}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}
            </CardContent>
          </Card>

          {/* Export Section */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Export</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="secondary"
                  className="rounded-2xl"
                  onClick={handleExportCsv}
                >
                  <Table className="mr-2 size-4" aria-hidden="true" />
                  Cleaned CSV
                </Button>
                <Button
                  type="button"
                  variant="secondary"
                  className="rounded-2xl"
                  onClick={handleExportJson}
                >
                  <FileText className="mr-2 size-4" aria-hidden="true" />
                  Summary JSON
                </Button>
                <Button
                  type="button"
                  className="rounded-2xl"
                  onClick={handleExportPdf}
                  disabled={pdfGenerating}
                >
                  {pdfGenerating ? (
                    <Loader2 className="mr-2 size-4 animate-spin" aria-hidden="true" />
                  ) : (
                    <Download className="mr-2 size-4" aria-hidden="true" />
                  )}
                  Monthly Summary PDF
                </Button>
              </div>

              {/* Optional Business Header */}
              <Separator />
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                  <Checkbox
                    id="show-business-header"
                    checked={showBusinessHeader}
                    onCheckedChange={(v) => setShowBusinessHeader(Boolean(v))}
                  />
                  <Label htmlFor="show-business-header" className="text-sm cursor-pointer">
                    Add business header to PDF
                  </Label>
                </div>

                {showBusinessHeader ? (
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div>
                      <Label htmlFor="biz-name" className="text-xs">Business Name</Label>
                      <input
                        id="biz-name"
                        className="mt-1 w-full rounded-xl border px-3 py-1.5 text-sm"
                        value={bizName}
                        onChange={(e) => setBizName(e.target.value)}
                        placeholder="Syarikat ABC Sdn Bhd"
                      />
                    </div>
                    <div>
                      <Label htmlFor="biz-reg" className="text-xs">Registration Number</Label>
                      <input
                        id="biz-reg"
                        className="mt-1 w-full rounded-xl border px-3 py-1.5 text-sm"
                        value={bizReg}
                        onChange={(e) => setBizReg(e.target.value)}
                        placeholder="202401001234"
                      />
                    </div>
                    <div>
                      <Label htmlFor="biz-address" className="text-xs">Address</Label>
                      <input
                        id="biz-address"
                        className="mt-1 w-full rounded-xl border px-3 py-1.5 text-sm"
                        value={bizAddress}
                        onChange={(e) => setBizAddress(e.target.value)}
                        placeholder="123, Jalan Example"
                      />
                    </div>
                    <div>
                      <Label htmlFor="biz-phone" className="text-xs">Phone</Label>
                      <input
                        id="biz-phone"
                        className="mt-1 w-full rounded-xl border px-3 py-1.5 text-sm"
                        value={bizPhone}
                        onChange={(e) => setBizPhone(e.target.value)}
                        placeholder="+60123456789"
                      />
                    </div>
                    <div>
                      <Label htmlFor="biz-email" className="text-xs">Email</Label>
                      <input
                        id="biz-email"
                        className="mt-1 w-full rounded-xl border px-3 py-1.5 text-sm"
                        value={bizEmail}
                        onChange={(e) => setBizEmail(e.target.value)}
                        placeholder="abc@example.com"
                      />
                    </div>
                    <div>
                      <Label htmlFor="biz-prepared-by" className="text-xs">Prepared By</Label>
                      <input
                        id="biz-prepared-by"
                        className="mt-1 w-full rounded-xl border px-3 py-1.5 text-sm"
                        value={bizPreparedBy}
                        onChange={(e) => setBizPreparedBy(e.target.value)}
                        placeholder="Ali bin Abu"
                      />
                    </div>
                  </div>
                ) : null}
              </div>
            </CardContent>
          </Card>
        </>
      ) : null}
    </div>
  );
}
