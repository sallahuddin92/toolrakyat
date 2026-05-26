import { z } from "zod";

// ---------------------------------------------------------------------------
// Audit Event Types
// ---------------------------------------------------------------------------

export const AUDIT_EVENTS = {
  // Auth
  USER_LOGIN: "auth.login",
  USER_LOGOUT: "auth.logout",
  USER_REGISTER: "auth.register",

  // Transactions
  TRANSACTION_CREATE: "transaction.create",
  TRANSACTION_UPDATE: "transaction.update",
  TRANSACTION_DELETE: "transaction.delete",
  TRANSACTION_IMPORT_CSV: "transaction.import_csv",

  // Receipts
  RECEIPT_CREATE: "receipt.create",
  RECEIPT_UPDATE: "receipt.update",
  RECEIPT_DELETE: "receipt.delete",

  // Matching
  MATCH_CREATE: "match.create",
  MATCH_DELETE: "match.delete",
  MATCH_MANUAL: "match.manual",

  // Accountant Pack
  PACK_GENERATE: "pack.generate",
  PACK_SEND: "pack.send",
  PACK_ARCHIVE: "pack.archive",

  // Business
  BUSINESS_UPDATE: "business.update",
  MEMBER_INVITE: "member.invite",
  MEMBER_REMOVE: "member.remove",
  MEMBER_ROLE_CHANGE: "member.role_change",
} as const;

export type AuditEvent = (typeof AUDIT_EVENTS)[keyof typeof AUDIT_EVENTS];

// ---------------------------------------------------------------------------
// Entity Types
// ---------------------------------------------------------------------------

export const ENTITY_TYPES = {
  TRANSACTION: "transaction",
  RECEIPT: "receipt",
  MATCH: "match",
  PACK: "accountant_pack",
  BUSINESS: "business",
  MEMBER: "member",
  USER: "user",
} as const;

export type EntityType = (typeof ENTITY_TYPES)[keyof typeof ENTITY_TYPES];

// ---------------------------------------------------------------------------
// Audit Entry Types
// ---------------------------------------------------------------------------

export interface AuditEntry {
  id: string;
  tenantId: string;
  businessId: string;
  userId: string;
  eventType: AuditEvent;
  entityType: EntityType;
  entityId: string;
  summary: string;
  metadata: Record<string, unknown>;
  createdAt: Date;
}

// ---------------------------------------------------------------------------
// Audit Logger Interface
// ---------------------------------------------------------------------------

export interface AuditLogger {
  log(entry: Omit<AuditEntry, "id" | "createdAt">): AuditEntry;
  getEntries(
    tenantId: string,
    businessId: string,
    options?: {
      entityType?: EntityType;
      entityId?: string;
      eventType?: AuditEvent;
      limit?: number;
      offset?: number;
    },
  ): AuditEntry[];
}

// ---------------------------------------------------------------------------
// Metadata Sanitization
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

function sanitizeMetadata(
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
// Factory
// ---------------------------------------------------------------------------

export function createAuditLogger(): AuditLogger {
  const entries: AuditEntry[] = [];

  return {
    log(entry) {
      // NEVER log raw file contents — metadata must be sanitized
      const sanitizedMeta = sanitizeMetadata(entry.metadata);
      const auditEntry: AuditEntry = {
        ...entry,
        metadata: sanitizedMeta,
        id: crypto.randomUUID(),
        createdAt: new Date(),
      };
      entries.push(auditEntry);
      console.log(`[AUDIT] ${entry.eventType} — ${entry.summary}`);
      return auditEntry;
    },
    getEntries(tenantId, businessId, options) {
      // Enforce tenant isolation
      let result = entries.filter(
        (e) => e.tenantId === tenantId && e.businessId === businessId,
      );
      if (options?.entityType)
        result = result.filter((e) => e.entityType === options.entityType);
      if (options?.entityId)
        result = result.filter((e) => e.entityId === options.entityId);
      if (options?.eventType)
        result = result.filter((e) => e.eventType === options.eventType);
      const offset = options?.offset ?? 0;
      const limit = options?.limit ?? 100;
      return result.slice(offset, offset + limit);
    },
  };
}

// ---------------------------------------------------------------------------
// Zod Schema for AuditEntry Input Validation
// ---------------------------------------------------------------------------

export const AuditEntryInputSchema = z.object({
  tenantId: z.string().min(1),
  businessId: z.string().min(1),
  userId: z.string().min(1),
  eventType: z.string().min(1),
  entityType: z.string().min(1),
  entityId: z.string().min(1),
  summary: z.string().min(1).max(500),
  metadata: z.record(z.string(), z.unknown()).default({}),
});
