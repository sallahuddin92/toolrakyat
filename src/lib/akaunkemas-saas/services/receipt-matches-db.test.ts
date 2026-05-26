import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from "vitest";
import type { MatchRepository } from "./receipt-matches-db";

// ---------------------------------------------------------------------------
// In-memory SQLite DB reference (populated by vi.mock factory)
// ---------------------------------------------------------------------------

let testSqlite: any; // eslint-disable-line @typescript-eslint/no-explicit-any

vi.mock("@/lib/db", async () => {
  const BetterSqlite3 = await import("better-sqlite3");
  const sqlite = new BetterSqlite3.default(":memory:");
  sqlite.pragma("journal_mode = WAL");
  sqlite.pragma("foreign_keys = ON");

  // Create all tables needed for matches + transactions + receipts + audit
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

function seedTransaction(overrides: Record<string, unknown> = {}) {
  const id = overrides.id as string ?? crypto.randomUUID();
  const now = new Date().toISOString();
  const tenantId = (overrides.tenantId as string) ?? "tenant-1";
  const businessId = (overrides.businessId as string) ?? "business-1";
  testSqlite!.prepare(
    `INSERT INTO transactions (id, tenant_id, business_id, date, description, debit, credit, amount, balance, category_slug, is_reconciled, notes, source, status, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id, tenantId, businessId,
    overrides.date ?? "2026-01-15",
    overrides.description ?? "Office supplies",
    overrides.debit ?? 150, overrides.credit ?? 0,
    overrides.amount ?? -150,
    overrides.balance ?? null,
    overrides.categorySlug ?? "office_supplies",
    overrides.isReconciled ?? 0,
    overrides.notes ?? "",
    overrides.source ?? "manual",
    overrides.status ?? "draft",
    now, now,
  );
  return {
    id,
    tenantId,
    businessId,
    date: (overrides.date as string) ?? "2026-01-15",
    description: (overrides.description as string) ?? "Office supplies",
    debit: (overrides.debit as number) ?? 150,
    credit: (overrides.credit as number) ?? 0,
    amount: (overrides.amount as number) ?? -150,
  };
}

function seedReceipt(overrides: Record<string, unknown> = {}) {
  const id = overrides.id as string ?? crypto.randomUUID();
  const now = new Date().toISOString();
  const tenantId = (overrides.tenantId as string) ?? "tenant-1";
  const businessId = (overrides.businessId as string) ?? "business-1";
  testSqlite!.prepare(
    `INSERT INTO receipts (id, tenant_id, business_id, date, merchant, amount, payment_method, category_slug, tax_amount, service_charge, notes, image_ref, status, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    id, tenantId, businessId,
    overrides.date ?? "2026-01-15",
    overrides.merchant ?? "Stationery Shop",
    overrides.amount ?? 150,
    overrides.paymentMethod ?? "card",
    overrides.categorySlug ?? "office_supplies",
    overrides.taxAmount ?? 0,
    overrides.serviceCharge ?? 0,
    overrides.notes ?? "",
    overrides.imageRef ?? null,
    overrides.status ?? "draft",
    now, now,
  );
  return {
    id,
    tenantId,
    businessId,
    date: (overrides.date as string) ?? "2026-01-15",
    merchant: (overrides.merchant as string) ?? "Stationery Shop",
    amount: (overrides.amount as number) ?? 150,
  };
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

let repo: MatchRepository;
let createMatchRepository: () => MatchRepository;

beforeAll(async () => {
  const mod = await import("./receipt-matches-db");
  createMatchRepository = mod.createMatchRepository;
});

beforeEach(() => {
  testSqlite!.exec("DELETE FROM receipt_matches");
  testSqlite!.exec("DELETE FROM transactions");
  testSqlite!.exec("DELETE FROM receipts");
  testSqlite!.exec("DELETE FROM audit_logs");
  repo = createMatchRepository();
});

afterAll(() => {
  testSqlite!.close();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("createMatchRepository", () => {
  describe("saveMatch()", () => {
    it("saves a match and returns it", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();

      const saved = repo.saveMatch(
        "tenant-1", "business-1", tx.id, receipt.id,
        "exact", 0, 0, "system",
      );

      expect(saved.id).toBeTruthy();
      expect(saved.transactionId).toBe(tx.id);
      expect(saved.receiptId).toBe(receipt.id);
      expect(saved.matchType).toBe("exact");
      expect(saved.dateDelta).toBe(0);
      expect(saved.amountDelta).toBe(0);
    });

    it("returns existing match for duplicate pair", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();

      const first = repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "exact", 0, 0, "system");
      const second = repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "fuzzy", 1, 0.01, "system");

      expect(second.id).toBe(first.id);
      expect(second.matchType).toBe("exact"); // Original not overwritten
    });
  });

  describe("getMatches()", () => {
    it("returns matches with joined transaction and receipt details", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();
      repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "exact", 0, 0, "system");

      const matches = repo.getMatches("tenant-1", "business-1");
      expect(matches).toHaveLength(1);
      expect(matches[0]!.transaction.description).toBe("Office supplies");
      expect(matches[0]!.receipt.merchant).toBe("Stationery Shop");
      expect(matches[0]!.match.matchType).toBe("exact");
    });

    it("filters out matches where transaction or receipt is missing", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();
      repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "exact", 0, 0, "system");

      // Delete the transaction
      testSqlite!.prepare("DELETE FROM transactions WHERE id = ?").run(tx.id);

      const matches = repo.getMatches("tenant-1", "business-1");
      expect(matches).toHaveLength(0);
    });
  });

  describe("getMatchRows()", () => {
    it("returns raw match rows", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();
      repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "manual", 2, 1.5, "user-1");

      const rows = repo.getMatchRows("tenant-1", "business-1");
      expect(rows).toHaveLength(1);
      expect(rows[0]!.matchType).toBe("manual");
    });
  });

  describe("deleteMatch()", () => {
    it("deletes a match by transaction+receipt pair", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();
      repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "exact", 0, 0, "system");

      expect(repo.deleteMatch("tenant-1", "business-1", tx.id, receipt.id)).toBe(true);
      expect(repo.getMatchRows("tenant-1", "business-1")).toHaveLength(0);
    });

    it("returns false for non-existent match", () => {
      expect(repo.deleteMatch("tenant-1", "business-1", "no-id", "no-id")).toBe(false);
    });
  });

  describe("deleteAllMatches()", () => {
    it("deletes all matches for tenant+business", () => {
      const tx1 = seedTransaction({ description: "Tx 1" });
      const tx2 = seedTransaction({ description: "Tx 2" });
      const r1 = seedReceipt({ merchant: "Shop 1" });
      const r2 = seedReceipt({ merchant: "Shop 2" });

      repo.saveMatch("tenant-1", "business-1", tx1.id, r1.id, "exact", 0, 0, "system");
      repo.saveMatch("tenant-1", "business-1", tx2.id, r2.id, "fuzzy", 1, 0.01, "system");

      repo.deleteAllMatches("tenant-1", "business-1");
      expect(repo.getMatchRows("tenant-1", "business-1")).toHaveLength(0);
    });
  });

  describe("getUnmatchedCounts()", () => {
    it("counts unmatched transactions and receipts", () => {
      const tx1 = seedTransaction({ description: "Tx 1" });
      seedTransaction({ description: "Tx 2" });
      const r1 = seedReceipt({ merchant: "Shop 1" });
      seedReceipt({ merchant: "Shop 2" });

      // Match tx1 -> r1
      repo.saveMatch("tenant-1", "business-1", tx1.id, r1.id, "exact", 0, 0, "system");

      const counts = repo.getUnmatchedCounts("tenant-1", "business-1");
      expect(counts.unmatchedTransactions).toBe(1); // tx2
      expect(counts.unmatchedReceipts).toBe(1); // r2
    });

    it("returns 0 when everything is matched", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();
      repo.saveMatch("tenant-1", "business-1", tx.id, receipt.id, "exact", 0, 0, "system");

      const counts = repo.getUnmatchedCounts("tenant-1", "business-1");
      expect(counts.unmatchedTransactions).toBe(0);
      expect(counts.unmatchedReceipts).toBe(0);
    });
  });

  describe("runMatching()", () => {
    it("automatically matches transactions to receipts with exact amounts", () => {
      // Create matching pair: same date, same amount
      seedTransaction({ date: "2026-01-15", amount: -150, debit: 150, credit: 0, description: "Office supplies" });
      seedReceipt({ date: "2026-01-15", amount: 150, merchant: "Stationery Shop" });

      const result = repo.runMatching("tenant-1", "business-1", 3);

      expect(result.matched).toHaveLength(1);
      expect(result.matched[0]!.match.matchType).toBe("exact");
      expect(result.unmatchedTransactions).toHaveLength(0);
      expect(result.unmatchedReceipts).toHaveLength(0);
    });

    it("automatically matches with fuzzy amount tolerance", () => {
      seedTransaction({ date: "2026-01-15", amount: -150.005, debit: 150.005, credit: 0, description: "Office supplies" });
      seedReceipt({ date: "2026-01-15", amount: 150.01, merchant: "Stationery Shop" });

      const result = repo.runMatching("tenant-1", "business-1", 3);

      expect(result.matched).toHaveLength(1);
      expect(result.matched[0]!.match.matchType).toBe("fuzzy");
    });

    it("respects date window", () => {
      seedTransaction({ date: "2026-01-10", amount: -150, debit: 150, credit: 0, description: "Office supplies" });
      seedReceipt({ date: "2026-01-15", amount: 150, merchant: "Stationery Shop" });

      // With 3-day window, dates are 5 days apart -> no match
      const result = repo.runMatching("tenant-1", "business-1", 3);
      expect(result.matched).toHaveLength(0);

      // With 7-day window, dates are within range -> match
      const result2 = repo.runMatching("tenant-1", "business-1", 7);
      expect(result2.matched).toHaveLength(1);
    });

    it("preserves manual matches when re-running", () => {
      seedTransaction({ date: "2026-01-15", amount: -150, debit: 150, credit: 0, description: "Office supplies" });
      seedReceipt({ date: "2026-01-15", amount: 150, merchant: "Stationery Shop" });

      // Create a manual match
      const tx = seedTransaction({ date: "2026-01-20", amount: -200, debit: 200, credit: 0, description: "Manual pair" });
      const receipt = seedReceipt({ date: "2026-01-22", amount: 200, merchant: "Manual merchant" });
      repo.addManualMatch("tenant-1", "business-1", tx.id, receipt.id, "user-1");

      // Run matching
      const result = repo.runMatching("tenant-1", "business-1", 3);

      // Manual match should be preserved
      const manualMatches = result.manualMatches;
      expect(manualMatches).toHaveLength(1);
      expect(manualMatches[0]!.match.matchType).toBe("manual");

      // Auto match should also exist
      expect(result.matched).toHaveLength(1);
      expect(result.matched[0]!.match.matchType).toBe("exact");
    });

    it("excludes manually matched items from auto-matching", () => {
      seedTransaction({ date: "2026-01-15", amount: -150, debit: 150, credit: 0, description: "Only tx" });
      seedReceipt({ date: "2026-01-15", amount: 150, merchant: "Only receipt" });

      // Manual match first
      const rows = testSqlite!.prepare("SELECT * FROM transactions").all() as any[]; // eslint-disable-line @typescript-eslint/no-explicit-any
      const rRows = testSqlite!.prepare("SELECT * FROM receipts").all() as any[]; // eslint-disable-line @typescript-eslint/no-explicit-any
      repo.addManualMatch("tenant-1", "business-1", rows[0]!.id, rRows[0]!.id, "user-1");

      // Run matching — only manual match should exist (items already matched manually)
      repo.runMatching("tenant-1", "business-1", 3);
      const allMatches = repo.getMatchRows("tenant-1", "business-1");
      expect(allMatches).toHaveLength(1);
      expect(allMatches[0]!.matchType).toBe("manual");
    });
  });

  describe("addManualMatch()", () => {
    it("creates a manual match", () => {
      const tx = seedTransaction();
      const receipt = seedReceipt();

      const match = repo.addManualMatch("tenant-1", "business-1", tx.id, receipt.id, "user-1");

      expect(match.matchType).toBe("manual");
      expect(match.transactionId).toBe(tx.id);
      expect(match.receiptId).toBe(receipt.id);
      expect(match.matchedBy).toBe("user-1");
    });

    it("throws for non-existent transaction", () => {
      const receipt = seedReceipt();
      expect(() =>
        repo.addManualMatch("tenant-1", "business-1", "no-id", receipt.id, "user-1"),
      ).toThrow("Transaction no-id not found");
    });

    it("throws for non-existent receipt", () => {
      const tx = seedTransaction();
      expect(() =>
        repo.addManualMatch("tenant-1", "business-1", tx.id, "no-id", "user-1"),
      ).toThrow("Receipt no-id not found");
    });
  });

  describe("tenant/business isolation", () => {
    it("getMatches only returns matches for the correct tenant+business", () => {
      const tx1 = seedTransaction({ tenantId: "tenant-1", businessId: "business-1" });
      const r1 = seedReceipt({ tenantId: "tenant-1", businessId: "business-1" });
      repo.saveMatch("tenant-1", "business-1", tx1.id, r1.id, "exact", 0, 0, "system");

      // Create data in another tenant
      seedTransaction({ tenantId: "tenant-2", businessId: "business-2" });
      seedReceipt({ tenantId: "tenant-2", businessId: "business-2" });

      const rows = testSqlite!.prepare("SELECT * FROM transactions WHERE tenant_id = ?").all("tenant-2") as any[]; // eslint-disable-line @typescript-eslint/no-explicit-any
      const rRows = testSqlite!.prepare("SELECT * FROM receipts WHERE tenant_id = ?").all("tenant-2") as any[]; // eslint-disable-line @typescript-eslint/no-explicit-any
      repo.saveMatch("tenant-2", "business-2", rows[0]!.id, rRows[0]!.id, "exact", 0, 0, "system");

      expect(repo.getMatches("tenant-1", "business-1")).toHaveLength(1);
      expect(repo.getMatches("tenant-2", "business-2")).toHaveLength(1);
    });
  });
});
