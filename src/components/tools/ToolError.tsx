import { AlertTriangle } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function ToolError({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <Card className="rounded-2xl border-destructive/40">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base text-destructive">
          <AlertTriangle className="size-4" aria-hidden="true" />
          Something went wrong
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 text-sm text-slate-600">
        <div className="rounded-xl bg-slate-50 p-3 text-slate-700">{message}</div>
        {onRetry ? (
          <Button onClick={onRetry} className="rounded-2xl">
            Retry
          </Button>
        ) : null}
      </CardContent>
    </Card>
  );
}

