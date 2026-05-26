import { Receipt, Plus } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export default function ReceiptsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Receipts</h1>
        <p className="text-sm text-slate-500">
          Manage and organise receipts for your bookkeeping.
        </p>
      </div>

      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">All Receipts</CardTitle>
        </CardHeader>
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
          <Button disabled className="gap-2">
            <Plus className="size-4" />
            Add Receipt
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
