import type { CategorySlug, CategorySummary, MonthlySummary, Transaction } from "./types";
import { isExpenseCategory, isIncomeCategory } from "./categories";

/**
 * Compute a monthly summary from a list of categorized transactions.
 */
export function computeMonthlySummary(transactions: Transaction[]): MonthlySummary {
  const categoryMap = new Map<CategorySlug, { total: number; count: number }>();

  let totalIncome = 0;
  let totalExpense = 0;

  for (const tx of transactions) {
    // Initialize category entry if not present
    if (!categoryMap.has(tx.category)) {
      categoryMap.set(tx.category, { total: 0, count: 0 });
    }
    const entry = categoryMap.get(tx.category)!;

    if (isIncomeCategory(tx.category)) {
      totalIncome += tx.amount;
      entry.total += tx.amount;
    } else if (isExpenseCategory(tx.category)) {
      // expense amounts are negative
      const absExpense = Math.abs(tx.amount);
      totalExpense += absExpense;
      entry.total += absExpense;
    } else {
      // neutral categories (transfer, uncategorised)
      entry.total += tx.amount;
    }

    entry.count += 1;
  }

  const categorySummaries: CategorySummary[] = Array.from(categoryMap.entries())
    .map(([category, { total, count }]) => ({ category, total, count }))
    .sort((a, b) => Math.abs(b.total) - Math.abs(a.total));

  return {
    totalIncome,
    totalExpense,
    netCashflow: totalIncome - totalExpense,
    transactionCount: transactions.length,
    categorySummaries,
  };
}

/**
 * Calculate totals and counts for display.
 */
export function calculateTotals(transactions: Transaction[]) {
  const totalDebit = transactions.reduce((sum, t) => sum + t.debit, 0);
  const totalCredit = transactions.reduce((sum, t) => sum + t.credit, 0);
  const net = totalCredit - totalDebit;

  return {
    totalDebit,
    totalCredit,
    net,
    count: transactions.length,
  };
}
