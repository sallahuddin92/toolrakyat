import { getCategoryLabel } from "./categories";
import type { LedgerEntry, LedgerTotals } from "./types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmt2dp(value: number): string {
  return value.toFixed(2);
}

/** Escape a CSV field – wrap in quotes if it contains a comma, quote, or newline. */
function csvEscape(value: string): string {
  if (value.includes(",") || value.includes('"') || value.includes("\n")) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

// ---------------------------------------------------------------------------
// CSV Export
// ---------------------------------------------------------------------------

/**
 * Export ledger entries as a CSV string.
 *
 * Columns: date, description, category, debit, credit, balance, notes
 *
 * Formatting rules:
 * - All amounts are 2 decimal places.
 * - Debit/credit columns show a value only when greater than 0; otherwise
 *   they are left empty (an empty string in CSV, not "0.00").
 * - Balance shows the numeric value (including 0.00) unless it is null, in
 *   which case the cell is empty.
 * - Category labels use the bilingual display names from `getCategoryLabel`.
 */
export function exportLedgerCsv(entries: LedgerEntry[]): string {
  const header = "date,description,category,debit,credit,balance,notes";

  const rows = entries.map((e) => {
    const date = csvEscape(e.date);
    const description = csvEscape(e.description);
    const category = csvEscape(getCategoryLabel(e.category));
    const debit = e.debit > 0 ? fmt2dp(e.debit) : "";
    const credit = e.credit > 0 ? fmt2dp(e.credit) : "";
    const balance =
      e.runningBalance !== null ? fmt2dp(e.runningBalance) : "";
    const notes = csvEscape(e.notes);

    return `${date},${description},${category},${debit},${credit},${balance},${notes}`;
  });

  return [header, ...rows].join("\n");
}

// ---------------------------------------------------------------------------
// JSON Export
// ---------------------------------------------------------------------------

/**
 * Export ledger entries and totals as a JSON string.
 *
 * The output object has the shape: { entries: LedgerEntry[], totals: LedgerTotals }
 */
export function exportLedgerJson(
  entries: LedgerEntry[],
  totals: LedgerTotals,
): string {
  return JSON.stringify({ entries, totals }, null, 2);
}
