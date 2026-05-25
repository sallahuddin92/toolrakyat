import { Download, RotateCcw } from "lucide-react";
import type React from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatBytes } from "@/lib/tools/format";

export function ResultDownloadCard({
  title = "Result",
  fileName,
  url,
  bytes,
  originalBytes,
  onReset,
  children,
}: {
  title?: string;
  fileName: string;
  url: string;
  bytes: number;
  originalBytes?: number;
  onReset: () => void;
  children?: React.ReactNode;
}) {
  const savingsPct =
    originalBytes && originalBytes > 0
      ? Math.round(((originalBytes - bytes) / originalBytes) * 100)
      : null;

  return (
    <Card className="rounded-2xl">
      <CardHeader>
        <CardTitle className="text-base">{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 text-sm text-slate-600">
        <div className="rounded-2xl border bg-white p-4">
          <div className="font-medium text-slate-900">{fileName}</div>
          <div className="mt-1 text-xs text-slate-500">
            Output size: {formatBytes(bytes)}
            {originalBytes ? ` • Original: ${formatBytes(originalBytes)}` : ""}
            {savingsPct !== null ? ` • Savings: ${savingsPct}%` : ""}
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          <Button asChild className="rounded-2xl">
            <a href={url} download={fileName}>
              <Download className="mr-2 size-4" aria-hidden="true" />
              Download
            </a>
          </Button>
          <Button
            type="button"
            variant="secondary"
            className="rounded-2xl"
            onClick={onReset}
          >
            <RotateCcw className="mr-2 size-4" aria-hidden="true" />
            Process another
          </Button>
        </div>
        {children}
      </CardContent>
    </Card>
  );
}

