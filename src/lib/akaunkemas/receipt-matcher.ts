import type { MatchingReport, MatchResult, Receipt, Transaction } from "./types";

// ---------------------------------------------------------------------------
// Date helpers
// ---------------------------------------------------------------------------

/**
 * Parse a date string in one of the supported formats:
 * - "YYYY-MM-DD"
 * - "DD/MM/YYYY"
 * - "DD-MM-YYYY"
 *
 * Returns a Date at midnight in the local timezone.
 */
export function parseDateStr(s: string): Date {
  const trimmed = s.trim();

  // YYYY-MM-DD
  if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) {
    const [y, m, d] = trimmed.split("-").map(Number);
    return new Date(y, m - 1, d);
  }

  // DD/MM/YYYY
  if (/^\d{2}\/\d{2}\/\d{4}$/.test(trimmed)) {
    const [d, m, y] = trimmed.split("/").map(Number);
    return new Date(y, m - 1, d);
  }

  // DD-MM-YYYY
  if (/^\d{2}-\d{2}-\d{4}$/.test(trimmed)) {
    const [d, m, y] = trimmed.split("-").map(Number);
    return new Date(y, m - 1, d);
  }

  // Fallback: try native parsing
  return new Date(trimmed);
}

/**
 * Return the absolute number of days between two dates (rounded down).
 */
export function daysBetween(a: Date, b: Date): number {
  const utcA = Date.UTC(a.getFullYear(), a.getMonth(), a.getDate());
  const utcB = Date.UTC(b.getFullYear(), b.getMonth(), b.getDate());
  return Math.floor(Math.abs(utcA - utcB) / (1000 * 60 * 60 * 24));
}

// ---------------------------------------------------------------------------
// Amount helpers
// ---------------------------------------------------------------------------

/**
 * Compare two amounts using their absolute values.
 * When `tolerance` is provided, the absolute difference must be <= tolerance.
 * Default tolerance is 0 (exact match required).
 */
export function amountsMatch(a: number, b: number, tolerance: number = 0): boolean {
  return Math.abs(Math.abs(a) - Math.abs(b)) <= tolerance;
}

// ---------------------------------------------------------------------------
// Internal matching engine
// ---------------------------------------------------------------------------

