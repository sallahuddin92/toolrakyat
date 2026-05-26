import { describe, it, expect, beforeEach } from "vitest";
import {
  createTransactionService,
  type TransactionService,
  type CreateTransactionInput,
} from "./transactions";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TENANT_A = "tenant-a";
const BUSINESS_A = "business-a";
const TENANT_B = "tenant-b";
const BUSINESS_B = "business-b";

function makeInput(
  overrides?: Partial<CreateTransactionInput>,
): CreateTransactionInput {
  return {
    tenantId: TENANT_A,
    businessId: BUSINESS_A,
    date: "2026-01-15",
    description: "Office supplies purchase",
    debit: 150.0,
    credit: 0,
    balance: null,
    categorySlug: "office_supplies",
    isReconciled: false,
    notes: "",
    source: "manual",
    status: "draft" as const,
    importHash: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("TransactionService", () => {
  let service: TransactionService;

  beforeEach(() => {
    service = createTransactionService();
  });

  // ---- create ---------------------------------------------------------

  describe("create()", () => {
    it("validates input with Zod (missing tenantId throws)", () => {
      expect(() =>
        service.create({} as CreateTransactionInput),
      ).toThrow();
    });

    it("returns SavedTransaction with generated id and timestamps", () => {
      const result = service.create(makeInput());

      expect(result.id).toBeTruthy();
      expect(typeof result.id).toBe("string");
      expect(result.tenantId).toBe(TENANT_A);
      expect(result.businessId).toBe(BUSINESS_A);
      expect(result.createdAt).toBeInstanceOf(Date);
      expect(result.updatedAt).toBeInstanceOf(Date);
    });

    it("computes amount as credit - debit", () => {
      const result = service.create(
        makeInput({ credit: 500, debit: 200 }),
      );

      expect(result.amount).toBe(300); // 500 - 200
    });

    it("applies defaults when fields are omitted", () => {
      const input = {
        tenantId: TENANT_A,
        businessId: BUSINESS_A,
        date: "2026-01-15",
        description: "Test",
        debit: 0,
        credit: 100,
      };

      const result = service.create(input as CreateTransactionInput);

      expect(result.categorySlug).toBe("uncategorised");
      expect(result.isReconciled).toBe(false);
      expect(result.notes).toBe("");
      expect(result.source).toBe("manual");
      expect(result.balance).toBeNull();
    });
  });

  // ---- getById --------------------------------------------------------

  describe("getById()", () => {
    it("returns the correct transaction", () => {
      const created = service.create(makeInput());
      const found = service.getById(TENANT_A, BUSINESS_A, created.id);

      expect(found).toBeDefined();
      expect(found!.id).toBe(created.id);
      expect(found!.description).toBe("Office supplies purchase");
    });

    it("returns undefined for wrong tenantId (tenant isolation)", () => {
      const created = service.create(makeInput());

      const found = service.getById(TENANT_B, BUSINESS_A, created.id);
      expect(found).toBeUndefined();
    });

    it("returns undefined for wrong businessId (business isolation)", () => {
      const created = service.create(makeInput());

      const found = service.getById(TENANT_A, BUSINESS_B, created.id);
      expect(found).toBeUndefined();
    });

    it("returns undefined for non-existent id", () => {
      const found = service.getById(TENANT_A, BUSINESS_A, "nonexistent");
      expect(found).toBeUndefined();
    });
  });

  // ---- list -----------------------------------------------------------

  describe("list()", () => {
    beforeEach(() => {
      // Tenant A / Business A
      service.create(makeInput({ date: "2026-01-10", description: "Rent payment", categorySlug: "rent", debit: 2000 }));
      service.create(makeInput({ date: "2026-01-15", description: "Office supplies", categorySlug: "office_supplies", debit: 150 }));
      service.create(makeInput({ date: "2026-01-20", description: "Sales income", categorySlug: "sales", credit: 5000 }));

      // Tenant A / Business B — should not appear in Business A queries
      service.create(makeInput({ businessId: BUSINESS_B, date: "2026-01-25", description: "Business B tx" }));

      // Tenant B / Business A — should not appear in Tenant A queries
      service.create(makeInput({ tenantId: TENANT_B, date: "2026-01-30", description: "Tenant B tx" }));
    });

    it("returns only transactions for the given tenant+business", () => {
      const results = service.list(TENANT_A, BUSINESS_A);

      expect(results).toHaveLength(3);
      for (const r of results) {
        expect(r.tenantId).toBe(TENANT_A);
        expect(r.businessId).toBe(BUSINESS_A);
      }
    });

    it("filters by categorySlug", () => {
      const results = service.list(TENANT_A, BUSINESS_A, { categorySlug: "rent" });

      expect(results).toHaveLength(1);
      expect(results[0].description).toBe("Rent payment");
    });

    it("filters by dateFrom/dateTo", () => {
      const results = service.list(TENANT_A, BUSINESS_A, {
        dateFrom: "2026-01-15",
        dateTo: "2026-01-20",
      });

      expect(results).toHaveLength(2); // Jan 15 and Jan 20
      const descs = results.map((r) => r.description);
      expect(descs).toContain("Office supplies");
      expect(descs).toContain("Sales income");
    });

    it("search matches description (case-insensitive)", () => {
      const results = service.list(TENANT_A, BUSINESS_A, {
        search: "SALES",
      });

      expect(results).toHaveLength(1);
      expect(results[0].description).toBe("Sales income");
    });

    it("respects limit and offset", () => {
      const results = service.list(TENANT_A, BUSINESS_A, {
        limit: 2,
        offset: 1,
      });

      expect(results).toHaveLength(2);
      // Sorted by date desc: Jan 20, Jan 15, Jan 10
      // offset 1 skips first => Jan 15 and Jan 10
      expect(results[0].description).toBe("Office supplies");
      expect(results[1].description).toBe("Rent payment");
    });
  });

  // ---- update ---------------------------------------------------------

  describe("update()", () => {
    it("modifies fields and updates updatedAt", () => {
      const created = service.create(makeInput());
      const originalUpdatedAt = created.updatedAt;

      // Small delay to ensure timestamp difference
      const updated = service.update(TENANT_A, BUSINESS_A, created.id, {
        description: "Updated office supplies",
        debit: 200,
      });

      expect(updated).toBeDefined();
      expect(updated!.description).toBe("Updated office supplies");
      expect(updated!.debit).toBe(200);
      expect(updated!.updatedAt.getTime()).toBeGreaterThanOrEqual(
        originalUpdatedAt.getTime(),
      );
      // Original fields unchanged
      expect(updated!.tenantId).toBe(TENANT_A);
      expect(updated!.date).toBe(created.date);
    });

    it("recomputes amount when debit or credit changes", () => {
      const created = service.create(makeInput({ debit: 100, credit: 500 }));
      expect(created.amount).toBe(400);

      const updated = service.update(TENANT_A, BUSINESS_A, created.id, {
        debit: 300,
      });

      expect(updated!.amount).toBe(200); // 500 - 300
    });

    it("returns undefined for wrong tenantId", () => {
      const created = service.create(makeInput());

      const result = service.update(TENANT_B, BUSINESS_A, created.id, {
        description: "Should not update",
      });

      expect(result).toBeUndefined();

      // Verify record unchanged
      const still = service.getById(TENANT_A, BUSINESS_A, created.id);
      expect(still!.description).toBe("Office supplies purchase");
    });

    it("returns undefined for non-existent id", () => {
      const result = service.update(TENANT_A, BUSINESS_A, "nonexistent", {
        description: "Nope",
      });
      expect(result).toBeUndefined();
    });
  });

  // ---- delete ---------------------------------------------------------

  describe("delete()", () => {
    it("removes the transaction", () => {
      const created = service.create(makeInput());

      const result = service.delete(TENANT_A, BUSINESS_A, created.id);
      expect(result).toBe(true);

      const found = service.getById(TENANT_A, BUSINESS_A, created.id);
      expect(found).toBeUndefined();
    });

    it("returns false for wrong tenantId", () => {
      const created = service.create(makeInput());

      const result = service.delete(TENANT_B, BUSINESS_A, created.id);
      expect(result).toBe(false);

      // Verify record still exists for the correct tenant
      const still = service.getById(TENANT_A, BUSINESS_A, created.id);
      expect(still).toBeDefined();
    });

    it("returns false for non-existent id", () => {
      const result = service.delete(TENANT_A, BUSINESS_A, "nonexistent");
      expect(result).toBe(false);
    });
  });

  // ---- count ----------------------------------------------------------

  describe("count()", () => {
    it("returns correct count", () => {
      expect(service.count(TENANT_A, BUSINESS_A)).toBe(0);

      service.create(makeInput());
      expect(service.count(TENANT_A, BUSINESS_A)).toBe(1);

      service.create(makeInput({ date: "2026-02-01" }));
      expect(service.count(TENANT_A, BUSINESS_A)).toBe(2);

      // Other tenant should have 0
      expect(service.count(TENANT_B, BUSINESS_A)).toBe(0);
    });
  });

  // ---- countUncategorised ---------------------------------------------

  describe("countUncategorised()", () => {
    it("returns correct count", () => {
      service.create(makeInput({ categorySlug: "rent" }));
      service.create(makeInput({ categorySlug: "uncategorised" }));
      service.create(makeInput({ categorySlug: "uncategorised" }));

      expect(service.countUncategorised(TENANT_A, BUSINESS_A)).toBe(2);

      // Other tenant should have 0
      expect(service.countUncategorised(TENANT_B, BUSINESS_A)).toBe(0);
    });
  });

  // ---- bulkCreate -----------------------------------------------------

  describe("bulkCreate()", () => {
    it("creates multiple transactions", () => {
      const results = service.bulkCreate(TENANT_A, BUSINESS_A, [
        { date: "2026-01-10", description: "Tx 1", debit: 100, credit: 0, categorySlug: "rent", notes: "", balance: null, isReconciled: false, source: "manual" as const, status: "draft" as const, importHash: null },
        { date: "2026-01-15", description: "Tx 2", debit: 0, credit: 200, categorySlug: "sales", notes: "", balance: null, isReconciled: false, source: "manual" as const, status: "draft" as const, importHash: null },
        { date: "2026-01-20", description: "Tx 3", debit: 50, credit: 0, categorySlug: "utilities", notes: "", balance: null, isReconciled: false, source: "manual" as const, status: "draft" as const, importHash: null },
      ]);

      expect(results).toHaveLength(3);
      expect(results[0].id).toBeTruthy();
      expect(results[1].id).toBeTruthy();
      expect(results[2].id).toBeTruthy();

      // All must have the same tenant+business
      for (const r of results) {
        expect(r.tenantId).toBe(TENANT_A);
        expect(r.businessId).toBe(BUSINESS_A);
      }

      // Verify count
      expect(service.count(TENANT_A, BUSINESS_A)).toBe(3);
    });

    it("validates each input with Zod", () => {
      expect(() =>
        service.bulkCreate(TENANT_A, BUSINESS_A, [
          { date: "2026-01-10", description: "Tx 1", debit: 100, credit: 0 },
          { date: "invalid-date", description: "", debit: -1, credit: 0 },
        ] as Omit<CreateTransactionInput, "tenantId" | "businessId">[]),
      ).toThrow();
    });
  });
});
