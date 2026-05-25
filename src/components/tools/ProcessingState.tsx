import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";

export type ProcessingStatus =
  | "idle"
  | "validating"
  | "uploading"
  | "processing"
  | "completed"
  | "failed";

function statusLabel(status: ProcessingStatus) {
  switch (status) {
    case "idle":
      return "Ready";
    case "validating":
      return "Validating";
    case "uploading":
      return "Uploading";
    case "processing":
      return "Processing";
    case "completed":
      return "Completed";
    case "failed":
      return "Failed";
  }
}

function statusProgress(status: ProcessingStatus) {
  switch (status) {
    case "idle":
      return 0;
    case "validating":
      return 20;
    case "uploading":
      return 45;
    case "processing":
      return 75;
    case "completed":
      return 100;
    case "failed":
      return 100;
  }
}

export function ProcessingState({
  status,
}: {
  status: ProcessingStatus;
}) {
  const variant =
    status === "failed"
      ? "destructive"
      : status === "completed"
        ? "default"
        : "secondary";

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="text-sm font-medium text-slate-900">Status</div>
        <Badge variant={variant} className="rounded-full">
          {statusLabel(status)}
        </Badge>
      </div>
      <Progress value={statusProgress(status)} />
    </div>
  );
}

