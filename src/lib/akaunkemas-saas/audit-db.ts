import { db, schema } from "@/lib/db";
import { eq, and, desc } from "drizzle-orm";
import type { AuditLogger, AuditEntry, AuditEvent, EntityType } from "./audit";

// ---------------------------------------------------------------------------
// Metadata Sanitization (reuses the same logic as audit.ts)
// ---------------------------------------------------------------------------

const SENSITIVE_KEYS = new Set([
  "fileContent",
  "rawCsv",
  "csvContent",
  "rawData",
  "fileBuffer",
  "password",
  "passwordHash",
  "token",
  "secret",
]);

function sanitizeForStorage(
  metadata: Record<string, unknown>,
): Record<string, unknown> {
  const sanitized: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(metadata)) {
    if (SENSITIVE_KEYS.has(key)) {
      sanitized[key] = "[REDACTED]";
    } else if (typeof value === "string" && value.length > 500) {
      sanitized[key] =
        value.slice(0, 100) + `... [${value.length - 100} chars truncated]`;
    } else {
      sanitized[key] = value;
    }
  }
  return sanitized;
}

// ---------------------------------------------------------------------------
// DB-backed Audit Logger Factory
// ---------------------------------------------------------------------------

export function createDbAuditLogger(): AuditLogger {
  return {
    log(entry) {
      const id = crypto.randomUUID();
      const createdAt = new Date();

      // Sanitize metadata before storing
      const safeMeta = sanitizeForStorage(entry.metadata);

      db.insert(schema.auditLogs)
        .values({
          id,
          tenantId: entry.tenantId,
          businessId: entry.businessId,
          userId: entry.userId,
          eventType: entry.eventType,
          entityType: entry.entityType,
          entityId: entry.entityId,
          summary: entry.summary,
          metadata: JSON.stringify(safeMeta),
          createdAt: createdAt.toISOString(),
        })
        .run();

      return {
        ...entry,
        id,
        createdAt,
        metadata: safeMeta,
      };
    },

    getEntries(tenantId, businessId, options) {
      const rows = db
        .select()
        .from(schema.auditLogs)
        .where(
          and(
            eq(schema.auditLogs.tenantId, tenantId),
            eq(schema.auditLogs.businessId, businessId),
            ...(options?.entityType
              ? [eq(schema.auditLogs.entityType, options.entityType)]
              : []),
            ...(options?.entityId
              ? [eq(schema.auditLogs.entityId, options.entityId)]
              : []),
            ...(options?.eventType
              ? [eq(schema.auditLogs.eventType, options.eventType)]
              : []),
          ),
        )
        .orderBy(desc(schema.auditLogs.createdAt))
        .all();

      const offset = options?.offset ?? 0;
      const limit = options?.limit ?? 100;

      return rows.slice(offset, offset + limit).map((row) => ({
        id: row.id,
        tenantId: row.tenantId,
        businessId: row.businessId,
        userId: row.userId,
        eventType: row.eventType as AuditEvent,
        entityType: row.entityType as EntityType,
        entityId: row.entityId,
        summary: row.summary,
        metadata: JSON.parse(row.metadata || "{}"),
        createdAt: new Date(row.createdAt),
      }));
    },
  };
}
