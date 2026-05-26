import { db, schema } from "@/lib/db";
import { eq, and, desc, like, gte, lte, count, type SQLWrapper } from "drizzle-orm";
import type {
  TransactionService,
  SavedTransaction,
  CreateTransactionInput,
} from "./transactions";
import {
  CreateTransactionInputSchema,
  UpdateTransactionInputSchema,
} from "./transactions";
import { logTransactionCreated, logTransactionUpdated, logTransactionDeleted } from "../audit-helpers";
import crypto from "node:crypto";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function computeImportHash(
  businessId: string,
  date: string,
  description: string,
  amount: number,
): string {
  return crypto
    .createHash("sha256")
    .update(`${businessId}|${date}|${description}|${amount.toFixed(2)}`)
    .digest("hex");
}

function rowToSavedTransaction(row: typeof schema.transactions.$inferSelect): SavedTransaction {
  return {
    id: row.id,
    tenantId: row.tenantId,
    businessId: row.businessId,
    date: row.date,
    description: row.description,
    debit: row.debit,
    credit: row.credit,
    amount: row.amount,
    balance: row.balance,
    categorySlug: row.categorySlug,
    isReconciled: row.isReconciled === 1,
    notes: row.notes,
    source: row.source as SavedTransaction["source"],
    status: row.status as SavedTransaction["status"],
    importHash: row.importHash,
    createdAt: new Date(row.createdAt),
    updatedAt: new Date(row.updatedAt),
  };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createDbTransactionService(): TransactionService {
  return {
    create(input: CreateTransactionInput): SavedTransaction {
      const parsed = CreateTransactionInputSchema.parse(input);
      const id = crypto.randomUUID();
      const now = new Date().toISOString();
      const amount = parsed.credit - parsed.debit;

      // Compute import hash for CSV-imported transactions
      let importHash: string | null = null;
      if (parsed.source === "csv_import") {
        importHash = computeImportHash(
          parsed.businessId,
          parsed.date,
          parsed.description,
          amount,
        );
      }

      try {
        db.insert(schema.transactions)
          .values({
            id,
            tenantId: parsed.tenantId,
            businessId: parsed.businessId,
            date: parsed.date,
            description: parsed.description,
            debit: parsed.debit,
            credit: parsed.credit,
            amount,
            balance: parsed.balance,
            categorySlug: parsed.categorySlug,
            isReconciled: parsed.isReconciled ? 1 : 0,
            notes: parsed.notes,
            source: parsed.source,
            status: parsed.status,
            importHash,
            createdAt: now,
            updatedAt: now,
          })
          .run();
      } catch {
        // Duplicate CSV import — return existing record
        if (importHash) {
          const existing = db
            .select()
            .from(schema.transactions)
            .where(
              and(
                eq(schema.transactions.tenantId, parsed.tenantId),
                eq(schema.transactions.businessId, parsed.businessId),
                eq(schema.transactions.importHash, importHash),
              ),
            )
            .get();
          if (existing) return rowToSavedTransaction(existing);
        }
        throw new Error("Failed to create transaction");
      }

      const record: SavedTransaction = {
        id,
        tenantId: parsed.tenantId,
        businessId: parsed.businessId,
        date: parsed.date,
        description: parsed.description,
        debit: parsed.debit,
        credit: parsed.credit,
        amount,
        balance: parsed.balance,
        categorySlug: parsed.categorySlug,
        isReconciled: parsed.isReconciled,
        notes: parsed.notes,
        source: parsed.source,
        status: parsed.status,
        importHash,
        createdAt: new Date(now),
        updatedAt: new Date(now),
      };

      // Audit log
      try {
        logTransactionCreated(
          parsed.tenantId,
          parsed.businessId,
          "system", // userId will be overridden by caller context
          id,
          `Created transaction: ${parsed.description}`,
        );
      } catch {
        // Audit failure must not break the operation
      }

      return record;
    },

    bulkCreate(
      tenantId: string,
      businessId: string,
      inputs: Omit<CreateTransactionInput, "tenantId" | "businessId">[],
    ): SavedTransaction[] {
      const results: SavedTransaction[] = [];

      for (const input of inputs) {
        const fullInput = { ...input, tenantId, businessId };
        const parsed = CreateTransactionInputSchema.parse(fullInput);
        const id = crypto.randomUUID();
        const now = new Date().toISOString();
        const amount = parsed.credit - parsed.debit;

        let importHash: string | null = null;
        if (parsed.source === "csv_import") {
          importHash = computeImportHash(
            businessId,
            parsed.date,
            parsed.description,
            amount,
          );
        }

        try {
          db.insert(schema.transactions)
            .values({
              id,
              tenantId,
              businessId,
              date: parsed.date,
              description: parsed.description,
              debit: parsed.debit,
              credit: parsed.credit,
              amount,
              balance: parsed.balance,
              categorySlug: parsed.categorySlug,
              isReconciled: parsed.isReconciled ? 1 : 0,
              notes: parsed.notes,
              source: parsed.source,
              status: parsed.status,
              importHash,
              createdAt: now,
              updatedAt: now,
            })
            .run();

          results.push({
            id,
            tenantId,
            businessId,
            date: parsed.date,
            description: parsed.description,
            debit: parsed.debit,
            credit: parsed.credit,
            amount,
            balance: parsed.balance,
            categorySlug: parsed.categorySlug,
            isReconciled: parsed.isReconciled,
            notes: parsed.notes,
            source: parsed.source,
            status: parsed.status,
            importHash,
            createdAt: new Date(now),
            updatedAt: new Date(now),
          });
        } catch {
          // Skip duplicates silently
          continue;
        }
      }

      return results;
    },

    getById(tenantId: string, businessId: string, id: string): SavedTransaction | undefined {
      const row = db
        .select()
        .from(schema.transactions)
        .where(
          and(
            eq(schema.transactions.id, id),
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
          ),
        )
        .get();

      return row ? rowToSavedTransaction(row) : undefined;
    },

    list(tenantId, businessId, options) {
      const conditions: (SQLWrapper | undefined)[] = [
        eq(schema.transactions.tenantId, tenantId),
        eq(schema.transactions.businessId, businessId),
      ];

      if (options?.categorySlug) {
        conditions.push(eq(schema.transactions.categorySlug, options.categorySlug));
      }
      if (options?.isReconciled !== undefined) {
        conditions.push(
          eq(schema.transactions.isReconciled, options.isReconciled ? 1 : 0),
        );
      }
      if (options?.source) {
        conditions.push(eq(schema.transactions.source, options.source));
      }
      if (options?.status) {
        conditions.push(eq(schema.transactions.status, options.status));
      }
      if (options?.dateFrom) {
        conditions.push(gte(schema.transactions.date, options.dateFrom));
      }
      if (options?.dateTo) {
        conditions.push(lte(schema.transactions.date, options.dateTo));
      }
      if (options?.search) {
        conditions.push(like(schema.transactions.description, `%${options.search}%`));
      }

      const rows = db
        .select()
        .from(schema.transactions)
        .where(and(...conditions))
        .orderBy(desc(schema.transactions.date))
        .all();

      const offset = options?.offset ?? 0;
      const limit = options?.limit ?? 100;

      return rows.slice(offset, offset + limit).map(rowToSavedTransaction);
    },

    update(tenantId, businessId, id, input) {
      // Fetch existing record with tenant isolation
      const existing = db
        .select()
        .from(schema.transactions)
        .where(
          and(
            eq(schema.transactions.id, id),
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
          ),
        )
        .get();

      if (!existing) return undefined;

      // Reject updates to locked transactions
      if (existing.status === "locked") return undefined;

      const parsed = UpdateTransactionInputSchema.parse(input);
      const raw = input as Record<string, unknown>;
      const now = new Date().toISOString();

      const newDebit = ("debit" in raw) ? (parsed.debit ?? existing.debit) : existing.debit;
      const newCredit = ("credit" in raw) ? (parsed.credit ?? existing.credit) : existing.credit;
      const newAmount = newCredit - newDebit;

      db.update(schema.transactions)
        .set({
          date: parsed.date ?? existing.date,
          description: parsed.description ?? existing.description,
          debit: newDebit,
          credit: newCredit,
          amount: newAmount,
          balance: ("balance" in raw) ? (parsed.balance ?? existing.balance) : existing.balance,
          categorySlug: parsed.categorySlug ?? existing.categorySlug,
          isReconciled: parsed.isReconciled !== undefined
            ? (parsed.isReconciled ? 1 : 0)
            : existing.isReconciled,
          notes: ("notes" in raw) ? (parsed.notes ?? existing.notes) : existing.notes,
          source: parsed.source ?? existing.source,
          status: parsed.status ?? existing.status,
          updatedAt: now,
        })
        .where(
          and(
            eq(schema.transactions.id, id),
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
          ),
        )
        .run();

      // Build the updated record
      const updated = rowToSavedTransaction({
        ...existing,
        date: parsed.date ?? existing.date,
        description: parsed.description ?? existing.description,
        debit: newDebit,
        credit: newCredit,
        amount: newAmount,
        balance: ("balance" in raw) ? (parsed.balance ?? existing.balance) : existing.balance,
        categorySlug: parsed.categorySlug ?? existing.categorySlug,
        isReconciled: parsed.isReconciled !== undefined
          ? (parsed.isReconciled ? 1 : 0)
          : existing.isReconciled,
        notes: ("notes" in raw) ? (parsed.notes ?? existing.notes) : existing.notes,
        source: parsed.source ?? existing.source,
        status: parsed.status ?? existing.status,
        updatedAt: now,
      });

      // Audit log
      try {
        logTransactionUpdated(
          tenantId,
          businessId,
          "system",
          id,
          `Updated transaction: ${updated.description}`,
        );
      } catch {
        // Audit failure must not break the operation
      }

      return updated;
    },

    delete(tenantId, businessId, id) {
      const existing = db
        .select()
        .from(schema.transactions)
        .where(
          and(
            eq(schema.transactions.id, id),
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
          ),
        )
        .get();

      if (!existing) return false;

      // Reject deletion of locked transactions
      if (existing.status === "locked") return false;

      db.delete(schema.transactions)
        .where(
          and(
            eq(schema.transactions.id, id),
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
          ),
        )
        .run();

      // Audit log
      try {
        logTransactionDeleted(
          tenantId,
          businessId,
          "system",
          id,
          `Deleted transaction: ${existing.description}`,
        );
      } catch {
        // Audit failure must not break the operation
      }

      return true;
    },

    count(tenantId, businessId) {
      const result = db
        .select({ count: count() })
        .from(schema.transactions)
        .where(
          and(
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
          ),
        )
        .get();

      return result?.count ?? 0;
    },

    countUncategorised(tenantId, businessId) {
      const result = db
        .select({ count: count() })
        .from(schema.transactions)
        .where(
          and(
            eq(schema.transactions.tenantId, tenantId),
            eq(schema.transactions.businessId, businessId),
            eq(schema.transactions.categorySlug, "uncategorised"),
          ),
        )
        .get();

      return result?.count ?? 0;
    },
  };
}
