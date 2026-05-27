"use client";

import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Upload, FileText, CheckCircle, AlertCircle,
  Sparkles, X, ChevronDown, ChevronUp,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { runImportPipeline } from "@/lib/akaunkemas-saas/import/import-pipeline";
import type { PipelineResult } from "@/lib/akaunkemas-saas/import/import-pipeline";
import { suggestCategory } from "@/lib/akaunkemas-saas/category-suggestions";
import { CATEGORIES, getCategoryLabel } from "@/lib/akaunkemas/categories";
import type { CategorySlug } from "@/lib/akaunkemas/types";
import {
  saveBankTransactions,
  saveReceipts,
  saveManualDocument,
} from "../import/actions";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ImportStep = "upload" | "detecting" | "preview" | "saving" | "done" | "error";

interface ManualFormData {
  documentType: "receipt" | "supporting_document" | "bank_statement_pending";
  date: string;
  merchant: string;
  amount: string;
  paymentMethod: string;
  categorySlug: string;
  notes: string;
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

function confidenceBadgeClass(c: string): string {
  if (c === "high") return "bg-green-50 text-green-700 border-green-200";
  if (c === "medium") return "bg-yellow-50 text-yellow-700 border-yellow-200";
  return "bg-slate-50 text-slate-500 border-slate-200";
}

function typeLabel(t: string): string {
  const map: Record<string, string> = {
    bank_csv: "Bank CSV",
    bank_xlsx: "Bank XLSX",
    receipt_csv: "Receipt CSV",
    receipt_json: "Receipt JSON",
    receipt_image: "Receipt Image",
    receipt_pdf: "Receipt PDF",
    generic_pdf: "PDF Document",
    unknown: "Unknown",
  };
  return map[t] ?? t;
}

function classificationLabel(l: string): string {
  const map: Record<string, string> = {
    bank_transactions: "Bank Transactions",
    receipts: "Receipts",
    invoice: "Invoice",
    marketplace_payout: "Marketplace Payout",
    payment_gateway_export: "Payment Gateway Export",
    supporting_document: "Supporting Document",
    unknown: "Unknown",
  };
  return map[l] ?? l;
}

const SUPPORTED_TYPES = [
  "CSV (.csv)", "Excel (.xlsx, .xls)", "PDF (.pdf)",
  "Images (.jpg, .png, .webp)", "JSON (.json)",
];

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export function UniversalImportClient() {
  const [step, setStep] = useState<ImportStep>("upload");
  const [file, setFile] = useState<File | null>(null);
  const [pipelineResult, setPipelineResult] = useState<PipelineResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [categoryOverrides, setCategoryOverrides] = useState<Map<number, CategorySlug>>(new Map());
  const [manualData, setManualData] = useState<ManualFormData>({
    documentType: "receipt",
    date: new Date().toISOString().slice(0, 10),
    merchant: "",
    amount: "",
    paymentMethod: "cash",
    categorySlug: "uncategorised",
    notes: "",
  });
  const [importResult, setImportResult] = useState<{
    success: boolean; imported?: number; skipped?: number; savedAs?: string; error?: string;
  } | null>(null);
  const [expandedRows, setExpandedRows] = useState<Set<number>>(new Set());

  // Handle file drop/select
  const handleFile = useCallback(async (selectedFile: File) => {
    setFile(selectedFile);
    setError(null);
    setStep("detecting");

    try {
      const buffer = await selectedFile.arrayBuffer();
      let textContent: string | undefined;
      if (selectedFile.type === "text/csv" || selectedFile.name.endsWith(".csv")) {
        textContent = await selectedFile.text();
      } else if (selectedFile.type === "application/json" || selectedFile.name.endsWith(".json")) {
        textContent = await selectedFile.text();
      }

      const result = await runImportPipeline({
        file: { name: selectedFile.name, type: selectedFile.type, size: selectedFile.size },
        contentBuffer: buffer,
        textContent,
      });

      if (result.stage === "rejected" && result.error) {
        setError(result.error);
        setStep("error");
        return;
      }

      if (result.error) {
        setError(result.error);
        setStep("error");
        return;
      }

      setPipelineResult(result);
      setCategoryOverrides(new Map());
      setStep("preview");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to process file");
      setStep("error");
    }
  }, []);

  // Drag & drop handlers
  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const f = e.dataTransfer.files[0];
    if (f) handleFile(f);
  }, [handleFile]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) handleFile(f);
  }, [handleFile]);

  // Category override
  function setOverride(idx: number, slug: CategorySlug | null) {
    setCategoryOverrides((prev) => {
      const next = new Map(prev);
      if (slug) next.set(idx, slug);
      else next.delete(idx);
      return next;
    });
  }

  function applyAllHighConfidence() {
    if (!pipelineResult?.bankTransactionsPreview) return;
    setCategoryOverrides((prev) => {
      const next = new Map(prev);
      pipelineResult.bankTransactionsPreview!.forEach((row, i) => {
        if (row.suggestion.confidence === "high") {
          next.set(i, row.suggestion.suggestedCategorySlug as CategorySlug);
        }
      });
      return next;
    });
  }

  // Save
  const handleSave = useCallback(async () => {
    if (!pipelineResult) return;
    setStep("saving");

    try {
      const cl = pipelineResult.classification;

      if (cl?.reviewMode === "bank_transactions_table" && pipelineResult.bankTransactionsPreview) {
        const transactions = pipelineResult.bankTransactionsPreview.map((row, i) => ({
          date: row.date,
          description: row.description,
          debit: row.debit,
          credit: row.credit,
          balance: row.balance,
          categorySlug: categoryOverrides.get(i) ?? row.suggestion.suggestedCategorySlug,
        }));
        const res = await saveBankTransactions(transactions);
        setImportResult(res);
      } else if (cl?.reviewMode === "receipt_form") {
        const rows = pipelineResult.receiptJsonRows ?? pipelineResult.receiptCsvRows ?? [];
        const receipts = rows.map((r) => ({
          date: r.date,
          merchant: r.merchant,
          amount: r.amount,
          paymentMethod: r.paymentMethod || "other",
          categorySlug: (r as any).categorySlug || "uncategorised",
          taxAmount: (r as any).taxAmount ?? 0,
          serviceCharge: (r as any).serviceCharge ?? 0,
          notes: (r as any).notes ?? "",
        }));
        const res = await saveReceipts(receipts);
        setImportResult(res);
      } else if (cl?.reviewMode === "manual_classification") {
        const res = await saveManualDocument({
          documentType: manualData.documentType,
          date: manualData.date,
          merchant: manualData.merchant || undefined,
          amount: manualData.amount ? parseFloat(manualData.amount) : undefined,
          paymentMethod: manualData.paymentMethod || undefined,
          categorySlug: manualData.categorySlug || undefined,
          notes: manualData.notes || undefined,
          fileName: file?.name ?? "unknown",
          fileSizeBytes: file?.size ?? 0,
        });
        setImportResult(res);
      }
      setStep("done");
    } catch (e) {
      setImportResult({
        success: false,
        error: e instanceof Error ? e.message : "Import failed",
      });
      setStep("done");
    }
  }, [pipelineResult, categoryOverrides, manualData, file]);

  function reset() {
    setStep("upload");
    setFile(null);
    setPipelineResult(null);
    setError(null);
    setCategoryOverrides(new Map());
    setImportResult(null);
  }

  // Summary for bank transactions
  const bankSummary = pipelineResult?.bankTransactionsPreview
    ? {
        count: pipelineResult.bankTransactionsPreview.length,
        totalDebit: pipelineResult.bankTransactionsPreview.reduce((s, r) => s + r.debit, 0),
        totalCredit: pipelineResult.bankTransactionsPreview.reduce((s, r) => s + r.credit, 0),
        highConf: pipelineResult.bankTransactionsPreview.filter((r) => r.suggestion.confidence === "high").length,
      }
    : null;

  // =========================================================================
  // Render
  // =========================================================================

  return (
    <div className="space-y-6 max-w-4xl">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Import</h1>
        <p className="text-sm text-slate-500">
          Drag and drop any financial file — CSV, Excel, PDF, images, or JSON. We&apos;ll detect what it is.
        </p>
      </div>

      {/* =================================================================
          Upload / Error states
          ================================================================= */}
      {(step === "upload" || step === "error") && (
        <>
          <Card className="rounded-2xl border-dashed">
            <CardContent
              className="flex flex-col items-center gap-4 py-12"
              onDragOver={(e) => e.preventDefault()}
              onDrop={handleDrop}
            >
              <div className="flex size-16 items-center justify-center rounded-2xl bg-sky-50">
                <Upload className="size-8 text-sky-500" />
              </div>
              <div className="text-center">
                <p className="text-sm font-medium text-slate-900">
                  Drop your file here
                </p>
                <p className="text-xs text-slate-500 mt-1">
                  We&apos;ll auto-detect the type and guide you through import
                </p>
              </div>
              <label className="cursor-pointer">
                <input
                  type="file"
                  accept=".csv,.xlsx,.xls,.pdf,.jpg,.jpeg,.png,.webp,.json"
                  className="hidden"
                  onChange={handleChange}
                />
                <Button variant="outline" asChild>
                  <span>Browse Files</span>
                </Button>
              </label>
            </CardContent>
          </Card>

          {error && (
            <Card className="rounded-2xl border-red-200 bg-red-50/50">
              <CardContent className="flex items-start gap-3 py-4">
                <AlertCircle className="size-5 text-red-500 mt-0.5 shrink-0" />
                <div>
                  <p className="text-sm font-medium text-red-800">Import failed</p>
                  <p className="text-xs text-red-600 mt-0.5">{error}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={reset}>
                    Try again
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          <Card className="rounded-2xl">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Supported file types</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-1.5">
                {SUPPORTED_TYPES.map((t) => (
                  <Badge key={t} variant="secondary" className="text-[11px] font-normal">
                    {t}
                  </Badge>
                ))}
              </div>
              <p className="text-xs text-slate-400 mt-3">
                Max file size: 10 MB. Files are not stored — only extracted data is saved.
              </p>
            </CardContent>
          </Card>
        </>
      )}

      {/* =================================================================
          Detecting (loading)
          ================================================================= */}
      {step === "detecting" && (
        <Card className="rounded-2xl">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-sky-50 animate-pulse">
              <FileText className="size-8 text-sky-500" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">Analysing file...</p>
              <p className="text-xs text-slate-500 mt-1">{file?.name}</p>
            </div>
          </CardContent>
        </Card>
      )}

      {/* =================================================================
          Preview (review before saving)
          ================================================================= */}
      {(step === "preview" || step === "saving") && pipelineResult && (
        <>
          {/* Detection + Classification info */}
          <Card className="rounded-2xl">
            <CardContent className="flex flex-wrap items-center gap-4 py-3">
              <div className="flex items-center gap-2">
                <FileText className="size-4 text-slate-400" />
                <span className="text-sm font-medium text-slate-700">{file?.name}</span>
                <span className="text-xs text-slate-400">
                  ({(file!.size / 1024).toFixed(1)} KB)
                </span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-xs text-slate-400">Detected:</span>
                <Badge variant="secondary" className="text-[10px] font-normal">
                  {typeLabel(pipelineResult.detection.detectedType)}
                </Badge>
                <span className={cn(
                  "inline-block size-1.5 rounded-full",
                  pipelineResult.detection.confidence === "high" ? "bg-green-500" :
                  pipelineResult.detection.confidence === "medium" ? "bg-yellow-500" :
                  "bg-slate-300",
                )} />
                <span className="text-[10px] text-slate-400">{pipelineResult.detection.confidence}</span>
              </div>
              {pipelineResult.classification && (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-slate-400">Classified as:</span>
                  <Badge className="text-[10px] font-normal bg-sky-50 text-sky-700 border-sky-200">
                    {classificationLabel(pipelineResult.classification.label)}
                  </Badge>
                </div>
              )}
              <Button variant="ghost" size="sm" onClick={reset} className="ml-auto">
                <X className="size-3.5" />
                Change file
              </Button>
            </CardContent>
          </Card>

          {/* Bank Transactions Preview */}
          {pipelineResult.classification?.reviewMode === "bank_transactions_table" && pipelineResult.bankTransactionsPreview && (
            <>
              {/* Summary stats */}
              {bankSummary && (
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                  <Card className="rounded-xl"><CardContent className="p-3 text-center">
                    <p className="text-[10px] uppercase tracking-wider text-slate-400">Rows</p>
                    <p className="text-lg font-bold text-slate-900">{bankSummary.count}</p>
                  </CardContent></Card>
                  <Card className="rounded-xl"><CardContent className="p-3 text-center">
                    <p className="text-[10px] uppercase tracking-wider text-slate-400">Total Debit</p>
                    <p className="text-lg font-bold text-red-600">{formatCurrency(bankSummary.totalDebit)}</p>
                  </CardContent></Card>
                  <Card className="rounded-xl"><CardContent className="p-3 text-center">
                    <p className="text-[10px] uppercase tracking-wider text-slate-400">Total Credit</p>
                    <p className="text-lg font-bold text-green-600">{formatCurrency(bankSummary.totalCredit)}</p>
                  </CardContent></Card>
                  <Card className="rounded-xl"><CardContent className="p-3 text-center">
                    <p className="text-[10px] uppercase tracking-wider text-slate-400">High Confidence</p>
                    <p className="text-lg font-bold text-sky-600">{bankSummary.highConf}</p>
                  </CardContent></Card>
                </div>
              )}

              {/* Bulk action */}
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm" onClick={applyAllHighConfidence} className="gap-1.5 text-xs">
                  <Sparkles className="size-3" />
                  Apply all high-confidence
                </Button>
              </div>

              {/* Preview table */}
              <Card className="rounded-2xl overflow-hidden">
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm">Review ({pipelineResult.bankTransactionsPreview.length} transactions)</CardTitle>
                </CardHeader>
                <CardContent className="p-0">
                  <div className="overflow-x-auto">
                    <table className="w-full">
                      <thead>
                        <tr className="border-b border-slate-200 bg-slate-50 text-left">
                          <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Date</th>
                          <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Description</th>
                          <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500 text-right">Amount</th>
                          <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Suggested</th>
                          <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Confidence</th>
                          <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Override</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-slate-50">
                        {pipelineResult.bankTransactionsPreview.map((row, i) => (
                          <tr key={i} className={cn("group text-xs", row.suggestion.confidence === "low" && "bg-amber-50/30")}>
                            <td className="px-3 py-2 text-slate-500 whitespace-nowrap">{row.date}</td>
                            <td className="px-3 py-2 max-w-[200px] truncate text-slate-900">{row.description}</td>
                            <td className={cn("px-3 py-2 text-right tabular-nums", row.amount < 0 ? "text-red-600" : "text-green-700")}>
                              {formatCurrency(row.amount)}
                            </td>
                            <td className="px-3 py-2 text-slate-600">{getCategoryLabel(row.suggestion.suggestedCategorySlug as CategorySlug)}</td>
                            <td className="px-3 py-2">
                              <Badge variant="outline" className={cn("text-[10px] font-normal", confidenceBadgeClass(row.suggestion.confidence))}>
                                {row.suggestion.confidence}
                              </Badge>
                            </td>
                            <td className="px-3 py-2">
                              <select
                                value={categoryOverrides.get(i) ?? ""}
                                onChange={(e) => setOverride(i, (e.target.value || null) as CategorySlug | null)}
                                className="h-7 rounded border border-slate-200 px-1.5 text-[11px] max-w-[140px]"
                              >
                                <option value="">Use suggestion</option>
                                {CATEGORIES.map((cat) => (
                                  <option key={cat.slug} value={cat.slug}>{cat.label}</option>
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
            </>
          )}

          {/* Receipt Preview */}
          {pipelineResult.classification?.reviewMode === "receipt_form" && (
            <Card className="rounded-2xl overflow-hidden">
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">
                  Receipts ({(pipelineResult.receiptJsonRows ?? pipelineResult.receiptCsvRows ?? []).length} items)
                </CardTitle>
              </CardHeader>
              <CardContent className="p-0">
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-slate-200 bg-slate-50 text-left">
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Date</th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Merchant</th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500 text-right">Amount</th>
                        <th className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-500">Payment</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-50">
                      {(pipelineResult.receiptJsonRows ?? pipelineResult.receiptCsvRows ?? []).map((row, i) => (
                        <tr key={i} className="text-xs">
                          <td className="px-3 py-2 text-slate-500 whitespace-nowrap">{row.date}</td>
                          <td className="px-3 py-2 text-slate-900">{row.merchant}</td>
                          <td className="px-3 py-2 text-right tabular-nums text-slate-900">{formatCurrency(row.amount)}</td>
                          <td className="px-3 py-2 text-slate-500">{row.paymentMethod || "other"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Manual Classification Form */}
          {pipelineResult.classification?.reviewMode === "manual_classification" && (
            <Card className="rounded-2xl">
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Manual entry</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="grid grid-cols-2 gap-3">
                  <div className="space-y-1">
                    <Label className="text-xs">Document type</Label>
                    <Select
                      value={manualData.documentType}
                      onValueChange={(v) => setManualData((prev) => ({ ...prev, documentType: v as ManualFormData["documentType"] }))}
                    >
                      <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="receipt">Receipt</SelectItem>
                        <SelectItem value="supporting_document">Supporting Document</SelectItem>
                        <SelectItem value="bank_statement_pending">Bank Statement (Pending)</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs">Date</Label>
                    <Input
                      type="date"
                      value={manualData.date}
                      onChange={(e) => setManualData((prev) => ({ ...prev, date: e.target.value }))}
                      className="h-8 text-xs"
                    />
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div className="space-y-1">
                    <Label className="text-xs">Merchant</Label>
                    <Input
                      value={manualData.merchant}
                      onChange={(e) => setManualData((prev) => ({ ...prev, merchant: e.target.value }))}
                      className="h-8 text-xs"
                      placeholder="e.g. Seven Eleven"
                    />
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs">Amount (MYR)</Label>
                    <Input
                      type="number"
                      step="0.01"
                      value={manualData.amount}
                      onChange={(e) => setManualData((prev) => ({ ...prev, amount: e.target.value }))}
                      className="h-8 text-xs"
                      placeholder="0.00"
                    />
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div className="space-y-1">
                    <Label className="text-xs">Payment method</Label>
                    <Select
                      value={manualData.paymentMethod}
                      onValueChange={(v) => setManualData((prev) => ({ ...prev, paymentMethod: v }))}
                    >
                      <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="cash">Cash</SelectItem>
                        <SelectItem value="card">Card</SelectItem>
                        <SelectItem value="bank_transfer">Bank Transfer</SelectItem>
                        <SelectItem value="e_wallet">E-Wallet</SelectItem>
                        <SelectItem value="cheque">Cheque</SelectItem>
                        <SelectItem value="other">Other</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs">Category</Label>
                    <Select
                      value={manualData.categorySlug}
                      onValueChange={(v) => setManualData((prev) => ({ ...prev, categorySlug: v }))}
                    >
                      <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        {CATEGORIES.map((cat) => (
                          <SelectItem key={cat.slug} value={cat.slug}>{cat.label}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <div className="space-y-1">
                  <Label className="text-xs">Notes</Label>
                  <Input
                    value={manualData.notes}
                    onChange={(e) => setManualData((prev) => ({ ...prev, notes: e.target.value }))}
                    className="h-8 text-xs"
                    placeholder="Optional notes"
                  />
                </div>
              </CardContent>
            </Card>
          )}

          {/* Save button */}
          <div className="flex items-center gap-3">
            <Button
              onClick={handleSave}
              disabled={step === "saving"}
              className="gap-1.5"
            >
              <CheckCircle className="size-4" />
              {step === "saving" ? "Saving..." : "Save import"}
            </Button>
            <Button variant="ghost" size="sm" onClick={reset}>
              Cancel
            </Button>
          </div>
        </>
      )}

      {/* =================================================================
          Done state
          ================================================================= */}
      {step === "done" && importResult && (
        <Card className={cn(
          "rounded-2xl",
          importResult.success ? "border-green-200 bg-green-50/50" : "border-red-200 bg-red-50/50",
        )}>
          <CardContent className="flex items-start gap-3 py-4">
            {importResult.success ? (
              <CheckCircle className="size-5 text-green-500 mt-0.5 shrink-0" />
            ) : (
              <AlertCircle className="size-5 text-red-500 mt-0.5 shrink-0" />
            )}
            <div>
              <p className={cn(
                "text-sm font-medium",
                importResult.success ? "text-green-800" : "text-red-800",
              )}>
                {importResult.success ? "Import complete" : "Import failed"}
              </p>
              {importResult.success ? (
                <p className="text-xs text-green-600 mt-0.5">
                  {importResult.imported != null ? `${importResult.imported} imported` : importResult.savedAs ? `Saved as ${importResult.savedAs}` : "Done"}
                  {importResult.skipped != null && importResult.skipped > 0 ? `, ${importResult.skipped} skipped` : ""}
                </p>
              ) : (
                <p className="text-xs text-red-600 mt-0.5">{importResult.error}</p>
              )}
              <Button variant="outline" size="sm" className="mt-2" onClick={reset}>
                Import another file
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}