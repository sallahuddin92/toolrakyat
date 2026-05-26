"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { Download, FileText, Loader2, Plus, Receipt, Table, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { ToolError } from "@/components/tools/ToolError";
import type { CategorySlug, Receipt as ReceiptType, PaymentMethod } from "@/lib/akaunkemas/types";
import { CATEGORIES, getCategoryLabel } from "@/lib/akaunkemas/categories";
import { createReceipt, addReceipt, updateReceipt, deleteReceipt } from "@/lib/akaunkemas/receipts";
import { computeReceiptSummary } from "@/lib/akaunkemas/receipt-summary";
import { exportReceiptCsv, exportReceiptJson, parseReceiptCsv } from "@/lib/akaunkemas/receipt-export";
import type { ToolDefinition } from "@/lib/tools/types";

const MYR = new Intl.NumberFormat("ms-MY", { style: "currency", currency: "MYR" });

function formatCurrency(n: number): string {
  return MYR.format(Number.isFinite(n) ? n : 0);
}

function downloadBlob(content: string, filename: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 250);
}

const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: "cash", label: "Tunai / Cash" },
  { value: "card", label: "Kad / Card" },
  { value: "bank_transfer", label: "Pindahan Bank / Bank Transfer" },
  { value: "e_wallet", label: "E-Dompet / E-Wallet" },
  { value: "cheque", label: "Cek / Cheque" },
  { value: "other", label: "Lain-lain / Other" },
];

