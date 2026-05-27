/**
 * In-memory ReceiptService for unit tests.
 *
 * This is NOT a production implementation — it stores receipts in a Map
 * and has no persistence. Use createDbReceiptService() from ../receipts-db
 * for production code.
 */
import { z } from "zod";
import {
  type ReceiptService,
  type SavedReceipt,
  type CreateReceiptInput,
  CreateReceiptInputSchema,
  UpdateReceiptInputSchema,
} from "../receipts";

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
      if (receipt.tenantId !== tenantId || receipt.businessId !== businessId) return undefined;
      return receipt;
    },

    list(tenantId, businessId, options) {
      let results = Array.from(store.values()).filter(
        (r) => r.tenantId === tenantId && r.businessId === businessId,
      );

      if (options?.categorySlug) {
        results = results.filter((r) => r.categorySlug === options.categorySlug);
      }
      if (options?.paymentMethod) {
        results = results.filter((r) => r.paymentMethod === options.paymentMethod);
      }
      if (options?.merchant) {
        const search = options.merchant.toLowerCase();
        results = results.filter((r) => r.merchant.toLowerCase().includes(search));
      }
      if (options?.dateFrom) {
        results = results.filter((r) => r.date >= options.dateFrom!);
      }
      if (options?.dateTo) {
        results = results.filter((r) => r.date <= options.dateTo!);
      }
      if (options?.status) {
        results = results.filter((r) => r.status === options.status);
      }

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
      if (receipt.tenantId !== tenantId || receipt.businessId !== businessId) return undefined;

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
