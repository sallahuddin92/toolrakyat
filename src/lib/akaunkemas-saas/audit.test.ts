import { describe, expect, it } from "vitest";
import {
  createAuditLogger,
  AuditEntryInputSchema,
  AUDIT_EVENTS,
  ENTITY_TYPES,
} from "./audit";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function baseEntry(overrides: Record<string, unknown> = {}) {
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
// createAuditLogger
// ---------------------------------------------------------------------------

describe("createAuditLogger", () => {
  it("returns a logger with log and getEntries", () => {
    const logger = createAuditLogger();
    expect(logger).toHaveProperty("log");
    expect(logger).toHaveProperty("getEntries");
    expect(typeof logger.log).toBe("function");
    expect(typeof logger.getEntries).toBe("function");
  });

  it("log() returns an AuditEntry with id and createdAt", () => {
    const logger = createAuditLogger();
    const entry = baseEntry();
    const result = logger.log(entry);

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

  it("getEntries enforces tenant isolation (different tenantId returns empty)", () => {
    const logger = createAuditLogger();
    logger.log(baseEntry());

    const results = logger.getEntries("other-tenant", "business-1");
    expect(results).toHaveLength(0);
  });

  it("getEntries enforces business isolation", () => {
    const logger = createAuditLogger();
    logger.log(baseEntry());

    const results = logger.getEntries("tenant-1", "other-business");
    expect(results).toHaveLength(0);
  });

  it("getEntries filters by entityType", () => {
    const logger = createAuditLogger();
    logger.log(baseEntry({ eventType: AUDIT_EVENTS.TRANSACTION_CREATE }));
    logger.log(
      baseEntry({
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
    const logger = createAuditLogger();
    logger.log(baseEntry({ eventType: AUDIT_EVENTS.USER_LOGIN }));
    logger.log(
      baseEntry({
        eventType: AUDIT_EVENTS.USER_LOGOUT,
        summary: "User logged out",
      }),
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
    const logger = createAuditLogger();
    logger.log(baseEntry({ entityId: "tx-001" }));
    logger.log(baseEntry({ entityId: "tx-002", summary: "Updated tx-002" }));

    const results = logger.getEntries("tenant-1", "business-1", {
      entityId: "tx-001",
    });
    expect(results).toHaveLength(1);
    expect(results[0]!.entityId).toBe("tx-001");
  });

  it("getEntries respects limit and offset", () => {
    const logger = createAuditLogger();
    for (let i = 0; i < 5; i++) {
      logger.log(
        baseEntry({ entityId: `tx-00${i}`, summary: `Transaction ${i}` }),
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
    expect(offsetResults[1]!.entityId).toBe("tx-003");
  });

  it("each log call produces a unique id", () => {
    const logger = createAuditLogger();
    const e1 = logger.log(baseEntry());
    const e2 = logger.log(baseEntry({ summary: "Second entry" }));

    expect(e1.id).not.toBe(e2.id);
  });
});

// ---------------------------------------------------------------------------
// Metadata Sanitization
// ---------------------------------------------------------------------------

describe("metadata sanitization", () => {
  it("fileContent is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { fileContent: "sensitive-file-contents", fileName: "test.csv" },
      }),
    );

    expect(result.metadata.fileContent).toBe("[REDACTED]");
    expect(result.metadata.fileName).toBe("test.csv");
  });

  it("long strings are truncated", () => {
    const logger = createAuditLogger();
    const longString = "a".repeat(600);
    const result = logger.log(
      baseEntry({
        metadata: { notes: longString },
      }),
    );

    expect(result.metadata.notes).toBeTypeOf("string");
    const notes = result.metadata.notes as string;
    expect(notes.length).toBeLessThan(longString.length);
    expect(notes).toContain("truncated");
  });

  it("password is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { password: "super-secret-pw", username: "john" },
      }),
    );

    expect(result.metadata.password).toBe("[REDACTED]");
    expect(result.metadata.username).toBe("john");
  });

  it("passwordHash is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { passwordHash: "abc123hash", userId: "user-1" },
      }),
    );

    expect(result.metadata.passwordHash).toBe("[REDACTED]");
    expect(result.metadata.userId).toBe("user-1");
  });

  it("token is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { token: "bearer-xyz-token", action: "login" },
      }),
    );

    expect(result.metadata.token).toBe("[REDACTED]");
    expect(result.metadata.action).toBe("login");
  });

  it("secret is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { secret: "api-key-secret", key: "public-key" },
      }),
    );

    expect(result.metadata.secret).toBe("[REDACTED]");
    expect(result.metadata.key).toBe("public-key");
  });

  it("rawCsv is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { rawCsv: "col1,col2\nval1,val2", fileName: "data.csv" },
      }),
    );

    expect(result.metadata.rawCsv).toBe("[REDACTED]");
    expect(result.metadata.fileName).toBe("data.csv");
  });

  it("csvContent is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { csvContent: "line1\nline2", rowCount: 2 },
      }),
    );

    expect(result.metadata.csvContent).toBe("[REDACTED]");
    expect(result.metadata.rowCount).toBe(2);
  });

  it("rawData is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { rawData: "binary stuff", format: "text" },
      }),
    );

    expect(result.metadata.rawData).toBe("[REDACTED]");
    expect(result.metadata.format).toBe("text");
  });

  it("fileBuffer is redacted", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: { fileBuffer: new ArrayBuffer(8), name: "file.bin" },
      }),
    );

    expect(result.metadata.fileBuffer).toBe("[REDACTED]");
    expect(result.metadata.name).toBe("file.bin");
  });

  it("normal metadata passes through", () => {
    const logger = createAuditLogger();
    const result = logger.log(
      baseEntry({
        metadata: {
          action: "update",
          field: "amount",
          oldValue: 100,
          newValue: 200,
          count: 5,
        },
      }),
    );

    expect(result.metadata.action).toBe("update");
    expect(result.metadata.field).toBe("amount");
    expect(result.metadata.oldValue).toBe(100);
    expect(result.metadata.newValue).toBe(200);
    expect(result.metadata.count).toBe(5);
  });

  it("empty metadata object is preserved", () => {
    const logger = createAuditLogger();
    const result = logger.log(baseEntry({ metadata: {} }));

    expect(result.metadata).toEqual({});
  });
});

