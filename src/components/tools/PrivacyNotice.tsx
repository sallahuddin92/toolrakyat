import { Shield } from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function PrivacyNotice({
  note,
  acceptedFileTypes,
  maxFileSizeMB,
}: {
  note: string;
  acceptedFileTypes?: string[];
  maxFileSizeMB?: number;
}) {
  return (
    <Card className="rounded-2xl">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Shield className="size-4" aria-hidden="true" />
          Privacy notice
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 text-sm text-slate-600">
        <div>{note}</div>
        {(acceptedFileTypes?.length || maxFileSizeMB) && (
          <div className="rounded-xl bg-slate-50 p-3 text-xs text-slate-600">
            {acceptedFileTypes?.length ? (
              <div>Accepted: {acceptedFileTypes.join(", ")}</div>
            ) : null}
            {maxFileSizeMB ? <div>Max size: {maxFileSizeMB} MB</div> : null}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

