import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from "vitest";
import { AUDIT_EVENTS, ENTITY_TYPES } from "./audit";
import type { AuditLogger } from "./audit";

// ---------------------------------------------------------------------------
// In-memory SQLite DB reference (populated by vi.mock factory)
// ---------------------------------------------------------------------------

let testSqlite: any; // eslint-disable-line @typescript-eslint/no-explicit-any

// Mock @/lib/db to use an in-memory SQLite database.
// The factory runs lazily when audit-db first imports @/lib/db.
vi.mock("@/lib/db", async () => {
  const BetterSqlite3 = await import("better-sqlite3");
  const sqlite = new BetterSqlite3.default(":memory:");
  sqlite.pragma("journal_mode = WAL");
  sqlite.pragma("foreign_keys = ON");

  // Create the audit_logs table so the drizzle schema matches the DB
  sqlite.exec(`
    CREATE TABLE IF NOT EXISTS audit_logs (
      id TEXT PRIMARY KEY,
      tenant_id TEXT NOT NULL,
      business_id TEXT NOT NULL,
      user_id TEXT NOT NULL,
      event_type TEXT NOT NULL,
      entity_type TEXT NOT NULL,
      entity_id TEXT NOT NULL,
      summary TEXT NOT NULL,
      metadata TEXT NOT NULL DEFAULT '{}',
      created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_business_created
      ON audit_logs(tenant_id, business_id, created_at);
    CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_business_event
      ON audit_logs(tenant_id, business_id, event_type);
  `);

  // Store reference for test cleanup
  testSqlite = sqlite;

  const d = await import("drizzle-orm/better-sqlite3");
  const s = await import("@/lib/db/schema");
  const db = d.drizzle(sqlite, { schema: s });

  return { db, schema: s };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function seedStandardEntry(overrides: Record<string, unknown> = {}) {
  return {
    tenantId: "tenant-1",
    businessId: "business-1",
    userId: "user-1",
    eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
    entityType: ENTITY_TYPES.TRANSACTION,
    entityId: "tx-001",
    summary: "Created a new transaction",
    metadata: {},
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

let logger: AuditLogger;
let createDbAuditLogger: () => AuditLogger;

beforeAll(async () => {
  // Dynamically import audit-db after vi.mock interception is active
  const mod = await import("./audit-db");
  createDbAuditLogger = mod.createDbAuditLogger;
});

beforeEach(() => {
  testSqlite!.exec("DELETE FROM audit_logs");
  logger = createDbAuditLogger();
});

afterAll(() => {
  testSqlite!.close();
});

// ---------------------------------------------------------------------------
// createDbAuditLogger
// ---------------------------------------------------------------------------

describe("createDbAuditLogger", () => {
  it("returns a logger with log and getEntries", () => {
    expect(logger).toHaveProperty("log");
    expect(logger).toHaveProperty("getEntries");
    expect(typeof logger.log).toBe("function");
    expect(typeof logger.getEntries).toBe("function");
  });

  it("log() returns an AuditEntry with id and createdAt", () => {
    const result = logger.log(seedStandardEntry());

    expect(result).toHaveProperty("id");
    expect(typeof result.id).toBe("string");
    expect(result.id.length).toBeGreaterThan(0);
    expect(result).toHaveProperty("createdAt");
    expect(result.createdAt).toBeInstanceOf(Date);
    expect(result.tenantId).toBe("tenant-1");
    expect(result.businessId).toBe("business-1");
    expect(result.userId).toBe("user-1");
    expect(result.eventType).toBe(AUDIT_EVENTS.TRANSACTION_CREATE);
    expect(result.entityType).toBe(ENTITY_TYPES.TRANSACTION);
    expect(result.entityId).toBe("tx-001");
    expect(result.summary).toBe("Created a new transaction");
  });

  it("log() persists an entry that can be retrieved via getEntries", () => {
    const entry = logger.log(seedStandardEntry());

    const results = logger.getEntries("tenant-1", "business-1");
    expect(results).toHaveLength(1);
    expect(results[0]!.id).toBe(entry.id);
  });

  it("getEntries enforces tenant isolation", () => {
    logger.log(seedStandardEntry());
    logger.log(
      seedStandardEntry({ tenantId: "tenant-2", summary: "Other tenant tx" }),
    );

    const results = logger.getEntries("other-tenant", "business-1");
    expect(results).toHaveLength(0);
  });

  it("getEntries enforces business isolation", () => {
    logger.log(seedStandardEntry());
    logger.log(
      seedStandardEntry({ businessId: "business-2", summary: "Other biz tx" }),
    );

    const results = logger.getEntries("tenant-1", "other-business");
    expect(results).toHaveLength(0);
  });

  it("getEntries filters by entityType", () => {
    logger.log(
      seedStandardEntry({ entityType: ENTITY_TYPES.TRANSACTION }),
    );
    logger.log(
      seedStandardEntry({
        entityType: ENTITY_TYPES.RECEIPT,
        entityId: "rec-001",
        eventType: AUDIT_EVENTS.RECEIPT_CREATE,
        summary: "Created a receipt",
      }),
    );

    const txResults = logger.getEntries("tenant-1", "business-1", {
      entityType: ENTITY_TYPES.TRANSACTION,
    });
    expect(txResults).toHaveLength(1);
    expect(txResults[0]!.entityType).toBe(ENTITY_TYPES.TRANSACTION);

    const receiptResults = logger.getEntries("tenant-1", "business-1", {
      entityType: ENTITY_TYPES.RECEIPT,
    });
    expect(receiptResults).toHaveLength(1);
    expect(receiptResults[0]!.entityType).toBe(ENTITY_TYPES.RECEIPT);
  });

  it("getEntries filters by eventType", () => {
    logger.log(
      seedStandardEntry({ eventType: AUDIT_EVENTS.USER_LOGIN, summary: "User logged in" }),
    );
    logger.log(
      seedStandardEntry({ eventType: AUDIT_EVENTS.USER_LOGOUT, summary: "User logged out" }),
    );

    const loginResults = logger.getEntries("tenant-1", "business-1", {
      eventType: AUDIT_EVENTS.USER_LOGIN,
    });
    expect(loginResults).toHaveLength(1);
    expect(loginResults[0]!.eventType).toBe(AUDIT_EVENTS.USER_LOGIN);

    const logoutResults = logger.getEntries("tenant-1", "business-1", {
      eventType: AUDIT_EVENTS.USER_LOGOUT,
    });
    expect(logoutResults).toHaveLength(1);
    expect(logoutResults[0]!.eventType).toBe(AUDIT_EVENTS.USER_LOGOUT);
  });

  it("getEntries filters by entityId", () => {
    logger.log(seedStandardEntry({ entityId: "tx-001" }));
    logger.log(
      seedStandardEntry({ entityId: "tx-002", summary: "Updated tx-002" }),
    );

    const results = logger.getEntries("tenant-1", "business-1", {
      entityId: "tx-001",
    });
    expect(results).toHaveLength(1);
    expect(results[0]!.entityId).toBe("tx-001");
  });

  it("getEntries respects limit and offset", () => {
    for (let i = 0; i < 5; i++) {
      logger.log(
        seedStandardEntry({ entityId: `tx-00${i}`, summary: `Transaction ${i}` }),
      );
    }

    const limited = logger.getEntries("tenant-1", "business-1", { limit: 2 });
    expect(limited).toHaveLength(2);

    const offsetResults = logger.getEntries("tenant-1", "business-1", {
      offset: 2,
      limit: 2,
    });
    expect(offsetResults).toHaveLength(2);
    expect(offsetResults[0]!.entityId).toBe("tx-002");
    expect(offsetResults[1]!.entityId).toBe("tx-001");
  });

  it("each log call produces a unique id", () => {
    const e1 = logger.log(seedStandardEntry());
    const e2 = logger.log(
      seedStandardEntry({ summary: "Second entry", entityId: "tx-002" }),
    );
    expect(e1.id).not.toBe(e2.id);
  });

  it("getEntries returns results ordered by createdAt descending", () => {
    logger.log(seedStandardEntry({ entityId: "tx-oldest", summary: "Oldest" }));
    logger.log(seedStandardEntry({ entityId: "tx-newest", summary: "Newest" }));

    const results = logger.getEntries("tenant-1", "business-1");
    expect(results[0]!.entityId).toBe("tx-newest");
    expect(results[1]!.entityId).toBe("tx-oldest");
  });
});

// ---------------------------------------------------------------------------
// Metadata Sanitization Tests
// ---------------------------------------------------------------------------

describe("metadata sanitization (DB-backed)", () => {
  it("fileContent is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { fileContent: "sensitive", fileName: "test.csv" } }),
    );
    expect(result.metadata.fileContent).toBe("[REDACTED]");
    expect(result.metadata.fileName).toBe("test.csv");
  });

  it("long strings are truncated", () => {
    const longString = "a".repeat(600);
    const result = logger.log(
      seedStandardEntry({ metadata: { notes: longString } }),
    );
    expect(result.metadata.notes).toBeTypeOf("string");
    const notes = result.metadata.notes as string;
    expect(notes.length).toBeLessThan(longString.length);
    expect(notes).toContain("truncated");
  });

  it("password is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { password: "secret", username: "john" } }),
    );
    expect(result.metadata.password).toBe("[REDACTED]");
    expect(result.metadata.username).toBe("john");
  });

  it("passwordHash is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { passwordHash: "abc123", userId: "user-1" } }),
    );
    expect(result.metadata.passwordHash).toBe("[REDACTED]");
    expect(result.metadata.userId).toBe("user-1");
  });

  it("token is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { token: "bearer-xyz", action: "login" } }),
    );
    expect(result.metadata.token).toBe("[REDACTED]");
    expect(result.metadata.action).toBe("login");
  });

  it("secret is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { secret: "api-key", key: "public" } }),
    );
    expect(result.metadata.secret).toBe("[REDACTED]");
    expect(result.metadata.key).toBe("public");
  });

  it("rawCsv is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { rawCsv: "col1,col2", fileName: "data.csv" } }),
    );
    expect(result.metadata.rawCsv).toBe("[REDACTED]");
    expect(result.metadata.fileName).toBe("data.csv");
  });

  it("csvContent is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { csvContent: "line1\nline2", rowCount: 2 } }),
    );
    expect(result.metadata.csvContent).toBe("[REDACTED]");
    expect(result.metadata.rowCount).toBe(2);
  });

  it("rawData is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { rawData: "binary", format: "text" } }),
    );
    expect(result.metadata.rawData).toBe("[REDACTED]");
    expect(result.metadata.format).toBe("text");
  });

  it("fileBuffer is redacted", () => {
    const result = logger.log(
      seedStandardEntry({ metadata: { fileBuffer: new ArrayBuffer(8), name: "file.bin" } }),
    );
    expect(result.metadata.fileBuffer).toBe("[REDACTED]");
    expect(result.metadata.name).toBe("file.bin");
  });

  it("normal metadata passes through", () => {
    const result = logger.log(
      seedStandardEntry({
        metadata: { action: "update", field: "amount", oldValue: 100, newValue: 200, count: 5 },
      }),
    );
    expect(result.metadata.action).toBe("update");
    expect(result.metadata.field).toBe("amount");
    expect(result.metadata.oldValue).toBe(100);
    expect(result.metadata.newValue).toBe(200);
    expect(result.metadata.count).toBe(5);
  });

  it("empty metadata object is preserved", () => {
    const result = logger.log(seedStandardEntry({ metadata: {} }));
    expect(result.metadata).toEqual({});
  });
});

