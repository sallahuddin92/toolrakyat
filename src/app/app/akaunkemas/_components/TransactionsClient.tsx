"use client";

import { useState, useCallback, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ArrowLeftRight, Upload, Pencil, Trash2, X, Check, Search } from "lucide-react";
import Link from "next/link";
import {
  updateTransaction,
  deleteTransaction,
} from "../transactions/actions";
import { CATEGORIES } from "@/lib/akaunkemas/categories";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface TransactionRow {
  id: string;
  date: string;
  description: string;
  debit: number;
  credit: number;
  amount: number;
  balance: number | null;
  categorySlug: string;
  isReconciled: boolean;
  notes: string;
  source: string;
  status: "draft" | "reviewed" | "locked";
  importHash?: string | null;
  createdAt: Date;
  updatedAt: Date;
}

interface Props {
  initialTransactions: TransactionRow[];
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

const STATUS_STYLES: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800",
  reviewed: "bg-green-100 text-green-800",
  locked: "bg-slate-200 text-slate-600",
};

// ---------------------------------------------------------------------------
// Inline Edit Row
// ---------------------------------------------------------------------------

function InlineEditRow({
  tx,
  onSave,
  onCancel,
}: {
  tx: TransactionRow;
  onSave: (id: string, data: Record<string, unknown>) => void;
  onCancel: () => void;
}) {
  const [description, setDescription] = useState(tx.description);
  const [categorySlug, setCategorySlug] = useState(tx.categorySlug);
  const [notes] = useState(tx.notes);
  const [status, setStatus] = useState<string>(tx.status);
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    await onSave(tx.id, { description, categorySlug, notes, status });
    setSaving(false);
  };

  return (
    <div className="grid grid-cols-12 gap-2 border-b border-slate-100 bg-blue-50/50 px-4 py-2 text-sm">
      <div className="col-span-1 text-xs text-slate-500">{formatDate(tx.date)}</div>
      <div className="col-span-3">
        <Input
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          className="h-8 text-xs"
        />
      </div>
      <div className="col-span-1 text-right text-xs tabular-nums">
        {tx.debit > 0 ? formatCurrency(tx.debit) : ""}
      </div>
      <div className="col-span-1 text-right text-xs tabular-nums">
        {tx.credit > 0 ? formatCurrency(tx.credit) : ""}
      </div>
      <div className="col-span-1 text-right text-xs tabular-nums font-medium">
        {formatCurrency(tx.amount)}
      </div>
      <div className="col-span-2">
        <Select value={categorySlug} onValueChange={setCategorySlug}>
          <SelectTrigger className="h-8 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CATEGORIES.map((c) => (
              <SelectItem key={c.slug} value={c.slug}>
                {c.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="col-span-1">
        <Select value={status} onValueChange={setStatus}>
          <SelectTrigger className="h-8 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="draft">Draft</SelectItem>
            <SelectItem value="reviewed">Reviewed</SelectItem>
            <SelectItem value="locked">Locked</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="col-span-2 flex items-center gap-1">
        <Button
          size="icon"
          variant="ghost"
          className="size-7"
          onClick={handleSave}
          disabled={saving}
        >
          <Check className="size-3 text-green-600" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="size-7"
          onClick={onCancel}
        >
          <X className="size-3 text-red-500" />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export function TransactionsClient({ initialTransactions }: Props) {
  const [transactions, setTransactions] = useState<TransactionRow[]>(initialTransactions);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [sourceFilter, setSourceFilter] = useState("all");
  const [deletingId, setDeletingId] = useState<string | null>(null);

  // Filters
  const filtered = transactions.filter((tx) => {
    if (search) {
      const s = search.toLowerCase();
      if (!tx.description.toLowerCase().includes(s)) return false;
    }
    if (categoryFilter !== "all" && tx.categorySlug !== categoryFilter) return false;
    if (statusFilter !== "all" && tx.status !== statusFilter) return false;
    if (sourceFilter !== "all" && tx.source !== sourceFilter) return false;
    return true;
  });

  // Inline save
  const handleSave = useCallback(async (id: string, data: Record<string, unknown>) => {
    const formData = new FormData();
    formData.set("id", id);
    for (const [key, value] of Object.entries(data)) {
      formData.set(key, String(value));
    }
    const result = await updateTransaction(formData);
    if (result.success) {
      setTransactions((prev) =>
        prev.map((tx) =>
          tx.id === id ? { ...tx, ...data } : tx,
        ),
      );
      setEditingId(null);
    }
  }, []);

  // Delete
  const handleDelete = useCallback(async (id: string) => {
    if (!confirm("Delete this transaction?")) return;
    setDeletingId(id);
    const formData = new FormData();
    formData.set("id", id);
    const result = await deleteTransaction(formData);
    if (result.success) {
      setTransactions((prev) => prev.filter((tx) => tx.id !== id));
    }
    setDeletingId(null);
  }, []);

  // Click outside to cancel editing
  useEffect(() => {
    if (!editingId) return;
    const handleClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest("[data-editing-row]")) {
        setEditingId(null);
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [editingId]);

  const getCategoryLabel = (slug: string) =>
    CATEGORIES.find((c) => c.slug === slug)?.label ?? slug;

  if (transactions.length === 0) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">Transactions</h1>
            <p className="text-sm text-slate-500">
              Import, categorise, and manage your bank transactions.
            </p>
          </div>
          <Link href="/app/akaunkemas/import-bank-csv">
            <Button className="gap-2">
              <Upload className="size-4" />
              Import CSV
            </Button>
          </Link>
        </div>

        <Card className="rounded-2xl">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
              <ArrowLeftRight className="size-8 text-slate-400" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">No transactions yet</p>
              <p className="text-xs text-slate-500">
                Upload a bank CSV to get started.
              </p>
            </div>
            <Link href="/app/akaunkemas/import-bank-csv">
              <Button className="gap-2">
                <Upload className="size-4" />
                Import CSV
              </Button>
            </Link>
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
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Transactions</h1>
          <p className="text-sm text-slate-500">
            {transactions.length} transaction{transactions.length !== 1 ? "s" : ""}
          </p>
        </div>
        <Link href="/app/akaunkemas/import-bank-csv">
          <Button className="gap-2">
            <Upload className="size-4" />
            Import CSV
          </Button>
        </Link>
      </div>

      {/* Filters */}
      <Card className="rounded-2xl">
        <CardContent className="flex flex-wrap items-center gap-3 px-4 py-3">
          <div className="relative flex-1 min-w-[200px]">
            <Search className="absolute left-2.5 top-2 size-4 text-slate-400" />
            <Input
              placeholder="Search descriptions..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 h-9 text-sm"
            />
          </div>
          <Select value={categoryFilter} onValueChange={setCategoryFilter}>
            <SelectTrigger className="h-9 w-[160px] text-sm">
              <SelectValue placeholder="Category" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Categories</SelectItem>
              {CATEGORIES.map((c) => (
                <SelectItem key={c.slug} value={c.slug}>
                  {c.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger className="h-9 w-[130px] text-sm">
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Status</SelectItem>
              <SelectItem value="draft">Draft</SelectItem>
              <SelectItem value="reviewed">Reviewed</SelectItem>
              <SelectItem value="locked">Locked</SelectItem>
            </SelectContent>
          </Select>
          <Select value={sourceFilter} onValueChange={setSourceFilter}>
            <SelectTrigger className="h-9 w-[140px] text-sm">
              <SelectValue placeholder="Source" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Sources</SelectItem>
              <SelectItem value="csv_import">CSV Import</SelectItem>
              <SelectItem value="manual">Manual</SelectItem>
              <SelectItem value="receipt_match">Receipt Match</SelectItem>
            </SelectContent>
          </Select>
        </CardContent>
      </Card>

      {/* Table */}
      <Card className="rounded-2xl overflow-hidden">
        {/* Header row */}
        <div className="grid grid-cols-12 gap-2 border-b border-slate-200 bg-slate-50 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
          <div className="col-span-1">Date</div>
          <div className="col-span-3">Description</div>
          <div className="col-span-1 text-right">Debit</div>
          <div className="col-span-1 text-right">Credit</div>
          <div className="col-span-1 text-right">Amount</div>
          <div className="col-span-2">Category</div>
          <div className="col-span-1">Status</div>
          <div className="col-span-2 text-right">Actions</div>
        </div>

        {/* Rows */}
        <div className="divide-y divide-slate-100">
          {filtered.map((tx) =>
            editingId === tx.id ? (
              <div key={tx.id} data-editing-row>
                <InlineEditRow
                  tx={tx}
                  onSave={handleSave}
                  onCancel={() => setEditingId(null)}
                />
              </div>
            ) : (
              <div
                key={tx.id}
                className={cn(
                  "grid grid-cols-12 gap-2 px-4 py-2.5 text-sm transition-colors hover:bg-slate-50",
                  tx.isReconciled && "opacity-70",
                )}
              >
                <div className="col-span-1 text-xs text-slate-500 self-center">
                  {formatDate(tx.date)}
                </div>
                <div className="col-span-3 self-center truncate">
                  <span className="text-slate-900">{tx.description}</span>
                  {tx.notes && (
                    <span className="ml-1 text-xs text-slate-400">— {tx.notes.slice(0, 40)}</span>
                  )}
                </div>
                <div className="col-span-1 self-center text-right text-xs tabular-nums text-red-600">
                  {tx.debit > 0 ? formatCurrency(tx.debit) : ""}
                </div>
                <div className="col-span-1 self-center text-right text-xs tabular-nums text-green-600">
                  {tx.credit > 0 ? formatCurrency(tx.credit) : ""}
                </div>
                <div
                  className={cn(
                    "col-span-1 self-center text-right text-xs tabular-nums font-medium",
                    tx.amount < 0 ? "text-red-600" : "text-green-700",
                  )}
                >
                  {formatCurrency(tx.amount)}
                </div>
                <div className="col-span-2 self-center">
                  <span className="text-xs text-slate-600">{getCategoryLabel(tx.categorySlug)}</span>
                </div>
                <div className="col-span-1 self-center">
                  <Badge
                    variant="secondary"
                    className={cn("text-[10px] h-5 px-1.5", STATUS_STYLES[tx.status])}
                  >
                    {tx.status}
                  </Badge>
                </div>
                <div className="col-span-2 self-center flex items-center justify-end gap-1">
                  {tx.status !== "locked" && (
                    <>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="size-7"
                        onClick={() => setEditingId(tx.id)}
                        title="Edit"
                      >
                        <Pencil className="size-3 text-slate-400" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="size-7"
                        onClick={() => handleDelete(tx.id)}
                        disabled={deletingId === tx.id}
                        title="Delete"
                      >
                        <Trash2 className="size-3 text-red-400" />
                      </Button>
                    </>
                  )}
                </div>
              </div>
            ),
          )}
        </div>

        {filtered.length === 0 && (
          <div className="px-4 py-8 text-center text-sm text-slate-400">
            No transactions match the current filters.
          </div>
        )}
      </Card>
    </div>
  );
}
