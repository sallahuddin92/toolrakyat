import { getCurrentUser } from "@/lib/auth/dal";
import { getAuditEntries } from "@/lib/akaunkemas-saas/audit-helpers";
import { AUDIT_EVENTS, ENTITY_TYPES } from "@/lib/akaunkemas-saas/audit";
import type { AuditEvent, EntityType } from "@/lib/akaunkemas-saas/audit";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ScrollText, Search } from "lucide-react";

export const dynamic = "force-dynamic";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const EVENT_LABELS: Record<string, string> = {
  [AUDIT_EVENTS.USER_LOGIN]: "User Login",
  [AUDIT_EVENTS.USER_LOGOUT]: "User Logout",
  [AUDIT_EVENTS.USER_REGISTER]: "User Register",
  [AUDIT_EVENTS.TRANSACTION_CREATE]: "Transaction Created",
  [AUDIT_EVENTS.TRANSACTION_UPDATE]: "Transaction Updated",
  [AUDIT_EVENTS.TRANSACTION_DELETE]: "Transaction Deleted",
  [AUDIT_EVENTS.TRANSACTION_IMPORT_CSV]: "CSV Imported",
  [AUDIT_EVENTS.RECEIPT_CREATE]: "Receipt Created",
  [AUDIT_EVENTS.RECEIPT_UPDATE]: "Receipt Updated",
  [AUDIT_EVENTS.RECEIPT_DELETE]: "Receipt Deleted",
  [AUDIT_EVENTS.MATCH_CREATE]: "Match Created",
  [AUDIT_EVENTS.MATCH_DELETE]: "Match Deleted",
  [AUDIT_EVENTS.MATCH_MANUAL]: "Manual Match",
  [AUDIT_EVENTS.PACK_GENERATE]: "Pack Generated",
  [AUDIT_EVENTS.PACK_SEND]: "Pack Sent",
  [AUDIT_EVENTS.PACK_ARCHIVE]: "Pack Archived",
  [AUDIT_EVENTS.BUSINESS_UPDATE]: "Business Updated",
  [AUDIT_EVENTS.MEMBER_INVITE]: "Member Invited",
  [AUDIT_EVENTS.MEMBER_REMOVE]: "Member Removed",
  [AUDIT_EVENTS.MEMBER_ROLE_CHANGE]: "Member Role Changed",
};

const ENTITY_LABELS: Record<string, string> = {
  [ENTITY_TYPES.TRANSACTION]: "Transaction",
  [ENTITY_TYPES.RECEIPT]: "Receipt",
  [ENTITY_TYPES.MATCH]: "Match",
  [ENTITY_TYPES.PACK]: "Pack",
  [ENTITY_TYPES.BUSINESS]: "Business",
  [ENTITY_TYPES.MEMBER]: "Member",
  [ENTITY_TYPES.USER]: "User",
};

const EVENT_COLORS: Record<string, string> = {
  auth: "bg-blue-50 text-blue-700 border-blue-200",
  transaction: "bg-sky-50 text-sky-700 border-sky-200",
  receipt: "bg-amber-50 text-amber-700 border-amber-200",
  match: "bg-violet-50 text-violet-700 border-violet-200",
  pack: "bg-emerald-50 text-emerald-700 border-emerald-200",
  business: "bg-slate-50 text-slate-700 border-slate-200",
  member: "bg-rose-50 text-rose-700 border-rose-200",
};

function eventColor(eventType: AuditEvent): string {
  const prefix = eventType.split(".")[0] ?? "";
  return EVENT_COLORS[prefix] ?? "bg-slate-50 text-slate-700 border-slate-200";
}

