"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { Download, FileSpreadsheet, FileText, Loader2, Table } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { ToolError } from "@/components/tools/ToolError";
import type { CategorySlug, Transaction } from "@/lib/akaunkemas/types";
import { CATEGORIES, getCategoryLabel } from "@/lib/akaunkemas/categories";
import { parseBankCsv } from "@/lib/akaunkemas/csv-parser";
import { transactionsToLedger, filterLedgerByMonth, filterLedgerByCategory, computeLedgerTotals, getAvailableMonths } from "@/lib/akaunkemas/ledger";
import { exportLedgerCsv, exportLedgerJson } from "@/lib/akaunkemas/ledger-export";
import type { ToolDefinition } from "@/lib/tools/types";

const MYR = new Intl.NumberFormat("ms-MY", { style: "currency", currency: "MYR" });

function formatCurrency(n: number): string {
  return MYR.format(Number.isFinite(n) ? n : 0);
}

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

function isValidCsvFile(f: File): boolean {
  return f.name.toLowerCase().endsWith(".csv");
}

export function SimpleLedgerTool(_props: { tool: ToolDefinition }) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [parsing, setParsing] = useState(false);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedMonth, setSelectedMonth] = useState<string>("all");
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [pdfGenerating, setPdfGenerating] = useState(false);

  const entries = useMemo(() => transactionsToLedger(transactions), [transactions]);

  const months = useMemo(() => getAvailableMonths(entries), [entries]);

  // Apply filters
  const filteredEntries = useMemo(() => {
    let result = entries;
    if (selectedMonth !== "all") {
      result = filterLedgerByMonth(result, selectedMonth);
    }
    if (selectedCategory !== "all") {
      result = filterLedgerByCategory(result, selectedCategory as CategorySlug);
    }
    return result;
  }, [entries, selectedMonth, selectedCategory]);

  const totals = useMemo(() => computeLedgerTotals(filteredEntries), [filteredEntries]);

  const handleFile = useCallback(
    async (f: File) => {
      if (!isValidCsvFile(f)) {
        setError("Invalid file type. Please upload a .csv file.");
        return;
      }
      if (transactions.length > 0 && !window.confirm("Uploading a new file will replace all current data. Continue?")) {
        return;
      }
      setFile(f);
      setError(null);
      setTransactions([]);
      setParsing(true);
      setSelectedMonth("all");
      setSelectedCategory("all");
      try {
        const text = await f.text();
        const parsed = parseBankCsv(text);
        if (parsed.transactions.length === 0 && parsed.errors.length > 0) {
          setError(parsed.errors.join("\n"));
        }
        setTransactions(parsed.transactions);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to parse CSV.");
      } finally {
        setParsing(false);
      }
    },
    [transactions.length],
  );

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

  // Export CSV
  const handleExportCsv = useCallback(() => {
    const csv = exportLedgerCsv(filteredEntries);
    downloadBlob(csv, "akaunkemas-ledger.csv", "text/csv");
  }, [filteredEntries]);

  // Export JSON
  const handleExportJson = useCallback(() => {
    const json = exportLedgerJson(filteredEntries, totals);
    downloadBlob(json, "akaunkemas-ledger.json", "application/json");
  }, [filteredEntries, totals]);

  // Export PDF
  const handleExportPdf = useCallback(async () => {
    if (filteredEntries.length === 0) return;
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

      page.drawText("AkaunKemas Simple Ledger", { x: MARGIN, y, size: 18, font: bold, color: rgb(0.07, 0.1, 0.2) });
      y -= 24;
      page.drawText(`Generated: ${new Date().toISOString().slice(0, 10)}`, { x: MARGIN, y, size: 10, font, color: rgb(0.4, 0.4, 0.4) });
      y -= 18;

      const totalItems = [
        ["Total Debit", formatCurrency(totals.totalDebit)],
        ["Total Credit", formatCurrency(totals.totalCredit)],
        ["Net Cashflow", formatCurrency(totals.netCashflow)],
      ];
      for (const [label, value] of totalItems) {
        page.drawText(label, { x: MARGIN, y, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
        const vw = font.widthOfTextAtSize(value, 10);
        page.drawText(value, { x: width - MARGIN - vw, y, size: 10, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 15;
      }

      y -= 6;
      page.drawLine({ start: { x: MARGIN, y }, end: { x: width - MARGIN, y }, thickness: 1, color: rgb(0.85, 0.87, 0.9) });
      y -= 18;

      // Mini table in PDF — entries
      page.drawText("Entries", { x: MARGIN, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
      y -= 16;
      const cols = { date: MARGIN, desc: MARGIN + 70, debit: MARGIN + 270, credit: MARGIN + 360, balance: MARGIN + 440 };
      page.drawText("Date", { x: cols.date, y, size: 9, font: bold, color: rgb(0.1, 0.1, 0.1) });
      page.drawText("Description", { x: cols.desc, y, size: 9, font: bold, color: rgb(0.1, 0.1, 0.1) });
      page.drawText("Debit", { x: cols.debit, y, size: 9, font: bold, color: rgb(0.1, 0.1, 0.1) });
      page.drawText("Credit", { x: cols.credit, y, size: 9, font: bold, color: rgb(0.1, 0.1, 0.1) });
      page.drawText("Balance", { x: cols.balance, y, size: 9, font: bold, color: rgb(0.1, 0.1, 0.1) });
      y -= 4;
      page.drawLine({ start: { x: MARGIN, y }, end: { x: width - MARGIN, y }, thickness: 0.5, color: rgb(0.85, 0.87, 0.9) });
      y -= 11;

      for (const e of filteredEntries) {
        if (y < 60) {
          const newPage = doc.addPage(A4);
          y = height - MARGIN;
          newPage.drawText("Entries (continued)", { x: MARGIN, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
          y -= 16;
        }
        page.drawText(e.date, { x: cols.date, y, size: 9, font, color: rgb(0.2, 0.2, 0.2) });
        const desc = e.description.length > 30 ? e.description.slice(0, 28) + "..." : e.description;
        page.drawText(desc, { x: cols.desc, y, size: 9, font, color: rgb(0.2, 0.2, 0.2) });
        page.drawText(e.debit > 0 ? e.debit.toFixed(2) : "-", { x: cols.debit, y, size: 9, font, color: rgb(0.2, 0.2, 0.2) });
        page.drawText(e.credit > 0 ? e.credit.toFixed(2) : "-", { x: cols.credit, y, size: 9, font, color: rgb(0.2, 0.2, 0.2) });
        page.drawText(e.runningBalance !== null ? e.runningBalance.toFixed(2) : "-", { x: cols.balance, y, size: 9, font, color: rgb(0.2, 0.2, 0.2) });
        y -= 12;
      }

      const footerY = 40;
      page.drawLine({ start: { x: MARGIN, y: footerY + 10 }, end: { x: width - MARGIN, y: footerY + 10 }, thickness: 0.5, color: rgb(0.85, 0.87, 0.9) });
      page.drawText("Generated by AkaunKemas / ToolRakyat. Please review with your accountant before submission.", { x: MARGIN, y: footerY - 8, size: 8, font, color: rgb(0.5, 0.5, 0.5) });

      const pdfBytes = await doc.save();
      downloadBlob(pdfBytes, "akaunkemas-ledger.pdf", "application/pdf");
    } catch (e) {
      setError(e instanceof Error ? e.message : "PDF generation failed.");
    } finally {
      setPdfGenerating(false);
    }
  }, [filteredEntries, totals]);

  return (
    <div className="space-y-6">
      {error ? <ToolError message={error} onRetry={() => { setError(null); if (file) handleFile(file); }} /> : null}

      {/* Upload */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">Upload Bank CSV</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div
            className={`rounded-2xl border border-dashed bg-white p-6 text-center transition-colors ${dragOver ? "border-sky-400 bg-sky-50/50" : "border-slate-200"}`}
            onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
            onDragLeave={() => setDragOver(false)}
            onDrop={(e) => { e.preventDefault(); setDragOver(false); const f = e.dataTransfer?.files?.[0]; validateAndHandleFile(f); }}
          >
            <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-900 ring-1 ring-slate-200">
              <FileSpreadsheet className="size-5" />
            </div>
            <div className="mt-3 text-sm font-medium text-slate-900">
              {file ? file.name : "Drop your bank CSV here, or click to browse"}
            </div>
            <div className="mt-1 text-xs text-slate-500">Accepted: .csv</div>
            <div className="mt-4 flex justify-center">
              <input
                ref={fileInputRef}
                type="file"
                accept=".csv,text/csv"
                className="sr-only"
                aria-label="Select a bank CSV file"
                onChange={(e) => { const f = e.target?.files?.[0]; validateAndHandleFile(f); }}
              />
              <Button type="button" className="rounded-2xl" onClick={() => { if (fileInputRef.current) fileInputRef.current.value = ""; fileInputRef.current?.click(); }}>
                Choose CSV file
              </Button>
            </div>
          </div>
          {parsing ? <div className="flex items-center gap-2 text-sm text-slate-600"><Loader2 className="size-4 animate-spin" /> Parsing CSV...</div> : null}
          {entries.length > 0 && !parsing ? <div className="text-xs text-slate-500">{entries.length} ledger entries.</div> : null}
        </CardContent>
      </Card>

      {/* Filters + Ledger Table */}
      {entries.length > 0 ? (
        <>
          {/* Filters */}
          <Card className="rounded-2xl">
            <CardContent className="pt-4">
              <div className="flex flex-wrap gap-3">
                <div>
                  <Label className="text-xs">Month</Label>
                  <Select value={selectedMonth} onValueChange={setSelectedMonth}>
                    <SelectTrigger className="mt-1 h-9 w-[160px] rounded-xl text-sm">
                      <SelectValue placeholder="All Months" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">All Months</SelectItem>
                      {months.map((m) => (
                        <SelectItem key={m} value={m}>{m}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Category</Label>
                  <Select value={selectedCategory} onValueChange={setSelectedCategory}>
                    <SelectTrigger className="mt-1 h-9 w-[220px] rounded-xl text-sm">
                      <SelectValue placeholder="All Categories" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">All Categories</SelectItem>
                      {CATEGORIES.map((cat) => (
                        <SelectItem key={cat.slug} value={cat.slug}>{cat.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Ledger Table */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Ledger ({filteredEntries.length} entries)</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left text-xs font-medium text-slate-500">
                      <th className="py-2 pr-3">#</th>
                      <th className="py-2 pr-3">Date</th>
                      <th className="py-2 pr-3">Description</th>
                      <th className="py-2 pr-3">Category</th>
                      <th className="py-2 pr-3 text-right">Debit</th>
                      <th className="py-2 pr-3 text-right">Credit</th>
                      <th className="py-2 pr-3 text-right">Balance</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredEntries.map((e) => (
                      <tr key={e.id} className="border-b last:border-0">
                        <td className="py-2 pr-3 text-slate-400">{e.id}</td>
                        <td className="py-2 pr-3 whitespace-nowrap">{e.date}</td>
                        <td className="py-2 pr-3 max-w-[180px] truncate">{e.description}</td>
                        <td className="py-2 pr-3 text-xs">{getCategoryLabel(e.category)}</td>
                        <td className="py-2 pr-3 text-right tabular-nums">{e.debit > 0 ? e.debit.toFixed(2) : "-"}</td>
                        <td className="py-2 pr-3 text-right tabular-nums">{e.credit > 0 ? e.credit.toFixed(2) : "-"}</td>
                        <td className="py-2 pr-3 text-right tabular-nums">{e.runningBalance !== null ? e.runningBalance.toFixed(2) : "-"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </CardContent>
          </Card>

          {/* Totals */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Totals</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
                <div>
                  <div className="text-xs text-slate-500">Total Debit</div>
                  <div className="text-lg font-semibold text-red-600 tabular-nums">{formatCurrency(totals.totalDebit)}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Total Credit</div>
                  <div className="text-lg font-semibold text-green-700 tabular-nums">{formatCurrency(totals.totalCredit)}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Net Cashflow</div>
                  <div className={`text-lg font-semibold tabular-nums ${totals.netCashflow >= 0 ? "text-green-700" : "text-red-600"}`}>
                    {formatCurrency(totals.netCashflow)}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Export */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Export</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportCsv} disabled={filteredEntries.length === 0}>
                  <Table className="mr-2 size-4" /> Ledger CSV
                </Button>
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportJson} disabled={filteredEntries.length === 0}>
                  <FileText className="mr-2 size-4" /> Ledger JSON
                </Button>
                <Button type="button" className="rounded-2xl" onClick={handleExportPdf} disabled={pdfGenerating || filteredEntries.length === 0}>
                  {pdfGenerating ? <Loader2 className="mr-2 size-4 animate-spin" /> : <Download className="mr-2 size-4" />}
                  Ledger PDF
                </Button>
              </div>
            </CardContent>
          </Card>
        </>
      ) : (
        !parsing ? (
          <Card className="rounded-2xl">
            <CardContent className="py-8 text-center">
              <div className="mx-auto grid size-12 place-items-center rounded-2xl bg-slate-50 text-slate-400 ring-1 ring-slate-200">
                <Table className="size-5" />
              </div>
              <div className="mt-3 text-sm font-medium text-slate-500">Upload a bank CSV to view ledger.</div>
              <div className="mt-1 text-xs text-slate-400">Shows running balance with month and category filters.</div>
            </CardContent>
          </Card>
        ) : null
      )}
    </div>
  );
}
