import Link from "next/link";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function ToolPlaceholder({
  title,
  note,
}: {
  title: string;
  note?: string;
}) {
  return (
    <Card className="rounded-2xl">
      <CardHeader>
        <CardTitle className="text-base">{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 text-sm text-slate-600">
        <div>
          This tool is not implemented yet. The page is ready so we can ship the
          UI and wire processing next.
        </div>
        {note ? <div className="rounded-xl bg-slate-50 p-3 text-slate-700">{note}</div> : null}
        <div className="flex flex-wrap gap-2">
          <Button asChild variant="secondary" className="rounded-xl">
            <Link href="/tools">Browse other tools</Link>
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

