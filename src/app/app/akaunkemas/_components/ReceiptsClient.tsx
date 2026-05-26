"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Receipt, Plus, Pencil, Trash2, X, Search, Download, Sparkles, ChevronDown, ChevronUp, AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  createReceipt,
  updateReceipt,
  deleteReceipt,
} from "../receipts/actions";
import { suggestCategory } from "@/lib/akaunkemas-saas/category-suggestions";
import { CATEGORIES, getCategoryLabel } from "@/lib/akaunkemas/categories";
import type { CategorySlug } from "@/lib/akaunkemas/types";
import type { CategorySuggestion } from "@/lib/akaunkemas-saas/category-suggestions";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ReceiptRow {
  id: string;
  date: string;
  merchant: string;
  amount: number;
  paymentMethod: string;
  categorySlug: string;
  taxAmount: number;
  serviceCharge: number;
  notes: string;
  imageRef: string | null;
  status: "draft" | "reviewed";
  createdAt: Date;
  updatedAt: Date;
}

interface Props {
  initialReceipts: ReceiptRow[];
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

const PAYMENT_METHODS = [
  { value: "cash", label: "Cash" },
  { value: "card", label: "Card" },
  { value: "bank_transfer", label: "Bank Transfer" },
  { value: "e_wallet", label: "E-Wallet" },
  { value: "cheque", label: "Cheque" },
  { value: "other", label: "Other" },
];

const STATUS_STYLES: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800",
  reviewed: "bg-green-100 text-green-800",
};

// ---------------------------------------------------------------------------
// Modal Component
// ---------------------------------------------------------------------------

