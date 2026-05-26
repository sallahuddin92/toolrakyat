import { getCurrentUser } from "@/lib/auth/dal";
import { createDbTransactionService } from "@/lib/akaunkemas-saas/services/transactions-db";
import { createDbReceiptService } from "@/lib/akaunkemas-saas/services/receipts-db";
import { createMatchRepository } from "@/lib/akaunkemas-saas/services/receipt-matches-db";
import { computeMonthlySummary } from "@/lib/akaunkemas/summary";
import { CATEGORIES } from "@/lib/akaunkemas/categories";
import type { CategorySlug } from "@/lib/akaunkemas/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export const dynamic = "force-dynamic";

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

function getCategoryLabel(slug: string): string {
  return CATEGORIES.find((c) => c.slug === slug)?.label ?? slug;
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default async function MonthlyReviewPage() {
  const session = await getCurrentUser();
  const { tenantId, businessId } = session;

  const txService = createDbTransactionService();
  const receiptService = createDbReceiptService();
  const matchRepo = createMatchRepository();

  // Use current month as default
  const now = new Date();
  const monthStart = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-01`;
  const monthEnd = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate()}`;

  // Load data for current month
  const transactions = txService.list(tenantId, businessId, {
    dateFrom: monthStart,
    dateTo: monthEnd,
  });
  const receipts = receiptService.list(tenantId, businessId, {
    dateFrom: monthStart,
    dateTo: monthEnd,
  });

  // Map DB transactions to matcher-compatible type for summary
  const summaryTx = transactions.map((tx) => ({
    id: 0,
    date: tx.date,
    description: tx.description,
    debit: tx.debit,
    credit: tx.credit,
    amount: tx.amount,
    balance: tx.balance,
    category: tx.categorySlug as CategorySlug,
  }));

  const summary = computeMonthlySummary(summaryTx);
  const { unmatchedTransactions, unmatchedReceipts } = matchRepo.getUnmatchedCounts(tenantId, businessId);

  // Receipt summary by category
  const receiptCategoryMap = new Map<string, { total: number; count: number }>();
  for (const r of receipts) {
    const existing = receiptCategoryMap.get(r.categorySlug);
    if (existing) {
      existing.total += r.amount;
      existing.count++;
    } else {
      receiptCategoryMap.set(r.categorySlug, { total: r.amount, count: 1 });
    }
  }

  const receiptCategories = Array.from(receiptCategoryMap.entries())
    .map(([slug, data]) => ({ slug, total: data.total, count: data.count }))
    .sort((a, b) => b.total - a.total);

  const monthLabel = now.toLocaleDateString("en-MY", { year: "numeric", month: "long" });

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Monthly Review</h1>
        <p className="text-sm text-slate-500">
          {monthLabel} — Financial summary and reconciliation status.
        </p>
      </div>

      {/* Summary cards */}
      <div className="grid gap-4 sm:grid-cols-3">
        <Card className="rounded-2xl border-emerald-200" style={{ borderLeftWidth: "3px" }}>
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Total Income</p>
            <p className="text-2xl font-bold text-emerald-700">{formatCurrency(summary.totalIncome)}</p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-red-200" style={{ borderLeftWidth: "3px" }}>
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Total Expense</p>
            <p className="text-2xl font-bold text-red-700">{formatCurrency(summary.totalExpense)}</p>
          </CardContent>
        </Card>
        <Card
          className={cn(
            "rounded-2xl border",
            summary.netCashflow >= 0 ? "border-emerald-200" : "border-red-200",
          )}
          style={{ borderLeftWidth: "3px" }}
        >
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Net Cashflow</p>
            <p
              className={cn(
                "text-2xl font-bold",
                summary.netCashflow >= 0 ? "text-emerald-700" : "text-red-700",
              )}
            >
              {formatCurrency(summary.netCashflow)}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Counts summary */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Card className="rounded-2xl">
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Transactions</p>
            <p className="text-xl font-bold text-slate-900">{summary.transactionCount}</p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl">
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Receipts</p>
            <p className="text-xl font-bold text-slate-900">{receipts.length}</p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl">
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Unmatched</p>
            <p className="text-xl font-bold text-slate-900">
              {unmatchedTransactions + unmatchedReceipts}
            </p>
            <p className="text-[10px] text-slate-400">
              {unmatchedTransactions} tx / {unmatchedReceipts} receipts
            </p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl">
          <CardContent className="pt-4">
            <p className="text-xs text-slate-500">Uncategorised</p>
            <p className="text-xl font-bold text-slate-900">
              {txService.countUncategorised(tenantId, businessId)}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Category breakdowns */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Transaction categories */}
        <Card className="rounded-2xl overflow-hidden">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Transaction by Category</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            {summary.categorySummaries.length === 0 ? (
              <div className="px-4 py-8 text-center text-sm text-slate-400">
                No transactions this month.
              </div>
            ) : (
              <div className="divide-y divide-slate-100">
                {summary.categorySummaries.map((cs) => (
                  <div key={cs.category} className="flex items-center justify-between px-4 py-2 text-sm">
                    <div>
                      <span className="text-slate-900">{getCategoryLabel(cs.category)}</span>
                      <span className="ml-2 text-xs text-slate-400">{cs.count}</span>
                    </div>
                    <span
                      className={cn(
                        "text-xs tabular-nums font-medium",
                        cs.total >= 0 ? "text-green-700" : "text-red-600",
                      )}
                    >
                      {formatCurrency(cs.total)}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Receipt categories */}
        <Card className="rounded-2xl overflow-hidden">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Receipts by Category</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            {receiptCategories.length === 0 ? (
              <div className="px-4 py-8 text-center text-sm text-slate-400">
                No receipts this month.
              </div>
            ) : (
              <div className="divide-y divide-slate-100">
                {receiptCategories.map((rc) => (
                  <div key={rc.slug} className="flex items-center justify-between px-4 py-2 text-sm">
                    <div>
                      <span className="text-slate-900">{getCategoryLabel(rc.slug)}</span>
                      <span className="ml-2 text-xs text-slate-400">{rc.count}</span>
                    </div>
                    <span className="text-xs tabular-nums font-medium text-slate-600">
                      {formatCurrency(rc.total)}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
