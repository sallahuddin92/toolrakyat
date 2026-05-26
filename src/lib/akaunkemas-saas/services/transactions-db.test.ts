import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from "vitest";
import type { TransactionService } from "./transactions";

// ---------------------------------------------------------------------------
// In-memory SQLite DB reference (populated by vi.mock factory)
// ---------------------------------------------------------------------------

let testSqlite: any; // eslint-disable-line @typescript-eslint/no-explicit-any

vi.mock("@/lib/db", async () => {
  const BetterSqlite3 = await import("better-sqlite3");
  const sqlite = new BetterSqlite3.default(":memory:");
  sqlite.pragma("journal_mode = WAL");
  sqlite.pragma("foreign_keys = ON");

  // Create all tables needed for transactions + audit
  sqlite.exec(`
    CREATE TABLE IF NOT EXISTS transactions (
      id TEXT PRIMARY KEY,
      tenant_id TEXT NOT NULL,
      business_id TEXT NOT NULL,
      date TEXT NOT NULL,
      description TEXT NOT NULL,
      debit REAL NOT NULL DEFAULT 0,
      credit REAL NOT NULL DEFAULT 0,
      amount REAL NOT NULL DEFAULT 0,
      balance REAL,
      category_slug TEXT NOT NULL DEFAULT 'uncategorised',
      is_reconciled INTEGER NOT NULL DEFAULT 0,
      notes TEXT NOT NULL DEFAULT '',
      source TEXT NOT NULL DEFAULT 'manual',
      status TEXT NOT NULL DEFAULT 'draft',
      import_hash TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_transactions_tenant_business_date
      ON transactions(tenant_id, business_id, date);
    CREATE INDEX IF NOT EXISTS idx_transactions_tenant_business_category
      ON transactions(tenant_id, business_id, category_slug);
    CREATE UNIQUE INDEX IF NOT EXISTS ux_transactions_tenant_business_import_hash
      ON transactions(tenant_id, business_id, import_hash)
      WHERE import_hash IS NOT NULL;

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
  `);

  testSqlite = sqlite;

  const d = await import("drizzle-orm/better-sqlite3");
  const s = await import("@/lib/db/schema");
  const db = d.drizzle(sqlite, { schema: s });

  return { db, schema: s };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function seedInput(overrides: Record<string, unknown> = {}) {
  return {
    tenantId: "tenant-1",
    businessId: "business-1",
    date: "2026-01-15",
    description: "Office supplies",
    debit: 150,
    credit: 0,
    balance: null,
    categorySlug: "office_supplies",
    isReconciled: false,
    notes: "",
    source: "manual" as const,
    status: "draft" as const,
    importHash: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

let service: TransactionService;
let createDbTransactionService: () => TransactionService;

beforeAll(async () => {
  const mod = await import("./transactions-db");
  createDbTransactionService = mod.createDbTransactionService;
});

beforeEach(() => {
  testSqlite!.exec("DELETE FROM transactions");
  testSqlite!.exec("DELETE FROM audit_logs");
  service = createDbTransactionService();
});

afterAll(() => {
  testSqlite!.close();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("createDbTransactionService", () => {
  describe("create()", () => {
    it("creates a transaction and returns it with id and timestamps", () => {
      const result = service.create(seedInput());

      expect(result.id).toBeTruthy();
      expect(typeof result.id).toBe("string");
      expect(result.tenantId).toBe("tenant-1");
      expect(result.businessId).toBe("business-1");
      expect(result.date).toBe("2026-01-15");
      expect(result.description).toBe("Office supplies");
      expect(result.debit).toBe(150);
      expect(result.credit).toBe(0);
      expect(result.amount).toBe(-150);
      expect(result.status).toBe("draft");
      expect(result.createdAt).toBeInstanceOf(Date);
      expect(result.updatedAt).toBeInstanceOf(Date);
    });

    it("persists in the database", () => {
      const entry = service.create(seedInput());
      const retrieved = service.getById("tenant-1", "business-1", entry.id);
      expect(retrieved).toBeTruthy();
      expect(retrieved?.description).toBe("Office supplies");
    });

    it("handles duplicate CSV imports gracefully", () => {
      const input = seedInput({ source: "csv_import" });
      const first = service.create(input);
      // Second import of same data should return existing record
      const second = service.create(input);
      expect(second.id).toBe(first.id);
    });
  });

  describe("getById()", () => {
    it("returns undefined for non-existent id", () => {
      expect(service.getById("tenant-1", "business-1", "nonexistent")).toBeUndefined();
    });

    it("enforces tenant isolation", () => {
      const entry = service.create(seedInput());
      expect(
        service.getById("other-tenant", "business-1", entry.id),
      ).toBeUndefined();
    });

    it("enforces business isolation", () => {
      const entry = service.create(seedInput());
      expect(
        service.getById("tenant-1", "other-business", entry.id),
      ).toBeUndefined();
    });
  });

  describe("list()", () => {
    it("returns transactions for the correct tenant+business", () => {
      service.create(seedInput());
      service.create(seedInput({ description: "Second tx" }));
      service.create(
        seedInput({ tenantId: "tenant-2", description: "Other tenant" }),
      );

      const results = service.list("tenant-1", "business-1");
      expect(results).toHaveLength(2);
    });

    it("filters by categorySlug", () => {
      service.create(seedInput({ categorySlug: "rent" }));
      service.create(seedInput({ categorySlug: "sales" }));

      const results = service.list("tenant-1", "business-1", { categorySlug: "rent" });
      expect(results).toHaveLength(1);
      expect(results[0]!.categorySlug).toBe("rent");
    });

    it("filters by status", () => {
      service.create(seedInput({ status: "draft" }));
      service.create(
        seedInput({ description: "Reviewed tx", status: "reviewed" }),
      );

      const draftResults = service.list("tenant-1", "business-1", { status: "draft" });
      expect(draftResults).toHaveLength(1);
      expect(draftResults[0]!.status).toBe("draft");
    });

    it("filters by source", () => {
      service.create(seedInput({ source: "manual" }));
      service.create(
        seedInput({ description: "CSV tx", source: "csv_import" }),
      );

      const csvResults = service.list("tenant-1", "business-1", { source: "csv_import" });
      expect(csvResults).toHaveLength(1);
      expect(csvResults[0]!.source).toBe("csv_import");
    });

    it("filters by isReconciled", () => {
      service.create(seedInput({ isReconciled: true }));
      service.create(
        seedInput({ description: "Not reconciled", isReconciled: false }),
      );

      const reconciled = service.list("tenant-1", "business-1", { isReconciled: true });
      expect(reconciled).toHaveLength(1);
      expect(reconciled[0]!.isReconciled).toBe(true);
    });

    it("filters by date range", () => {
      service.create(seedInput({ date: "2026-01-10" }));
      service.create(
        seedInput({ date: "2026-02-20", description: "Feb tx" }),
      );

      const results = service.list("tenant-1", "business-1", {
        dateFrom: "2026-02-01",
        dateTo: "2026-02-28",
      });
      expect(results).toHaveLength(1);
      expect(results[0]!.date).toBe("2026-02-20");
    });

    it("filters by search term", () => {
      service.create(seedInput({ description: "Rent payment" }));
      service.create(
        seedInput({ description: "Office supplies" }),
      );

      const results = service.list("tenant-1", "business-1", { search: "rent" });
      expect(results).toHaveLength(1);
      expect(results[0]!.description).toBe("Rent payment");
    });

    it("respects limit and offset", () => {
      for (let i = 0; i < 5; i++) {
        service.create(
          seedInput({ description: `Tx ${i}`, date: `2026-01-0${i + 1}` }),
        );
      }

      const limited = service.list("tenant-1", "business-1", { limit: 2 });
      expect(limited).toHaveLength(2);

      const offsetResults = service.list("tenant-1", "business-1", { offset: 2, limit: 2 });
      expect(offsetResults).toHaveLength(2);
    });

    it("returns results ordered by date descending", () => {
      service.create(seedInput({ date: "2026-01-01", description: "Oldest" }));
      service.create(seedInput({ date: "2026-03-01", description: "Newest" }));

      const results = service.list("tenant-1", "business-1");
      expect(results[0]!.description).toBe("Newest");
      expect(results[1]!.description).toBe("Oldest");
    });
  });

  describe("update()", () => {
    it("updates fields and returns the updated record", () => {
      const entry = service.create(seedInput());
      const updated = service.update("tenant-1", "business-1", entry.id, {
        description: "Updated description",
        categorySlug: "rent",
      });

      expect(updated).toBeTruthy();
      expect(updated!.description).toBe("Updated description");
      expect(updated!.categorySlug).toBe("rent");
      expect(updated!.updatedAt.getTime()).toBeGreaterThanOrEqual(
        entry.updatedAt.getTime(),
      );
    });

    it("returns undefined for non-existent transaction", () => {
      const result = service.update("tenant-1", "business-1", "nonexistent", {
        description: "nope",
      });
      expect(result).toBeUndefined();
    });

    it("rejects update on locked transactions", () => {
      const entry = service.create(seedInput({ status: "locked" }));
      const result = service.update("tenant-1", "business-1", entry.id, {
        description: "Should fail",
      });
      expect(result).toBeUndefined();
    });

    it("can change status from draft to reviewed", () => {
      const entry = service.create(seedInput({ status: "draft" }));
      const result = service.update("tenant-1", "business-1", entry.id, {
        status: "reviewed",
      });
      expect(result!.status).toBe("reviewed");
    });
  });

  describe("delete()", () => {
    it("deletes the transaction and returns true", () => {
      const entry = service.create(seedInput());
      expect(service.delete("tenant-1", "business-1", entry.id)).toBe(true);
      expect(
        service.getById("tenant-1", "business-1", entry.id),
      ).toBeUndefined();
    });

    it("returns false for non-existent transaction", () => {
      expect(service.delete("tenant-1", "business-1", "nonexistent")).toBe(false);
    });

    it("rejects deletion of locked transactions", () => {
      const entry = service.create(seedInput({ status: "locked" }));
      expect(service.delete("tenant-1", "business-1", entry.id)).toBe(false);
      // Still exists
      expect(
        service.getById("tenant-1", "business-1", entry.id),
      ).toBeTruthy();
    });
  });

  describe("bulkCreate()", () => {
    it("creates multiple transactions", () => {
      const results = service.bulkCreate("tenant-1", "business-1", [
        { date: "2026-01-10", description: "Tx 1", debit: 100, credit: 0, categorySlug: "rent", notes: "", balance: null, isReconciled: false, source: "manual" as const, status: "draft" as const, importHash: null },
        { date: "2026-01-15", description: "Tx 2", debit: 0, credit: 200, categorySlug: "sales", notes: "", balance: null, isReconciled: false, source: "manual" as const, status: "draft" as const, importHash: null },
      ]);

      expect(results).toHaveLength(2);
      expect(results[0]!.id).toBeTruthy();
      expect(results[1]!.id).toBeTruthy();
      expect(service.count("tenant-1", "business-1")).toBe(2);
    });
  });

  describe("count()", () => {
    it("returns 0 for empty tenant", () => {
      expect(service.count("tenant-1", "business-1")).toBe(0);
    });

    it("counts transactions for the correct tenant+business", () => {
      service.create(seedInput());
      service.create(seedInput({ description: "Tx 2" }));
      service.create(seedInput({ tenantId: "tenant-2", description: "Other" }));

      expect(service.count("tenant-1", "business-1")).toBe(2);
    });
  });

  describe("countUncategorised()", () => {
    it("counts only uncategorised transactions", () => {
      service.create(seedInput({ categorySlug: "uncategorised" }));
      service.create(seedInput({ categorySlug: "rent", description: "Rent" }));
      service.create(seedInput({ categorySlug: "uncategorised", description: "Tx 3" }));

      expect(service.countUncategorised("tenant-1", "business-1")).toBe(2);
    });

    it("counts only for the correct tenant", () => {
      service.create(seedInput({ categorySlug: "uncategorised" }));
      service.create(
        seedInput({ categorySlug: "uncategorised", tenantId: "tenant-2", description: "Other" }),
      );

      expect(service.countUncategorised("tenant-1", "business-1")).toBe(1);
    });
  });
});
