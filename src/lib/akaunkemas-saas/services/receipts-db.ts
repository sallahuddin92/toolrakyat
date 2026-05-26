import { db, schema } from "@/lib/db";
import { eq, and, desc, like, gte, lte, count, sum, type SQLWrapper } from "drizzle-orm";
import type {
  ReceiptService,
  SavedReceipt,
  CreateReceiptInput,
} from "./receipts";
import {
  CreateReceiptInputSchema,
  UpdateReceiptInputSchema,
} from "./receipts";
import { logReceiptCreated, logReceiptUpdated, logReceiptDeleted } from "../audit-helpers";
import crypto from "node:crypto";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function rowToSavedReceipt(row: typeof schema.receipts.$inferSelect): SavedReceipt {
  return {
    id: row.id,
    tenantId: row.tenantId,
    businessId: row.businessId,
    date: row.date,
    merchant: row.merchant,
    amount: row.amount,
    paymentMethod: row.paymentMethod as SavedReceipt["paymentMethod"],
    categorySlug: row.categorySlug,
    taxAmount: row.taxAmount,
    serviceCharge: row.serviceCharge,
    notes: row.notes,
    imageRef: row.imageRef,
    status: row.status as SavedReceipt["status"],
    createdAt: new Date(row.createdAt),
    updatedAt: new Date(row.updatedAt),
  };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createDbReceiptService(): ReceiptService {
  return {
    create(input: CreateReceiptInput): SavedReceipt {
      const parsed = CreateReceiptInputSchema.parse(input);
      const id = crypto.randomUUID();
      const now = new Date().toISOString();

      db.insert(schema.receipts)
        .values({
          id,
          tenantId: parsed.tenantId,
          businessId: parsed.businessId,
          date: parsed.date,
          merchant: parsed.merchant,
          amount: parsed.amount,
          paymentMethod: parsed.paymentMethod,
          categorySlug: parsed.categorySlug,
          taxAmount: parsed.taxAmount,
          serviceCharge: parsed.serviceCharge,
          notes: parsed.notes,
          imageRef: parsed.imageRef,
          status: parsed.status,
          createdAt: now,
          updatedAt: now,
        })
        .run();

      const record: SavedReceipt = {
        ...parsed,
        id,
        createdAt: new Date(now),
        updatedAt: new Date(now),
      };

      // Audit log
      try {
        logReceiptCreated(
          parsed.tenantId,
          parsed.businessId,
          "system",
          id,
          `Created receipt: ${parsed.merchant}`,
        );
      } catch {
        // Audit failure must not break the operation
      }

      return record;
    },

    getById(tenantId: string, businessId: string, id: string): SavedReceipt | undefined {
      const row = db
        .select()
        .from(schema.receipts)
        .where(
          and(
            eq(schema.receipts.id, id),
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .get();

      return row ? rowToSavedReceipt(row) : undefined;
    },

    list(tenantId, businessId, options) {
      const conditions: (SQLWrapper | undefined)[] = [
        eq(schema.receipts.tenantId, tenantId),
        eq(schema.receipts.businessId, businessId),
      ];

      if (options?.categorySlug) {
        conditions.push(eq(schema.receipts.categorySlug, options.categorySlug));
      }
      if (options?.paymentMethod) {
        conditions.push(eq(schema.receipts.paymentMethod, options.paymentMethod));
      }
      if (options?.merchant) {
        conditions.push(like(schema.receipts.merchant, `%${options.merchant}%`));
      }
      if (options?.dateFrom) {
        conditions.push(gte(schema.receipts.date, options.dateFrom));
      }
      if (options?.dateTo) {
        conditions.push(lte(schema.receipts.date, options.dateTo));
      }
      if (options?.status) {
        conditions.push(eq(schema.receipts.status, options.status));
      }

      const rows = db
        .select()
        .from(schema.receipts)
        .where(and(...conditions))
        .orderBy(desc(schema.receipts.date))
        .all();

      const offset = options?.offset ?? 0;
      const limit = options?.limit ?? 100;

      return rows.slice(offset, offset + limit).map(rowToSavedReceipt);
    },

    update(tenantId, businessId, id, input) {
      // Fetch existing record with tenant isolation
      const existing = db
        .select()
        .from(schema.receipts)
        .where(
          and(
            eq(schema.receipts.id, id),
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .get();

      if (!existing) return undefined;

      const parsed = UpdateReceiptInputSchema.parse(input);
      const now = new Date().toISOString();

      db.update(schema.receipts)
        .set({
          date: parsed.date ?? existing.date,
          merchant: parsed.merchant ?? existing.merchant,
          amount: parsed.amount ?? existing.amount,
          paymentMethod: parsed.paymentMethod ?? existing.paymentMethod,
          categorySlug: parsed.categorySlug ?? existing.categorySlug,
          taxAmount: parsed.taxAmount ?? existing.taxAmount,
          serviceCharge: parsed.serviceCharge ?? existing.serviceCharge,
          notes: parsed.notes ?? existing.notes,
          imageRef: parsed.imageRef ?? existing.imageRef,
          status: parsed.status ?? existing.status,
          updatedAt: now,
        })
        .where(
          and(
            eq(schema.receipts.id, id),
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .run();

      const updated = rowToSavedReceipt({
        ...existing,
        date: parsed.date ?? existing.date,
        merchant: parsed.merchant ?? existing.merchant,
        amount: parsed.amount ?? existing.amount,
        paymentMethod: parsed.paymentMethod ?? existing.paymentMethod,
        categorySlug: parsed.categorySlug ?? existing.categorySlug,
        taxAmount: parsed.taxAmount ?? existing.taxAmount,
        serviceCharge: parsed.serviceCharge ?? existing.serviceCharge,
        notes: parsed.notes ?? existing.notes,
        imageRef: parsed.imageRef ?? existing.imageRef,
        status: parsed.status ?? existing.status,
        updatedAt: now,
      });

      // Audit log
      try {
        logReceiptUpdated(
          tenantId,
          businessId,
          "system",
          id,
          `Updated receipt: ${updated.merchant}`,
        );
      } catch {
        // Audit failure must not break the operation
      }

      return updated;
    },

    delete(tenantId, businessId, id) {
      const existing = db
        .select()
        .from(schema.receipts)
        .where(
          and(
            eq(schema.receipts.id, id),
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .get();

      if (!existing) return false;

      db.delete(schema.receipts)
        .where(
          and(
            eq(schema.receipts.id, id),
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .run();

      // Audit log
      try {
        logReceiptDeleted(
          tenantId,
          businessId,
          "system",
          id,
          `Deleted receipt: ${existing.merchant}`,
        );
      } catch {
        // Audit failure must not break the operation
      }

      return true;
    },

    count(tenantId, businessId) {
      const result = db
        .select({ count: count() })
        .from(schema.receipts)
        .where(
          and(
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .get();

      return result?.count ?? 0;
    },

    getTotalAmount(tenantId, businessId) {
      const result = db
        .select({ total: sum(schema.receipts.amount) })
        .from(schema.receipts)
        .where(
          and(
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .get();

      const total = Number(result?.total) || 0;
      return total;
    },

    getByCategory(tenantId, businessId) {
      const rows = db
        .select({
          categorySlug: schema.receipts.categorySlug,
          total: sum(schema.receipts.amount),
          count: count(),
        })
        .from(schema.receipts)
        .where(
          and(
            eq(schema.receipts.tenantId, tenantId),
            eq(schema.receipts.businessId, businessId),
          ),
        )
        .groupBy(schema.receipts.categorySlug)
        .orderBy(desc(sum(schema.receipts.amount)))
        .all();

      return rows.map((row) => ({
        categorySlug: row.categorySlug,
        total: Number(row.total) || 0,
        count: row.count,
      }));
    },
  };
}
