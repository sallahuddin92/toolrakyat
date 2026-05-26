import Link from "next/link";
import {
  ArrowLeftRight,
  Receipt,
  GitMerge,
  Tag,
  Package,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

const stats = [
  {
    label: "Transactions",
    value: "0",
    icon: ArrowLeftRight,
    color: "text-sky-600",
    bg: "bg-sky-50",
  },
  {
    label: "Receipts",
    value: "0",
    icon: Receipt,
    color: "text-amber-600",
    bg: "bg-amber-50",
  },
  {
    label: "Unmatched",
    value: "0",
    icon: GitMerge,
    color: "text-violet-600",
    bg: "bg-violet-50",
  },
  {
    label: "Uncategorised",
    value: "0",
    icon: Tag,
    color: "text-rose-600",
    bg: "bg-rose-50",
  },
];

const quickActions = [
  {
    label: "Import Transactions",
    description: "Upload a bank CSV",
    href: "/app/akaunkemas/transactions",
    icon: ArrowLeftRight,
  },
  {
    label: "Add Receipt",
    description: "Manually add or import",
    href: "/app/akaunkemas/receipts",
    icon: Receipt,
  },
  {
    label: "Run Matching",
    description: "Match receipts to transactions",
    href: "/app/akaunkemas/matching",
    icon: GitMerge,
  },
  {
    label: "Generate Pack",
    description: "Create accountant pack",
    href: "/app/akaunkemas/accountant-packs",
    icon: Package,
  },
];

export default function DashboardPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Dashboard</h1>
        <p className="text-sm text-slate-500">Overview of your AkaunKemas account.</p>
      </div>

      {/* Quick Stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat) => (
          <Card key={stat.label} className="rounded-2xl">
            <CardContent className="flex items-center gap-4 pt-4">
              <div className={`flex size-10 items-center justify-center rounded-xl ${stat.bg}`}>
                <stat.icon className={`size-5 ${stat.color}`} />
              </div>
              <div>
                <p className="text-xs text-slate-500">{stat.label}</p>
                <p className="text-2xl font-bold text-slate-900">{stat.value}</p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Latest Accountant Pack */}
      <Card className="rounded-2xl border-sky-200 bg-sky-50/50">
        <CardHeader>
          <CardTitle className="text-base">Latest Accountant Pack</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-slate-500">
            No packs yet. Generate your first pack from the Accountant Packs section.
          </p>
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <div>
        <h2 className="mb-3 text-base font-semibold text-slate-900">Quick Actions</h2>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {quickActions.map((action) => (
            <Link key={action.label} href={action.href}>
              <Card className="rounded-2xl transition-shadow hover:shadow-md">
                <CardContent className="flex items-center gap-4 pt-4">
                  <div className="flex size-10 items-center justify-center rounded-xl bg-slate-100">
                    <action.icon className="size-5 text-slate-600" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-slate-900">{action.label}</p>
                    <p className="text-xs text-slate-500">{action.description}</p>
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
