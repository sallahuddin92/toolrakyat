import { z } from "zod";

// ---------------------------------------------------------------------------
// 1. SavedReceipt
// ---------------------------------------------------------------------------

export interface SavedReceipt {
  id: string;
  tenantId: string;
  businessId: string;
  date: string; // "YYYY-MM-DD"
  merchant: string;
  amount: number;
  paymentMethod: "cash" | "card" | "bank_transfer" | "e_wallet" | "cheque" | "other";
  categorySlug: string;
  taxAmount: number;
  serviceCharge: number;
  notes: string;
  imageRef: string | null;
  createdAt: Date;
  updatedAt: Date;
}

// ---------------------------------------------------------------------------
// 2. Create Receipt Input Schema
// ---------------------------------------------------------------------------

export const CreateReceiptInputSchema = z.object({
  tenantId: z.string().min(1),
  businessId: z.string().min(1),
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  merchant: z.string().min(1).max(200),
  amount: z.number().positive(),
  paymentMethod: z
    .enum(["cash", "card", "bank_transfer", "e_wallet", "cheque", "other"])
    .default("cash"),
  categorySlug: z.string().min(1).default("uncategorised"),
  taxAmount: z.number().min(0).default(0),
  serviceCharge: z.number().min(0).default(0),
  notes: z.string().max(1000).default(""),
  imageRef: z.string().nullable().default(null),
});

export type CreateReceiptInput = z.infer<typeof CreateReceiptInputSchema>;

// ---------------------------------------------------------------------------
// 3. Update Receipt Input Schema
// ---------------------------------------------------------------------------

export const UpdateReceiptInputSchema = CreateReceiptInputSchema.partial().omit({
  tenantId: true,
  businessId: true,
});

export type UpdateReceiptInput = z.infer<typeof UpdateReceiptInputSchema>;

// ---------------------------------------------------------------------------
// 4. ReceiptService Interface
// ---------------------------------------------------------------------------

export interface ReceiptService {
  create(input: CreateReceiptInput): SavedReceipt;
  getById(tenantId: string, businessId: string, id: string): SavedReceipt | undefined;
  list(
    tenantId: string,
    businessId: string,
    options?: {
      categorySlug?: string;
      paymentMethod?: string;
      merchant?: string; // search, case-insensitive substring
      dateFrom?: string;
      dateTo?: string;
      limit?: number;
      offset?: number;
    },
  ): SavedReceipt[];
  update(
    tenantId: string,
    businessId: string,
    id: string,
    input: z.infer<typeof UpdateReceiptInputSchema>,
  ): SavedReceipt | undefined;
  delete(tenantId: string, businessId: string, id: string): boolean;
  count(tenantId: string, businessId: string): number;
  getTotalAmount(tenantId: string, businessId: string): number;
  getByCategory(
    tenantId: string,
    businessId: string,
  ): { categorySlug: string; total: number; count: number }[];
}

// ---------------------------------------------------------------------------
// 5. Factory
// ---------------------------------------------------------------------------

export function createReceiptService(): ReceiptService {
  const store = new Map<string, SavedReceipt>();

  function generateId(): string {
    return crypto.randomUUID();
  }

  return {
    create(input) {
      const parsed = CreateReceiptInputSchema.parse(input);
      const now = new Date();
      const receipt: SavedReceipt = {
        ...parsed,
        id: generateId(),
        createdAt: now,
        updatedAt: now,
      };
      store.set(receipt.id, receipt);
      return receipt;
    },

    getById(tenantId, businessId, id) {
      const receipt = store.get(id);
      if (!receipt) return undefined;
      // Enforce tenant+business isolation
      if (receipt.tenantId !== tenantId || receipt.businessId !== businessId) return undefined;
      return receipt;
    },

    list(tenantId, businessId, options) {
      let results = Array.from(store.values()).filter(
        (r) => r.tenantId === tenantId && r.businessId === businessId,
      );

      // Filter by categorySlug
      if (options?.categorySlug) {
        results = results.filter((r) => r.categorySlug === options.categorySlug);
      }

      // Filter by paymentMethod
      if (options?.paymentMethod) {
        results = results.filter((r) => r.paymentMethod === options.paymentMethod);
      }

      // Search by merchant name (case-insensitive substring)
      if (options?.merchant) {
        const search = options.merchant.toLowerCase();
        results = results.filter((r) => r.merchant.toLowerCase().includes(search));
      }

      // Filter by dateFrom (inclusive)
      if (options?.dateFrom) {
        results = results.filter((r) => r.date >= options.dateFrom!);
      }

      // Filter by dateTo (inclusive)
      if (options?.dateTo) {
        results = results.filter((r) => r.date <= options.dateTo!);
      }

      // Pagination
      const offset = options?.offset ?? 0;
      const limit = options?.limit;

      if (limit !== undefined) {
        return results.slice(offset, offset + limit);
      }
      return results.slice(offset);
    },

    update(tenantId, businessId, id, input) {
      const receipt = store.get(id);
      if (!receipt) return undefined;
      // Enforce tenant+business isolation
      if (receipt.tenantId !== tenantId || receipt.businessId !== businessId) return undefined;

      // Parse the update input through the schema
      const parsed = UpdateReceiptInputSchema.parse(input);
      const updated: SavedReceipt = {
        ...receipt,
        ...parsed,
        updatedAt: new Date(),
      };
      store.set(id, updated);
      return updated;
    },

    delete(tenantId, businessId, id) {
      const receipt = store.get(id);
      if (!receipt) return false;
      // Enforce tenant+business isolation
      if (receipt.tenantId !== tenantId || receipt.businessId !== businessId) return false;
      return store.delete(id);
    },

    count(tenantId, businessId) {
      let count = 0;
      for (const [, r] of store) {
        if (r.tenantId === tenantId && r.businessId === businessId) {
          count++;
        }
      }
      return count;
    },

    getTotalAmount(tenantId, businessId) {
      let total = 0;
      for (const [, r] of store) {
        if (r.tenantId === tenantId && r.businessId === businessId) {
          total += r.amount;
        }
      }
      return total;
    },

    getByCategory(tenantId, businessId) {
      const map = new Map<string, { total: number; count: number }>();

      for (const [, r] of store) {
        if (r.tenantId !== tenantId || r.businessId !== businessId) continue;

        const existing = map.get(r.categorySlug);
        if (existing) {
          existing.total += r.amount;
          existing.count += 1;
        } else {
          map.set(r.categorySlug, { total: r.amount, count: 1 });
        }
      }

      // Convert to array and sort by total descending
      return Array.from(map.entries())
        .map(([categorySlug, data]) => ({
          categorySlug,
          total: data.total,
          count: data.count,
        }))
        .sort((a, b) => b.total - a.total);
    },
  };
}
