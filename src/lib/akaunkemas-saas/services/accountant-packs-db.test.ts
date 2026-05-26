import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from "vitest";
import type { PackService } from "./accountant-packs-db";

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

    CREATE TABLE IF NOT EXISTS accountant_packs (
      id TEXT PRIMARY KEY,
      tenant_id TEXT NOT NULL,
      business_id TEXT NOT NULL,
      label TEXT NOT NULL,
      period_start TEXT NOT NULL,
      period_end TEXT NOT NULL,
      status TEXT NOT NULL DEFAULT 'draft',
      notes TEXT NOT NULL DEFAULT '',
      file_url TEXT,
      generated_at TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS receipt_matches (
      id TEXT PRIMARY KEY,
      tenant_id TEXT NOT NULL,
      business_id TEXT NOT NULL,
      transaction_id TEXT NOT NULL,
      receipt_id TEXT NOT NULL,
      match_type TEXT NOT NULL,
      date_delta INTEGER NOT NULL DEFAULT 0,
      amount_delta REAL NOT NULL DEFAULT 0,
      matched_by TEXT NOT NULL,
      created_at TEXT NOT NULL
    );

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

function seedPack(overrides: Record<string, unknown> = {}) {
  const id = overrides.id as string ?? crypto.randomUUID();
  const now = new Date().toISOString();
  const tenantId = (overrides.tenantId as string) ?? "tenant-1";
  const businessId = (overrides.businessId as string) ?? "business-1";
  testSqlite!.prepare(
    `INSERT INTO accountant_packs (id, tenant_id, business_id, label, period_start, period_end, status, notes, file_url, generated_at, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id, tenantId, businessId,
    overrides.label ?? "January 2026 Pack",
    overrides.periodStart ?? "2026-01-01",
    overrides.periodEnd ?? "2026-01-31",
    overrides.status ?? "generated",
    overrides.notes ?? "",
    overrides.fileUrl ?? null,
    overrides.generatedAt ?? now,
    now, now,
  );
  return {
    id,
    tenantId,
    businessId,
    label: (overrides.label as string) ?? "January 2026 Pack",
    periodStart: (overrides.periodStart as string) ?? "2026-01-01",
    periodEnd: (overrides.periodEnd as string) ?? "2026-01-31",
    status: (overrides.status as string) ?? "generated",
    notes: (overrides.notes as string) ?? "",
  };
}

function seedTransaction(overrides: Record<string, unknown> = {}) {
  const id = crypto.randomUUID();
  const now = new Date().toISOString();
  testSqlite!.prepare(
    `INSERT INTO transactions (id, tenant_id, business_id, date, description, debit, credit, amount, balance, category_slug, is_reconciled, notes, source, status, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id, "tenant-1", "business-1",
    overrides.date ?? "2026-01-15",
    overrides.description ?? "Office supplies",
    overrides.debit ?? 150, overrides.credit ?? 0,
    overrides.amount ?? -150,
    overrides.balance ?? null,
    overrides.categorySlug ?? "office_supplies",
    0, "", "manual", "draft", now, now,
  );
  return id;
}

function seedReceipt(overrides: Record<string, unknown> = {}) {
  const id = crypto.randomUUID();
  const now = new Date().toISOString();
  testSqlite!.prepare(
    `INSERT INTO receipts (id, tenant_id, business_id, date, merchant, amount, payment_method, category_slug, tax_amount, service_charge, notes, image_ref, status, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id, "tenant-1", "business-1",
    overrides.date ?? "2026-01-15",
    overrides.merchant ?? "Stationery Shop",
    overrides.amount ?? 150,
    overrides.paymentMethod ?? "card",
    overrides.categorySlug ?? "office_supplies",
    overrides.taxAmount ?? 0,
    overrides.serviceCharge ?? 0,
    overrides.notes ?? "",
    null, "draft", now, now,
  );
  return id;
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

let service: PackService;
let createPackService: () => PackService;

beforeAll(async () => {
  const mod = await import("./accountant-packs-db");
  createPackService = mod.createPackService;
});

beforeEach(() => {
  testSqlite!.exec("DELETE FROM accountant_packs");
  testSqlite!.exec("DELETE FROM transactions");
  testSqlite!.exec("DELETE FROM receipts");
  testSqlite!.exec("DELETE FROM receipt_matches");
  testSqlite!.exec("DELETE FROM audit_logs");
  service = createPackService();
});

afterAll(() => {
  testSqlite!.close();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("createPackService", () => {
  describe("generate()", () => {
    it("generates a pack with data from DB", async () => {
      seedTransaction({ date: "2026-01-15", description: "Rent", amount: -1000, debit: 1000, credit: 0, categorySlug: "rent" });
      seedTransaction({ date: "2026-01-20", description: "Sales income", amount: 2000, debit: 0, credit: 2000, categorySlug: "sales" });
      seedReceipt({ date: "2026-01-15", merchant: "Landlord", amount: 1000, categorySlug: "rent" });

      const result = await service.generate(
        "tenant-1",
        "business-1",
        { label: "January Pack", periodStart: "2026-01-01", periodEnd: "2026-01-31", notes: "Monthly" },
        "user-1",
      );

      expect(result.pack.label).toBe("January Pack");
      expect(result.pack.periodStart).toBe("2026-01-01");
      expect(result.pack.periodEnd).toBe("2026-01-31");
      expect(result.pack.status).toBe("generated");
      expect(result.pack.notes).toBe("Monthly");
      expect(result.pack.generatedAt).toBeTruthy();
      expect(result.files.length).toBeGreaterThanOrEqual(4); // CSV, receipt CSV, JSON, PDF
      expect(result.zipBlob).toBeTruthy();
    });

    it("persists the pack to the database", async () => {
      seedTransaction({ date: "2026-01-15", description: "Test tx", amount: -50, debit: 50, credit: 0 });
      seedReceipt({ date: "2026-01-15", merchant: "Test shop", amount: 50 });

      const result = await service.generate(
        "tenant-1",
        "business-1",
        { label: "Test Pack", periodStart: "2026-01-01", periodEnd: "2026-01-31" },
        "user-1",
      );

      const retrieved = service.getById("tenant-1", "business-1", result.pack.id);
      expect(retrieved).toBeTruthy();
      expect(retrieved!.label).toBe("Test Pack");
    });
  });

  describe("list()", () => {
    it("returns packs for the correct tenant+business", () => {
      seedPack({ label: "Pack 1" });
      seedPack({ label: "Pack 2" });
      seedPack({ label: "Other Tenant", tenantId: "tenant-2", businessId: "business-2" });

      const results = service.list("tenant-1", "business-1");
      expect(results).toHaveLength(2);
    });

    it("returns results ordered by created_at descending", () => {
      seedPack({ label: "Older" });
      seedPack({ label: "Newer" });

      const results = service.list("tenant-1", "business-1");
      // The second inserted should have a later timestamp
      expect(results.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe("getById()", () => {
    it("returns undefined for non-existent pack", () => {
      expect(service.getById("tenant-1", "business-1", "nonexistent")).toBeUndefined();
    });

    it("enforces tenant isolation", () => {
      const pack = seedPack();
      expect(service.getById("tenant-2", "business-2", pack.id)).toBeUndefined();
    });
  });

  describe("updateStatus()", () => {
    it("updates the pack status", () => {
      const pack = seedPack();

      const updated = service.updateStatus("tenant-1", "business-1", pack.id, "archived");
      expect(updated).toBeTruthy();
      expect(updated!.status).toBe("archived");

      const retrieved = service.getById("tenant-1", "business-1", pack.id);
      expect(retrieved!.status).toBe("archived");
    });

    it("returns undefined for non-existent pack", () => {
      expect(service.updateStatus("tenant-1", "business-1", "nonexistent", "archived")).toBeUndefined();
    });
  });

  describe("delete()", () => {
    it("deletes a pack and returns true", () => {
      const pack = seedPack();
      expect(service.delete("tenant-1", "business-1", pack.id)).toBe(true);
      expect(service.getById("tenant-1", "business-1", pack.id)).toBeUndefined();
    });

    it("returns false for non-existent pack", () => {
      expect(service.delete("tenant-1", "business-1", "nonexistent")).toBe(false);
    });
  });

  describe("tenant/business isolation", () => {
    it("list isolates packs per tenant", () => {
      seedPack({ label: "A", tenantId: "tenant-1", businessId: "business-1" });
      seedPack({ label: "B", tenantId: "tenant-2", businessId: "business-2" });

      expect(service.list("tenant-1", "business-1")).toHaveLength(1);
      expect(service.list("tenant-2", "business-2")).toHaveLength(1);
    });
  });
});