// ---------------------------------------------------------------------------
// AuditEntryInputSchema
// ---------------------------------------------------------------------------

describe("AuditEntryInputSchema", () => {
  it("validates correct data", () => {
    const data = {
      tenantId: "tenant-1",
      businessId: "business-1",
      userId: "user-1",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "Created a transaction",
      metadata: { source: "api" },
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(true);
  });

  it("rejects missing tenantId", () => {
    const data = {
      tenantId: "",
      businessId: "business-1",
      userId: "user-1",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "Created a transaction",
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it("rejects missing businessId", () => {
    const data = {
      tenantId: "tenant-1",
      businessId: "",
      userId: "user-1",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "Created a transaction",
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it("rejects missing userId", () => {
    const data = {
      tenantId: "tenant-1",
      businessId: "business-1",
      userId: "",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "Created a transaction",
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it("rejects summary longer than 500 chars", () => {
    const data = {
      tenantId: "tenant-1",
      businessId: "business-1",
      userId: "user-1",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "x".repeat(501),
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it("rejects empty summary", () => {
    const data = {
      tenantId: "tenant-1",
      businessId: "business-1",
      userId: "user-1",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "",
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(false);
  });

  it("metadata defaults to empty object when omitted", () => {
    const data = {
      tenantId: "tenant-1",
      businessId: "business-1",
      userId: "user-1",
      eventType: AUDIT_EVENTS.TRANSACTION_CREATE,
      entityType: ENTITY_TYPES.TRANSACTION,
      entityId: "tx-001",
      summary: "Created a transaction",
    };

    const result = AuditEntryInputSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.metadata).toEqual({});
    }
  });
});

// ---------------------------------------------------------------------------
// AUDIT_EVENTS and ENTITY_TYPES constants
// ---------------------------------------------------------------------------

describe("AUDIT_EVENTS", () => {
  it("contains all expected event types", () => {
    expect(AUDIT_EVENTS.USER_LOGIN).toBe("auth.login");
    expect(AUDIT_EVENTS.USER_LOGOUT).toBe("auth.logout");
    expect(AUDIT_EVENTS.USER_REGISTER).toBe("auth.register");
    expect(AUDIT_EVENTS.TRANSACTION_CREATE).toBe("transaction.create");
    expect(AUDIT_EVENTS.TRANSACTION_UPDATE).toBe("transaction.update");
    expect(AUDIT_EVENTS.TRANSACTION_DELETE).toBe("transaction.delete");
    expect(AUDIT_EVENTS.TRANSACTION_IMPORT_CSV).toBe("transaction.import_csv");
    expect(AUDIT_EVENTS.RECEIPT_CREATE).toBe("receipt.create");
    expect(AUDIT_EVENTS.RECEIPT_UPDATE).toBe("receipt.update");
    expect(AUDIT_EVENTS.RECEIPT_DELETE).toBe("receipt.delete");
    expect(AUDIT_EVENTS.MATCH_CREATE).toBe("match.create");
    expect(AUDIT_EVENTS.MATCH_DELETE).toBe("match.delete");
    expect(AUDIT_EVENTS.MATCH_MANUAL).toBe("match.manual");
    expect(AUDIT_EVENTS.PACK_GENERATE).toBe("pack.generate");
    expect(AUDIT_EVENTS.PACK_SEND).toBe("pack.send");
    expect(AUDIT_EVENTS.PACK_ARCHIVE).toBe("pack.archive");
    expect(AUDIT_EVENTS.BUSINESS_UPDATE).toBe("business.update");
    expect(AUDIT_EVENTS.MEMBER_INVITE).toBe("member.invite");
    expect(AUDIT_EVENTS.MEMBER_REMOVE).toBe("member.remove");
    expect(AUDIT_EVENTS.MEMBER_ROLE_CHANGE).toBe("member.role_change");
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
