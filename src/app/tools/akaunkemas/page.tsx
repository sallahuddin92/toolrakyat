import { getToolsByCategory } from "@/lib/tools/registry";
import Link from "next/link";
import { ArrowRight, FileSpreadsheet, Receipt, GitMerge, Package, BookOpen } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";

const toolIcons: Record<string, React.ComponentType<{ className?: string }>> = {
  "bank-csv-cleaner": FileSpreadsheet,
  "receipt-organizer": Receipt,
  "receipt-matcher": GitMerge,
  "accountant-pack": Package,
  "simple-ledger": BookOpen,
};

export default function AkaunKemasHubPage() {
  const tools = getToolsByCategory("akaunkemas");

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">AkaunKemas</h1>
        <p className="text-slate-500">Malaysian SME bookkeeping assistant — process bank statements, organise receipts, and prepare accountant packages.</p>
      </div>

      {/* Recommended Workflow */}
      <Card className="rounded-2xl border-sky-200 bg-sky-50/50">
        <CardHeader>
          <CardTitle className="text-base">Recommended Workflow</CardTitle>
        </CardHeader>
        <CardContent>
          <ol className="space-y-2 text-sm text-slate-600">
            <li className="flex items-center gap-2">
              <span className="flex size-5 items-center justify-center rounded-full bg-sky-100 text-xs font-medium text-sky-700">1</span>
              Clean bank CSV → categorise transactions
            </li>
            <li className="flex items-center gap-2">
              <span className="flex size-5 items-center justify-center rounded-full bg-sky-100 text-xs font-medium text-sky-700">2</span>
              Organise receipts → add receipt records
            </li>
            <li className="flex items-center gap-2">
              <span className="flex size-5 items-center justify-center rounded-full bg-sky-100 text-xs font-medium text-sky-700">3</span>
              Match receipts to bank transactions
            </li>
            <li className="flex items-center gap-2">
              <span className="flex size-5 items-center justify-center rounded-full bg-sky-100 text-xs font-medium text-sky-700">4</span>
              Generate accountant pack → ZIP for submission
            </li>
            <li className="flex items-center gap-2">
              <span className="flex size-5 items-center justify-center rounded-full bg-sky-100 text-xs font-medium text-sky-700">5</span>
              Review simple ledger → cashbook view
            </li>
          </ol>
        </CardContent>
      </Card>

      {/* Tool Cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {tools.map((tool) => {
          const Icon = toolIcons[tool.slug] ?? FileSpreadsheet;
          return (
            <Link key={tool.id} href={tool.route} className="group">
              <Card className="rounded-2xl h-full transition-shadow hover:shadow-md">
                <CardHeader>
                  <div className="flex items-center gap-2">
                    <div className="flex size-8 items-center justify-center rounded-xl bg-slate-100">
                      <Icon className="size-4 text-slate-600" />
                    </div>
                    <CardTitle className="text-sm">{tool.name}</CardTitle>
                  </div>
                </CardHeader>
                <CardContent>
                  <p className="text-xs text-slate-500 line-clamp-2">{tool.description}</p>
                  <div className="mt-3 flex items-center gap-1 text-xs font-medium text-sky-600">
                    Open tool <ArrowRight className="size-3" />
                  </div>
                </CardContent>
              </Card>
            </Link>
          );
        })}
      </div>

      <Separator />

      <p className="text-xs text-slate-400">
        Phase 2 tools — all processing happens locally in your browser. No data is uploaded or stored.
      </p>
    </div>
  );
}
