import type { CategorySlug, LedgerEntry, LedgerTotals, Transaction } from "./types";

// ---------------------------------------------------------------------------
// Core Ledger Operations
// ---------------------------------------------------------------------------

/**
 * Convert bank transactions to ledger entries. Computes running balance.
 *
 * Running-balance logic:
 * - If the transaction has an explicit `balance`, use it directly.
 * - Otherwise compute from the previous running balance + credit - debit.
 * - The initial running balance is 0 (or the first transaction's balance if
 *   available).
 */
export function transactionsToLedger(transactions: Transaction[]): LedgerEntry[] {
  const entries: LedgerEntry[] = [];
  let runningBalance = 0;

  for (const tx of transactions) {
    if (tx.balance !== null) {
      runningBalance = tx.balance;
    } else {
      runningBalance = runningBalance + tx.credit - tx.debit;
    }

    entries.push({
      id: tx.id,
      date: tx.date,
      description: tx.description,
      category: tx.category,
      debit: tx.debit,
      credit: tx.credit,
      runningBalance,
      notes: "",
    });
  }

  return entries;
}

/** Filter ledger entries to those in a given YYYY-MM month (e.g. "2024-01"). */
export function filterLedgerByMonth(
  entries: LedgerEntry[],
  yearMonth: string,
): LedgerEntry[] {
  return entries.filter((e) => e.date.startsWith(yearMonth));
}

/** Filter ledger entries to those matching a given category slug. */
export function filterLedgerByCategory(
  entries: LedgerEntry[],
  category: CategorySlug,
): LedgerEntry[] {
  return entries.filter((e) => e.category === category);
}

/** Compute totals (debit, credit, net cashflow) for a set of ledger entries. */
export function computeLedgerTotals(entries: LedgerEntry[]): LedgerTotals {
  const totalDebit = entries.reduce((sum, e) => sum + e.debit, 0);
  const totalCredit = entries.reduce((sum, e) => sum + e.credit, 0);

  return {
    totalDebit,
    totalCredit,
    netCashflow: totalCredit - totalDebit,
  };
}

/** Return unique YYYY-MM strings from ledger entries, sorted ascending. */
export function getAvailableMonths(entries: LedgerEntry[]): string[] {
  const months = new Set<string>();
  for (const e of entries) {
    // Extract the first 7 characters "YYYY-MM" from the date string
    if (e.date.length >= 7) {
      months.add(e.date.slice(0, 7));
    }
  }
  return Array.from(months).sort();
}
