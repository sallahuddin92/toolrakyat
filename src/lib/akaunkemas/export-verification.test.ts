import { describe, expect, it } from "vitest";
import Papa from "papaparse";
import { exportCleanedCsv } from "./export-cleaned-csv";
import { buildPdfData, generateMonthlySummaryPdf } from "./pdf-export";
import { computeMonthlySummary } from "./summary";
import { getCategoryLabel } from "./categories";
import type { Transaction, CategorySlug } from "./types";

// Intl.NumberFormat("ms-MY") uses non-breaking spaces (U+00A0)
const NBSP = "\u00A0";

function rm(value: number): string {
  if (value >= 0) return `RM${NBSP}${value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `-RM${NBSP}${Math.abs(value).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

/** Reusable factory matching the pattern from summary.test.ts */
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

// ---------------------------------------------------------------------------
// 1. exportCleanedCsv()
// ---------------------------------------------------------------------------
describe("exportCleanedCsv", () => {
  const sampleTransactions: Transaction[] = [
    makeTx({ id: 1, date: "2024-01-05", description: "Customer Invoice #001", amount: 5000, credit: 5000, balance: 15000, category: "sales" }),
    makeTx({ id: 2, date: "2024-01-08", description: "Office Rent Jan", amount: -2000, debit: 2000, balance: 13000, category: "rent" }),
    makeTx({ id: 3, date: "2024-01-12", description: "Electricity Bill", amount: -350.5, debit: 350.5, balance: 12649.5, category: "utilities" }),
    makeTx({ id: 4, date: "2024-01-15", description: "Refund from Vendor", amount: 125.25, credit: 125.25, balance: 12774.75, category: "other_income" }),
    makeTx({ id: 5, date: "2024-01-20", description: "Transfer to Savings", amount: -1000, debit: 1000, category: "transfer" }),
  ];

  it("produces valid CSV that Papa can parse back", () => {
    const csv = exportCleanedCsv(sampleTransactions);
    const parsed = Papa.parse(csv, { header: true, skipEmptyLines: true });
    expect(parsed.errors).toHaveLength(0);
    expect(parsed.data).toHaveLength(5);
  });

  it("contains the correct columns", () => {
    const csv = exportCleanedCsv(sampleTransactions);
    const parsed = Papa.parse<string[]>(csv, { skipEmptyLines: true });
    const headers = parsed.data[0]!;
    expect(headers).toEqual([
      "date",
      "description",
      "debit",
      "credit",
      "amount",
      "balance",
      "category",
    ]);
  });

  it("uses bilingual category labels, not slugs", () => {
    const csv = exportCleanedCsv(sampleTransactions);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });

    // Row 1 -> sales
    expect(parsed.data[0]!["category"]).toBe(getCategoryLabel("sales"));
    // Row 2 -> rent
    expect(parsed.data[1]!["category"]).toBe(getCategoryLabel("rent"));
    // Row 3 -> utilities
    expect(parsed.data[2]!["category"]).toBe(getCategoryLabel("utilities"));
    // Row 4 -> other_income
    expect(parsed.data[3]!["category"]).toBe(getCategoryLabel("other_income"));
    // Row 5 -> transfer
    expect(parsed.data[4]!["category"]).toBe(getCategoryLabel("transfer"));

    // Confirm no slugs leak through
    for (const row of parsed.data) {
      const cat = row["category"] ?? "";
      expect(cat).not.toBe("sales");
      expect(cat).not.toBe("rent");
      expect(cat).toContain(" / ");
    }
  });

  it("formats decimal numbers with exactly 2 decimal places", () => {
    const csv = exportCleanedCsv(sampleTransactions);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });

    for (const row of parsed.data) {
      // amount and balance should always have 2 decimal places when present
      const amount = row["amount"];
      if (amount !== undefined && amount !== "") {
        expect(amount).toMatch(/^-?\d+\.\d{2}$/);
      }

      const balance = row["balance"];
      if (balance !== undefined && balance !== "") {
        expect(balance).toMatch(/^\d+\.\d{2}$/);
      }

      // debit/credit may be empty strings, but when present: 2dp
      const debit = row["debit"];
      if (debit !== undefined && debit !== "") {
        expect(debit).toMatch(/^\d+\.\d{2}$/);
      }

      const credit = row["credit"];
      if (credit !== undefined && credit !== "") {
        expect(credit).toMatch(/^\d+\.\d{2}$/);
      }
    }
  });

  it("leaves debit empty for credit-only rows and vice-versa", () => {
    const csv = exportCleanedCsv(sampleTransactions);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });

    // Row 1 = credit 5000, debit 0 -> debit should be ""
    expect(parsed.data[0]!["debit"]).toBe("");
    expect(parsed.data[0]!["credit"]).toBe("5000.00");

    // Row 2 = debit 2000, credit 0 -> credit should be ""
    expect(parsed.data[1]!["credit"]).toBe("");
    expect(parsed.data[1]!["debit"]).toBe("2000.00");
  });

  it("leaves balance empty when null", () => {
    const csv = exportCleanedCsv([makeTx({ id: 1, amount: 100, credit: 100, balance: null })]);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });
    expect(parsed.data[0]!["balance"]).toBe("");
  });

  it("handles exactly 0.00 values consistently", () => {
    const csv = exportCleanedCsv([makeTx({ id: 1, amount: 0, debit: 0, credit: 0, balance: 0 })]);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });
    // debit = 0, so the condition `tx.debit > 0` is false -> ""
    expect(parsed.data[0]!["debit"]).toBe("");
    expect(parsed.data[0]!["credit"]).toBe("");
    expect(parsed.data[0]!["amount"]).toBe("0.00");
    // balance = 0 is not null, so it should appear
    expect(parsed.data[0]!["balance"]).toBe("0.00");
  });
});

