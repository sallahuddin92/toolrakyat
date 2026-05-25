import { describe, expect, it } from "vitest";
import { buildPdfData } from "./pdf-export";
import { computeMonthlySummary } from "./summary";
import type { Transaction, CategorySlug } from "./types";

// Intl.NumberFormat("ms-MY") uses non-breaking spaces (U+00A0)
const NBSP = "\u00A0";

function rm(value: number): string {
  // Match what the production Intl.NumberFormat("ms-MY") produces
  if (value >= 0) return `RM${NBSP}${value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `-RM${NBSP}${Math.abs(value).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function makeTx(overrides: Partial<Transaction> = {}): Transaction {
  return {
    id: 1,
    date: "2024-01-15",
    description: "Test",
    debit: 0,
    credit: 0,
    amount: 0,
    balance: null,
    category: "uncategorised" as CategorySlug,
    ...overrides,
  };
}

describe("buildPdfData", () => {
  it("builds PDF data from a monthly summary", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: -1500, debit: 1500, category: "rent" }),
      makeTx({ id: 3, amount: -200, debit: 200, category: "utilities" }),
    ];
    const summary = computeMonthlySummary(transactions);
    const data = buildPdfData(summary, "2024-01-01", "2024-01-31");

    expect(data.title).toBe("AkaunKemas Monthly Summary");
    expect(data.totalIncome).toBe(rm(5000));
    expect(data.totalExpense).toBe(rm(1700));
    expect(data.netCashflow).toBe(rm(3300));
    expect(data.transactionCount).toBe(3);
    expect(data.periodStart).toBe("2024-01-01");
    expect(data.periodEnd).toBe("2024-01-31");
    expect(data.generatedDate).toBeTruthy();
    expect(data.categoryRows).toHaveLength(3);
    expect(data.categoryRows[0]!.category).toBe("Jualan / Sales");
    expect(data.categoryRows[0]!.total).toBe(rm(5000));
    expect(data.categoryRows[0]!.count).toBe(1);
  });

  it("handles empty summary", () => {
    const summary = computeMonthlySummary([]);
    const data = buildPdfData(summary);

    expect(data.totalIncome).toBe(rm(0));
    expect(data.totalExpense).toBe(rm(0));
    expect(data.netCashflow).toBe(rm(0));
    expect(data.transactionCount).toBe(0);
    expect(data.categoryRows).toHaveLength(0);
  });

  it("includes period dates when provided", () => {
    const summary = computeMonthlySummary([]);
    const data = buildPdfData(summary, "2024-03-01", "2024-03-31");
    expect(data.periodStart).toBe("2024-03-01");
    expect(data.periodEnd).toBe("2024-03-31");
  });

  it("includes generated date", () => {
    const summary = computeMonthlySummary([]);
    const data = buildPdfData(summary);
    const today = new Date().toISOString().slice(0, 10);
    expect(data.generatedDate).toBe(today);
  });

  it("formats very large amounts correctly for PDF data", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 1500000, credit: 1500000, category: "sales" }),
      makeTx({ id: 2, amount: -2750000.5, debit: 2750000.5, category: "purchases" }),
    ];
    const summary = computeMonthlySummary(transactions);
    const data = buildPdfData(summary, "2024-01-01", "2024-01-31");

    expect(data.totalIncome).toBe(rm(1500000));
    expect(data.totalExpense).toBe(rm(2750000.5));
    expect(data.netCashflow).toBe(rm(-1250000.5));
    expect(data.transactionCount).toBe(2);
  });

  it("handles negative net cashflow display", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: -500, debit: 500, category: "rent" }),
      makeTx({ id: 2, amount: -300, debit: 300, category: "utilities" }),
    ];
    const summary = computeMonthlySummary(transactions);
    const data = buildPdfData(summary);

    expect(data.totalIncome).toBe(rm(0));
    expect(data.totalExpense).toBe(rm(800));
    expect(data.netCashflow).toBe(rm(-800));
  });
});
