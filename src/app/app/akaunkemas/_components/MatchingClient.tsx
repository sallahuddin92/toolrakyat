"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { GitMerge, Link2, Unlink2, Search, Zap } from "lucide-react";
import { runMatching, addManualMatch, removeMatch } from "../matching/actions";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface TxRow {
  id: string;
  date: string;
  description: string;
  debit: number;
  credit: number;
  amount: number;
}

interface ReceiptRow {
  id: string;
  date: string;
  merchant: string;
  amount: number;
}

interface MatchedPair {
  transaction: TxRow;
  receipt: ReceiptRow;
  matchType: "exact" | "fuzzy" | "manual";
}

interface Props {
  initialMatched: MatchedPair[];
  initialUnmatchedTxs: TxRow[];
  initialUnmatchedReceipts: ReceiptRow[];
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

function formatDate(d: string): string {
  return new Date(d).toLocaleDateString("en-MY", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

const MATCH_TYPE_STYLES: Record<string, string> = {
  exact: "bg-green-100 text-green-800",
  fuzzy: "bg-amber-100 text-amber-800",
  manual: "bg-blue-100 text-blue-800",
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function MatchingClient({
  initialMatched,
  initialUnmatchedTxs,
  initialUnmatchedReceipts,
}: Props) {
  const [matched] = useState(initialMatched);
  const [unmatchedTxs] = useState(initialUnmatchedTxs);
  const [unmatchedReceipts] = useState(initialUnmatchedReceipts);
  const [dateWindow, setDateWindow] = useState("3");
  const [running, setRunning] = useState(false);
  const [unmatching, setUnmatching] = useState<string | null>(null);
  const [manualTxId, setManualTxId] = useState("");
  const [manualReceiptId, setManualReceiptId] = useState("");
  const [manualAdding, setManualAdding] = useState(false);
  const [searchMatched, setSearchMatched] = useState("");
  const [searchUnmatched, setSearchUnmatched] = useState("");

  const filteredMatched = matched.filter((m) => {
    if (!searchMatched) return true;
    const s = searchMatched.toLowerCase();
    return (
      m.transaction.description.toLowerCase().includes(s) ||
      m.receipt.merchant.toLowerCase().includes(s)
    );
  });

  const filteredUnmatchedTxs = unmatchedTxs.filter((tx) => {
    if (!searchUnmatched) return true;
    return tx.description.toLowerCase().includes(searchUnmatched.toLowerCase());
  });

  const filteredUnmatchedReceipts = unmatchedReceipts.filter((r) => {
    if (!searchUnmatched) return true;
    return r.merchant.toLowerCase().includes(searchUnmatched.toLowerCase());
  });

  const handleRunMatching = async () => {
    setRunning(true);
    const formData = new FormData();
    formData.set("dateWindowDays", dateWindow);
    await runMatching(formData);
    setRunning(false);
    window.location.reload();
  };

  const handleUnmatch = async (txId: string, receiptId: string) => {
    const key = `${txId}|${receiptId}`;
    setUnmatching(key);
    const formData = new FormData();
    formData.set("transactionId", txId);
    formData.set("receiptId", receiptId);
    await removeMatch(formData);
    setUnmatching(null);
    window.location.reload();
  };

  const handleManualMatch = async () => {
    if (!manualTxId || !manualReceiptId) return;
    setManualAdding(true);
    const formData = new FormData();
    formData.set("transactionId", manualTxId);
    formData.set("receiptId", manualReceiptId);
    await addManualMatch(formData);
    setManualAdding(false);
    setManualTxId("");
    setManualReceiptId("");
    window.location.reload();
  };

  if (matched.length === 0 && unmatchedTxs.length === 0 && unmatchedReceipts.length === 0) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">Receipt Matching</h1>
            <p className="text-sm text-slate-500">
              Match receipts to bank transactions automatically.
            </p>
          </div>
        </div>

        <Card className="rounded-2xl">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
              <GitMerge className="size-8 text-slate-400" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">No data to match</p>
              <p className="text-xs text-slate-500">
                Upload bank transactions and add receipts to run matching.
              </p>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Receipt Matching</h1>
          <p className="text-sm text-slate-500">
            {matched.length} matched, {unmatchedTxs.length} unmatched transactions,{" "}
            {unmatchedReceipts.length} unmatched receipts
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Select value={dateWindow} onValueChange={setDateWindow}>
            <SelectTrigger className="h-9 w-[130px] text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="1">1 day</SelectItem>
              <SelectItem value="3">3 days</SelectItem>
              <SelectItem value="5">5 days</SelectItem>
              <SelectItem value="7">7 days</SelectItem>
            </SelectContent>
          </Select>
          <Button onClick={handleRunMatching} disabled={running} className="gap-2">
            <Zap className="size-4" />
            {running ? "Matching..." : "Run Auto-Matching"}
          </Button>
        </div>
      </div>

      {/* Matched pairs */}
      <Card className="rounded-2xl overflow-hidden">
        <div className="border-b border-slate-200 bg-slate-50 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
          Matched Pairs ({filteredMatched.length})
        </div>
        {matched.length > 0 && (
          <div className="px-4 py-2">
            <div className="relative">
              <Search className="absolute left-2.5 top-2 size-4 text-slate-400" />
              <input
                placeholder="Search matched pairs..."
                value={searchMatched}
                onChange={(e) => setSearchMatched(e.target.value)}
                className="pl-8 h-8 text-sm w-full border rounded-md border-slate-200"
              />
            </div>
          </div>
        )}
        {filteredMatched.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-slate-400">
            No matches yet. Run auto-matching or add a manual match below.
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {filteredMatched.map(({ transaction, receipt, matchType }) => (
              <div
                key={`${transaction.id}-${receipt.id}`}
                className="grid grid-cols-12 gap-2 px-4 py-2.5 text-sm hover:bg-slate-50"
              >
                <div className="col-span-2 self-center text-xs text-slate-500">
                  {formatDate(transaction.date)}
                </div>
                <div className="col-span-3 self-center truncate">
                  <span className="font-medium text-slate-900">{transaction.description}</span>
                </div>
                <div className="col-span-1 self-center text-right text-xs tabular-nums">
                  <span className={transaction.amount < 0 ? "text-red-600" : "text-green-700"}>
                    {formatCurrency(transaction.amount)}
                  </span>
                </div>
                <div className="col-span-1 self-center flex justify-center">
                  <Link2 className="size-3.5 text-slate-300" />
                </div>
                <div className="col-span-2 self-center truncate">
                  <span className="font-medium text-slate-900">{receipt.merchant}</span>
                </div>
                <div className="col-span-1 self-center text-right text-xs tabular-nums text-slate-600">
                  {formatCurrency(receipt.amount)}
                </div>
                <div className="col-span-1 self-center">
                  <Badge
                    variant="secondary"
                    className={cn("text-[10px] h-5 px-1.5", MATCH_TYPE_STYLES[matchType])}
                  >
                    {matchType}
                  </Badge>
                </div>
                <div className="col-span-1 self-center text-right">
                  <Button
                    size="icon"
                    variant="ghost"
                    className="size-7"
                    onClick={() => handleUnmatch(transaction.id, receipt.id)}
                    disabled={unmatching === `${transaction.id}|${receipt.id}`}
                    title="Unmatch"
                  >
                    <Unlink2 className="size-3 text-red-400" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Unmatched items */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Unmatched Transactions */}
        <Card className="rounded-2xl overflow-hidden">
          <div className="border-b border-slate-200 bg-slate-50 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
            Unmatched Transactions ({filteredUnmatchedTxs.length})
          </div>
          {unmatchedTxs.length > 0 && (
            <div className="px-4 py-2">
              <div className="relative">
                <Search className="absolute left-2.5 top-2 size-4 text-slate-400" />
                <input
                  placeholder="Search transactions..."
                  value={searchUnmatched}
                  onChange={(e) => setSearchUnmatched(e.target.value)}
                  className="pl-8 h-8 text-sm w-full border rounded-md border-slate-200"
                />
              </div>
            </div>
          )}
          {filteredUnmatchedTxs.length === 0 ? (
            <div className="px-4 py-8 text-center text-sm text-slate-400">
              All transactions are matched.
            </div>
          ) : (
            <div className="divide-y divide-slate-100 max-h-[400px] overflow-y-auto">
              {filteredUnmatchedTxs.map((tx) => (
                <div key={tx.id} className="flex items-center gap-3 px-4 py-2 text-sm hover:bg-slate-50">
                  <div className="w-20 shrink-0 text-xs text-slate-500">{formatDate(tx.date)}</div>
                  <div className="flex-1 min-w-0 truncate text-slate-900">{tx.description}</div>
                  <div className={cn("text-xs tabular-nums shrink-0", tx.amount < 0 ? "text-red-600" : "text-green-700")}>
                    {formatCurrency(tx.amount)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        {/* Unmatched Receipts */}
        <Card className="rounded-2xl overflow-hidden">
          <div className="border-b border-slate-200 bg-slate-50 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
            Unmatched Receipts ({filteredUnmatchedReceipts.length})
          </div>
          {filteredUnmatchedReceipts.length === 0 ? (
            <div className="px-4 py-8 text-center text-sm text-slate-400">
              All receipts are matched.
            </div>
          ) : (
            <div className="divide-y divide-slate-100 max-h-[400px] overflow-y-auto">
              {filteredUnmatchedReceipts.map((r) => (
                <div key={r.id} className="flex items-center gap-3 px-4 py-2 text-sm hover:bg-slate-50">
                  <div className="w-20 shrink-0 text-xs text-slate-500">{formatDate(r.date)}</div>
                  <div className="flex-1 min-w-0 truncate text-slate-900">{r.merchant}</div>
                  <div className="text-xs tabular-nums shrink-0 text-slate-600">{formatCurrency(r.amount)}</div>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      {/* Manual match */}
      {unmatchedTxs.length > 0 && unmatchedReceipts.length > 0 && (
        <Card className="rounded-2xl">
          <CardContent className="flex items-end gap-4 px-4 py-4">
            <div className="flex-1">
              <label className="text-xs font-medium text-slate-500 mb-1 block">Transaction</label>
              <Select value={manualTxId} onValueChange={setManualTxId}>
                <SelectTrigger className="h-9 text-sm">
                  <SelectValue placeholder="Select transaction..." />
                </SelectTrigger>
                <SelectContent>
                  {unmatchedTxs.map((tx) => (
                    <SelectItem key={tx.id} value={tx.id}>
                      {tx.description} ({formatCurrency(tx.amount)})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <Link2 className="size-4 text-slate-300 mb-2" />
            <div className="flex-1">
              <label className="text-xs font-medium text-slate-500 mb-1 block">Receipt</label>
              <Select value={manualReceiptId} onValueChange={setManualReceiptId}>
                <SelectTrigger className="h-9 text-sm">
                  <SelectValue placeholder="Select receipt..." />
                </SelectTrigger>
                <SelectContent>
                  {unmatchedReceipts.map((r) => (
                    <SelectItem key={r.id} value={r.id}>
                      {r.merchant} ({formatCurrency(r.amount)})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <Button
              onClick={handleManualMatch}
              disabled={!manualTxId || !manualReceiptId || manualAdding}
              className="gap-1.5"
            >
              <Link2 className="size-3.5" />
              {manualAdding ? "Matching..." : "Match"}
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
