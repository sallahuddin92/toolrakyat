import { describe, expect, it } from "vitest";
import { computeMonthlySummary, calculateTotals } from "./summary";
import type { Transaction, CategorySlug } from "./types";

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

describe("computeMonthlySummary", () => {
  it("calculates total income from sales and other_income", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: 200, credit: 200, category: "other_income" }),
    ];
    const summary = computeMonthlySummary(transactions);
    expect(summary.totalIncome).toBe(5200);
    expect(summary.totalExpense).toBe(0);
    expect(summary.netCashflow).toBe(5200);
  });

  it("calculates total expense from expense categories", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: -1500, debit: 1500, category: "rent" }),
      makeTx({ id: 2, amount: -300, debit: 300, category: "utilities" }),
      makeTx({ id: 3, amount: -50, debit: 50, category: "bank_charges" }),
    ];
    const summary = computeMonthlySummary(transactions);
    expect(summary.totalExpense).toBe(1850);
    expect(summary.totalIncome).toBe(0);
    expect(summary.netCashflow).toBe(-1850);
  });

  it("calculates net cashflow (income minus expense)", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: -2000, debit: 2000, category: "salary" }),
    ];
    const summary = computeMonthlySummary(transactions);
    expect(summary.netCashflow).toBe(3000);
  });

  it("aggregates by category", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 1000, category: "sales" }),
      makeTx({ id: 2, amount: 500, category: "sales" }),
      makeTx({ id: 3, amount: -200, debit: 200, category: "utilities" }),
    ];
    const summary = computeMonthlySummary(transactions);
    const salesSummary = summary.categorySummaries.find((c) => c.category === "sales");
    expect(salesSummary?.total).toBe(1500);
    expect(salesSummary?.count).toBe(2);

    const utilitiesSummary = summary.categorySummaries.find((c) => c.category === "utilities");
    expect(utilitiesSummary?.total).toBe(200);
    expect(utilitiesSummary?.count).toBe(1);
  });

  it("handles empty transactions array", () => {
    const summary = computeMonthlySummary([]);
    expect(summary.totalIncome).toBe(0);
    expect(summary.totalExpense).toBe(0);
    expect(summary.netCashflow).toBe(0);
    expect(summary.categorySummaries).toHaveLength(0);
    expect(summary.transactionCount).toBe(0);
  });

  it("handles neutral categories (transfer, uncategorised)", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: -1000, debit: 1000, category: "transfer" }),
    ];
    const summary = computeMonthlySummary(transactions);
    // transfers don't count as income or expense
    expect(summary.totalIncome).toBe(0);
    expect(summary.totalExpense).toBe(0);
  });

  it("handles re-categorisation from income to expense (e.g. sales refund)", () => {
    // Simulate: transaction was "sales" (income), re-categorised to "other_expense"
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: -5000, debit: 5000, category: "other_expense" }),
    ];
    const summary = computeMonthlySummary(transactions);
    expect(summary.totalIncome).toBe(5000);
    expect(summary.totalExpense).toBe(5000);
    expect(summary.netCashflow).toBe(0);
  });

  it("handles very large amounts in summary", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 1500000, credit: 1500000, category: "sales" }),
      makeTx({ id: 2, amount: -2750000.5, debit: 2750000.5, category: "purchases" }),
    ];
    const summary = computeMonthlySummary(transactions);
    expect(summary.totalIncome).toBe(1500000);
    expect(summary.totalExpense).toBe(2750000.5);
    expect(summary.netCashflow).toBe(-1250000.5);
  });

  it("handles mix of income, expense, and neutral categories together", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, amount: 10000, credit: 10000, category: "sales" }),
      makeTx({ id: 2, amount: -3000, debit: 3000, category: "rent" }),
      makeTx({ id: 3, amount: -200, debit: 200, category: "utilities" }),
      makeTx({ id: 4, amount: 500, credit: 500, category: "other_income" }),
      makeTx({ id: 5, amount: -1000, debit: 1000, category: "transfer" }),
      makeTx({ id: 6, amount: 0, debit: 0, category: "uncategorised" }),
    ];
    const summary = computeMonthlySummary(transactions);
    expect(summary.totalIncome).toBe(10500);
    expect(summary.totalExpense).toBe(3200);
    expect(summary.netCashflow).toBe(7300);
    expect(summary.transactionCount).toBe(6);
    // Category summaries should include all categories even neutral ones
    expect(summary.categorySummaries).toHaveLength(6);
  });
});

describe("calculateTotals", () => {
  it("sums debit and credit columns", () => {
    const transactions: Transaction[] = [
      makeTx({ id: 1, debit: 1500, credit: 0, amount: -1500 }),
      makeTx({ id: 2, debit: 0, credit: 3000, amount: 3000 }),
    ];
    const totals = calculateTotals(transactions);
    expect(totals.totalDebit).toBe(1500);
    expect(totals.totalCredit).toBe(3000);
    expect(totals.net).toBe(1500);
    expect(totals.count).toBe(2);
  });
});
