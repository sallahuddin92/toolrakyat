import Papa from "papaparse";
import type { Transaction } from "./types";
import { getCategoryLabel } from "./categories";

/**
 * Export categorized transactions as a cleaned CSV string.
 */
export function exportCleanedCsv(transactions: Transaction[]): string {
  const rows = transactions.map((tx) => ({
    date: tx.date,
    description: tx.description,
    debit: tx.debit > 0 ? tx.debit.toFixed(2) : "",
    credit: tx.credit > 0 ? tx.credit.toFixed(2) : "",
    amount: tx.amount.toFixed(2),
    balance: tx.balance !== null ? tx.balance.toFixed(2) : "",
    category: getCategoryLabel(tx.category),
  }));

  return Papa.unparse(rows);
}
