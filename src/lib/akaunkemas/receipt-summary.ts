import type { Receipt, ReceiptSummary, ReceiptCategorySummary, CategorySlug } from "./types";

/**
 * Compute a summary from a list of receipts.
 *
 * Groups receipts by category, sums amounts, and returns a ReceiptSummary
 * with category summaries sorted by absolute total descending.
 */
export function computeReceiptSummary(receipts: Receipt[]): ReceiptSummary {
  const categoryMap = new Map<CategorySlug, { total: number; count: number }>();

  let totalAmount = 0;
  let totalTax = 0;
  let totalServiceCharge = 0;

  for (const r of receipts) {
    totalAmount += r.amount;
    totalTax += r.taxAmount;
    totalServiceCharge += r.serviceCharge;

    if (!categoryMap.has(r.category)) {
      categoryMap.set(r.category, { total: 0, count: 0 });
    }
    const entry = categoryMap.get(r.category)!;
    entry.total += r.amount;
    entry.count += 1;
  }

  const categorySummaries: ReceiptCategorySummary[] = Array.from(categoryMap.entries())
    .map(([category, { total, count }]) => ({ category, total, count }))
    .sort((a, b) => b.total - a.total);

  return {
    totalAmount,
    totalTax,
    totalServiceCharge,
    receiptCount: receipts.length,
    categorySummaries,
  };
}