function ReceiptModal({
  open,
  onClose,
  onSave,
  initial,
  existingReceipts,
}: {
  open: boolean;
  onClose: () => void;
  onSave: (data: Record<string, string>) => Promise<void>;
  initial?: ReceiptRow;
  existingReceipts: ReceiptRow[];
}) {
  const [saving, setSaving] = useState(false);
  const [showMore, setShowMore] = useState(false);
  const [merchant, setMerchant] = useState(initial?.merchant ?? "");
  const [amount, setAmount] = useState(initial?.amount?.toString() ?? "");
  const [date, setDate] = useState(initial?.date ?? "");
  const [categorySuggestion, setCategorySuggestion] = useState<CategorySuggestion | null>(null);
  const [selectedCategory, setSelectedCategory] = useState(initial?.categorySlug ?? "uncategorised");
  const [duplicateWarning, setDuplicateWarning] = useState<string | null>(null);
  const isEdit = !!initial;

  // Auto-suggest category when merchant changes
  function handleMerchantChange(value: string) {
    setMerchant(value);
    if (value.trim().length >= 2) {
      const suggestion = suggestCategory(value);
      setCategorySuggestion(suggestion);
      if (suggestion.confidence === "high" && !initial) {
        setSelectedCategory(suggestion.suggestedCategorySlug);
      }
    } else {
      setCategorySuggestion(null);
    }
  }

  // Check for duplicates when merchant, amount, or date changes
  function checkDuplicate(m: string, a: string, d: string) {
    setDuplicateWarning(null);
    if (!m || !a || !d) return;

    const amt = parseFloat(a);
    if (isNaN(amt)) return;

    const month = d.slice(0, 7); // "2026-05"
    const similar = existingReceipts.filter((r) => {
      if (initial && r.id === initial.id) return false;
      const rMonth = r.date.slice(0, 7);
      const merchantMatch = r.merchant.toLowerCase() === m.toLowerCase();
      const amountClose = Math.abs(r.amount - amt) < 0.02;
      return merchantMatch && amountClose && rMonth === month;
    });

    if (similar.length > 0) {
      setDuplicateWarning(
        `A receipt from "${similar[0]!.merchant}" for ${formatCurrency(similar[0]!.amount)} already exists this month.`,
      );
    }
  }

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setSaving(true);
    const form = new FormData(e.currentTarget);
    const data: Record<string, string> = {};
    form.forEach((v, k) => {
      data[k] = v as string;
    });
    // Ensure categorySlug override is included
    data.categorySlug = selectedCategory;
    await onSave(data);
    setSaving(false);
    onClose();
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div className="w-full max-w-lg rounded-2xl bg-white p-6 shadow-xl max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-slate-900">
            {isEdit ? "Edit Receipt" : "Add Receipt"}
          </h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="size-4" />
          </Button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {isEdit && <input type="hidden" name="id" value={initial.id} />}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label className="text-xs">Date</Label>
              <Input
                name="date"
                type="date"
                value={date}
                onChange={(e) => {
                  setDate(e.target.value);
                  checkDuplicate(merchant, amount, e.target.value);
                }}
                required
                className="h-9 text-sm"
              />
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs">Amount (RM)</Label>
              <Input
                name="amount"
                type="number"
                step="0.01"
                min="0.01"
                value={amount}
                onChange={(e) => {
                  setAmount(e.target.value);
                  checkDuplicate(merchant, e.target.value, date);
                }}
                required
                className="h-9 text-sm"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label className="text-xs">Merchant</Label>
            <Input
              name="merchant"
              value={merchant}
              onChange={(e) => {
                handleMerchantChange(e.target.value);
                checkDuplicate(e.target.value, amount, date);
              }}
              required
              className="h-9 text-sm"
              placeholder="e.g. Petronas, Grab, Tesco"
            />
            {/* Category suggestion indicator */}
            {categorySuggestion && (
              <div className="flex items-center gap-1.5 mt-1">
                <Sparkles className="size-3 text-sky-500" />
                <span className="text-xs text-slate-500">Suggested:</span>
                <span className="text-xs font-medium text-sky-700">
                  {getCategoryLabel(categorySuggestion.suggestedCategorySlug)}
                </span>
                <Badge
                  variant="outline"
                  className={cn(
                    "text-[10px] font-normal",
                    categorySuggestion.confidence === "high"
                      ? "bg-green-50 text-green-700 border-green-200"
                      : "bg-yellow-50 text-yellow-700 border-yellow-200",
                  )}
                >
                  {categorySuggestion.confidence}
                </Badge>
              </div>
            )}
          </div>

          {/* Duplicate warning */}
          {duplicateWarning && (
            <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800">
              <AlertCircle className="size-3.5 mt-0.5 shrink-0" />
              {duplicateWarning}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label className="text-xs">Payment Method</Label>
              <Select name="paymentMethod" defaultValue={initial?.paymentMethod ?? "cash"}>
                <SelectTrigger className="h-9 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PAYMENT_METHODS.map((pm) => (
                    <SelectItem key={pm.value} value={pm.value}>
                      {pm.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs">Category</Label>
              <Select
                value={selectedCategory}
                onValueChange={setSelectedCategory}
              >
                <SelectTrigger className="h-9 text-sm">
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
          </div>

          {/* More details toggle */}
          <div>
            <button
              type="button"
              onClick={() => setShowMore(!showMore)}
              className="flex items-center gap-1.5 text-xs text-slate-500 hover:text-slate-700 transition-colors"
            >
              {showMore ? (
                <ChevronUp className="size-3.5" />
              ) : (
                <ChevronDown className="size-3.5" />
              )}
              More details
            </button>
          </div>

          {showMore && (
            <div className="space-y-4 pt-1">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label className="text-xs">Tax Amount (RM)</Label>
                  <Input
                    name="taxAmount"
                    type="number"
                    step="0.01"
                    min="0"
                    defaultValue={initial?.taxAmount ?? "0"}
                    className="h-9 text-sm"
                  />
                </div>
                <div className="space-y-1.5">
                  <Label className="text-xs">Service Charge (RM)</Label>
                  <Input
                    name="serviceCharge"
                    type="number"
                    step="0.01"
                    min="0"
                    defaultValue={initial?.serviceCharge ?? "0"}
                    className="h-9 text-sm"
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <Label className="text-xs">Notes</Label>
                <Input
                  name="notes"
                  defaultValue={initial?.notes ?? ""}
                  className="h-9 text-sm"
                  placeholder="Optional notes"
                />
              </div>
            </div>
          )}

          {/* Hidden input to carry the category override */}
          <input type="hidden" name="categorySlug" value={selectedCategory} />

          <div className="flex justify-end gap-3 pt-2">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? "Saving..." : isEdit ? "Save Changes" : "Add Receipt"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export function ReceiptsClient({ initialReceipts }: Props) {
  const [receipts] = useState<ReceiptRow[]>(initialReceipts);
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [paymentFilter, setPaymentFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [modalOpen, setModalOpen] = useState(false);
  const [editingReceipt, setEditingReceipt] = useState<ReceiptRow | undefined>();
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const filtered = receipts.filter((r) => {
    if (search) {
      const s = search.toLowerCase();
      if (!r.merchant.toLowerCase().includes(s)) return false;
    }
    if (categoryFilter !== "all" && r.categorySlug !== categoryFilter) return false;
    if (paymentFilter !== "all" && r.paymentMethod !== paymentFilter) return false;
    if (statusFilter !== "all" && r.status !== statusFilter) return false;
    return true;
  });

  const handleCreate = async (data: Record<string, string>) => {
    const formData = new FormData();
    for (const [k, v] of Object.entries(data)) {
      formData.set(k, v);
    }
    const result = await createReceipt(formData);
    if (result.success) {
      // Re-fetch handled by revalidatePath in server action
      window.location.reload();
    }
  };

  const handleUpdate = async (data: Record<string, string>) => {
    const formData = new FormData();
    for (const [k, v] of Object.entries(data)) {
      formData.set(k, v);
    }
    const result = await updateReceipt(formData);
    if (result.success) {
      window.location.reload();
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this receipt?")) return;
    setDeletingId(id);
    const formData = new FormData();
    formData.set("id", id);
    const result = await deleteReceipt(formData);
    if (result.success) {
      window.location.reload();
    }
    setDeletingId(null);
  };

  const handleExportCSV = () => {
    const headers = [
      "Date", "Merchant", "Amount", "Payment Method", "Category",
      "Tax Amount", "Service Charge", "Notes", "Status",
    ];
    const rows = filtered.map((r) => [
      r.date, r.merchant, r.amount, r.paymentMethod, r.categorySlug,
      r.taxAmount, r.serviceCharge, r.notes, r.status,
    ]);
    const csv = [headers, ...rows]
      .map((row) => row.map((v) => `"${String(v).replace(/"/g, '""')}"`).join(","))
      .join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "receipts-export.csv";
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleExportJSON = () => {
    const data = filtered.map((r) => ({
      date: r.date,
      merchant: r.merchant,
      amount: r.amount,
      paymentMethod: r.paymentMethod,
      category: r.categorySlug,
      taxAmount: r.taxAmount,
      serviceCharge: r.serviceCharge,
      notes: r.notes,
      status: r.status,
    }));
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "receipts-export.json";
    a.click();
    URL.revokeObjectURL(url);
  };

  const getPaymentLabel = (slug: string) =>
    PAYMENT_METHODS.find((p) => p.value === slug)?.label ?? slug;

  if (receipts.length === 0) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">Receipts</h1>
            <p className="text-sm text-slate-500">
              Manage and organise receipts for your bookkeeping.
            </p>
          </div>
          <Button
            className="gap-2"
            onClick={() => {
              setEditingReceipt(undefined);
              setModalOpen(true);
            }}
          >
            <Plus className="size-4" />
            Add Receipt
          </Button>
        </div>

        <Card className="rounded-2xl">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
              <Receipt className="size-8 text-slate-400" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">No receipts yet</p>
              <p className="text-xs text-slate-500">
                Add receipts manually or import a CSV.
              </p>
            </div>
            <Button
              className="gap-2"
              onClick={() => {
                setEditingReceipt(undefined);
                setModalOpen(true);
              }}
            >
              <Plus className="size-4" />
              Add Receipt
            </Button>
          </CardContent>
        </Card>

        <ReceiptModal
          open={modalOpen}
          onClose={() => setModalOpen(false)}
          onSave={handleCreate}
          initial={editingReceipt}
          existingReceipts={receipts}
        />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Receipts</h1>
          <p className="text-sm text-slate-500">
            {receipts.length} receipt{receipts.length !== 1 ? "s" : ""}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={handleExportCSV}
          >
            <Download className="size-3.5" />
            CSV
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={handleExportJSON}
          >
            <Download className="size-3.5" />
            JSON
          </Button>
          <Button
            className="gap-2"
            onClick={() => {
              setEditingReceipt(undefined);
              setModalOpen(true);
            }}
          >
            <Plus className="size-4" />
            Add Receipt
          </Button>
        </div>
      </div>

      {/* Filters */}
      <Card className="rounded-2xl">
        <CardContent className="flex flex-wrap items-center gap-3 px-4 py-3">
          <div className="relative flex-1 min-w-[200px]">
            <Search className="absolute left-2.5 top-2 size-4 text-slate-400" />
            <Input
              placeholder="Search merchants..."
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
          <Select value={paymentFilter} onValueChange={setPaymentFilter}>
            <SelectTrigger className="h-9 w-[150px] text-sm">
              <SelectValue placeholder="Payment" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Methods</SelectItem>
              {PAYMENT_METHODS.map((pm) => (
                <SelectItem key={pm.value} value={pm.value}>
                  {pm.label}
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
            </SelectContent>
          </Select>
        </CardContent>
      </Card>

      {/* Table */}
      <Card className="rounded-2xl overflow-hidden">
        <div className="grid grid-cols-12 gap-2 border-b border-slate-200 bg-slate-50 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
          <div className="col-span-1">Date</div>
          <div className="col-span-2">Merchant</div>
          <div className="col-span-1 text-right">Amount</div>
          <div className="col-span-1">Payment</div>
          <div className="col-span-2">Category</div>
          <div className="col-span-1">Status</div>
          <div className="col-span-2">Notes</div>
          <div className="col-span-2 text-right">Actions</div>
        </div>

        <div className="divide-y divide-slate-100">
          {filtered.map((r) => (
            <div
              key={r.id}
              className="grid grid-cols-12 gap-2 px-4 py-2.5 text-sm transition-colors hover:bg-slate-50"
            >
              <div className="col-span-1 self-center text-xs text-slate-500">
                {formatDate(r.date)}
              </div>
              <div className="col-span-2 self-center truncate font-medium text-slate-900">
                {r.merchant}
              </div>
              <div className="col-span-1 self-center text-right text-xs tabular-nums font-medium text-slate-800">
                {formatCurrency(r.amount)}
              </div>
              <div className="col-span-1 self-center">
                <span className="text-xs text-slate-500">{getPaymentLabel(r.paymentMethod)}</span>
              </div>
              <div className="col-span-2 self-center">
                <span className="text-xs text-slate-600">{getCategoryLabel(r.categorySlug as CategorySlug)}</span>
              </div>
              <div className="col-span-1 self-center">
                <Badge
                  variant="secondary"
                  className={cn("text-[10px] h-5 px-1.5", STATUS_STYLES[r.status])}
                >
                  {r.status}
                </Badge>
              </div>
              <div className="col-span-2 self-center text-xs text-slate-400 truncate">
                {r.notes || "—"}
              </div>
              <div className="col-span-2 self-center flex items-center justify-end gap-1">
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-7"
                  onClick={() => {
                    setEditingReceipt(r);
                    setModalOpen(true);
                  }}
                  title="Edit"
                >
                  <Pencil className="size-3 text-slate-400" />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-7"
                  onClick={() => handleDelete(r.id)}
                  disabled={deletingId === r.id}
                  title="Delete"
                >
                  <Trash2 className="size-3 text-red-400" />
                </Button>
              </div>
            </div>
          ))}
        </div>

        {filtered.length === 0 && (
          <div className="px-4 py-8 text-center text-sm text-slate-400">
            No receipts match the current filters.
          </div>
        )}
      </Card>

      {/* Modal */}
      <ReceiptModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        onSave={editingReceipt ? handleUpdate : handleCreate}
        initial={editingReceipt}
        existingReceipts={receipts}
      />
    </div>
  );
}
