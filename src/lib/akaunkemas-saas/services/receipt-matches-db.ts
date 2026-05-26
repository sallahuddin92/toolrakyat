import { db, schema } from "@/lib/db";
import { eq, and } from "drizzle-orm";
import type { SavedTransaction } from "./transactions";
import type { SavedReceipt } from "./receipts";
import { createDbTransactionService } from "./transactions-db";
import { createDbReceiptService } from "./receipts-db";
import { matchReceiptsToTransactions } from "@/lib/akaunkemas/receipt-matcher";
import type {
  Transaction,
  Receipt,
  MatchingReport,
  CategorySlug,
  PaymentMethod,
} from "@/lib/akaunkemas/types";
import { logMatchCreated, logMatchDeleted, logMatchManual } from "../audit-helpers";
import crypto from "node:crypto";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SavedMatch {
  id: string;
  tenantId: string;
  businessId: string;
  transactionId: string;
  receiptId: string;
  matchType: "exact" | "fuzzy" | "manual";
  dateDelta: number;
  amountDelta: number;
  matchedBy: string;
  createdAt: string;
}

export interface MatchWithDetails {
  match: SavedMatch;
  transaction: SavedTransaction;
  receipt: SavedReceipt;
}

export interface UnmatchedCounts {
  unmatchedTransactions: number;
  unmatchedReceipts: number;
}

export interface MatchRepository {
  /** Save a single match to the DB. Returns the saved match. */
  saveMatch(
    tenantId: string,
    businessId: string,
    transactionId: string,
    receiptId: string,
    matchType: "exact" | "fuzzy" | "manual",
    dateDelta: number,
    amountDelta: number,
    matchedBy: string,
  ): SavedMatch;

  /** Get all matches for tenant+business, with joined tx/receipt details. */
  getMatches(tenantId: string, businessId: string): MatchWithDetails[];

  /** Get raw match rows without joins. */
  getMatchRows(tenantId: string, businessId: string): SavedMatch[];

  /** Delete a specific match by transaction+receipt pair. */
  deleteMatch(
    tenantId: string,
    businessId: string,
    transactionId: string,
    receiptId: string,
  ): boolean;

  /** Delete all matches for tenant+business. */
  deleteAllMatches(tenantId: string, businessId: string): void;

  /** Count unmatched transactions and receipts. */
  getUnmatchedCounts(tenantId: string, businessId: string): UnmatchedCounts;

  /**
   * Run the matching engine on all transactions and receipts.
   * Clears existing auto-matches (exact/fuzzy) first, then runs the matcher.
   * Manual matches are preserved.
   */
  runMatching(
    tenantId: string,
    businessId: string,
    dateWindowDays?: number,
  ): {
    matched: MatchWithDetails[];
    unmatchedTransactions: SavedTransaction[];
    unmatchedReceipts: SavedReceipt[];
    manualMatches: MatchWithDetails[];
  };

  /** Add a manual match between a transaction and receipt. */
  addManualMatch(
    tenantId: string,
    businessId: string,
    transactionId: string,
    receiptId: string,
    matchedBy: string,
  ): SavedMatch;
}

// ---------------------------------------------------------------------------
// Helpers: ID mapping
// ---------------------------------------------------------------------------

/**
 * Map DB string UUIDs to sequential integer IDs for the matcher engine.
 * Returns mapped arrays and reverse-lookup maps.
 */
function dbToMatcher(
  transactions: SavedTransaction[],
  receipts: SavedReceipt[],
): {
  matcherTransactions: Transaction[];
  matcherReceipts: Receipt[];
  txIdToDbId: Map<number, string>;
  receiptIdToDbId: Map<number, string>;
} {
  const txIdToDbId = new Map<number, string>();
  const receiptIdToDbId = new Map<number, string>();

  const matcherTransactions: Transaction[] = transactions.map((tx, i) => {
    txIdToDbId.set(i, tx.id);
    return {
      id: i,
      date: tx.date,
      description: tx.description,
      debit: tx.debit,
      credit: tx.credit,
      amount: tx.amount,
      balance: tx.balance,
      category: tx.categorySlug as CategorySlug,
    };
  });

  const matcherReceipts: Receipt[] = receipts.map((r, i) => {
    receiptIdToDbId.set(i, r.id);
    return {
      id: i,
      date: r.date,
      merchant: r.merchant,
      amount: r.amount,
      paymentMethod: r.paymentMethod as PaymentMethod,
      category: r.categorySlug as CategorySlug,
      taxAmount: r.taxAmount,
      serviceCharge: r.serviceCharge,
      notes: r.notes,
      imageRef: r.imageRef,
    };
  });

  return {
    matcherTransactions,
    matcherReceipts,
    txIdToDbId,
    receiptIdToDbId,
  };
}

