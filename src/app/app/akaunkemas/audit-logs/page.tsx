import { ScrollText, Search } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

export default function AuditLogsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Audit Logs</h1>
        <p className="text-sm text-slate-500">
          Track changes and activity across your AkaunKemas account.
        </p>
      </div>

      {/* Filter bar placeholder */}
      <Card className="rounded-2xl">
        <CardContent className="flex flex-wrap items-end gap-3 pt-4">
          <div className="flex-1">
            <label className="mb-1 block text-xs font-medium text-slate-600">Event Type</label>
            <Input disabled placeholder="All events" className="max-w-[200px]" />
          </div>
          <div className="flex-1">
            <label className="mb-1 block text-xs font-medium text-slate-600">Date Range</label>
            <Input disabled placeholder="Select dates" className="max-w-[200px]" />
          </div>
          <Button disabled variant="outline" size="sm" className="gap-2">
            <Search className="size-3" />
            Filter
          </Button>
        </CardContent>
      </Card>

      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">Activity Log</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col items-center gap-4 py-12">
          <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
            <ScrollText className="size-8 text-slate-400" />
          </div>
          <div className="text-center">
            <p className="text-sm font-medium text-slate-900">No audit events yet</p>
            <p className="text-xs text-slate-500">
              Activity will appear here as you use AkaunKemas.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
