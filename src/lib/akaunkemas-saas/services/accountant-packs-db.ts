import { db, schema } from "@/lib/db";
import { eq, and, desc } from "drizzle-orm";
import { createDbTransactionService } from "./transactions-db";
import { createDbReceiptService } from "./receipts-db";
import { createMatchRepository } from "./receipt-matches-db";
import { generateAccountantPack, generateAccountantPackZip } from "@/lib/akaunkemas/accountant-pack";
import type { AccountantPackFiles } from "@/lib/akaunkemas/types";
import type { CategorySlug, PaymentMethod } from "@/lib/akaunkemas/types";
import { logPackGenerated } from "../audit-helpers";
import crypto from "node:crypto";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SavedPack {
  id: string;
  tenantId: string;
  businessId: string;
  label: string;
  periodStart: string;
  periodEnd: string;
  status: "draft" | "generated" | "sent" | "archived";
  notes: string;
  fileUrl: string | null;
  generatedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface GeneratePackInput {
  label: string;
  periodStart: string;
  periodEnd: string;
  notes?: string;
}

export interface GeneratePackResult {
  pack: SavedPack;
  files: AccountantPackFiles[];
  zipBlob: Blob;
}

export interface PackService {
  generate(
    tenantId: string,
    businessId: string,
    input: GeneratePackInput,
    userId: string,
  ): Promise<GeneratePackResult>;

  list(tenantId: string, businessId: string): SavedPack[];

  getById(
    tenantId: string,
    businessId: string,
    id: string,
  ): SavedPack | undefined;

  updateStatus(
    tenantId: string,
    businessId: string,
    id: string,
    status: "draft" | "generated" | "sent" | "archived",
  ): SavedPack | undefined;

  delete(tenantId: string, businessId: string, id: string): boolean;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function rowToSavedPack(row: typeof schema.accountantPacks.$inferSelect): SavedPack {
  return {
    id: row.id,
    tenantId: row.tenantId,
    businessId: row.businessId,
    label: row.label,
    periodStart: row.periodStart,
    periodEnd: row.periodEnd,
    status: row.status as SavedPack["status"],
    notes: row.notes,
    fileUrl: row.fileUrl,
    generatedAt: row.generatedAt,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createPackService(): PackService {
  const txService = createDbTransactionService();
  const receiptService = createDbReceiptService();
  const matchRepo = createMatchRepository();

  return {
    async generate(tenantId, businessId, input, userId) {
      const { label, periodStart, periodEnd } = input;
      const notes = input.notes ?? "";

      // Load data in date range
      const transactions = txService.list(tenantId, businessId, {
        dateFrom: periodStart,
        dateTo: periodEnd,
      });

      const receipts = receiptService.list(tenantId, businessId, {
        dateFrom: periodStart,
        dateTo: periodEnd,
      });

      // Get matches to compute unmatched transactions
      const matches = matchRepo.getMatches(tenantId, businessId);
      const matchedTxIds = new Set(matches.map((m) => m.transaction.id));

      const unmatchedTransactions = transactions.filter((tx) => !matchedTxIds.has(tx.id));

      // Map DB types to matcher-compatible types for the pack generator
      const packTransactions = transactions.map((tx) => ({
        id: 0, // Not relevant for pack generation
        date: tx.date,
        description: tx.description,
        debit: tx.debit,
        credit: tx.credit,
        amount: tx.amount,
        balance: tx.balance,
        category: tx.categorySlug as CategorySlug,
      }));

      const packReceipts = receipts.map((r) => ({
        id: 0, // Not relevant
        date: r.date,
        merchant: r.merchant,
        amount: r.amount,
        paymentMethod: r.paymentMethod as PaymentMethod,
        category: r.categorySlug as CategorySlug,
        taxAmount: r.taxAmount,
        serviceCharge: r.serviceCharge,
        notes: r.notes,
        imageRef: r.imageRef,
      }));

      const packUnmatched = unmatchedTransactions.map((tx) => ({
        id: 0,
        date: tx.date,
        description: tx.description,
        debit: tx.debit,
        credit: tx.credit,
        amount: tx.amount,
        balance: tx.balance,
        category: tx.categorySlug as CategorySlug,
      }));

      // Generate pack files and ZIP
      const files = await generateAccountantPack({
        transactions: packTransactions,
        receipts: packReceipts,
        unmatchedTransactions: packUnmatched.length > 0 ? packUnmatched : undefined,
        notes: notes || undefined,
      });

      const zipBlob = await generateAccountantPackZip({
        transactions: packTransactions,
        receipts: packReceipts,
        unmatchedTransactions: packUnmatched.length > 0 ? packUnmatched : undefined,
        notes: notes || undefined,
      });

      // Save pack metadata to DB
      const id = crypto.randomUUID();
      const now = new Date().toISOString();

      db.insert(schema.accountantPacks)
        .values({
          id,
          tenantId,
          businessId,
          label,
          periodStart,
          periodEnd,
          status: "generated",
          notes,
          fileUrl: null, // Placeholder for future S3/local storage
          generatedAt: now,
          createdAt: now,
          updatedAt: now,
        })
        .run();

      // Audit log
      try {
        logPackGenerated(tenantId, businessId, userId, id, `Generated pack: ${label}`);
      } catch {
        // Audit failure must not break the operation
      }

      const pack: SavedPack = {
        id,
        tenantId,
        businessId,
        label,
        periodStart,
        periodEnd,
        status: "generated",
        notes,
        fileUrl: null,
        generatedAt: now,
        createdAt: now,
        updatedAt: now,
      };

      return { pack, files, zipBlob };
    },

    list(tenantId, businessId) {
      const rows = db
        .select()
        .from(schema.accountantPacks)
        .where(
          and(
            eq(schema.accountantPacks.tenantId, tenantId),
            eq(schema.accountantPacks.businessId, businessId),
          ),
        )
        .orderBy(desc(schema.accountantPacks.createdAt))
        .all();

      return rows.map(rowToSavedPack);
    },

    getById(tenantId, businessId, id) {
      const row = db
        .select()
        .from(schema.accountantPacks)
        .where(
          and(
            eq(schema.accountantPacks.id, id),
            eq(schema.accountantPacks.tenantId, tenantId),
            eq(schema.accountantPacks.businessId, businessId),
          ),
        )
        .get();

      return row ? rowToSavedPack(row) : undefined;
    },

    updateStatus(tenantId, businessId, id, status) {
      const existing = db
        .select()
        .from(schema.accountantPacks)
        .where(
          and(
            eq(schema.accountantPacks.id, id),
            eq(schema.accountantPacks.tenantId, tenantId),
            eq(schema.accountantPacks.businessId, businessId),
          ),
        )
        .get();

      if (!existing) return undefined;

      const now = new Date().toISOString();

      db.update(schema.accountantPacks)
        .set({
          status,
          updatedAt: now,
        })
        .where(
          and(
            eq(schema.accountantPacks.id, id),
            eq(schema.accountantPacks.tenantId, tenantId),
            eq(schema.accountantPacks.businessId, businessId),
          ),
        )
        .run();

      return rowToSavedPack({ ...existing, status, updatedAt: now });
    },

    delete(tenantId, businessId, id) {
      const existing = db
        .select()
        .from(schema.accountantPacks)
        .where(
          and(
            eq(schema.accountantPacks.id, id),
            eq(schema.accountantPacks.tenantId, tenantId),
            eq(schema.accountantPacks.businessId, businessId),
          ),
        )
        .get();

      if (!existing) return false;

      db.delete(schema.accountantPacks)
        .where(
          and(
            eq(schema.accountantPacks.id, id),
            eq(schema.accountantPacks.tenantId, tenantId),
            eq(schema.accountantPacks.businessId, businessId),
          ),
        )
        .run();

      return true;
    },
  };
}