function runMatching(
  bankTransactions: Transaction[],
  receipts: Receipt[],
  dateWindowDays: number,
  alreadyMatchedBankIds: Set<number>,
  alreadyMatchedReceiptIds: Set<number>,
): MatchResult[] {
  const results: MatchResult[] = [];

  // --- Exact match pass ---
  const availableBank = bankTransactions.filter(
    (tx) => !alreadyMatchedBankIds.has(tx.id),
  );

  for (const bankTx of availableBank) {
    if (alreadyMatchedBankIds.has(bankTx.id)) continue;

    const absTxAmount = Math.abs(bankTx.amount);
    const txDate = parseDateStr(bankTx.date);

    const candidates = receipts
      .filter((r) => !alreadyMatchedReceiptIds.has(r.id))
      .filter((r) => amountsMatch(absTxAmount, r.amount, 0))
      .filter((r) => daysBetween(txDate, parseDateStr(r.date)) <= dateWindowDays)
      .sort(
        (a, b) =>
          daysBetween(txDate, parseDateStr(a.date)) -
          daysBetween(txDate, parseDateStr(b.date)),
      );

    if (candidates.length > 0) {
      const receipt = candidates[0];
      const dateDelta = daysBetween(txDate, parseDateStr(receipt.date));
      const amountDelta = Math.abs(absTxAmount - receipt.amount);

      results.push({
        bankTxId: bankTx.id,
        receiptId: receipt.id,
        matchType: "exact",
        dateDelta,
        amountDelta,
      });

      alreadyMatchedBankIds.add(bankTx.id);
      alreadyMatchedReceiptIds.add(receipt.id);
    }
  }

  // --- Fuzzy match pass ---
  const fuzzyAvailableBank = bankTransactions.filter(
    (tx) => !alreadyMatchedBankIds.has(tx.id),
  );

  for (const bankTx of fuzzyAvailableBank) {
    if (alreadyMatchedBankIds.has(bankTx.id)) continue;

    const absTxAmount = Math.abs(bankTx.amount);
    const txDate = parseDateStr(bankTx.date);

    const candidates = receipts
      .filter((r) => !alreadyMatchedReceiptIds.has(r.id))
      .filter((r) => amountsMatch(absTxAmount, r.amount, 0.01))
      .filter((r) => daysBetween(txDate, parseDateStr(r.date)) <= dateWindowDays)
      .sort(
        (a, b) =>
          daysBetween(txDate, parseDateStr(a.date)) -
          daysBetween(txDate, parseDateStr(b.date)),
      );

    if (candidates.length > 0) {
      const receipt = candidates[0];
      const dateDelta = daysBetween(txDate, parseDateStr(receipt.date));
      const amountDelta = Math.abs(absTxAmount - receipt.amount);

      results.push({
        bankTxId: bankTx.id,
        receiptId: receipt.id,
        matchType: "fuzzy",
        dateDelta,
        amountDelta,
      });

      alreadyMatchedBankIds.add(bankTx.id);
      alreadyMatchedReceiptIds.add(receipt.id);
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Main export
// ---------------------------------------------------------------------------

/**
 * Match receipts to bank transactions.
 *
 * Matching is deterministic and greedy:
 * 1. **Exact**: `abs(bankTx.amount)` == receipt.amount AND dates within ±window
 * 2. **Fuzzy**: same but with 0.01 amount tolerance (rounding)
 *
 * In each pass, when multiple receipts match a bank transaction, the one with
 * the closest date is preferred. Each bank tx and receipt is used at most once.
 */
export function matchReceiptsToTransactions(
  bankTransactions: Transaction[],
  receipts: Receipt[],
  dateWindowDays: number = 3,
): MatchingReport {
  const matchedBankIds = new Set<number>();
  const matchedReceiptIds = new Set<number>();

  const matched = runMatching(
    bankTransactions,
    receipts,
    dateWindowDays,
    matchedBankIds,
    matchedReceiptIds,
  );

  const unmatchedBank = bankTransactions.filter(
    (tx) => !matchedBankIds.has(tx.id),
  );
  const unmatchedReceipts = receipts.filter(
    (r) => !matchedReceiptIds.has(r.id),
  );

  return { matched, unmatchedBank, unmatchedReceipts, dateWindowDays };
}

// ---------------------------------------------------------------------------
// Manual operations on an existing report
// ---------------------------------------------------------------------------

/**
 * Add a manual match between an unmatched bank transaction and an unmatched
 * receipt. Both items must be present in the unmatched lists.
 *
 * `allBank` and `allReceipts` are the original full lists used to look up
 * transaction and receipt objects when they get moved back to unmatched
 * during a subsequent `removeMatch`. Pass the same arrays you gave to
 * `matchReceiptsToTransactions`.
 */
export function addManualMatch(
  report: MatchingReport,
  bankTxId: number,
  receiptId: number,
  _allBank: Transaction[],
  _allReceipts: Receipt[],
): MatchingReport {
  const bankTx = report.unmatchedBank.find((tx) => tx.id === bankTxId);
  const receipt = report.unmatchedReceipts.find((r) => r.id === receiptId);

  if (!bankTx) {
    throw new Error(`Bank transaction ${bankTxId} is not in the unmatched list.`);
  }
  if (!receipt) {
    throw new Error(`Receipt ${receiptId} is not in the unmatched list.`);
  }

  const txDate = parseDateStr(bankTx.date);
  const receiptDate = parseDateStr(receipt.date);

  const matchResult: MatchResult = {
    bankTxId,
    receiptId,
    matchType: "manual",
    dateDelta: daysBetween(txDate, receiptDate),
    amountDelta: Math.abs(Math.abs(bankTx.amount) - receipt.amount),
  };

  return {
    matched: [...report.matched, matchResult],
    unmatchedBank: report.unmatchedBank.filter((tx) => tx.id !== bankTxId),
    unmatchedReceipts: report.unmatchedReceipts.filter((r) => r.id !== receiptId),
    dateWindowDays: report.dateWindowDays,
  };
}

/**
 * Remove a match (of any type) from the report. The bank transaction and
 * receipt are returned to their respective unmatched lists.
 *
 * `allBank` and `allReceipts` are the original full lists used to look up
 * the transaction and receipt objects. This is necessary because the report
 * only retains the unmatched subsets.
 */
export function removeMatch(
  report: MatchingReport,
  bankTxId: number,
  receiptId: number,
  allBank: Transaction[],
  allReceipts: Receipt[],
): MatchingReport {
  const idx = report.matched.findIndex(
    (m) => m.bankTxId === bankTxId && m.receiptId === receiptId,
  );

  if (idx === -1) {
    throw new Error(
      `No match found for bankTxId=${bankTxId} and receiptId=${receiptId}.`,
    );
  }

  const bankTx = allBank.find((tx) => tx.id === bankTxId);
  const receipt = allReceipts.find((r) => r.id === receiptId);

  if (!bankTx) {
    throw new Error(
      `Bank transaction ${bankTxId} not in the full bank list.`,
    );
  }
  if (!receipt) {
    throw new Error(`Receipt ${receiptId} not in the full receipt list.`);
  }

  const newMatched = [...report.matched];
  newMatched.splice(idx, 1);

  return {
    matched: newMatched,
    unmatchedBank: [...report.unmatchedBank, bankTx],
    unmatchedReceipts: [...report.unmatchedReceipts, receipt],
    dateWindowDays: report.dateWindowDays,
  };
}

/**
 * Re-run the matching engine on the currently unmatched items in a report.
 * Existing matches (exact, fuzzy, or manual) are preserved.
 *
 * `allBank` and `allReceipts` are the original full lists; they are used as
 * the reference set so that matched items can be correctly identified.
 */
export function recomputeMatches(
  report: MatchingReport,
  allBank: Transaction[],
  allReceipts: Receipt[],
): MatchingReport {
  const alreadyMatchedBankIds = new Set(report.matched.map((m) => m.bankTxId));
  const alreadyMatchedReceiptIds = new Set(
    report.matched.map((m) => m.receiptId),
  );

  const newMatches = runMatching(
    allBank,
    allReceipts,
    report.dateWindowDays,
    alreadyMatchedBankIds,
    alreadyMatchedReceiptIds,
  );

  const allMatched = [...report.matched, ...newMatches];

  const unmatchedBank = allBank.filter(
    (tx) => !alreadyMatchedBankIds.has(tx.id),
  );
  const unmatchedReceipts = allReceipts.filter(
    (r) => !alreadyMatchedReceiptIds.has(r.id),
  );

  return {
    matched: allMatched,
    unmatchedBank,
    unmatchedReceipts,
    dateWindowDays: report.dateWindowDays,
  };
}
