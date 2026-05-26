import { GitMerge } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export default function MatchingPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Receipt Matching</h1>
        <p className="text-sm text-slate-500">
          Match receipts to bank transactions automatically.
        </p>
      </div>

      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">Match Receipts</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col items-center gap-4 py-12">
          <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
            <GitMerge className="size-8 text-slate-400" />
          </div>
          <div className="text-center">
            <p className="text-sm font-medium text-slate-900">No data to match</p>
            <p className="text-xs text-slate-500">
              Upload bank transactions and receipts to run matching.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
