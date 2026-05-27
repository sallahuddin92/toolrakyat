import { describe, it, expect, beforeEach } from "vitest";
import { ZodError } from "zod";
import { createReceiptService } from "./__fixtures__/receipts-memory";
import type { ReceiptService, CreateReceiptInput } from "./receipts";

// Minimal required (reasonable-default) fields
const RECEIPT_BASE: Omit<CreateReceiptInput, "tenantId" | "businessId"> = {
  date: "2025-06-15",
  merchant: "Seven Eleven Taman Desa",
  amount: 30.5,
  paymentMethod: "cash" as const,
  categorySlug: "office_supplies",
  taxAmount: 0,
  serviceCharge: 0,
  notes: "stationery",
  imageRef: null,
  status: "draft" as const,
};

function makeInput(
  overrides: Partial<CreateReceiptInput> = {},
): CreateReceiptInput {
  return {
    tenantId: "t1",
    businessId: "b1",
    ...RECEIPT_BASE,
    ...overrides,
  } as CreateReceiptInput;
}

describe("ReceiptService", () => {
  let svc: ReceiptService;

  beforeEach(() => {
    // every test gets a fresh in-memory service
    svc = createReceiptService();
  });

  // -------------------------------------------------------------------
  // create()
  // -------------------------------------------------------------------
  describe("create()", () => {
    it("parses input and returns a SavedReceipt with a generated id and timestamps", () => {
      const created = svc.create(makeInput());

      expect(created.id).toBeDefined();
      expect(typeof created.id).toBe("string");
      expect(created.id.length).toBeGreaterThan(0);

      expect(created.createdAt).toBeInstanceOf(Date);
      expect(created.updatedAt).toBeInstanceOf(Date);
      expect(created.createdAt.getTime()).toBe(created.updatedAt.getTime());
    });

    it("saves all supplied fields correctly", () => {
      const created = svc.create(makeInput({ merchant: "Mr DIY", amount: 99.9 }));

      expect(created.tenantId).toBe("t1");
      expect(created.businessId).toBe("b1");
      expect(created.date).toBe("2025-06-15");
      expect(created.merchant).toBe("Mr DIY");
      expect(created.amount).toBe(99.9);
      expect(created.paymentMethod).toBe("cash");
      expect(created.categorySlug).toBe("office_supplies");
      expect(created.taxAmount).toBe(0);
      expect(created.serviceCharge).toBe(0);
      expect(created.notes).toBe("stationery");
      expect(created.imageRef).toBeNull();
    });

    it("throws ZodError when tenantId is missing", () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const input = makeInput({ tenantId: undefined as any });
      expect(() => svc.create(input)).toThrow(ZodError);
    });

    it("throws ZodError when merchant is missing", () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const input = makeInput({ merchant: undefined as any });
      expect(() => svc.create(input)).toThrow(ZodError);
    });

    it("throws ZodError when amount is zero or negative", () => {
      expect(() => svc.create(makeInput({ amount: 0 }))).toThrow(ZodError);
      expect(() => svc.create(makeInput({ amount: -5 }))).toThrow(ZodError);
    });

    it("throws ZodError when date format is invalid", () => {
      expect(() =>
        svc.create(makeInput({ date: "15-06-2025" })),
      ).toThrow(ZodError);
      expect(() => svc.create(makeInput({ date: "not-a-date" }))).toThrow(
        ZodError,
      );
    });

    it("throws ZodError when paymentMethod is invalid", () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect(() => svc.create(makeInput({ paymentMethod: "bitcoin" as any }))).toThrow(
        ZodError,
      );
    });

    it("applies defaults when optional fields are omitted (via schema .parse with default)", () => {
      // Create input with only the bare required fields
      const created = svc.create({
        tenantId: "t9",
        businessId: "b9",
        date: "2025-01-01",
        merchant: "Test",
        amount: 10,
        // paymentMethod, categorySlug, taxAmount, serviceCharge, notes, imageRef omitted
      } as CreateReceiptInput);

      expect(created.paymentMethod).toBe("cash");
      expect(created.categorySlug).toBe("uncategorised");
      expect(created.taxAmount).toBe(0);
      expect(created.serviceCharge).toBe(0);
      expect(created.notes).toBe("");
      expect(created.imageRef).toBeNull();
    });
  });

  // -------------------------------------------------------------------
  // getById()
  // -------------------------------------------------------------------
  describe("getById()", () => {
    it("returns receipt for correct tenantId + businessId", () => {
      const created = svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      const found = svc.getById("t1", "b1", created.id);
      expect(found).toBeDefined();
      expect(found!.id).toBe(created.id);
    });

    it("returns undefined when the id doesn't exist", () => {
      expect(svc.getById("t1", "b1", "nonexistent")).toBeUndefined();
    });

    it("returns undefined for wrong tenantId (tenant isolation)", () => {
      const created = svc.create(makeInput({ tenantId: "t1" }));
      expect(svc.getById("t2", "b1", created.id)).toBeUndefined();
    });

    it("returns undefined for wrong businessId", () => {
      const created = svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      expect(svc.getById("t1", "b99", created.id)).toBeUndefined();
    });

    it("returns undefined when both tenantId and businessId are wrong", () => {
      const created = svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      expect(svc.getById("tX", "bX", created.id)).toBeUndefined();
    });
  });

  // -------------------------------------------------------------------
  // list()
  // -------------------------------------------------------------------
  describe("list()", () => {
    it("returns all receipts for the given tenant+business", () => {
      svc.create(makeInput({ tenantId: "t1", businessId: "b1", merchant: "A" }));
      svc.create(makeInput({ tenantId: "t1", businessId: "b1", merchant: "B" }));
      svc.create(makeInput({ tenantId: "t2", businessId: "b2", merchant: "C" }));

      const results = svc.list("t1", "b1");
      expect(results).toHaveLength(2);
      expect(results.every((r) => r.tenantId === "t1" && r.businessId === "b1")).toBe(true);
    });

    it("returns empty array when there are no receipts for the tenant+business", () => {
      expect(svc.list("t99", "b99")).toEqual([]);
    });

    it("filters by categorySlug", () => {
      svc.create(makeInput({ categorySlug: "sales", merchant: "X" }));
      svc.create(makeInput({ categorySlug: "utilities", merchant: "Y" }));
      svc.create(makeInput({ categorySlug: "sales", merchant: "Z" }));

      const results = svc.list("t1", "b1", { categorySlug: "sales" });
      expect(results).toHaveLength(2);
      expect(results.every((r) => r.categorySlug === "sales")).toBe(true);
    });

    it("filters by paymentMethod", () => {
      svc.create(makeInput({ paymentMethod: "cash", merchant: "A" }));
      svc.create(makeInput({ paymentMethod: "card", merchant: "B" }));
      svc.create(makeInput({ paymentMethod: "cash", merchant: "C" }));

      const results = svc.list("t1", "b1", { paymentMethod: "cash" });
      expect(results).toHaveLength(2);
      expect(results.every((r) => r.paymentMethod === "cash")).toBe(true);
    });

    it("searches by merchant name case-insensitive substring", () => {
      svc.create(makeInput({ merchant: "Seven Eleven" }));
      svc.create(makeInput({ merchant: "seven bakery" }));
      svc.create(makeInput({ merchant: "Giant Supermarket" }));

      const results = svc.list("t1", "b1", { merchant: "seven" });
      expect(results).toHaveLength(2);
      expect(results.map((r) => r.merchant)).toContain("Seven Eleven");
      expect(results.map((r) => r.merchant)).toContain("seven bakery");
    });

    it("filters by dateFrom (inclusive)", () => {
      svc.create(makeInput({ date: "2025-01-01", merchant: "A" }));
      svc.create(makeInput({ date: "2025-06-15", merchant: "B" }));
      svc.create(makeInput({ date: "2025-12-31", merchant: "C" }));

      const results = svc.list("t1", "b1", { dateFrom: "2025-06-15" });
      expect(results).toHaveLength(2);
      expect(results.map((r) => r.date)).toContain("2025-06-15");
      expect(results.map((r) => r.date)).toContain("2025-12-31");
    });

    it("filters by dateTo (inclusive)", () => {
      svc.create(makeInput({ date: "2025-01-01", merchant: "A" }));
      svc.create(makeInput({ date: "2025-06-15", merchant: "B" }));
      svc.create(makeInput({ date: "2025-12-31", merchant: "C" }));

      const results = svc.list("t1", "b1", { dateTo: "2025-06-15" });
      expect(results).toHaveLength(2);
      expect(results.map((r) => r.date)).toContain("2025-01-01");
      expect(results.map((r) => r.date)).toContain("2025-06-15");
    });

    it("combines dateFrom + dateTo for a range", () => {
      svc.create(makeInput({ date: "2025-03-01", merchant: "A" }));
      svc.create(makeInput({ date: "2025-06-15", merchant: "B" }));
      svc.create(makeInput({ date: "2025-09-30", merchant: "C" }));

      const results = svc.list("t1", "b1", {
        dateFrom: "2025-04-01",
        dateTo: "2025-08-31",
      });
      expect(results).toHaveLength(1);
      expect(results[0].date).toBe("2025-06-15");
    });

    it("respects limit and offset", () => {
      for (let i = 0; i < 10; i++) {
        svc.create(makeInput({ merchant: `M${i}`, date: `2025-01-${String(i + 1).padStart(2, "0")}` }));
      }

      const page1 = svc.list("t1", "b1", { limit: 3, offset: 0 });
      expect(page1).toHaveLength(3);

      const page2 = svc.list("t1", "b1", { limit: 3, offset: 3 });
      expect(page2).toHaveLength(3);

      // Ensure no overlap
      const ids1 = new Set(page1.map((r) => r.id));
      for (const r of page2) {
        expect(ids1.has(r.id)).toBe(false);
      }
    });

    it("handles limit larger than result set", () => {
      svc.create(makeInput());
      svc.create(makeInput());
      const results = svc.list("t1", "b1", { limit: 100 });
      expect(results).toHaveLength(2);
    });

    it("handles offset beyond result set", () => {
      svc.create(makeInput());
      const results = svc.list("t1", "b1", { offset: 10 });
      expect(results).toHaveLength(0);
    });
  });

  // -------------------------------------------------------------------
  // update()
  // -------------------------------------------------------------------
  describe("update()", () => {
    it("modifies allowed fields (merchant, amount, notes, etc.)", () => {
      const created = svc.create(makeInput({ merchant: "Original", amount: 50 }));

      const updated = svc.update("t1", "b1", created.id, {
        merchant: "Updated Merchant",
        notes: "new note",
      });
      expect(updated).toBeDefined();
      expect(updated!.merchant).toBe("Updated Merchant");
      expect(updated!.amount).toBe(50); // unchanged
      expect(updated!.notes).toBe("new note");
      expect(updated!.updatedAt.getTime()).toBeGreaterThanOrEqual(created.updatedAt.getTime());
    });

    it("keeps tenantId+businessId intact even if attempted in update payload", () => {
      const created = svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));

      // UpdateReceiptInputSchema omits tenantId and businessId, so they are stripped
      const updated = svc.update("t1", "b1", created.id, {
        merchant: "Changed",
      });

      expect(updated!.tenantId).toBe("t1");
      expect(updated!.businessId).toBe("b1");
    });

    it("returns undefined for wrong tenantId", () => {
      const created = svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      expect(svc.update("t2", "b1", created.id, { merchant: "X" })).toBeUndefined();
    });

    it("returns undefined for wrong businessId", () => {
      const created = svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      expect(svc.update("t1", "b99", created.id, { merchant: "X" })).toBeUndefined();
    });

    it("returns undefined for nonexistent id", () => {
      expect(svc.update("t1", "b1", "nonexistent", { merchant: "X" })).toBeUndefined();
    });

    it("throws ZodError if updated amount is negative", () => {
      const created = svc.create(makeInput());
      expect(() =>
        svc.update("t1", "b1", created.id, { amount: -1 }),
      ).toThrow(ZodError);
    });

    it("allows updating imageRef to null or a string", () => {
      const created = svc.create(makeInput({ imageRef: null }));
      const withRef = svc.update("t1", "b1", created.id, { imageRef: "/uploads/img1.jpg" });
      expect(withRef!.imageRef).toBe("/uploads/img1.jpg");

      const backToNull = svc.update("t1", "b1", created.id, { imageRef: null });
      expect(backToNull!.imageRef).toBeNull();
    });
  });

  // -------------------------------------------------------------------
  // delete()
  // -------------------------------------------------------------------
  describe("delete()", () => {
    it("removes the receipt and returns true", () => {
      const created = svc.create(makeInput());
      expect(svc.delete("t1", "b1", created.id)).toBe(true);
      expect(svc.getById("t1", "b1", created.id)).toBeUndefined();
    });

    it("returns false for nonexistent id", () => {
      expect(svc.delete("t1", "b1", "nonexistent")).toBe(false);
    });

    it("returns false for wrong tenantId", () => {
      const created = svc.create(makeInput({ tenantId: "t1" }));
      expect(svc.delete("t2", "b1", created.id)).toBe(false);
      // Receipt should still exist for the correct tenant
      expect(svc.getById("t1", "b1", created.id)).toBeDefined();
    });

    it("returns false for wrong businessId", () => {
      const created = svc.create(makeInput({ businessId: "b1" }));
      expect(svc.delete("t1", "b99", created.id)).toBe(false);
      // Receipt should still exist for the correct business
      expect(svc.getById("t1", "b1", created.id)).toBeDefined();
    });
  });

  // -------------------------------------------------------------------
  // count()
  // -------------------------------------------------------------------
  describe("count()", () => {
    it("returns 0 for fresh service", () => {
      expect(svc.count("t1", "b1")).toBe(0);
    });

    it("returns the correct count for a tenant+business", () => {
      svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      svc.create(makeInput({ tenantId: "t1", businessId: "b1" }));
      svc.create(makeInput({ tenantId: "t1", businessId: "b2" }));
      svc.create(makeInput({ tenantId: "t2", businessId: "b1" }));

      expect(svc.count("t1", "b1")).toBe(2);
      expect(svc.count("t1", "b2")).toBe(1);
      expect(svc.count("t2", "b1")).toBe(1);
      expect(svc.count("t99", "b99")).toBe(0);
    });
  });

  // -------------------------------------------------------------------
  // getTotalAmount()
  // -------------------------------------------------------------------
  describe("getTotalAmount()", () => {
    it("returns 0 for empty store", () => {
      expect(svc.getTotalAmount("t1", "b1")).toBe(0);
    });

    it("sums all receipt amounts for the given tenant+business", () => {
      svc.create(makeInput({ amount: 10 }));
      svc.create(makeInput({ amount: 20.5 }));
      svc.create(makeInput({ amount: 5 }));
      svc.create(makeInput({ tenantId: "t2", amount: 999 }));

      expect(svc.getTotalAmount("t1", "b1")).toBe(35.5);
    });

    it("returns 0 for a different tenant+business with no receipts", () => {
      svc.create(makeInput({ amount: 100 }));
      expect(svc.getTotalAmount("t99", "b99")).toBe(0);
    });
  });

  // -------------------------------------------------------------------
  // getByCategory()
  // -------------------------------------------------------------------
  describe("getByCategory()", () => {
    it("returns empty array when no receipts exist", () => {
      expect(svc.getByCategory("t1", "b1")).toEqual([]);
    });

    it("returns empty array for wrong tenantId", () => {
      svc.create(makeInput({ tenantId: "t1" }));
      expect(svc.getByCategory("t99", "b1")).toEqual([]);
    });

    it("groups receipts by categorySlug, summing amount and counting", () => {
      svc.create(makeInput({ categorySlug: "sales", amount: 100 }));
      svc.create(makeInput({ categorySlug: "sales", amount: 50 }));
      svc.create(makeInput({ categorySlug: "utilities", amount: 30 }));

      const groups = svc.getByCategory("t1", "b1");
      expect(groups).toHaveLength(2);

      const salesGroup = groups.find((g) => g.categorySlug === "sales")!;
      expect(salesGroup).toBeDefined();
      expect(salesGroup.total).toBe(150);
      expect(salesGroup.count).toBe(2);

      const utilGroup = groups.find((g) => g.categorySlug === "utilities")!;
      expect(utilGroup.total).toBe(30);
      expect(utilGroup.count).toBe(1);
    });

    it("sorts by total descending", () => {
      svc.create(makeInput({ categorySlug: "sales", amount: 10 }));
      svc.create(makeInput({ categorySlug: "utilities", amount: 200 }));
      svc.create(makeInput({ categorySlug: "rent", amount: 50 }));

      const groups = svc.getByCategory("t1", "b1");
      expect(groups).toHaveLength(3);
      expect(groups[0].categorySlug).toBe("utilities"); // 200
      expect(groups[1].categorySlug).toBe("rent"); // 50
      expect(groups[2].categorySlug).toBe("sales"); // 10
    });

    it("ignores receipts from other tenants/businesses", () => {
      svc.create(makeInput({ tenantId: "t1", businessId: "b1", categorySlug: "sales", amount: 10 }));
      svc.create(makeInput({ tenantId: "t2", businessId: "b1", categorySlug: "sales", amount: 999 }));

      const groups = svc.getByCategory("t1", "b1");
      expect(groups).toHaveLength(1);
      expect(groups[0].total).toBe(10);
    });
  });

  // -------------------------------------------------------------------
  // Edge cases and data integrity
  // -------------------------------------------------------------------
  describe("data integrity", () => {
    it("create() returns a new object (no reference leak)", () => {
      const input: CreateReceiptInput = makeInput({ merchant: "Original" });
      const created = svc.create(input);

      // Mutate input, shouldn't affect the stored receipt
      input.merchant = "Hacked";
      const fetched = svc.getById("t1", "b1", created.id);
      expect(fetched!.merchant).toBe("Original");
    });

    it("list() returns a new array on every call", () => {
      svc.create(makeInput());
      const a = svc.list("t1", "b1");
      const b = svc.list("t1", "b1");
      expect(a).not.toBe(b); // different array instances
    });

    it("updatedAt changes on update but createdAt stays the same", () => {
      const created = svc.create(makeInput());
      const originalCreatedAt = created.createdAt;

      // Small delay to ensure timestamp difference
      const updated = svc.update("t1", "b1", created.id, { notes: "updated" });
      expect(updated!.createdAt.getTime()).toBe(originalCreatedAt.getTime());
      expect(updated!.updatedAt.getTime()).toBeGreaterThanOrEqual(originalCreatedAt.getTime());
    });
  });
});