// ---------------------------------------------------------------------------
// 2. buildPdfData()
// ---------------------------------------------------------------------------
describe("buildPdfData", () => {
  const transactions: Transaction[] = [
    makeTx({ id: 1, amount: 10000, credit: 10000, category: "sales" }),
    makeTx({ id: 2, amount: -2500, debit: 2500, category: "rent" }),
    makeTx({ id: 3, amount: -150.75, debit: 150.75, category: "utilities" }),
    makeTx({ id: 4, amount: 500, credit: 500, category: "other_income" }),
  ];

  const summary = computeMonthlySummary(transactions);

  it("has the correct data structure", () => {
    const data = buildPdfData(summary, "2024-01-01", "2024-01-31");
    expect(data).toHaveProperty("title");
    expect(data).toHaveProperty("generatedDate");
    expect(data).toHaveProperty("periodStart");
    expect(data).toHaveProperty("periodEnd");
    expect(data).toHaveProperty("totalIncome");
    expect(data).toHaveProperty("totalExpense");
    expect(data).toHaveProperty("netCashflow");
    expect(data).toHaveProperty("transactionCount");
    expect(data).toHaveProperty("categoryRows");
    expect(Array.isArray(data.categoryRows)).toBe(true);
  });

  it("formats all monetary values as RM X,XXX.XX", () => {
    const data = buildPdfData(summary, "2024-01-01", "2024-01-31");
    const currencyPattern = /^(-)?RM\u00A0\d{1,3}(,\d{3})*\.\d{2}$/;

    expect(data.totalIncome).toMatch(currencyPattern);
    expect(data.totalExpense).toMatch(currencyPattern);
    expect(data.netCashflow).toMatch(currencyPattern);

    for (const row of data.categoryRows) {
      expect(row.total).toMatch(currencyPattern);
    }
  });

  it("has correct monetary values", () => {
    const data = buildPdfData(summary, "2024-01-01", "2024-01-31");
    expect(data.totalIncome).toBe(rm(10500));
    expect(data.totalExpense).toBe(rm(2650.75));
    expect(data.netCashflow).toBe(rm(7849.25));
  });

  it("category breakdown matches input", () => {
    const data = buildPdfData(summary, "2024-01-01", "2024-01-31");
    expect(data.categoryRows).toHaveLength(4);

    // Category rows use bilingual labels
    const catLabels = data.categoryRows.map((r) => r.category);
    expect(catLabels).toContain(getCategoryLabel("sales"));
    expect(catLabels).toContain(getCategoryLabel("rent"));
    expect(catLabels).toContain(getCategoryLabel("utilities"));
    expect(catLabels).toContain(getCategoryLabel("other_income"));

    // Sorted by abs(total) descending
    const salesRow = data.categoryRows.find((r) => r.category === getCategoryLabel("sales"))!;
    expect(salesRow.total).toBe(rm(10000));
    expect(salesRow.count).toBe(1);

    const rentRow = data.categoryRows.find((r) => r.category === getCategoryLabel("rent"))!;
    expect(rentRow.total).toBe(rm(2500));
    expect(rentRow.count).toBe(1);
  });

  it("generated date is today", () => {
    const data = buildPdfData(summary);
    const today = new Date().toISOString().slice(0, 10);
    expect(data.generatedDate).toBe(today);
  });

  it("handles empty summary gracefully", () => {
    const emptySummary = computeMonthlySummary([]);
    const data = buildPdfData(emptySummary);
    expect(data.totalIncome).toBe(rm(0));
    expect(data.totalExpense).toBe(rm(0));
    expect(data.netCashflow).toBe(rm(0));
    expect(data.transactionCount).toBe(0);
    expect(data.categoryRows).toHaveLength(0);
    expect(data.periodStart).toBe("");
    expect(data.periodEnd).toBe("");
  });

  it("handles negative net cashflow", () => {
    const expenseTx: Transaction[] = [
      makeTx({ id: 1, amount: -500, debit: 500, category: "rent" }),
    ];
    const negSummary = computeMonthlySummary(expenseTx);
    const data = buildPdfData(negSummary);
    expect(data.netCashflow).toBe(rm(-500));
  });
});

