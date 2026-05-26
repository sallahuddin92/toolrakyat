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
  ArrowRight,
  Sun,
  Moon,
  Sparkles,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
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

/** Time-based greeting for Malaysian hours. */
function getGreeting(): { text: string; Icon: typeof Sun } {
  const hour = new Date().getHours();
  if (hour < 12) return { text: "Selamat Pagi", Icon: Sun };
  if (hour < 17) return { text: "Selamat Petang", Icon: Sun };
  return { text: "Selamat Malam", Icon: Moon };
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
  href?: string;
}

function StatCard({ stat }: { stat: Stat }) {
  const content = (
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

  if (stat.href) {
    return <Link href={stat.href}>{content}</Link>;
  }
  return content;
}

// ---------------------------------------------------------------------------
// Workflow progress bar
// ---------------------------------------------------------------------------

interface WorkflowStep {
  label: string;
  done: boolean;
  current: boolean;
}

function WorkflowProgress({ steps }: { steps: WorkflowStep[] }) {
  const doneCount = steps.filter((s) => s.done).length;
  const pct = steps.length > 0 ? Math.round((doneCount / steps.length) * 100) : 0;

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-slate-700">Setup progress</span>
        <span className="text-xs text-slate-500">{pct}% complete</span>
      </div>
      <div className="h-2 rounded-full bg-slate-100 overflow-hidden">
        <div
          className="h-full rounded-full bg-gradient-to-r from-sky-500 to-emerald-500 transition-all duration-500"
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="flex items-center justify-between text-xs text-slate-500">
        {steps.map((step, i) => (
          <span
            key={step.label}
            className={cn(
              "flex items-center gap-1",
              step.done && "text-emerald-700 font-medium",
              step.current && "text-sky-700 font-medium",
            )}
          >
            {step.done ? (
              <CheckCircle2 className="size-3 text-emerald-500" />
            ) : step.current ? (
              <Circle className="size-3 text-sky-500 fill-sky-200" />
            ) : (
              <Circle className="size-3 text-slate-300" />
            )}
            {step.label}
          </span>
        ))}
      </div>
    </div>
  );
}

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

  // Workflow steps
  const workflowSteps: WorkflowStep[] = [
    {
      label: "Import",
      done: hasTransactions,
      current: !hasTransactions,
    },
    {
      label: "Receipts",
      done: hasReceipts,
      current: hasTransactions && !hasReceipts,
    },
    {
      label: "Match",
      done: hasMatches,
      current: hasTransactions && hasReceipts && !hasMatches,
    },
    {
      label: "Pack",
      done: hasPacks,
      current: hasTransactions && hasReceipts && hasMatches && !hasPacks,
    },
  ];

  // Next best action
  interface NextAction {
    label: string;
    description: string;
    href: string;
  }

  let nextAction: NextAction;
  if (!hasTransactions) {
    nextAction = {
      label: "Import your bank statement",
      description: "Upload a CSV from Maybank, CIMB, or any Malaysian bank",
      href: "/app/akaunkemas/import-bank-csv",
    };
  } else if (uncategorisedCount > 0) {
    nextAction = {
      label: `Review ${uncategorisedCount} uncategorised transaction${uncategorisedCount > 1 ? "s" : ""}`,
      description: "Categorise your transactions for accurate reports",
      href: "/app/akaunkemas/transactions",
    };
  } else if (!hasReceipts) {
    nextAction = {
      label: "Add your receipts",
      description: "Upload or manually enter receipts for matching",
      href: "/app/akaunkemas/receipts",
    };
  } else if (unmatchedTotal > 0) {
    nextAction = {
      label: `Match ${unmatchedTotal} item${unmatchedTotal > 1 ? "s" : ""}`,
      description: "Link receipts to transactions for your accountant",
      href: "/app/akaunkemas/matching",
    };
  } else if (!hasPacks) {
    nextAction = {
      label: "Prepare your accountant pack",
      description: "Generate a ZIP with CSV, JSON, and PDF reports",
      href: "/app/akaunkemas/accountant-packs",
    };
  } else {
    nextAction = {
      label: "View your latest pack",
      description: "Your accountant pack is ready for download",
      href: "/app/akaunkemas/accountant-packs",
    };
  }

  const greeting = getGreeting();

  const stats: Stat[] = [
    {
      label: "Transactions",
      value: String(txCount),
      icon: ArrowLeftRight,
      color: "text-sky-600",
      bg: "bg-sky-50",
      border: "border-sky-200",
      href: "/app/akaunkemas/transactions",
    },
    {
      label: "Receipts",
      value: String(receiptCount),
      icon: Receipt,
      color: "text-amber-600",
      bg: "bg-amber-50",
      border: "border-amber-200",
      href: "/app/akaunkemas/receipts",
    },
    {
      label: "Ready to match",
      value: String(unmatchedTotal),
      icon: GitMerge,
      color: "text-violet-600",
      bg: "bg-violet-50",
      border: "border-violet-200",
      href: "/app/akaunkemas/matching",
    },
    {
      label: "Need review",
      value: String(uncategorisedCount),
      icon: Tag,
      color: uncategorisedCount > 0 ? "text-rose-600" : "text-slate-400",
      bg: uncategorisedCount > 0 ? "bg-rose-50" : "bg-slate-50",
      border: uncategorisedCount > 0 ? "border-rose-200" : "border-slate-200",
      href: uncategorisedCount > 0 ? "/app/akaunkemas/transactions" : undefined,
    },
  ];

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      {/* Greeting */}
      <div className="flex items-center gap-3">
        <greeting.Icon className="size-6 text-amber-500" />
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">
            {greeting.text}
          </h1>
          <p className="mt-1 text-sm text-slate-500">
            Here&apos;s your bookkeeping overview for today.
          </p>
        </div>
      </div>

      {/* Quick Stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat) => (
          <StatCard key={stat.label} stat={stat} />
        ))}
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Left: Workflow progress + Cashflow */}
        <div className="lg:col-span-2 space-y-6">
          {/* Workflow Progress */}
          <Card className="rounded-2xl">
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2 text-base">
                <TrendingUp className="size-4 text-sky-600" />
                Setup progress
              </CardTitle>
            </CardHeader>
            <CardContent>
              <WorkflowProgress steps={workflowSteps} />
            </CardContent>
          </Card>

          {/* Net Cashflow */}
          <Card className="rounded-2xl border-emerald-200" style={{ borderLeftWidth: "3px" }}>
            <CardContent className="flex items-center gap-4 pt-4">
              <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-emerald-50">
                <DollarSign className="size-5 text-emerald-600" />
              </div>
              <div>
                <p className="text-xs text-slate-500">Net cashflow this month</p>
                <p className={cn(
                  "text-2xl font-bold",
                  netCashflow >= 0 ? "text-emerald-700" : "text-red-700",
                )}>
                  {formatCurrency(netCashflow)}
                </p>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Right: Next best action */}
        <div>
          <Card className="rounded-2xl border-sky-200 bg-gradient-to-br from-sky-50/50 to-white h-full">
            <CardHeader className="pb-2">
              <CardTitle className="flex items-center gap-2 text-base">
                <Sparkles className="size-4 text-sky-600" />
                What&apos;s next
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              <p className="text-sm text-slate-500">{nextAction.description}</p>
              <Link href={nextAction.href}>
                <span className="inline-flex items-center gap-2 rounded-lg bg-sky-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-sky-700 transition-colors cursor-pointer">
                  {nextAction.label}
                  <ArrowRight className="size-3.5" />
                </span>
              </Link>

              {/* Quick links */}
              <div className="mt-4 border-t border-slate-100 pt-4 space-y-2">
                <p className="text-xs font-medium text-slate-400 uppercase tracking-wider">
                  Quick links
                </p>
                <Link
                  href="/app/akaunkemas/import-bank-csv"
                  className="flex items-center gap-2 text-sm text-slate-600 hover:text-sky-700 transition-colors"
                >
                  <Upload className="size-3.5" />
                  Import bank statement
                </Link>
                <Link
                  href="/app/akaunkemas/receipts"
                  className="flex items-center gap-2 text-sm text-slate-600 hover:text-sky-700 transition-colors"
                >
                  <Receipt className="size-3.5" />
                  Add receipt
                </Link>
                <Link
                  href="/app/akaunkemas/matching"
                  className="flex items-center gap-2 text-sm text-slate-600 hover:text-sky-700 transition-colors"
                >
                  <GitMerge className="size-3.5" />
                  Match receipts
                </Link>
                <Link
                  href="/app/akaunkemas/accountant-packs"
                  className="flex items-center gap-2 text-sm text-slate-600 hover:text-sky-700 transition-colors"
                >
                  <Package className="size-3.5" />
                  Prepare accountant pack
                </Link>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Latest Accountant Pack */}
      {latestPack && (
        <Card className="rounded-2xl border-emerald-200 bg-emerald-50/50">
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-base">
              <CheckCircle2 className="size-4 text-emerald-600" />
              Latest accountant pack
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-slate-900">{latestPack.label}</p>
                <p className="text-xs text-slate-500">
                  {formatDate(latestPack.periodStart)} — {formatDate(latestPack.periodEnd)}
                  {latestPack.generatedAt && ` · Generated ${formatDate(latestPack.generatedAt)}`}
                </p>
              </div>
              <Link href="/app/akaunkemas/accountant-packs">
                <span className="text-sm text-sky-600 hover:text-sky-700 font-medium cursor-pointer">
                  View all packs
                </span>
              </Link>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
