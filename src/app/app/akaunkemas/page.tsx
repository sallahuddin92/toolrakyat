import Link from "next/link";
import {
  ArrowLeftRight,
  Receipt,
  GitMerge,
  Tag,
  Package,
  Upload,
  CheckCircle2,
  Circle,
  TrendingUp,
  DollarSign,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { getCurrentUser } from "@/lib/auth/dal";
import { createDbTransactionService } from "@/lib/akaunkemas-saas/services/transactions-db";
import { createDbReceiptService } from "@/lib/akaunkemas-saas/services/receipts-db";
import { createMatchRepository } from "@/lib/akaunkemas-saas/services/receipt-matches-db";
import { createPackService } from "@/lib/akaunkemas-saas/services/accountant-packs-db";

export const dynamic = "force-dynamic";

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

// ---------------------------------------------------------------------------
// Stat cards
// ---------------------------------------------------------------------------

interface Stat {
  label: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  bg: string;
  border: string;
}

function StatCard({ stat }: { stat: Stat }) {
  return (
    <Card
      className={cn(
        "rounded-2xl border transition-shadow duration-150 hover:shadow-md",
        stat.border,
      )}
      style={{ borderLeftWidth: "3px" }}
    >
      <CardContent className="flex items-center gap-4 pt-4">
        <div
          className={cn(
            "flex size-10 shrink-0 items-center justify-center rounded-xl",
            stat.bg,
          )}
        >
          <stat.icon className={cn("size-5", stat.color)} />
        </div>
        <div className="min-w-0">
          <p className="truncate text-xs text-slate-500">{stat.label}</p>
          <p className="text-2xl font-bold text-slate-900">{stat.value}</p>
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Quick action cards
// ---------------------------------------------------------------------------

interface QuickAction {
  label: string;
  description: string;
  href: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  bg: string;
}

const quickActions: QuickAction[] = [
  {
    label: "Import Transactions",
    description: "Upload a bank CSV",
    href: "/app/akaunkemas/transactions",
    icon: Upload,
    color: "text-sky-600",
    bg: "bg-sky-50",
  },
  {
    label: "Add Receipt",
    description: "Manually add or import",
    href: "/app/akaunkemas/receipts",
    icon: Receipt,
    color: "text-amber-600",
    bg: "bg-amber-50",
  },
  {
    label: "Run Matching",
    description: "Match receipts to transactions",
    href: "/app/akaunkemas/matching",
    icon: GitMerge,
    color: "text-violet-600",
    bg: "bg-violet-50",
  },
  {
    label: "Generate Pack",
    description: "Create accountant pack",
    href: "/app/akaunkemas/accountant-packs",
    icon: Package,
    color: "text-sky-600",
    bg: "bg-sky-50",
  },
];

// ---------------------------------------------------------------------------
// Dashboard page
// ---------------------------------------------------------------------------

export default async function DashboardPage() {
  const session = await getCurrentUser();
  const { tenantId, businessId } = session;

  const txService = createDbTransactionService();
  const receiptService = createDbReceiptService();
  const matchRepo = createMatchRepository();
  const packService = createPackService();

  // Get current month boundaries
  const now = new Date();
  const monthStart = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-01`;
  const monthEnd = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate()}`;

  // Real counts
  const txCount = txService.count(tenantId, businessId);
  const receiptCount = receiptService.count(tenantId, businessId);
  const uncategorisedCount = txService.countUncategorised(tenantId, businessId);
  const { unmatchedTransactions, unmatchedReceipts } = matchRepo.getUnmatchedCounts(tenantId, businessId);
  const unmatchedTotal = unmatchedTransactions + unmatchedReceipts;

  // Net cashflow this month
  const monthTransactions = txService.list(tenantId, businessId, {
    dateFrom: monthStart,
    dateTo: monthEnd,
  });
  const netCashflow = monthTransactions.reduce((sum, tx) => sum + tx.amount, 0);

  // Latest pack
  const packs = packService.list(tenantId, businessId);
  const latestPack = packs.length > 0 ? packs[0] : null;

  // Checklist items
  const hasTransactions = txCount > 0;
  const hasReceipts = receiptCount > 0;
  const hasMatches = matchRepo.getMatchRows(tenantId, businessId).length > 0;
  const hasPacks = packs.length > 0;

  const checklistItems = [
    { label: "Upload your first bank CSV", href: "/app/akaunkemas/transactions", done: hasTransactions },
    { label: "Add receipts", href: "/app/akaunkemas/receipts", done: hasReceipts },
    { label: "Run receipt matching", href: "/app/akaunkemas/matching", done: hasMatches },
    { label: "Generate accountant pack", href: "/app/akaunkemas/accountant-packs", done: hasPacks },
  ];

  const allDone = checklistItems.every((item) => item.done);

  const stats: Stat[] = [
    {
      label: "Transactions",
      value: String(txCount),
      icon: ArrowLeftRight,
      color: "text-sky-600",
      bg: "bg-sky-50",
      border: "border-sky-200",
    },
    {
      label: "Receipts",
      value: String(receiptCount),
      icon: Receipt,
      color: "text-amber-600",
      bg: "bg-amber-50",
      border: "border-amber-200",
    },
    {
      label: "Unmatched",
      value: String(unmatchedTotal),
      icon: GitMerge,
      color: "text-violet-600",
      bg: "bg-violet-50",
      border: "border-violet-200",
    },
    {
      label: "Uncategorised",
      value: String(uncategorisedCount),
      icon: Tag,
      color: "text-rose-600",
      bg: "bg-rose-50",
      border: "border-rose-200",
    },
  ];

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      {/* Page header */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">
          Welcome back
        </h1>
        <p className="mt-1 text-sm text-slate-500">
          Overview of your AkaunKemas account.
        </p>
      </div>

      {/* Quick Stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat) => (
          <StatCard key={stat.label} stat={stat} />
        ))}
      </div>

      {/* Net Cashflow */}
      <Card className="rounded-2xl border-emerald-200" style={{ borderLeftWidth: "3px" }}>
        <CardContent className="flex items-center gap-4 pt-4">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-emerald-50">
            <DollarSign className="size-5 text-emerald-600" />
          </div>
          <div>
            <p className="text-xs text-slate-500">Net Cashflow (This Month)</p>
            <p className={cn(
              "text-2xl font-bold",
              netCashflow >= 0 ? "text-emerald-700" : "text-red-700",
            )}>
              {formatCurrency(netCashflow)}
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Getting Started */}
      <Card className={cn(
        "rounded-2xl border transition-colors",
        allDone ? "border-emerald-200 bg-emerald-50/50" : "border-sky-200 bg-sky-50/50",
      )}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <TrendingUp className={cn("size-4", allDone ? "text-emerald-600" : "text-sky-600")} />
            Getting Started
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-slate-500">
            {allDone
              ? "All steps completed! Your account is set up."
              : "Complete these steps to set up your AkaunKemas account."}
          </p>
          <ul className="space-y-2">
            {checklistItems.map((item) => (
              <li key={item.label}>
                <Link
                  href={item.href}
                  className="flex items-center gap-3 rounded-xl px-3 py-2 text-sm text-slate-600 transition-colors duration-150 hover:bg-sky-100/50 hover:text-sky-700"
                >
                  {item.done ? (
                    <CheckCircle2 className="size-4 shrink-0 text-emerald-500" />
                  ) : (
                    <Circle className="size-4 shrink-0 text-slate-300" />
                  )}
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>

      {/* Latest Accountant Pack */}
      <Card className={cn(
        "rounded-2xl border",
        latestPack ? "border-sky-200 bg-sky-50/50" : "border-sky-200 bg-sky-50/50",
      )}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <CheckCircle2 className="size-4 text-sky-600" />
            Latest Accountant Pack
          </CardTitle>
        </CardHeader>
        <CardContent>
          {latestPack ? (
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-slate-900">{latestPack.label}</p>
                <p className="text-xs text-slate-500">
                  {formatDate(latestPack.periodStart)} — {formatDate(latestPack.periodEnd)}
                  {latestPack.generatedAt && ` | Generated ${formatDate(latestPack.generatedAt)}`}
                </p>
              </div>
              <Link href="/app/akaunkemas/accountant-packs">
                <span className="text-sm text-sky-600 hover:text-sky-700 font-medium cursor-pointer">
                  View All
                </span>
              </Link>
            </div>
          ) : (
            <p className="text-sm text-slate-500">
              No packs yet. Generate your first pack from the Accountant Packs
              section.
            </p>
          )}
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <div>
        <h2 className="mb-3 text-base font-semibold text-slate-900">
          Quick Actions
</h2>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {quickActions.map((action) => (
            <Link key={action.label} href={action.href} className="group">
              <Card className="rounded-2xl transition-all duration-150 hover:shadow-md hover:-translate-y-0.5">
                <CardContent className="flex items-center gap-4 pt-4">
                  <div
                    className={cn(
                      "flex size-10 shrink-0 items-center justify-center rounded-xl transition-colors duration-150",
                      action.bg,
                    )}
                  >
                    <action.icon className={cn("size-5", action.color)} />
                  </div>
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-slate-900 group-hover:text-sky-600 transition-colors duration-150">
                      {action.label}
                    </p>
                    <p className="truncate text-xs text-slate-500">
                      {action.description}
                    </p>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