// ---------------------------------------------------------------------------
// AUDIT_EVENTS and ENTITY_TYPES constants
// ---------------------------------------------------------------------------

describe("AUDIT_EVENTS", () => {
  it("contains all expected event types", () => {
    expect(AUDIT_EVENTS.USER_LOGIN).toBe("auth.login");
    expect(AUDIT_EVENTS.TRANSACTION_CREATE).toBe("transaction.create");
    expect(AUDIT_EVENTS.RECEIPT_CREATE).toBe("receipt.create");
    expect(AUDIT_EVENTS.MATCH_CREATE).toBe("match.create");
    expect(AUDIT_EVENTS.PACK_GENERATE).toBe("pack.generate");
    expect(AUDIT_EVENTS.BUSINESS_UPDATE).toBe("business.update");
    expect(AUDIT_EVENTS.MEMBER_INVITE).toBe("member.invite");
  });
});

describe("ENTITY_TYPES", () => {
  it("contains all expected entity types", () => {
    expect(ENTITY_TYPES.TRANSACTION).toBe("transaction");
    expect(ENTITY_TYPES.RECEIPT).toBe("receipt");
    expect(ENTITY_TYPES.MATCH).toBe("match");
    expect(ENTITY_TYPES.PACK).toBe("accountant_pack");
    expect(ENTITY_TYPES.BUSINESS).toBe("business");
    expect(ENTITY_TYPES.MEMBER).toBe("member");
    expect(ENTITY_TYPES.USER).toBe("user");
  });
});
