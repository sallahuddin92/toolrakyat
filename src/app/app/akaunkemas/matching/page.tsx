import { getCurrentUser } from "@/lib/auth/dal";
import { createMatchRepository } from "@/lib/akaunkemas-saas/services/receipt-matches-db";
import { MatchingClient } from "../_components/MatchingClient";

export const dynamic = "force-dynamic";

export default async function MatchingPage() {
  const session = await getCurrentUser();
  const { tenantId, businessId } = session;

  const matchRepo = createMatchRepository();
  const matches = matchRepo.getMatches(tenantId, businessId);

  // Load unmatched transactions and receipts
  const { createDbTransactionService } = await import(
    "@/lib/akaunkemas-saas/services/transactions-db"
  );
  const { createDbReceiptService } = await import(
    "@/lib/akaunkemas-saas/services/receipts-db"
  );

  const txService = createDbTransactionService();
  const receiptService = createDbReceiptService();

  const allTransactions = txService.list(tenantId, businessId);
  const allReceipts = receiptService.list(tenantId, businessId);

  const matchRows = matchRepo.getMatchRows(tenantId, businessId);
  const matchedTxIds = new Set(matchRows.map((m) => m.transactionId));
  const matchedReceiptIds = new Set(matchRows.map((m) => m.receiptId));

  const matched = matches.map((m) => ({
    transaction: {
      id: m.transaction.id,
      date: m.transaction.date,
      description: m.transaction.description,
      debit: m.transaction.debit,
      credit: m.transaction.credit,
      amount: m.transaction.amount,
    },
    receipt: {
      id: m.receipt.id,
      date: m.receipt.date,
      merchant: m.receipt.merchant,
      amount: m.receipt.amount,
    },
    matchType: m.match.matchType,
  }));

  const unmatchedTxs = allTransactions
    .filter((tx) => !matchedTxIds.has(tx.id))
    .map((tx) => ({
      id: tx.id,
      date: tx.date,
      description: tx.description,
      debit: tx.debit,
      credit: tx.credit,
      amount: tx.amount,
    }));

  const unmatchedReceipts = allReceipts
    .filter((r) => !matchedReceiptIds.has(r.id))
    .map((r) => ({
      id: r.id,
      date: r.date,
      merchant: r.merchant,
      amount: r.amount,
    }));

  return (
    <MatchingClient
      initialMatched={matched}
      initialUnmatchedTxs={unmatchedTxs}
      initialUnmatchedReceipts={unmatchedReceipts}
    />
  );
}