function rowToSavedMatch(row: typeof schema.receiptMatches.$inferSelect): SavedMatch {
  return {
    id: row.id,
    tenantId: row.tenantId,
    businessId: row.businessId,
    transactionId: row.transactionId,
    receiptId: row.receiptId,
    matchType: row.matchType as SavedMatch["matchType"],
    dateDelta: row.dateDelta,
    amountDelta: row.amountDelta,
    matchedBy: row.matchedBy,
    createdAt: row.createdAt,
  };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createMatchRepository(): MatchRepository {
  const txService = createDbTransactionService();
  const receiptService = createDbReceiptService();

  return {
    saveMatch(
      tenantId,
      businessId,
      transactionId,
      receiptId,
      matchType,
      dateDelta,
      amountDelta,
      matchedBy,
    ) {
      const id = crypto.randomUUID();
      const now = new Date().toISOString();

      // Check for existing duplicate match
      const existing = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
            eq(schema.receiptMatches.transactionId, transactionId),
            eq(schema.receiptMatches.receiptId, receiptId),
          ),
        )
        .get();

      if (existing) {
        return rowToSavedMatch(existing);
      }

      db.insert(schema.receiptMatches)
        .values({
          id,
          tenantId,
          businessId,
          transactionId,
          receiptId,
          matchType,
          dateDelta,
          amountDelta,
          matchedBy,
          createdAt: now,
        })
        .run();

      // Audit log
      try {
        if (matchType === "manual") {
          logMatchManual(tenantId, businessId, matchedBy, id, `Manual match created`);
        } else {
          logMatchCreated(tenantId, businessId, matchedBy, id, `Auto-match created (${matchType})`);
        }
      } catch {
        // Audit failure must not break the operation
      }

      return {
        id,
        tenantId,
        businessId,
        transactionId,
        receiptId,
        matchType,
        dateDelta,
        amountDelta,
        matchedBy,
        createdAt: now,
      };
    },

    getMatches(tenantId, businessId) {
      const rows = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .all();

      return rows.map((row) => {
        const match = rowToSavedMatch(row);
        const transaction = txService.getById(tenantId, businessId, match.transactionId);
        const receipt = receiptService.getById(tenantId, businessId, match.receiptId);

        return {
          match,
          transaction: transaction!,
          receipt: receipt!,
        };
      }).filter((m) => m.transaction && m.receipt);
    },

    getMatchRows(tenantId, businessId) {
      const rows = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .all();

      return rows.map(rowToSavedMatch);
    },

    deleteMatch(tenantId, businessId, transactionId, receiptId) {
      const existing = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
            eq(schema.receiptMatches.transactionId, transactionId),
            eq(schema.receiptMatches.receiptId, receiptId),
          ),
        )
        .get();

      if (!existing) return false;

      db.delete(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
            eq(schema.receiptMatches.transactionId, transactionId),
            eq(schema.receiptMatches.receiptId, receiptId),
          ),
        )
        .run();

      // Audit log
      try {
        logMatchDeleted(tenantId, businessId, "system", existing.id, `Match deleted`);
      } catch {
        // Audit failure must not break the operation
      }

      return true;
    },

    deleteAllMatches(tenantId, businessId) {
      db.delete(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .run();
    },

    getUnmatchedCounts(tenantId, businessId) {
      const allTransactions = txService.list(tenantId, businessId);
      const allReceipts = receiptService.list(tenantId, businessId);

      const matchRows = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .all();

      const matchedTxIds = new Set(matchRows.map((r) => r.transactionId));
      const matchedReceiptIds = new Set(matchRows.map((r) => r.receiptId));

      const unmatchedTransactions = allTransactions.filter((tx) => !matchedTxIds.has(tx.id)).length;
      const unmatchedReceipts = allReceipts.filter((r) => !matchedReceiptIds.has(r.id)).length;

      return { unmatchedTransactions, unmatchedReceipts };
    },

    runMatching(tenantId, businessId, dateWindowDays = 3) {
      // Load all transactions and receipts
      const allTransactions = txService.list(tenantId, businessId);
      const allReceipts = receiptService.list(tenantId, businessId);

      // Get existing manual matches (preserve them)
      const existingMatches = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .all();

      const manualMatches = existingMatches.filter((m) => m.matchType === "manual");
      const manualTxIds = new Set(manualMatches.map((m) => m.transactionId));
      const manualReceiptIds = new Set(manualMatches.map((m) => m.receiptId));

      // Filter out already-manually-matched items
      const availableTransactions = allTransactions.filter((tx) => !manualTxIds.has(tx.id));
      const availableReceipts = allReceipts.filter((r) => !manualReceiptIds.has(r.id));

      // Delete old auto-matches (exact/fuzzy) — keep manual
      db.delete(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .run();

      // Re-insert manual matches
      for (const m of manualMatches) {
        db.insert(schema.receiptMatches)
          .values({
            id: m.id,
            tenantId: m.tenantId,
            businessId: m.businessId,
            transactionId: m.transactionId,
            receiptId: m.receiptId,
            matchType: m.matchType,
            dateDelta: m.dateDelta,
            amountDelta: m.amountDelta,
            matchedBy: m.matchedBy,
            createdAt: m.createdAt,
          })
          .run();
      }

      // Map to matcher-compatible types
      const { matcherTransactions, matcherReceipts, txIdToDbId, receiptIdToDbId } =
        dbToMatcher(availableTransactions, availableReceipts);

      // Run the matcher
      const report: MatchingReport = matchReceiptsToTransactions(
        matcherTransactions,
        matcherReceipts,
        dateWindowDays,
      );

      // Save auto-matches to DB
      const now = new Date().toISOString();
      for (const result of report.matched) {
        const dbTxId = txIdToDbId.get(result.bankTxId);
        const dbReceiptId = receiptIdToDbId.get(result.receiptId);

        if (!dbTxId || !dbReceiptId) continue;

        const matchId = crypto.randomUUID();
        try {
          db.insert(schema.receiptMatches)
            .values({
              id: matchId,
              tenantId,
              businessId,
              transactionId: dbTxId,
              receiptId: dbReceiptId,
              matchType: result.matchType,
              dateDelta: result.dateDelta,
              amountDelta: result.amountDelta,
              matchedBy: "system",
              createdAt: now,
            })
            .run();

          // Audit log
          try {
            logMatchCreated(
              tenantId,
              businessId,
              "system",
              matchId,
              `Auto-match created (${result.matchType})`,
            );
          } catch {
            // Audit failure must not break the operation
          }
        } catch {
          // Skip duplicates
        }
      }

      // Build return data
      const matchRows = db
        .select()
        .from(schema.receiptMatches)
        .where(
          and(
            eq(schema.receiptMatches.tenantId, tenantId),
            eq(schema.receiptMatches.businessId, businessId),
          ),
        )
        .all();

      const allMatchedTxIds = new Set(matchRows.map((r) => r.transactionId));
      const allMatchedReceiptIds = new Set(matchRows.map((r) => r.receiptId));

      const matched: MatchWithDetails[] = [];
      const manualMatchDetails: MatchWithDetails[] = [];

      for (const row of matchRows) {
        const match = rowToSavedMatch(row);
        const transaction = txService.getById(tenantId, businessId, match.transactionId);
        const receipt = receiptService.getById(tenantId, businessId, match.receiptId);
        if (!transaction || !receipt) continue;

        const detail = { match, transaction, receipt };
        if (match.matchType === "manual") {
          manualMatchDetails.push(detail);
        } else {
          matched.push(detail);
        }
      }

      const unmatchedTransactions = allTransactions.filter(
        (tx) => !allMatchedTxIds.has(tx.id),
      );
      const unmatchedReceipts = allReceipts.filter(
        (r) => !allMatchedReceiptIds.has(r.id),
      );

      return {
        matched,
        unmatchedTransactions,
        unmatchedReceipts,
        manualMatches: manualMatchDetails,
      };
    },

    addManualMatch(tenantId, businessId, transactionId, receiptId, matchedBy) {
      // Verify both exist
      const tx = txService.getById(tenantId, businessId, transactionId);
      if (!tx) throw new Error(`Transaction ${transactionId} not found`);

      const receipt = receiptService.getById(tenantId, businessId, receiptId);
      if (!receipt) throw new Error(`Receipt ${receiptId} not found`);

      // Compute date/amount delta
      const txDate = new Date(tx.date);
      const receiptDate = new Date(receipt.date);
      const dateDelta = Math.abs(
        Math.floor((txDate.getTime() - receiptDate.getTime()) / (1000 * 60 * 60 * 24)),
      );
      const amountDelta = Math.abs(Math.abs(tx.amount) - receipt.amount);

      return this.saveMatch(
        tenantId,
        businessId,
        transactionId,
        receiptId,
        "manual",
        dateDelta,
        amountDelta,
        matchedBy,
      );
    },
  };
}