export function ReceiptOrganizerTool(_props: { tool: ToolDefinition }) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [receipts, setReceipts] = useState<ReceiptType[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [pdfGenerating, setPdfGenerating] = useState(false);

  // Add form state
  const [showForm, setShowForm] = useState(false);
  const [formDate, setFormDate] = useState(new Date().toISOString().slice(0, 10));
  const [formMerchant, setFormMerchant] = useState("");
  const [formAmount, setFormAmount] = useState("");
  const [formPaymentMethod, setFormPaymentMethod] = useState<PaymentMethod>("cash");
  const [formCategory, setFormCategory] = useState<CategorySlug>("uncategorised");
  const [formTax, setFormTax] = useState("");
  const [formServiceCharge, setFormServiceCharge] = useState("");
  const [formNotes, setFormNotes] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);

  // Import CSV
  const handleImportCsv = useCallback(
    async (f: File) => {
      if (!f.name.toLowerCase().endsWith(".csv")) {
        setError("Invalid file type. Please upload a .csv file.");
        return;
      }
      if (receipts.length > 0 && !window.confirm("Importing a CSV will merge with current receipts. Continue?")) {
        return;
      }
      setError(null);
      setImporting(true);
      try {
        const text = await f.text();
        const parsed = parseReceiptCsv(text);
        if (parsed.receipts.length === 0 && parsed.errors.length > 0) {
          setError(parsed.errors.join("\n"));
        } else {
          setReceipts((prev) => [...prev, ...parsed.receipts]);
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to parse receipt CSV.");
      } finally {
        setImporting(false);
      }
    },
    [receipts.length],
  );

  const summary = useMemo(() => computeReceiptSummary(receipts), [receipts]);

  // Add / Update receipt
  const resetForm = () => {
    setFormDate(new Date().toISOString().slice(0, 10));
    setFormMerchant("");
    setFormAmount("");
    setFormPaymentMethod("cash");
    setFormCategory("uncategorised");
    setFormTax("");
    setFormServiceCharge("");
    setFormNotes("");
    setEditingId(null);
  };

  const handleSubmit = useCallback(() => {
    if (!formMerchant.trim() || !formAmount) return;
    const amount = Number(formAmount);
    if (!isFinite(amount)) return;

    if (editingId !== null) {
      setReceipts((prev) =>
        updateReceipt(prev, editingId, {
          date: formDate,
          merchant: formMerchant.trim(),
          amount,
          paymentMethod: formPaymentMethod,
          category: formCategory,
          taxAmount: formTax ? Number(formTax) : 0,
          serviceCharge: formServiceCharge ? Number(formServiceCharge) : 0,
          notes: formNotes.trim(),
        }),
      );
    } else {
      const receipt = createReceipt({
        date: formDate,
        merchant: formMerchant.trim(),
        amount,
        paymentMethod: formPaymentMethod,
        category: formCategory,
        taxAmount: formTax ? Number(formTax) : 0,
        serviceCharge: formServiceCharge ? Number(formServiceCharge) : 0,
        notes: formNotes.trim(),
      });
      setReceipts((prev) => addReceipt(prev, receipt));
    }
    resetForm();
    setShowForm(false);
  }, [formDate, formMerchant, formAmount, formPaymentMethod, formCategory, formTax, formServiceCharge, formNotes, editingId]);

  const handleEdit = useCallback((r: ReceiptType) => {
    setFormDate(r.date);
    setFormMerchant(r.merchant);
    setFormAmount(r.amount.toString());
    setFormPaymentMethod(r.paymentMethod);
    setFormCategory(r.category);
    setFormTax(r.taxAmount > 0 ? r.taxAmount.toString() : "");
    setFormServiceCharge(r.serviceCharge > 0 ? r.serviceCharge.toString() : "");
    setFormNotes(r.notes);
    setEditingId(r.id);
    setShowForm(true);
  }, []);

  const handleDelete = useCallback((id: number) => {
    if (!window.confirm("Delete this receipt?")) return;
    setReceipts((prev) => deleteReceipt(prev, id));
  }, []);

  // Export CSV
  const handleExportCsv = useCallback(() => {
    const csv = exportReceiptCsv(receipts);
    downloadBlob(csv, "akaunkemas-receipts.csv", "text/csv");
  }, [receipts]);

  // Export JSON
  const handleExportJson = useCallback(() => {
    const json = exportReceiptJson(receipts);
    downloadBlob(json, "akaunkemas-receipts.json", "application/json");
  }, [receipts]);

  // Export PDF
  const handleExportPdf = useCallback(async () => {
    if (receipts.length === 0) return;
    setPdfGenerating(true);
    try {
      const { PDFDocument, StandardFonts, rgb } = await import("pdf-lib");
      const doc = await PDFDocument.create();
      const font = await doc.embedFont(StandardFonts.Helvetica);
      const bold = await doc.embedFont(StandardFonts.HelveticaBold);

      const A4: [number, number] = [595.28, 841.89];
      const MARGIN = 50;
      const page = doc.addPage(A4);
      const { width, height } = page.getSize();
      let y = height - MARGIN;

      page.drawText("AkaunKemas Receipt Summary", { x: MARGIN, y, size: 18, font: bold, color: rgb(0.07, 0.1, 0.2) });
      y -= 24;
      page.drawText(`Generated: ${new Date().toISOString().slice(0, 10)}`, { x: MARGIN, y, size: 10, font, color: rgb(0.4, 0.4, 0.4) });
      y -= 18;
      page.drawText(`Receipts: ${summary.receiptCount}`, { x: MARGIN, y, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
      y -= 6;
      page.drawLine({ start: { x: MARGIN, y }, end: { x: width - MARGIN, y }, thickness: 1, color: rgb(0.85, 0.87, 0.9) });
      y -= 18;

      const items = [
        ["Total Amount", formatCurrency(summary.totalAmount)],
        ["Total Tax", formatCurrency(summary.totalTax)],
        ["Total Service Charge", formatCurrency(summary.totalServiceCharge)],
      ];
      for (const [label, value] of items) {
        page.drawText(label, { x: MARGIN, y, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
        const vw = font.widthOfTextAtSize(value, 10);
        page.drawText(value, { x: width - MARGIN - vw, y, size: 10, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 15;
      }

      y -= 4;
      page.drawLine({ start: { x: MARGIN, y }, end: { x: width - MARGIN, y }, thickness: 1, color: rgb(0.85, 0.87, 0.9) });
      y -= 18;

      if (summary.categorySummaries.length > 0) {
        page.drawText("Category Breakdown", { x: MARGIN, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 20;
        for (const cs of summary.categorySummaries) {
          const label = getCategoryLabel(cs.category);
          page.drawText(label, { x: MARGIN, y, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
          const str = `${formatCurrency(cs.total)} (${cs.count})`;
          const sw = font.widthOfTextAtSize(str, 10);
          page.drawText(str, { x: width - MARGIN - sw, y, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
          y -= 14;
        }
      }

      const footerY = 40;
      page.drawLine({ start: { x: MARGIN, y: footerY + 10 }, end: { x: width - MARGIN, y: footerY + 10 }, thickness: 0.5, color: rgb(0.85, 0.87, 0.9) });
      page.drawText("Generated by AkaunKemas / ToolRakyat. Please review with your accountant before submission.", { x: MARGIN, y: footerY - 8, size: 8, font, color: rgb(0.5, 0.5, 0.5) });

      const pdfBytes = await doc.save();
      const blob = new Blob([pdfBytes as BlobPart], { type: "application/pdf" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "akaunkemas-receipt-summary.pdf";
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 250);
    } catch (e) {
      setError(e instanceof Error ? e.message : "PDF generation failed.");
    } finally {
      setPdfGenerating(false);
    }
  }, [receipts, summary]);

  return (
    <div className="space-y-6">
      {error ? <ToolError message={error} onRetry={() => setError(null)} /> : null}

      {/* Import + Add */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">Receipts</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <input
              ref={fileInputRef}
              type="file"
              accept=".csv,text/csv"
              className="sr-only"
              aria-label="Import receipt CSV"
              onChange={(e) => {
                const f = e.target?.files?.[0];
                if (f) handleImportCsv(f);
              }}
            />
            <Button
              type="button"
              variant="secondary"
              className="rounded-2xl"
              onClick={() => {
                if (fileInputRef.current) fileInputRef.current.value = "";
                fileInputRef.current?.click();
              }}
              disabled={importing}
            >
              {importing ? <Loader2 className="mr-2 size-4 animate-spin" /> : <FileText className="mr-2 size-4" />}
              Import CSV
            </Button>
            <Button
              type="button"
              className="rounded-2xl"
              onClick={() => {
                resetForm();
                setShowForm(true);
              }}
            >
              <Plus className="mr-2 size-4" />
              Add Receipt
            </Button>
          </div>

          {showForm ? (
            <Card className="rounded-2xl border-sky-200 bg-sky-50/50">
              <CardContent className="pt-4">
                <div className="grid gap-3 sm:grid-cols-2">
                  <div>
                    <Label htmlFor="rec-date" className="text-xs">Date</Label>
                    <Input id="rec-date" type="date" className="mt-1 rounded-xl" value={formDate} onChange={(e) => setFormDate(e.target.value)} />
                  </div>
                  <div>
                    <Label htmlFor="rec-merchant" className="text-xs">Merchant</Label>
                    <Input id="rec-merchant" className="mt-1 rounded-xl" value={formMerchant} onChange={(e) => setFormMerchant(e.target.value)} placeholder="Kedai Ali" />
                  </div>
                  <div>
                    <Label htmlFor="rec-amount" className="text-xs">Amount (RM)</Label>
                    <Input id="rec-amount" type="number" step="0.01" className="mt-1 rounded-xl" value={formAmount} onChange={(e) => setFormAmount(e.target.value)} placeholder="0.00" />
                  </div>
                  <div>
                    <Label className="text-xs">Payment Method</Label>
                    <Select value={formPaymentMethod} onValueChange={(v) => setFormPaymentMethod(v as PaymentMethod)}>
                      <SelectTrigger className="mt-1 h-9 rounded-xl text-sm">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {PAYMENT_METHODS.map((pm) => (
                          <SelectItem key={pm.value} value={pm.value}>{pm.label}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label className="text-xs">Category</Label>
                    <Select value={formCategory} onValueChange={(v) => setFormCategory(v as CategorySlug)}>
                      <SelectTrigger className="mt-1 h-9 rounded-xl text-sm">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {CATEGORIES.map((cat) => (
                          <SelectItem key={cat.slug} value={cat.slug}>{cat.label}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label htmlFor="rec-tax" className="text-xs">Tax Amount (RM)</Label>
                    <Input id="rec-tax" type="number" step="0.01" className="mt-1 rounded-xl" value={formTax} onChange={(e) => setFormTax(e.target.value)} placeholder="0.00" />
                  </div>
                  <div>
                    <Label htmlFor="rec-svc" className="text-xs">Service Charge (RM)</Label>
                    <Input id="rec-svc" type="number" step="0.01" className="mt-1 rounded-xl" value={formServiceCharge} onChange={(e) => setFormServiceCharge(e.target.value)} placeholder="0.00" />
                  </div>
                  <div>
                    <Label htmlFor="rec-notes" className="text-xs">Notes</Label>
                    <Textarea id="rec-notes" className="mt-1 rounded-xl" rows={2} value={formNotes} onChange={(e) => setFormNotes(e.target.value)} placeholder="Optional notes..." />
                  </div>
                </div>
                <div className="mt-4 flex gap-2">
                  <Button type="button" className="rounded-2xl" onClick={handleSubmit} disabled={!formMerchant.trim() || !formAmount}>
                    {editingId !== null ? "Update" : "Save"}
                  </Button>
                  <Button
                    type="button"
                    variant="secondary"
                    className="rounded-2xl"
                    onClick={() => {
                      resetForm();
                      setShowForm(false);
                    }}
                  >
                    Cancel
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : null}

          {importing ? (
            <div className="flex items-center gap-2 text-sm text-slate-600">
              <Loader2 className="size-4 animate-spin" /> Importing...
            </div>
          ) : null}
          {receipts.length > 0 ? (
            <div className="text-xs text-slate-500">{receipts.length} receipt(s).</div>
          ) : null}
        </CardContent>
      </Card>

      {/* Receipt Table */}
      {receipts.length > 0 ? (
        <>
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Receipts ({receipts.length})</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left text-xs font-medium text-slate-500">
                      <th className="py-2 pr-3">Date</th>
                      <th className="py-2 pr-3">Merchant</th>
                      <th className="py-2 pr-3 text-right">Amount</th>
                      <th className="py-2 pr-3">Payment</th>
                      <th className="py-2 pr-3">Category</th>
                      <th className="py-2 pr-3 text-right">Tax</th>
                      <th className="py-2 pr-3">Notes</th>
                      <th className="py-2 pr-3">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {receipts.map((r) => (
                      <tr key={r.id} className="border-b last:border-0">
                        <td className="py-2 pr-3 whitespace-nowrap">{r.date}</td>
                        <td className="py-2 pr-3 max-w-[150px] truncate">{r.merchant}</td>
                        <td className="py-2 pr-3 text-right tabular-nums">{r.amount.toFixed(2)}</td>
                        <td className="py-2 pr-3 text-xs">{r.paymentMethod}</td>
                        <td className="py-2 pr-3 text-xs">{getCategoryLabel(r.category)}</td>
                        <td className="py-2 pr-3 text-right tabular-nums">{r.taxAmount > 0 ? r.taxAmount.toFixed(2) : "-"}</td>
                        <td className="py-2 pr-3 max-w-[100px] truncate text-xs text-slate-500">{r.notes || "-"}</td>
                        <td className="py-2 pr-3">
                          <div className="flex gap-1">
                            <Button type="button" variant="ghost" size="sm" className="h-7 rounded-xl text-xs" onClick={() => handleEdit(r)}>
                              Edit
                            </Button>
                            <Button type="button" variant="ghost" size="sm" className="h-7 rounded-xl text-xs text-red-600" onClick={() => handleDelete(r.id)}>
                              <Trash2 className="size-3" />
                            </Button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
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
                  <div className="text-xs text-slate-500">Total Amount</div>
                  <div className="text-lg font-semibold tabular-nums">{formatCurrency(summary.totalAmount)}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Total Tax</div>
                  <div className="text-lg font-semibold tabular-nums">{formatCurrency(summary.totalTax)}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Total Service Charge</div>
                  <div className="text-lg font-semibold tabular-nums">{formatCurrency(summary.totalServiceCharge)}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Receipts</div>
                  <div className="text-lg font-semibold tabular-nums">{summary.receiptCount}</div>
                </div>
              </div>
              {summary.categorySummaries.length > 0 ? (
                <div className="mt-4">
                  <Separator className="my-3" />
                  <div className="text-xs font-medium text-slate-500 mb-2">Category Breakdown</div>
                  <div className="space-y-1">
                    {summary.categorySummaries.map((cs) => (
                      <div key={cs.category} className="flex items-center justify-between text-sm">
                        <span className="text-slate-700">
                          {getCategoryLabel(cs.category)}
                          <span className="ml-1 text-xs text-slate-400">({cs.count})</span>
                        </span>
                        <span className="tabular-nums font-medium">{formatCurrency(cs.total)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}
            </CardContent>
          </Card>

          {/* Export */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Export</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportCsv} disabled={receipts.length === 0}>
                  <Table className="mr-2 size-4" /> Receipt CSV
                </Button>
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportJson} disabled={receipts.length === 0}>
                  <FileText className="mr-2 size-4" /> Receipt JSON
                </Button>
                <Button type="button" className="rounded-2xl" onClick={handleExportPdf} disabled={pdfGenerating || receipts.length === 0}>
                  {pdfGenerating ? <Loader2 className="mr-2 size-4 animate-spin" /> : <Download className="mr-2 size-4" />}
                  Receipt Summary PDF
                </Button>
              </div>
            </CardContent>
          </Card>
        </>
      ) : (
        <Card className="rounded-2xl">
          <CardContent className="py-8 text-center">
            <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-400 ring-1 ring-slate-200">
              <Receipt className="size-5" />
            </div>
            <div className="mt-3 text-sm font-medium text-slate-500">No receipts yet.</div>
            <div className="mt-1 text-xs text-slate-400">Add a receipt manually or import a CSV file.</div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
