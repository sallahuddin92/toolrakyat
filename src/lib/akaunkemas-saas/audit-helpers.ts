import type { AuditLogger, AuditEntry, AuditEvent, EntityType } from "./audit";
import { AUDIT_EVENTS, ENTITY_TYPES } from "./audit";
import { createDbAuditLogger } from "./audit-db";

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

let _logger: AuditLogger | null = null;

export function getAuditLogger(): AuditLogger {
  if (!_logger) {
    _logger = createDbAuditLogger();
  }
  return _logger;
}

/**
 * Reset the singleton (useful in tests).
 */
export function resetAuditLogger(): void {
  _logger = null;
}

// ---------------------------------------------------------------------------
// Generic audit log helper
// ---------------------------------------------------------------------------

export function auditLog(
  tenantId: string,
  businessId: string,
  userId: string,
  eventType: AuditEvent,
  entityType: EntityType,
  entityId: string,
  summary: string,
  metadata: Record<string, unknown> = {},
): AuditEntry {
  return getAuditLogger().log({
    tenantId,
    businessId,
    userId,
    eventType,
    entityType,
    entityId,
    summary,
    metadata,
  });
}

// ---------------------------------------------------------------------------
// Specific event helpers
// ---------------------------------------------------------------------------

export function logUserLogin(
  tenantId: string,
  businessId: string,
  userId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.USER_LOGIN,
    ENTITY_TYPES.USER,
    userId,
    summary,
  );
}

export function logUserLogout(
  tenantId: string,
  businessId: string,
  userId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.USER_LOGOUT,
    ENTITY_TYPES.USER,
    userId,
    summary,
  );
}

export function logUserRegister(
  tenantId: string,
  businessId: string,
  userId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.USER_REGISTER,
    ENTITY_TYPES.USER,
    userId,
    summary,
  );
}

export function logTransactionCreated(
  tenantId: string,
  businessId: string,
  userId: string,
  txId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.TRANSACTION_CREATE,
    ENTITY_TYPES.TRANSACTION,
    txId,
    summary,
  );
}

export function logTransactionUpdated(
  tenantId: string,
  businessId: string,
  userId: string,
  txId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.TRANSACTION_UPDATE,
    ENTITY_TYPES.TRANSACTION,
    txId,
    summary,
  );
}

export function logTransactionDeleted(
  tenantId: string,
  businessId: string,
  userId: string,
  txId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.TRANSACTION_DELETE,
    ENTITY_TYPES.TRANSACTION,
    txId,
    summary,
  );
}

export function logTransactionImportCsv(
  tenantId: string,
  businessId: string,
  userId: string,
  txId: string,
  summary: string,
  metadata?: Record<string, unknown>,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.TRANSACTION_IMPORT_CSV,
    ENTITY_TYPES.TRANSACTION,
    txId,
    summary,
    metadata,
  );
}

export function logReceiptCreated(
  tenantId: string,
  businessId: string,
  userId: string,
  receiptId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.RECEIPT_CREATE,
    ENTITY_TYPES.RECEIPT,
    receiptId,
    summary,
  );
}

export function logReceiptUpdated(
  tenantId: string,
  businessId: string,
  userId: string,
  receiptId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.RECEIPT_UPDATE,
    ENTITY_TYPES.RECEIPT,
    receiptId,
    summary,
  );
}

export function logReceiptDeleted(
  tenantId: string,
  businessId: string,
  userId: string,
  receiptId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.RECEIPT_DELETE,
    ENTITY_TYPES.RECEIPT,
    receiptId,
    summary,
  );
}

export function logMatchCreated(
  tenantId: string,
  businessId: string,
  userId: string,
  matchId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.MATCH_CREATE,
    ENTITY_TYPES.MATCH,
    matchId,
    summary,
  );
}

export function logMatchDeleted(
  tenantId: string,
  businessId: string,
  userId: string,
  matchId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.MATCH_DELETE,
    ENTITY_TYPES.MATCH,
    matchId,
    summary,
  );
}

export function logMatchManual(
  tenantId: string,
  businessId: string,
  userId: string,
  matchId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.MATCH_MANUAL,
    ENTITY_TYPES.MATCH,
    matchId,
    summary,
  );
}

export function logPackGenerated(
  tenantId: string,
  businessId: string,
  userId: string,
  packId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.PACK_GENERATE,
    ENTITY_TYPES.PACK,
    packId,
    summary,
  );
}

export function logPackSent(
  tenantId: string,
  businessId: string,
  userId: string,
  packId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.PACK_SEND,
    ENTITY_TYPES.PACK,
    packId,
    summary,
  );
}

export function logPackArchived(
  tenantId: string,
  businessId: string,
  userId: string,
  packId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.PACK_ARCHIVE,
    ENTITY_TYPES.PACK,
    packId,
    summary,
  );
}

export function logBusinessUpdated(
  tenantId: string,
  businessId: string,
  userId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.BUSINESS_UPDATE,
    ENTITY_TYPES.BUSINESS,
    businessId,
    summary,
  );
}

export function logMemberInvite(
  tenantId: string,
  businessId: string,
  userId: string,
  memberId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.MEMBER_INVITE,
    ENTITY_TYPES.MEMBER,
    memberId,
    summary,
  );
}

export function logMemberRemove(
  tenantId: string,
  businessId: string,
  userId: string,
  memberId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.MEMBER_REMOVE,
    ENTITY_TYPES.MEMBER,
    memberId,
    summary,
  );
}

export function logMemberRoleChange(
  tenantId: string,
  businessId: string,
  userId: string,
  memberId: string,
  summary: string,
) {
  return auditLog(
    tenantId,
    businessId,
    userId,
    AUDIT_EVENTS.MEMBER_ROLE_CHANGE,
    ENTITY_TYPES.MEMBER,
    memberId,
    summary,
  );
}

// ---------------------------------------------------------------------------
// Fetch helpers (convenience wrappers around getEntries)
// ---------------------------------------------------------------------------

export function getAuditEntries(
  tenantId: string,
  businessId: string,
  options?: {
    entityType?: EntityType;
    entityId?: string;
    eventType?: AuditEvent;
    limit?: number;
    offset?: number;
  },
) {
  return getAuditLogger().getEntries(tenantId, businessId, options as Parameters<AuditLogger["getEntries"]>[2]);
}

export function getEntityAuditHistory(
  tenantId: string,
  businessId: string,
  entityType: EntityType,
  entityId: string,
  limit?: number,
) {
  return getAuditLogger().getEntries(tenantId, businessId, {
    entityType,
    entityId,
    limit,
  });
}
