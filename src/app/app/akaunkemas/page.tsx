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
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Demo business data
// ---------------------------------------------------------------------------

const DEMO_BUSINESS = {
  name: "Demo Business",
};

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

const stats: Stat[] = [
  {
    label: "Transactions",
    value: "0",
    icon: ArrowLeftRight,
    color: "text-sky-600",
    bg: "bg-sky-50",
    border: "border-sky-200",
  },
  {
    label: "Receipts",
    value: "0",
    icon: Receipt,
    color: "text-amber-600",
    bg: "bg-amber-50",
    border: "border-amber-200",
  },
  {
    label: "Unmatched",
    value: "0",
    icon: GitMerge,
    color: "text-violet-600",
    bg: "bg-violet-50",
    border: "border-violet-200",
  },
  {
    label: "Uncategorised",
    value: "0",
    icon: Tag,
    color: "text-rose-600",
    bg: "bg-rose-50",
    border: "border-rose-200",
  },
];

// ---------------------------------------------------------------------------
// Getting started checklist
// ---------------------------------------------------------------------------

interface ChecklistItem {
  label: string;
  href: string;
}

const checklistItems: ChecklistItem[] = [
  { label: "Upload your first bank CSV", href: "/app/akaunkemas/transactions" },
  { label: "Add receipts", href: "/app/akaunkemas/receipts" },
  { label: "Run receipt matching", href: "/app/akaunkemas/matching" },
  { label: "Generate accountant pack", href: "/app/akaunkemas/accountant-packs" },
];

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

export default function DashboardPage() {
  return (
    <div className="mx-auto max-w-6xl space-y-6">
      {/* Page header */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">
          Welcome back, {DEMO_BUSINESS.name}
        </h1>
        <p className="mt-1 text-sm text-slate-500">
          Overview of your AkaunKemas account.
        </p>
      </div>

      {/* Quick Stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat) => (
          <Card
            key={stat.label}
            className={cn(
              "rounded-2xl border transition-shadow duration-150 hover:shadow-md",
              stat.border
            )}
            style={{ borderLeftWidth: "3px" }}
          >
            <CardContent className="flex items-center gap-4 pt-4">
              <div
                className={cn(
                  "flex size-10 shrink-0 items-center justify-center rounded-xl",
                  stat.bg
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
        ))}
      </div>

      {/* Getting Started */}
      <Card className="rounded-2xl border-sky-200 bg-sky-50/50">
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <TrendingUp className="size-4 text-sky-600" />
            Getting Started
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-slate-500">
            Complete these steps to set up your AkaunKemas account.
          </p>
          <ul className="space-y-2">
            {checklistItems.map((item) => (
              <li key={item.label}>
                <Link
                  href={item.href}
                  className="flex items-center gap-3 rounded-xl px-3 py-2 text-sm text-slate-600 transition-colors duration-150 hover:bg-sky-100/50 hover:text-sky-700"
                >
                  <Circle className="size-4 shrink-0 text-slate-300" />
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>

      {/* Latest Accountant Pack */}
      <Card className="rounded-2xl border-sky-200 bg-sky-50/50">
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <CheckCircle2 className="size-4 text-sky-600" />
            Latest Accountant Pack
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-slate-500">
            No packs yet. Generate your first pack from the Accountant Packs
            section.
          </p>
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
                      action.bg
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
