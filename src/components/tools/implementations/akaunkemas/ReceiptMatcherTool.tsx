"use client";

import { useCallback, useRef, useState } from "react";
import { Download, FileSpreadsheet, FileText, GitMerge, Loader2, Table, X, ClipboardPaste, ChevronDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { ToolError } from "@/components/tools/ToolError";
import type { MatchingReport, Receipt, Transaction } from "@/lib/akaunkemas/types";
import { getCategoryLabel } from "@/lib/akaunkemas/categories";
import { parseBankCsv } from "@/lib/akaunkemas/csv-parser";
import { parseReceiptCsv } from "@/lib/akaunkemas/receipt-export";
import { matchReceiptsToTransactions, addManualMatch, removeMatch } from "@/lib/akaunkemas/receipt-matcher";
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

export function ReceiptMatcherTool(_props: { tool: ToolDefinition }) {
  const csvInputRef = useRef<HTMLInputElement>(null);
  const receiptInputRef = useRef<HTMLInputElement>(null);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [receipts, setReceipts] = useState<Receipt[]>([]);
  const [csvFile, setCsvFile] = useState<File | null>(null);
  const [receiptFile, setReceiptFile] = useState<File | null>(null);
  const [csvParsing, setCsvParsing] = useState(false);
  const [receiptParsing, setReceiptParsing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dateWindow, setDateWindow] = useState(3);
  const [report, setReport] = useState<MatchingReport | null>(null);
  const [activeTab, setActiveTab] = useState("matched");
  const [showPasteCsv, setShowPasteCsv] = useState(false);
  const [pasteCsvText, setPasteCsvText] = useState("");
  const [pdfGenerating, setPdfGenerating] = useState(false);

  // Parse bank CSV
  const handleCsvFile = useCallback(
    async (f: File) => {
      if (!isValidCsvFile(f)) { setError("Invalid file type. Please upload a .csv file."); return; }
      if (transactions.length > 0 && !window.confirm("Uploading a new CSV will replace all current transactions. Continue?")) return;
      setCsvFile(f);
      setError(null);
      setReport(null);
      setCsvParsing(true);
      try {
        const text = await f.text();
        const parsed = parseBankCsv(text);
        if (parsed.transactions.length === 0 && parsed.errors.length > 0) setError(parsed.errors.join("\n"));
        setTransactions(parsed.transactions);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to parse CSV.");
      } finally { setCsvParsing(false); }
    },
    [transactions.length],
  );

  // Parse pasted bank CSV text
  const handlePasteCsv = useCallback(() => {
    if (!pasteCsvText.trim()) return;
    if (transactions.length > 0 && !window.confirm("Parsing pasted CSV will replace all current transactions. Continue?")) return;
    setError(null);
    setReport(null);
    setCsvParsing(true);
    try {
      const parsed = parseBankCsv(pasteCsvText);
      if (parsed.transactions.length === 0 && parsed.errors.length > 0) setError(parsed.errors.join("\n"));
      setTransactions(parsed.transactions);
      setCsvFile(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to parse CSV.");
    } finally { setCsvParsing(false); }
  }, [pasteCsvText, transactions.length]);

  // Parse receipt file (CSV or JSON)
  const handleReceiptFile = useCallback(
    async (f: File) => {
      if (receipts.length > 0 && !window.confirm("Uploading a new file will replace all current receipts. Continue?")) return;
      setReceiptFile(f);
      setError(null);
      setReport(null);
      setReceiptParsing(true);
      try {
        const text = await f.text();
        if (f.name.toLowerCase().endsWith(".json")) {
          const data = JSON.parse(text);
          const parsedReceipts: Receipt[] = Array.isArray(data) ? data : Array.isArray(data.receipts) ? data.receipts : [];
          setReceipts(parsedReceipts);
        } else {
          const parsed = parseReceiptCsv(text);
          if (parsed.receipts.length === 0 && parsed.errors.length > 0) setError(parsed.errors.join("\n"));
          setReceipts(parsed.receipts);
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to parse receipt file.");
      } finally { setReceiptParsing(false); }
    },
    [receipts.length],
  );

  // Run matching
  const handleMatch = useCallback(() => {
    if (transactions.length === 0 || receipts.length === 0) return;
    setError(null);
    try {
      const r = matchReceiptsToTransactions(transactions, receipts, dateWindow);
      setReport(r);
      setActiveTab("matched");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Matching failed.");
    }
  }, [transactions, receipts, dateWindow]);

  // Manual match
  const handleManualMatch = useCallback(
    (bankTxId: number, receiptId: number) => {
      if (!report) return;
      try {
        const updated = addManualMatch(report, bankTxId, receiptId, transactions, receipts);
        setReport(updated);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Manual match failed.");
      }
    },
    [report, transactions, receipts],
  );

  // Remove match
  const handleRemoveMatch = useCallback(
    (bankTxId: number, receiptId: number) => {
      if (!report) return;
      try {
        const updated = removeMatch(report, bankTxId, receiptId, transactions, receipts);
        setReport(updated);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Remove match failed.");
      }
    },
    [report, transactions, receipts],
  );

  // Export report as CSV
  const handleExportCsv = useCallback(() => {
    if (!report) return;
    const rows = report.matched.map((m) => {
      const tx = transactions.find((t) => t.id === m.bankTxId);
      const rec = receipts.find((r) => r.id === m.receiptId);
      return {
        matchType: m.matchType,
        bankDate: tx?.date ?? "",
        bankDesc: tx?.description ?? "",
        bankAmount: tx?.amount.toFixed(2) ?? "",
        receiptDate: rec?.date ?? "",
        receiptMerchant: rec?.merchant ?? "",
        receiptAmount: rec?.amount.toFixed(2) ?? "",
        dateDelta: m.dateDelta,
        amountDelta: m.amountDelta.toFixed(2),
      };
    });
    const header = "matchType,bankDate,bankDesc,bankAmount,receiptDate,receiptMerchant,receiptAmount,dateDelta,amountDelta";
    const csv = [header, ...rows.map((r) => Object.values(r).join(","))].join("\n");
    downloadBlob(csv, "akaunkemas-matched.csv", "text/csv");
  }, [report, transactions, receipts]);

  // Export report as JSON
  const handleExportJson = useCallback(() => {
    if (!report) return;
    const json = JSON.stringify({
      matched: report.matched,
      unmatchedBankCount: report.unmatchedBank.length,
      unmatchedReceiptCount: report.unmatchedReceipts.length,
      dateWindowDays: report.dateWindowDays,
    }, null, 2);
    downloadBlob(json, "akaunkemas-matching-report.json", "application/json");
  }, [report]);

  // Export unmatched items as CSV
  const handleExportUnmatchedCsv = useCallback(() => {
    if (!report) return;
    const bankRows = report.unmatchedBank.map((tx) => ({
      type: "BANK",
      date: tx.date,
      description: tx.description,
      amount: tx.amount.toFixed(2),
      category: getCategoryLabel(tx.category),
      detail: "",
    }));
    const receiptRows = report.unmatchedReceipts.map((r) => ({
      type: "RECEIPT",
      date: r.date,
      description: r.merchant,
      amount: r.amount.toFixed(2),
      category: getCategoryLabel(r.category),
      detail: r.paymentMethod,
    }));
    const allRows = [...bankRows, ...receiptRows];
    const header = "type,date,description,amount,category,detail";
    const csv = [header, ...allRows.map((r) => Object.values(r).join(","))].join("\n");
    downloadBlob(csv, "akaunkemas-unmatched.csv", "text/csv");
  }, [report]);

  // Export matching summary as PDF
  const handleExportPdf = useCallback(async () => {
    if (!report) return;
    setPdfGenerating(true);
    try {
      const { PDFDocument, StandardFonts, rgb } = await import("pdf-lib");

      const doc = await PDFDocument.create();
      const font = await doc.embedFont(StandardFonts.Helvetica);
      const bold = await doc.embedFont(StandardFonts.HelveticaBold);

      let page = doc.addPage([595.28, 841.89]);
      const { width, height } = page.getSize();
      const M = 50;

      const addPage = (): number => {
        page = doc.addPage([595.28, 841.89]);
        return height - M;
      };

      let y = height - M;

      // Title
      page.drawText("AkaunKemas Matching Summary", {
        x: M, y, size: 16, font: bold, color: rgb(0.07, 0.1, 0.2),
      });
      y -= 24;
      page.drawText(`Generated: ${new Date().toISOString().slice(0, 10)}`, {
        x: M, y, size: 10, font, color: rgb(0.4, 0.4, 0.4),
      });
      y -= 18;
      page.drawText(`Date Window: \u00B1${report.dateWindowDays} days`, {
        x: M, y, size: 10, font, color: rgb(0.2, 0.2, 0.2),
      });
      y -= 20;

      page.drawLine({
        start: { x: M, y }, end: { x: width - M, y },
        thickness: 1, color: rgb(0.85, 0.87, 0.9),
      });
      y -= 18;

      // Stats
      page.drawText("Summary", { x: M, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
      y -= 18;

      const stats = [
        ["Matched", String(report.matched.length)],
        ["Unmatched Bank", String(report.unmatchedBank.length)],
        ["Unmatched Receipts", String(report.unmatchedReceipts.length)],
      ];
      for (const [label, value] of stats) {
        page.drawText(label, { x: M, y, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
        const vw = font.widthOfTextAtSize(value, 10);
        page.drawText(value, { x: width - M - vw, y, size: 10, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 15;
      }

      y -= 6;
      page.drawLine({
        start: { x: M, y }, end: { x: width - M, y },
        thickness: 1, color: rgb(0.85, 0.87, 0.9),
      });
      y -= 18;

      // Matched section
      if (report.matched.length > 0) {
        if (y < 120) y = addPage();
        page.drawText("Matched Transactions", { x: M, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 18;

        page.drawText("Type", { x: M, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Bank Date", { x: M + 50, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Rec Date", { x: M + 120, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Merchant", { x: M + 190, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Amount", { x: M + 330, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        y -= 4;
        page.drawLine({
          start: { x: M, y }, end: { x: width - M, y },
          thickness: 0.5, color: rgb(0.85, 0.87, 0.9),
        });
        y -= 12;

        for (const m of report.matched) {
          if (y < 60) y = addPage();
          const tx = transactions.find((t) => t.id === m.bankTxId);
          const rec = receipts.find((r) => r.id === m.receiptId);
          page.drawText(m.matchType, { x: M, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(tx?.date.slice(0, 10) ?? "-", { x: M + 50, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(rec?.date.slice(0, 10) ?? "-", { x: M + 120, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText((rec?.merchant ?? "-").slice(0, 20), { x: M + 190, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(formatCurrency(rec?.amount ?? 0), { x: M + 330, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          y -= 11;
        }
      }

      // Unmatched Bank section
      if (report.unmatchedBank.length > 0) {
        if (y < 120) y = addPage();
        page.drawText("Unmatched Bank Transactions", { x: M, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 18;

        page.drawText("Date", { x: M, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Description", { x: M + 100, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Amount", { x: M + 320, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        y -= 4;
        page.drawLine({
          start: { x: M, y }, end: { x: width - M, y },
          thickness: 0.5, color: rgb(0.85, 0.87, 0.9),
        });
        y -= 12;

        for (const tx of report.unmatchedBank) {
          if (y < 60) y = addPage();
          page.drawText(tx.date.slice(0, 10), { x: M, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(tx.description.slice(0, 35), { x: M + 100, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(formatCurrency(Math.abs(tx.amount)), { x: M + 320, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          y -= 11;
        }
      }

      // Unmatched Receipts section
      if (report.unmatchedReceipts.length > 0) {
        if (y < 120) y = addPage();
        page.drawText("Unmatched Receipts", { x: M, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
        y -= 18;

        page.drawText("Date", { x: M, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Merchant", { x: M + 100, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        page.drawText("Amount", { x: M + 320, y, size: 8, font: bold, color: rgb(0.3, 0.3, 0.3) });
        y -= 4;
        page.drawLine({
          start: { x: M, y }, end: { x: width - M, y },
          thickness: 0.5, color: rgb(0.85, 0.87, 0.9),
        });
        y -= 12;

        for (const r of report.unmatchedReceipts) {
          if (y < 60) y = addPage();
          page.drawText(r.date.slice(0, 10), { x: M, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(r.merchant.slice(0, 35), { x: M + 100, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          page.drawText(formatCurrency(r.amount), { x: M + 320, y, size: 8, font, color: rgb(0.2, 0.2, 0.2) });
          y -= 11;
        }
      }

      // Footer
      const footerY = 40;
      page.drawLine({
        start: { x: M, y: footerY + 10 },
        end: { x: width - M, y: footerY + 10 },
        thickness: 0.5, color: rgb(0.85, 0.87, 0.9),
      });
      page.drawText(
        "Generated by AkaunKemas / ToolRakyat.",
        { x: M, y: footerY - 8, size: 8, font, color: rgb(0.5, 0.5, 0.5) },
      );

      const pdfBytes = await doc.save();
      downloadBlob(pdfBytes, "akaunkemas-matching-summary.pdf", "application/pdf");
    } catch (e) {
      setError(e instanceof Error ? e.message : "PDF generation failed.");
    } finally { setPdfGenerating(false); }
  }, [report, transactions, receipts]);

  return (
    <div className="space-y-6">
      {error ? <ToolError message={error} onRetry={() => setError(null)} /> : null}

      {/* Upload Section — two panels */}
      <div className="grid gap-4 lg:grid-cols-2">
        {/* Bank CSV */}
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle className="text-base">Bank Transactions (CSV)</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <input ref={csvInputRef} type="file" accept=".csv,text/csv" className="sr-only"
              onChange={(e) => { const f = e.target?.files?.[0]; if (f) handleCsvFile(f); }} />
            <Button type="button" variant="secondary" className="rounded-2xl w-full" onClick={() => { if (csvInputRef.current) csvInputRef.current.value = ""; csvInputRef.current?.click(); }}>
              <FileSpreadsheet className="mr-2 size-4" /> Choose Bank CSV
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="rounded-xl w-full text-xs text-slate-500"
              onClick={() => setShowPasteCsv((v) => !v)}
            >
              <ClipboardPaste className="mr-1 size-3" />
              {showPasteCsv ? "Hide" : "Or paste CSV text"}
              <ChevronDown className={`ml-1 size-3 transition-transform ${showPasteCsv ? "rotate-180" : ""}`} />
            </Button>
            {showPasteCsv ? (
              <div className="space-y-2">
                <textarea
                  className="w-full rounded-xl border p-2 text-xs font-mono h-24 resize-y"
                  placeholder="Paste your bank CSV text here..."
                  value={pasteCsvText}
                  onChange={(e) => setPasteCsvText(e.target.value)}
                  disabled={csvParsing}
                />
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  className="rounded-xl text-xs"
                  onClick={handlePasteCsv}
                  disabled={!pasteCsvText.trim() || csvParsing}
                >
                  Parse Pasted CSV
                </Button>
              </div>
            ) : null}
            {csvParsing ? <div className="flex items-center gap-2 text-sm text-slate-600"><Loader2 className="size-4 animate-spin" /> Parsing...</div> : null}
            {transactions.length > 0 && !csvParsing ? (
              <div className="text-xs text-slate-500">{transactions.length} transactions loaded {csvFile ? `(${csvFile.name})` : "(from paste)"}</div>
            ) : null}
          </CardContent>
        </Card>

        {/* Receipt File */}
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle className="text-base">Receipts (CSV or JSON)</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <input ref={receiptInputRef} type="file" accept=".csv,.json,text/csv,application/json" className="sr-only"
              onChange={(e) => { const f = e.target?.files?.[0]; if (f) handleReceiptFile(f); }} />
            <Button type="button" variant="secondary" className="rounded-2xl w-full" onClick={() => { if (receiptInputRef.current) receiptInputRef.current.value = ""; receiptInputRef.current?.click(); }}>
              <FileText className="mr-2 size-4" /> Choose Receipt File
            </Button>
            {receiptParsing ? <div className="flex items-center gap-2 text-sm text-slate-600"><Loader2 className="size-4 animate-spin" /> Parsing...</div> : null}
            {receipts.length > 0 && !receiptParsing ? (
              <div className="text-xs text-slate-500">{receipts.length} receipts loaded {receiptFile ? `(${receiptFile.name})` : ""}</div>
            ) : null}
          </CardContent>
        </Card>
      </div>

      {/* Match Controls */}
      {transactions.length > 0 && receipts.length > 0 ? (
        <Card className="rounded-2xl">
          <CardContent className="pt-4">
            <div className="flex flex-wrap items-end gap-3">
              <div>
                <Label htmlFor="date-window" className="text-xs">Date Window (days)</Label>
                <Input id="date-window" type="number" min={0} max={30} className="mt-1 w-24 rounded-xl" value={dateWindow} onChange={(e) => setDateWindow(Number(e.target.value) || 0)} />
              </div>
              <Button type="button" className="rounded-2xl" onClick={handleMatch}>
                <GitMerge className="mr-2 size-4" /> Run Matching
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : null}

      {/* Results */}
      {report ? (
        <>
          {/* Summary */}
          <Card className="rounded-2xl">
            <CardContent className="pt-4">
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                <div>
                  <div className="text-xs text-slate-500">Matched</div>
                  <div className="text-lg font-semibold tabular-nums text-green-700">{report.matched.length}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Unmatched Bank</div>
                  <div className="text-lg font-semibold tabular-nums text-amber-600">{report.unmatchedBank.length}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Unmatched Receipts</div>
                  <div className="text-lg font-semibold tabular-nums text-amber-600">{report.unmatchedReceipts.length}</div>
                </div>
                <div>
                  <div className="text-xs text-slate-500">Date Window</div>
                  <div className="text-lg font-semibold tabular-nums">&plusmn;{report.dateWindowDays} days</div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Tabs */}
          <Card className="rounded-2xl">
            <CardContent className="pt-4">
              <Tabs value={activeTab} onValueChange={setActiveTab}>
                <TabsList className="rounded-xl">
                  <TabsTrigger value="matched" className="rounded-xl">
                    Matched ({report.matched.length})
                  </TabsTrigger>
                  <TabsTrigger value="unmatchedBank" className="rounded-xl">
                    Unmatched Bank ({report.unmatchedBank.length})
                  </TabsTrigger>
                  <TabsTrigger value="unmatchedReceipts" className="rounded-xl">
                    Unmatched Receipts ({report.unmatchedReceipts.length})
                  </TabsTrigger>
                </TabsList>

                {/* Matched Tab */}
                <TabsContent value="matched" className="mt-4">
                  {report.matched.length === 0 ? (
                    <div className="text-center py-8 text-sm text-slate-500">No matches found.</div>
                  ) : (
                    <div className="overflow-x-auto">
                      <table className="w-full text-sm">
                        <thead>
                          <tr className="border-b text-left text-xs font-medium text-slate-500">
                            <th className="py-2 pr-3">Type</th>
                            <th className="py-2 pr-3">Bank Date</th>
                            <th className="py-2 pr-3">Bank Description</th>
                            <th className="py-2 pr-3 text-right">Bank Amount</th>
                            <th className="py-2 pr-3">Receipt Date</th>
                            <th className="py-2 pr-3">Merchant</th>
                            <th className="py-2 pr-3 text-right">Receipt Amount</th>
                            <th className="py-2 pr-3">Delta</th>
                            <th className="py-2 pr-3">Actions</th>
                          </tr>
                        </thead>
                        <tbody>
                          {report.matched.map((m) => {
                            const tx = transactions.find((t) => t.id === m.bankTxId);
                            const rec = receipts.find((r) => r.id === m.receiptId);
                            return (
                              <tr key={`${m.bankTxId}-${m.receiptId}`} className="border-b last:border-0">
                                <td className="py-2 pr-3">
                                  <Badge variant={m.matchType === "exact" ? "default" : m.matchType === "manual" ? "secondary" : "outline"} className="text-xs">
                                    {m.matchType}
                                  </Badge>
                                </td>
                                <td className="py-2 pr-3 whitespace-nowrap">{tx?.date ?? "-"}</td>
                                <td className="py-2 pr-3 max-w-[150px] truncate">{tx?.description ?? "-"}</td>
                                <td className="py-2 pr-3 text-right tabular-nums">{tx ? formatCurrency(Math.abs(tx.amount)) : "-"}</td>
                                <td className="py-2 pr-3 whitespace-nowrap">{rec?.date ?? "-"}</td>
                                <td className="py-2 pr-3 max-w-[120px] truncate">{rec?.merchant ?? "-"}</td>
                                <td className="py-2 pr-3 text-right tabular-nums">{rec ? formatCurrency(rec.amount) : "-"}</td>
                                <td className="py-2 pr-3 text-xs">
                                  {m.dateDelta}d, RM {m.amountDelta.toFixed(2)}
                                </td>
                                <td className="py-2 pr-3">
                                  <Button type="button" variant="ghost" size="sm" className="h-7 rounded-xl text-xs text-red-600"
                                    onClick={() => handleRemoveMatch(m.bankTxId, m.receiptId)}>
                                    <X className="size-3 mr-1" /> Unmatch
                                  </Button>
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    </div>
                  )}
                </TabsContent>

                {/* Unmatched Bank Tab */}
                <TabsContent value="unmatchedBank" className="mt-4">
                  {report.unmatchedBank.length === 0 ? (
                    <div className="text-center py-8 text-sm text-slate-500">All bank transactions matched.</div>
                  ) : (
                    <div className="overflow-x-auto">
                      <table className="w-full text-sm">
                        <thead>
                          <tr className="border-b text-left text-xs font-medium text-slate-500">
                            <th className="py-2 pr-3">#</th>
                            <th className="py-2 pr-3">Date</th>
                            <th className="py-2 pr-3">Description</th>
                            <th className="py-2 pr-3 text-right">Amount</th>
                            <th className="py-2 pr-3">Category</th>
                            <th className="py-2 pr-3">Manual Match</th>
                          </tr>
                        </thead>
                        <tbody>
                          {report.unmatchedBank.map((tx) => (
                            <tr key={tx.id} className="border-b last:border-0">
                              <td className="py-2 pr-3 text-slate-400">{tx.id}</td>
                              <td className="py-2 pr-3 whitespace-nowrap">{tx.date}</td>
                              <td className="py-2 pr-3 max-w-[150px] truncate">{tx.description}</td>
                              <td className="py-2 pr-3 text-right tabular-nums">{formatCurrency(Math.abs(tx.amount))}</td>
                              <td className="py-2 pr-3 text-xs">{getCategoryLabel(tx.category)}</td>
                              <td className="py-2 pr-3">
                                <select
                                  className="rounded-xl border text-xs h-8 px-2"
                                  defaultValue=""
                                  onChange={(e) => {
                                    const rid = Number(e.target.value);
                                    if (rid) handleManualMatch(tx.id, rid);
                                  }}
                                >
                                  <option value="">-- Match to receipt --</option>
                                  {report.unmatchedReceipts.map((r) => (
                                    <option key={r.id} value={r.id}>
                                      {r.date} {r.merchant} ({formatCurrency(r.amount)})
                                    </option>
                                  ))}
                                </select>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </TabsContent>

                {/* Unmatched Receipts Tab */}
                <TabsContent value="unmatchedReceipts" className="mt-4">
                  {report.unmatchedReceipts.length === 0 ? (
                    <div className="text-center py-8 text-sm text-slate-500">All receipts matched.</div>
                  ) : (
                    <div className="overflow-x-auto">
                      <table className="w-full text-sm">
                        <thead>
                          <tr className="border-b text-left text-xs font-medium text-slate-500">
                            <th className="py-2 pr-3">#</th>
                            <th className="py-2 pr-3">Date</th>
                            <th className="py-2 pr-3">Merchant</th>
                            <th className="py-2 pr-3 text-right">Amount</th>
                            <th className="py-2 pr-3">Payment</th>
                            <th className="py-2 pr-3">Category</th>
                          </tr>
                        </thead>
                        <tbody>
                          {report.unmatchedReceipts.map((r) => (
                            <tr key={r.id} className="border-b last:border-0">
                              <td className="py-2 pr-3 text-slate-400">{r.id}</td>
                              <td className="py-2 pr-3 whitespace-nowrap">{r.date}</td>
                              <td className="py-2 pr-3 max-w-[150px] truncate">{r.merchant}</td>
                              <td className="py-2 pr-3 text-right tabular-nums">{formatCurrency(r.amount)}</td>
                              <td className="py-2 pr-3 text-xs">{r.paymentMethod}</td>
                              <td className="py-2 pr-3 text-xs">{getCategoryLabel(r.category)}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </TabsContent>
              </Tabs>
            </CardContent>
          </Card>

          {/* Export */}
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Export Report</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportCsv} disabled={report.matched.length === 0}>
                  <Table className="mr-2 size-4" /> Matched CSV
                </Button>
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportUnmatchedCsv} disabled={report.unmatchedBank.length === 0 && report.unmatchedReceipts.length === 0}>
                  <FileText className="mr-2 size-4" /> Unmatched CSV
                </Button>
                <Button type="button" variant="secondary" className="rounded-2xl" onClick={handleExportJson}>
                  <FileText className="mr-2 size-4" /> Report JSON
                </Button>
                <Button type="button" className="rounded-2xl" onClick={handleExportPdf} disabled={pdfGenerating}>
                  {pdfGenerating ? (
                    <Loader2 className="mr-2 size-4 animate-spin" />
                  ) : (
                    <Download className="mr-2 size-4" />
                  )}
                  Summary PDF
                </Button>
              </div>
            </CardContent>
          </Card>
        </>
      ) : null}
    </div>
  );
}