// ---------------------------------------------------------------------------
// 3. generateMonthlySummaryPdf() – requires pdf-lib
// ---------------------------------------------------------------------------
describe("generateMonthlySummaryPdf", () => {
  const transactions: Transaction[] = [
    makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    makeTx({ id: 2, amount: -1500, debit: 1500, category: "rent" }),
  ];
  const summary = computeMonthlySummary(transactions);

  it("returns a Uint8Array", async () => {
    const bytes = await generateMonthlySummaryPdf({
      summary,
      periodStart: "2024-01-01",
      periodEnd: "2024-01-31",
    });
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes.length).toBeGreaterThan(0);
  });

  it("produces bytes that start with a valid PDF header (%PDF)", async () => {
    const bytes = await generateMonthlySummaryPdf({
      summary,
      periodStart: "2024-01-01",
      periodEnd: "2024-01-31",
    });

    // Slice the first 5 bytes and decode as ASCII
    const header = new TextDecoder().decode(bytes.slice(0, 5));
    expect(header).toBe("%PDF-");
  });

  it("produces a valid PDF with proper version in header", async () => {
    const bytes = await generateMonthlySummaryPdf({
      summary,
    });
    const headerStr = new TextDecoder().decode(bytes.slice(0, 8));
    expect(headerStr).toMatch(/^%PDF-1\.\d/);
  });

  it("produces a PDF that ends with %%EOF", async () => {
    const bytes = await generateMonthlySummaryPdf({
      summary,
    });
    // PDF files end with %%EOF (with optional EOL markers after)
    const tail = new TextDecoder().decode(bytes.slice(bytes.length - 10));
    expect(tail).toContain("%%EOF");
  });

  it("includes business header when provided", async () => {
    const bytes = await generateMonthlySummaryPdf({
      summary,
      periodStart: "2024-01-01",
      periodEnd: "2024-01-31",
      businessHeader: {
        name: "Syarikat ABC Sdn Bhd",
        registrationNumber: "202401001234",
        address: "123 Jalan Example, Kuala Lumpur",
        phone: "+60123456789",
        email: "abc@example.com",
        preparedBy: "Ali bin Abu",
      },
    });
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes.length).toBeGreaterThan(0);

    // Content streams are compressed (FlateDecode / LZW), so we cannot
    // grep for the business name as plaintext.  Verify by checking that
    // the PDF with a business header is larger than an equivalent one
    // without.
    const bytesWithoutHeader = await generateMonthlySummaryPdf({
      summary,
      periodStart: "2024-01-01",
      periodEnd: "2024-01-31",
    });
    expect(bytes.length).toBeGreaterThan(bytesWithoutHeader.length);
  });

  it("produces different output for different summaries", async () => {
    const bytesA = await generateMonthlySummaryPdf({ summary });

    const otherSummary = computeMonthlySummary([
      makeTx({ id: 1, amount: 99999, credit: 99999, category: "sales" }),
    ]);
    const bytesB = await generateMonthlySummaryPdf({ summary: otherSummary });

    // Different data should produce different PDF bytes
    expect(bytesA.length).not.toBe(bytesB.length);
  });
});
