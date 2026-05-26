import { ArrowLeftRight, Upload } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export default function TransactionsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Transactions</h1>
        <p className="text-sm text-slate-500">
          Import, categorise, and manage your bank transactions.
        </p>
      </div>

      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">All Transactions</CardTitle>
        </CardHeader>
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
          <Button disabled className="gap-2">
            <Upload className="size-4" />
            Import CSV
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
