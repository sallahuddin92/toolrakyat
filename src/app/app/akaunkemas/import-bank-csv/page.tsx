"use client";

import { useState, useCallback, useMemo } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Upload, FileText, CheckCircle, AlertCircle, ArrowLeft,
  ArrowRight, RotateCcw,
} from "lucide-react";
import Link from "next/link";
import { importTransactionsCsv } from "../transactions/actions";
import { parseBankCsv } from "@/lib/akaunkemas/csv-parser";
import { suggestCategory } from "@/lib/akaunkemas-saas/category-suggestions";
import type { CategorySuggestion } from "@/lib/akaunkemas-saas/category-suggestions";
import { getCategoryLabel, CATEGORIES } from "@/lib/akaunkemas/categories";
import type { DetectedColumns, CategorySlug } from "@/lib/akaunkemas/types";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Step = "upload" | "review" | "result";

interface PreviewRow {
  date: string;
  description: string;
  debit: number;
  credit: number;
  amount: number;
  balance: number | null;
  suggestion: CategorySuggestion;
  overrideSlug: CategorySlug | null; // user override from dropdown
}

interface ImportResult {
  success: boolean;
  imported?: number;
  skipped?: number;
  errors?: string[];
  error?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatCurrency(n: number): string {
  return new Intl.NumberFormat("en-MY", {
    style: "currency",
    currency: "MYR",
    minimumFractionDigits: 2,
  }).format(n);
}

function confidenceBadge(c: string) {
  if (c === "high") return "bg-green-50 text-green-700 border-green-200";
  if (c === "medium") return "bg-yellow-50 text-yellow-700 border-yellow-200";
  return "bg-slate-50 text-slate-500 border-slate-200";
}

function StepIndicator({ step }: { step: Step }) {
  const steps = [
    { key: "upload", label: "Upload CSV", num: 1 },
    { key: "review", label: "Review & Categorise", num: 2 },
    { key: "result", label: "Import", num: 3 },
  ];
  return (
    <div className="flex items-center gap-2 mb-6">
      {steps.map((s, i) => {
        const active = step === s.key;
        const done =
          (step === "review" && s.key === "upload") ||
          (step === "result" && s.key !== "result");
        return (
          <div key={s.key} className="flex items-center gap-2">
            <div
              className={cn(
                "flex size-7 items-center justify-center rounded-full text-xs font-semibold border-2 transition-colors",
                active && "border-sky-500 bg-sky-500 text-white",
                done && "border-green-500 bg-green-500 text-white",
                !active && !done && "border-slate-200 text-slate-400",
              )}
            >
              {done ? <CheckCircle className="size-3.5" /> : s.num}
            </div>
            <span
              className={cn(
                "text-xs font-medium",
                active && "text-sky-700",
                done && "text-green-700",
                !active && !done && "text-slate-400",
              )}
            >
              {s.label}
            </span>
            {i < steps.length - 1 && (
              <div
                className={cn(
                  "w-8 h-px mx-1",
                  done ? "bg-green-300" : "bg-slate-200",
                )}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

const SUPPORTED_BANKS = [
  "Maybank", "CIMB", "Public Bank", "RHB", "Hong Leong",
  "AmBank", "Bank Islam", "UOB Malaysia", "Standard Chartered",
  "OCBC", "HSBC", "Alliance Bank", "Affin Bank",
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function ImportBankCsvPage() {
  const [step, setStep] = useState<Step>("upload");
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<PreviewRow[]>([]);
  const [detectedColumns, setDetectedColumns] = useState<DetectedColumns | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);

  // -----------------------------------------------------------------------
  // Parse CSV and apply category suggestions
  // -----------------------------------------------------------------------

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

      // Apply category suggestions to every transaction
      const rows: PreviewRow[] = parseResult.transactions.map((tx) => ({
        date: tx.date,
        description: tx.description,
        debit: tx.debit,
        credit: tx.credit,
        amount: tx.amount,
        balance: tx.balance,
        suggestion: suggestCategory(tx.description),
        overrideSlug: null,
      }));

      setPreview(rows);

      if (parseResult.errors.length > 0) {
        setParseError(parseResult.errors.join("; "));
      }

      // Auto-advance to review step
      setStep("review");
    } catch (e) {
      setParseError(e instanceof Error ? e.message : "Failed to parse CSV");
    }
  }, []);

  // -----------------------------------------------------------------------
  // Drag & drop
  // -----------------------------------------------------------------------

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

  // -----------------------------------------------------------------------
  // Category overrides
  // -----------------------------------------------------------------------

  function overrideCategory(idx: number, slug: CategorySlug | null) {
    setPreview((prev) => {
      const next = [...prev];
      next[idx] = { ...next[idx]!, overrideSlug: slug };
      return next;
    });
  }

  function effectiveCategory(row: PreviewRow): CategorySlug {
    return row.overrideSlug ?? row.suggestion.suggestedCategorySlug;
  }

  function applyAllHighConfidence() {
    setPreview((prev) =>
      prev.map((row) => ({
        ...row,
        overrideSlug:
          row.suggestion.confidence === "high"
            ? row.suggestion.suggestedCategorySlug
            : row.overrideSlug,
      })),
    );
  }

  function resetAllOverrides() {
    setPreview((prev) => prev.map((row) => ({ ...row, overrideSlug: null })));
  }

  // -----------------------------------------------------------------------
  // Summary stats for review step
  // -----------------------------------------------------------------------

  const summary = useMemo(() => {
    const highCount = preview.filter((r) => r.suggestion.confidence === "high").length;
    const mediumCount = preview.filter((r) => r.suggestion.confidence === "medium").length;
    const lowCount = preview.filter((r) => r.suggestion.confidence === "low").length;
    const totalDebit = preview.reduce((s, r) => s + r.debit, 0);
    const totalCredit = preview.reduce((s, r) => s + r.credit, 0);
    const overridden = preview.filter((r) => r.overrideSlug !== null).length;

    // Category breakdown (using effective category)
    const catCounts: Record<string, number> = {};
    for (const row of preview) {
      const cat = row.overrideSlug ?? row.suggestion.suggestedCategorySlug;
      catCounts[cat] = (catCounts[cat] || 0) + 1;
    }

    return { highCount, mediumCount, lowCount, totalDebit, totalCredit, overridden, catCounts };
  }, [preview]);

  // -----------------------------------------------------------------------
  // Import (step 2 → 3)
  // -----------------------------------------------------------------------

  const handleImport = useCallback(async () => {
    if (!file) return;
    setImporting(true);

    // Build category overrides map
    const categoryOverrides: Record<number, string> = {};
    preview.forEach((row, i) => {
      const cat = effectiveCategory(row);
      if (cat !== "uncategorised") {
        categoryOverrides[i] = cat;
      }
    });

    const formData = new FormData();
    formData.set("file", file);
    formData.set("categoryOverrides", JSON.stringify(categoryOverrides));

    const res = await importTransactionsCsv(formData);
    setResult(res);
    setImporting(false);
    setStep("result");
  }, [file, preview]);

  // -----------------------------------------------------------------------
  // Reset
  // -----------------------------------------------------------------------

  function reset() {
    setStep("upload");
    setFile(null);
    setPreview([]);
    setDetectedColumns(null);
    setParseError(null);
    setResult(null);
    setImporting(false);
  }

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

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

      <StepIndicator step={step} />

      {/* =================================================================
          Step 1 — Upload
          ================================================================= */}
      {step === "upload" && (
        <>
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
                  We&apos;ll auto-detect columns and suggest categories
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

          {/* Supported banks */}
          <Card className="rounded-2xl">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Supported Banks</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-1.5">
                {SUPPORTED_BANKS.map((bank) => (
                  <Badge key={bank} variant="secondary" className="text-[11px] font-normal">
                    {bank}
                  </Badge>
                ))}
              </div>
              <p className="text-xs text-slate-400 mt-3">
                Other bank formats may also work if they include date, description, and amount columns.
              </p>
            </CardContent>
          </Card>
        </>
      )}

      {/* =================================================================
          Step 2 — Review
          ================================================================= */}
      {step === "review" && (
        <>
          {/* File info bar */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <FileText className="size-5 text-slate-400" />
              <div>
                <p className="text-sm font-medium text-slate-900">{file?.name}</p>
                <p className="text-xs text-slate-500">
                  {(file!.size / 1024).toFixed(1)} KB &middot; {preview.length} transactions
                </p>
              </div>
            </div>
            <Button variant="ghost" size="sm" onClick={reset}>
              Change file
            </Button>
          </div>

          {parseError && (
            <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
              <AlertCircle className="size-4 mt-0.5 shrink-0" />
              {parseError}
            </div>
          )}

          {/* Summary cards */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <Card className="rounded-xl">
              <CardContent className="p-3 text-center">
                <p className="text-[10px] uppercase tracking-wider text-slate-400">Rows</p>
                <p className="text-lg font-bold text-slate-900">{preview.length}</p>
              </CardContent>
            </Card>
            <Card className="rounded-xl">
              <CardContent className="p-3 text-center">
                <p className="text-[10px] uppercase tracking-wider text-slate-400">Total Debit</p>
                <p className="text-lg font-bold text-red-600 tabular-nums">
                  {formatCurrency(summary.totalDebit)}
                </p>
              </CardContent>
            </Card>
            <Card className="rounded-xl">
              <CardContent className="p-3 text-center">
                <p className="text-[10px] uppercase tracking-wider text-slate-400">Total Credit</p>
                <p className="text-lg font-bold text-green-600 tabular-nums">
                  {formatCurrency(summary.totalCredit)}
                </p>
              </CardContent>
            </Card>
            <Card className="rounded-xl">
              <CardContent className="p-3 text-center">
                <p className="text-[10px] uppercase tracking-wider text-slate-400">Net</p>
                <p
                  className={cn(
                    "text-lg font-bold tabular-nums",
                    summary.totalCredit - summary.totalDebit >= 0
                      ? "text-green-600"
                      : "text-red-600",
                  )}
                >
                  {formatCurrency(summary.totalCredit - summary.totalDebit)}
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Suggestion quality */}
          <div className="flex items-center gap-4 text-xs text-slate-500">
            <span>
              <span className="inline-block size-2 rounded-full bg-green-500 mr-1" />
              {summary.highCount} high confidence
            </span>
            <span>
              <span className="inline-block size-2 rounded-full bg-yellow-500 mr-1" />
              {summary.mediumCount} medium confidence
            </span>
            <span>
              <span className="inline-block size-2 rounded-full bg-slate-300 mr-1" />
              {summary.lowCount} need review
            </span>
            {summary.overridden > 0 && (
              <span className="text-sky-600">
                {summary.overridden} manually adjusted
              </span>
            )}
          </div>

          {/* Bulk actions */}
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={applyAllHighConfidence}
              className="gap-1.5 text-xs"
            >
              <CheckCircle className="size-3" />
              Apply all high-confidence
            </Button>
            {summary.overridden > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={resetAllOverrides}
                className="gap-1.5 text-xs"
              >
                <RotateCcw className="size-3" />
                Reset adjustments
              </Button>
            )}
          </div>

          {/* Detected columns */}
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
                <CardTitle className="text-sm">
                  Review Categories ({preview.length} transactions)
                </CardTitle>
              </CardHeader>
              <CardContent className="p-0">
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-slate-200 bg-slate-50 text-left">
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                          Date
                        </th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                          Description
                        </th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500 text-right">
                          Amount
                        </th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                          Suggested Category
                        </th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                          Confidence
                        </th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                          Override
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-50">
                      {preview.map((row, i) => (
                        <tr
                          key={i}
                          className={cn(
                            "group text-xs transition-colors",
                            row.suggestion.confidence === "low" && "bg-amber-50/30",
                          )}
                        >
                            <td className="px-3 py-2 text-slate-500 whitespace-nowrap">
                              {row.date}
                            </td>
                            <td className="px-3 py-2 max-w-[200px] truncate text-slate-900">
                              {row.description}
                              {row.suggestion.confidence !== "low" && (
                                <span
                                  className="ml-1 text-[10px] text-slate-400 cursor-help"
                                  title={row.suggestion.reason}
                                >
                                  &#9432;
                                </span>
                              )}
                            </td>
                            <td
                              className={cn(
                                "px-3 py-2 text-right tabular-nums whitespace-nowrap",
                                row.amount < 0 ? "text-red-600" : "text-green-700",
                              )}
                            >
                              {formatCurrency(row.amount)}
                            </td>
                            <td className="px-3 py-2">
                              <span
                                className={cn(
                                  "text-xs",
                                  row.suggestion.confidence === "low" && "text-slate-400 italic",
                                )}
                              >
                                {getCategoryLabel(row.suggestion.suggestedCategorySlug)}
                              </span>
                            </td>
                            <td className="px-3 py-2">
                              <Badge
                                variant="outline"
                                className={cn(
                                  "text-[10px] font-normal",
                                  confidenceBadge(row.suggestion.confidence),
                                )}
                              >
                                {row.suggestion.confidence}
                              </Badge>
                            </td>
                            <td className="px-3 py-2">
                              <select
                                value={row.overrideSlug ?? ""}
                                onChange={(e) =>
                                  overrideCategory(
                                    i,
                                    (e.target.value || null) as CategorySlug | null,
                                  )
                                }
                                className={cn(
                                  "h-7 rounded border px-1.5 text-[11px] max-w-[140px]",
                                  row.overrideSlug
                                    ? "border-sky-300 bg-sky-50 text-sky-700"
                                    : "border-slate-200 text-slate-500",
                                )}
                              >
                                <option value="">Use suggestion</option>
                                {CATEGORIES.map((cat) => (
                                  <option key={cat.slug} value={cat.slug}>
                                    {cat.label}
                                  </option>
                                ))}
                              </select>
                            </td>
                          </tr>
                        ))}
                    </tbody>
                  </table>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Navigation buttons */}
          <div className="flex justify-between gap-3">
            <Button variant="outline" onClick={reset}>
              <ArrowLeft className="size-4" />
              Back
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
                  Import {preview.length} Transactions
                  <ArrowRight className="size-4" />
                </>
              )}
            </Button>
          </div>
        </>
      )}

      {/* =================================================================
          Step 3 — Result
          ================================================================= */}
      {step === "result" && result && (
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
                    {result.imported} imported
                    {result.skipped ? `, ${result.skipped} skipped (duplicates)` : ""}
                  </p>
                </div>
                <div className="flex gap-3">
                  <Link href="/app/akaunkemas/transactions">
                    <Button>View Transactions</Button>
                  </Link>
                  <Button variant="outline" onClick={reset}>
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
                <Button variant="outline" onClick={() => setStep("review")}>
                  Back to Review
                </Button>
              </>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
