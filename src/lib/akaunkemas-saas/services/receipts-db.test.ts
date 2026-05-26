import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from "vitest";
import type { ReceiptService } from "./receipts";

// ---------------------------------------------------------------------------
// In-memory SQLite DB reference (populated by vi.mock factory)
// ---------------------------------------------------------------------------

let testSqlite: any; // eslint-disable-line @typescript-eslint/no-explicit-any

vi.mock("@/lib/db", async () => {
  const BetterSqlite3 = await import("better-sqlite3");
  const sqlite = new BetterSqlite3.default(":memory:");
  sqlite.pragma("journal_mode = WAL");
  sqlite.pragma("foreign_keys = ON");

  sqlite.exec(`
    CREATE TABLE IF NOT EXISTS receipts (
      id TEXT PRIMARY KEY,
      tenant_id TEXT NOT NULL,
      business_id TEXT NOT NULL,
      date TEXT NOT NULL,
      merchant TEXT NOT NULL,
      amount REAL NOT NULL,
      payment_method TEXT NOT NULL DEFAULT 'cash',
      category_slug TEXT NOT NULL DEFAULT 'uncategorised',
      tax_amount REAL NOT NULL DEFAULT 0,
      service_charge REAL NOT NULL DEFAULT 0,
      notes TEXT NOT NULL DEFAULT '',
      image_ref TEXT,
      status TEXT NOT NULL DEFAULT 'draft',
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_receipts_tenant_business_date
      ON receipts(tenant_id, business_id, date);

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
    date: "2025-06-15",
    merchant: "Seven Eleven",
    amount: 30.5,
    paymentMethod: "cash" as const,
    categorySlug: "office_supplies",
    taxAmount: 0,
    serviceCharge: 0,
    notes: "",
    imageRef: null,
    status: "draft" as const,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

let service: ReceiptService;
let createDbReceiptService: () => ReceiptService;

beforeAll(async () => {
  const mod = await import("./receipts-db");
  createDbReceiptService = mod.createDbReceiptService;
});

beforeEach(() => {
  testSqlite!.exec("DELETE FROM receipts");
  testSqlite!.exec("DELETE FROM audit_logs");
  service = createDbReceiptService();
});

afterAll(() => {
  testSqlite!.close();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("createDbReceiptService", () => {
  describe("create()", () => {
    it("creates a receipt and returns it with id and timestamps", () => {
      const result = service.create(seedInput());

      expect(result.id).toBeTruthy();
      expect(typeof result.id).toBe("string");
      expect(result.merchant).toBe("Seven Eleven");
      expect(result.amount).toBe(30.5);
      expect(result.paymentMethod).toBe("cash");
      expect(result.status).toBe("draft");
      expect(result.createdAt).toBeInstanceOf(Date);
      expect(result.updatedAt).toBeInstanceOf(Date);
    });

    it("persists in the database", () => {
      const entry = service.create(seedInput());
      const retrieved = service.getById("tenant-1", "business-1", entry.id);
      expect(retrieved).toBeTruthy();
      expect(retrieved?.merchant).toBe("Seven Eleven");
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
    it("returns receipts for the correct tenant+business", () => {
      service.create(seedInput());
      service.create(seedInput({ merchant: "Giant" }));
      service.create(seedInput({ tenantId: "tenant-2", merchant: "Other" }));

      const results = service.list("tenant-1", "business-1");
      expect(results).toHaveLength(2);
    });

    it("filters by categorySlug", () => {
      service.create(seedInput({ categorySlug: "office_supplies" }));
      service.create(seedInput({ merchant: "Rent", categorySlug: "rent" }));

      const results = service.list("tenant-1", "business-1", { categorySlug: "rent" });
      expect(results).toHaveLength(1);
      expect(results[0]!.categorySlug).toBe("rent");
    });

    it("filters by paymentMethod", () => {
      service.create(seedInput({ paymentMethod: "cash" }));
      service.create(seedInput({ merchant: "Card pay", paymentMethod: "card" }));

      const results = service.list("tenant-1", "business-1", { paymentMethod: "card" });
      expect(results).toHaveLength(1);
      expect(results[0]!.paymentMethod).toBe("card");
    });

    it("filters by merchant search", () => {
      service.create(seedInput({ merchant: "Seven Eleven" }));
      service.create(seedInput({ merchant: "Giant Hypermarket" }));

      const results = service.list("tenant-1", "business-1", { merchant: "seven" });
      expect(results).toHaveLength(1);
      expect(results[0]!.merchant).toBe("Seven Eleven");
    });

    it("filters by status", () => {
      service.create(seedInput({ status: "draft" }));
      service.create(seedInput({ merchant: "Reviewed", status: "reviewed" }));

      const results = service.list("tenant-1", "business-1", { status: "reviewed" });
      expect(results).toHaveLength(1);
      expect(results[0]!.status).toBe("reviewed");
    });

    it("filters by date range", () => {
      service.create(seedInput({ date: "2025-01-10" }));
      service.create(seedInput({ date: "2025-06-20", merchant: "June" }));

      const results = service.list("tenant-1", "business-1", {
        dateFrom: "2025-06-01",
        dateTo: "2025-06-30",
      });
      expect(results).toHaveLength(1);
      expect(results[0]!.date).toBe("2025-06-20");
    });

    it("respects limit and offset", () => {
      for (let i = 0; i < 4; i++) {
        service.create(seedInput({ merchant: `Store ${i}`, amount: 10 + i }));
      }

      const limited = service.list("tenant-1", "business-1", { limit: 2 });
      expect(limited).toHaveLength(2);

      const offsetResults = service.list("tenant-1", "business-1", { offset: 2, limit: 2 });
      expect(offsetResults).toHaveLength(2);
    });
  });

  describe("update()", () => {
    it("updates fields and returns the updated record", () => {
      const entry = service.create(seedInput());
      const updated = service.update("tenant-1", "business-1", entry.id, {
        merchant: "Updated Store",
        amount: 50,
      });

      expect(updated).toBeTruthy();
      expect(updated!.merchant).toBe("Updated Store");
      expect(updated!.amount).toBe(50);
      expect(updated!.updatedAt.getTime()).toBeGreaterThanOrEqual(
        entry.updatedAt.getTime(),
      );
    });

    it("returns undefined for non-existent receipt", () => {
      const result = service.update("tenant-1", "business-1", "nonexistent", {
        merchant: "nope",
      });
      expect(result).toBeUndefined();
    });
  });

  describe("delete()", () => {
    it("deletes the receipt and returns true", () => {
      const entry = service.create(seedInput());
      expect(service.delete("tenant-1", "business-1", entry.id)).toBe(true);
      expect(
        service.getById("tenant-1", "business-1", entry.id),
      ).toBeUndefined();
    });

    it("returns false for non-existent receipt", () => {
      expect(service.delete("tenant-1", "business-1", "nonexistent")).toBe(false);
    });
  });

  describe("count()", () => {
    it("returns 0 for empty", () => {
      expect(service.count("tenant-1", "business-1")).toBe(0);
    });

    it("counts receipts for the correct tenant+business", () => {
      service.create(seedInput());
      service.create(seedInput({ merchant: "Store 2" }));
      service.create(seedInput({ tenantId: "tenant-2", merchant: "Other" }));

      expect(service.count("tenant-1", "business-1")).toBe(2);
    });
  });

  describe("getTotalAmount()", () => {
    it("returns 0 for empty", () => {
      expect(service.getTotalAmount("tenant-1", "business-1")).toBe(0);
    });

    it("sums all receipt amounts for the tenant+business", () => {
      service.create(seedInput({ amount: 30 }));
      service.create(seedInput({ amount: 70, merchant: "Store 2" }));
      service.create(seedInput({ amount: 999, tenantId: "tenant-2", merchant: "Other" }));

      expect(service.getTotalAmount("tenant-1", "business-1")).toBe(100);
    });
  });

  describe("getByCategory()", () => {
    it("returns empty array for no data", () => {
      expect(service.getByCategory("tenant-1", "business-1")).toEqual([]);
    });

    it("groups receipts by category with totals and counts", () => {
      service.create(seedInput({ categorySlug: "office_supplies", amount: 30 }));
      service.create(seedInput({ categorySlug: "office_supplies", amount: 20, merchant: "Store 2" }));
      service.create(seedInput({ categorySlug: "rent", amount: 500, merchant: "Rent" }));

      const results = service.getByCategory("tenant-1", "business-1");
      expect(results).toHaveLength(2);

      const officeSupplies = results.find((r) => r.categorySlug === "office_supplies")!;
      expect(officeSupplies.total).toBe(50);
      expect(officeSupplies.count).toBe(2);

      const rent = results.find((r) => r.categorySlug === "rent")!;
      expect(rent.total).toBe(500);
      expect(rent.count).toBe(1);
    });

    it("is tenant-isolated", () => {
      service.create(seedInput({ categorySlug: "rent", amount: 100 }));
      service.create(seedInput({ categorySlug: "rent", amount: 200, tenantId: "tenant-2" }));

      const results = service.getByCategory("tenant-1", "business-1");
      const rent = results.find((r) => r.categorySlug === "rent")!;
      expect(rent.total).toBe(100);
    });
  });
});