function formatDate(d: Date): string {
  return d.toLocaleString("en-MY", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Unique event types for filter dropdown. */
const EVENT_TYPE_OPTIONS = Object.entries(AUDIT_EVENTS).map(([, value]) => value);

/** Unique entity types for filter dropdown. */
const ENTITY_TYPE_OPTIONS = Object.entries(ENTITY_TYPES).map(([, value]) => value);

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default async function AuditLogsPage({
  searchParams,
}: {
  searchParams: Promise<{ eventType?: string; entityType?: string }>;
}) {
  const session = await getCurrentUser();
  const { tenantId, businessId } = session;

  const params = await searchParams;
  const filterEventType = (params.eventType as AuditEvent) || undefined;
  const filterEntityType = (params.entityType as EntityType) || undefined;

  const entries = getAuditEntries(tenantId, businessId, {
    eventType: filterEventType,
    entityType: filterEntityType,
    limit: 200,
  });

  const hasFilters = filterEventType || filterEntityType;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Audit Logs</h1>
        <p className="text-sm text-slate-500">
          Track changes and activity across your AkaunKemas account.
        </p>
      </div>

      {/* Filter bar */}
      <Card className="rounded-2xl">
        <CardContent className="pt-4">
          <form method="GET" className="flex flex-wrap items-end gap-3">
            <div className="flex-1 min-w-[160px]">
              <label
                htmlFor="eventType"
                className="mb-1 block text-xs font-medium text-slate-600"
              >
                Event Type
              </label>
              <select
                id="eventType"
                name="eventType"
                defaultValue={filterEventType ?? ""}
                className="h-9 w-full max-w-[200px] rounded-lg border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-ring focus:ring-3 focus:ring-ring/50"
              >
                <option value="">All events</option>
                {EVENT_TYPE_OPTIONS.map((ev) => (
                  <option key={ev} value={ev}>
                    {EVENT_LABELS[ev] ?? ev}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex-1 min-w-[160px]">
              <label
                htmlFor="entityType"
                className="mb-1 block text-xs font-medium text-slate-600"
              >
                Entity Type
              </label>
              <select
                id="entityType"
                name="entityType"
                defaultValue={filterEntityType ?? ""}
                className="h-9 w-full max-w-[200px] rounded-lg border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-ring focus:ring-3 focus:ring-ring/50"
              >
                <option value="">All entities</option>
                {ENTITY_TYPE_OPTIONS.map((et) => (
                  <option key={et} value={et}>
                    {ENTITY_LABELS[et] ?? et}
                  </option>
                ))}
              </select>
            </div>
            <button
              type="submit"
              className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-lg border border-border bg-background px-3 text-sm font-medium transition-colors hover:bg-muted"
            >
              <Search className="size-3.5" />
              Filter
            </button>
            {hasFilters && (
              <a
                href="/app/akaunkemas/audit-logs"
                className="inline-flex h-9 shrink-0 items-center px-2 text-xs text-slate-500 hover:text-slate-700"
              >
                Clear filters
              </a>
            )}
          </form>
        </CardContent>
      </Card>

      {/* Activity log */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">
            Activity Log
            {entries.length > 0 && (
              <span className="ml-2 text-sm font-normal text-slate-500">
                ({entries.length} {entries.length === 1 ? "entry" : "entries"})
              </span>
            )}
          </CardTitle>
        </CardHeader>

        {entries.length === 0 ? (
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
              <ScrollText className="size-8 text-slate-400" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">
                {hasFilters ? "No matching audit events" : "No audit events yet"}
              </p>
              <p className="text-xs text-slate-500">
                {hasFilters
                  ? "Try adjusting your filters."
                  : "Activity will appear here as you use AkaunKemas."}
              </p>
            </div>
          </CardContent>
        ) : (
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-slate-100 text-left">
                    <th className="px-4 py-2.5 text-xs font-medium text-slate-500">
                      Timestamp
                    </th>
                    <th className="px-4 py-2.5 text-xs font-medium text-slate-500">
                      Event
                    </th>
                    <th className="px-4 py-2.5 text-xs font-medium text-slate-500">
                      Entity
                    </th>
                    <th className="px-4 py-2.5 text-xs font-medium text-slate-500">
                      Summary
                    </th>
                    <th className="px-4 py-2.5 text-xs font-medium text-slate-500">
                      Metadata
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-50">
                  {entries.map((entry) => (
                    <tr
                      key={entry.id}
                      className="group hover:bg-slate-50/50 transition-colors"
                    >
                      <td className="px-4 py-2.5 text-xs text-slate-500 whitespace-nowrap">
                        {formatDate(entry.createdAt)}
                      </td>
                      <td className="px-4 py-2.5 whitespace-nowrap">
                        <Badge
                          variant="outline"
                          className={`text-xs font-normal ${eventColor(entry.eventType)}`}
                        >
                          {EVENT_LABELS[entry.eventType] ?? entry.eventType}
                        </Badge>
                      </td>
                      <td className="px-4 py-2.5 text-xs text-slate-600 whitespace-nowrap">
                        <span className="text-slate-400">
                          {ENTITY_LABELS[entry.entityType] ?? entry.entityType}
                        </span>
                        <span className="ml-1.5 font-mono text-[10px] text-slate-400">
                          {entry.entityId.slice(0, 8)}...
                        </span>
                      </td>
                      <td className="px-4 py-2.5 text-sm text-slate-700 max-w-xs truncate">
                        {entry.summary}
                      </td>
                      <td className="px-4 py-2.5 text-xs">
                        {Object.keys(entry.metadata).length > 0 ? (
                          <details className="group/details">
                            <summary className="cursor-pointer text-slate-400 hover:text-slate-600 transition-colors">
                              {Object.keys(entry.metadata).length}{" "}
                              {Object.keys(entry.metadata).length === 1
                                ? "field"
                                : "fields"}
                            </summary>
                            <div className="mt-1 max-h-32 overflow-auto rounded-md bg-slate-50 p-2 font-mono text-[10px] text-slate-600">
                              {Object.entries(entry.metadata).map(
                                ([key, value]) => (
                                  <div key={key} className="flex gap-2">
                                    <span className="shrink-0 text-slate-400">
                                      {key}:
                                    </span>
                                    <span className="break-all">
                                      {typeof value === "string"
                                        ? value
                                        : JSON.stringify(value)}
                                    </span>
                                  </div>
                                ),
                              )}
                            </div>
                          </details>
                        ) : (
                          <span className="text-slate-300">—</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        )}
      </Card>
    </div>
  );
}
