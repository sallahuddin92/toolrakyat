import { describe, expect, it } from "vitest";
import Papa from "papaparse";
import { getCategoryLabel } from "./categories";
import { exportLedgerCsv, exportLedgerJson } from "./ledger-export";
import {
  computeLedgerTotals,
  filterLedgerByCategory,
  filterLedgerByMonth,
  getAvailableMonths,
  transactionsToLedger,
} from "./ledger";
import type { CategorySlug, Transaction } from "./types";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

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

// Reusable 5-mixed-transaction data set
const sampleTxs: Transaction[] = [
  makeTx({ id: 1, date: "2024-01-05", description: "Customer Invoice #001", amount: 5000, credit: 5000, balance: 15000, category: "sales" }),
  makeTx({ id: 2, date: "2024-01-08", description: "Office Rent Jan", amount: -2000, debit: 2000, balance: 13000, category: "rent" }),
  makeTx({ id: 3, date: "2024-01-12", description: "Electricity Bill", amount: -350.5, debit: 350.5, balance: 12649.5, category: "utilities" }),
  makeTx({ id: 4, date: "2024-02-05", description: "Refund from Vendor", amount: 125.25, credit: 125.25, balance: 12774.75, category: "other_income" }),
  makeTx({ id: 5, date: "2024-02-18", description: "Transfer to Savings", amount: -1000, debit: 1000, category: "transfer" }),
];

// ---------------------------------------------------------------------------
// transactionsToLedger
// ---------------------------------------------------------------------------
describe("transactionsToLedger", () => {
  it("converts 5 mixed transactions to ledger entries with correct running balance", () => {
    const entries = transactionsToLedger(sampleTxs);

    expect(entries).toHaveLength(5);

    // First 3 entries have explicit balance, 4th and 5th should have explicit too
    expect(entries[0]!.id).toBe(1);
    expect(entries[0]!.date).toBe("2024-01-05");
    expect(entries[0]!.description).toBe("Customer Invoice #001");
    expect(entries[0]!.category).toBe("sales");
    expect(entries[0]!.debit).toBe(0);
    expect(entries[0]!.credit).toBe(5000);
    expect(entries[0]!.runningBalance).toBe(15000);
    expect(entries[0]!.notes).toBe("");

    expect(entries[1]!.id).toBe(2);
    expect(entries[1]!.runningBalance).toBe(13000);

    expect(entries[2]!.id).toBe(3);
    expect(entries[2]!.runningBalance).toBe(12649.5);

    expect(entries[3]!.id).toBe(4);
    expect(entries[3]!.runningBalance).toBe(12774.75);

    // Entry 5 has balance: null -> computed from entry 4's balance + credit - debit
    // Entry 4 balance = 12774.75, credit = 125.25, debit = 0 -> 12900
    // But entry 5 is: credit = 0, debit = 1000, so runningBalance = 12900 - 1000 = 11900
    // Actually wait, entry 4 balance is explicit 12774.75. So the loop sets runningBalance to that.
    // Then entry 5 has balance null, so runningBalance = 12774.75 + 0 - 1000 = 11774.75
    expect(entries[4]!.id).toBe(5);
    expect(entries[4]!.runningBalance).toBeCloseTo(11774.75, 2);
  });

  it("entries with null balance compute from previous runningBalance + credit - debit", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2024-01-01", credit: 1000, balance: null }),
      makeTx({ id: 2, date: "2024-01-02", debit: 300, balance: null }),
      makeTx({ id: 3, date: "2024-01-03", credit: 50, balance: null }),
    ];

    const entries = transactionsToLedger(txs);

    // All compute from previous
    expect(entries[0]!.runningBalance).toBeCloseTo(1000, 2);
    expect(entries[1]!.runningBalance).toBeCloseTo(700, 2);
    expect(entries[2]!.runningBalance).toBeCloseTo(750, 2);
  });

  it("entries with explicit balance use it directly (not computed)", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2024-01-01", credit: 100, balance: 99999 }),
      makeTx({ id: 2, date: "2024-01-02", credit: 200, balance: null }),
    ];

    const entries = transactionsToLedger(txs);

    // Entry 1 uses explicit balance 99999, ignoring credit
    expect(entries[0]!.runningBalance).toBe(99999);
    // Entry 2 computes from entry 1's balance + credit - debit = 99999 + 200 - 0
    expect(entries[1]!.runningBalance).toBeCloseTo(100199, 2);
  });

  it("handles empty array", () => {
    const entries = transactionsToLedger([]);
    expect(entries).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// filterLedgerByMonth
// ---------------------------------------------------------------------------
describe("filterLedgerByMonth", () => {
  const entries = transactionsToLedger(sampleTxs);

  it("returns only entries in given YYYY-MM", () => {
    const janEntries = filterLedgerByMonth(entries, "2024-01");
    expect(janEntries).toHaveLength(3);
    expect(janEntries.map((e) => e.id)).toEqual([1, 2, 3]);
  });

  it("returns empty array for a month with no entries", () => {
    const decEntries = filterLedgerByMonth(entries, "2024-12");
    expect(decEntries).toHaveLength(0);
  });

  it("handles empty ledger", () => {
    const result = filterLedgerByMonth([], "2024-01");
    expect(result).toHaveLength(0);
  });

  it("filters correctly for Feb which has 2 entries", () => {
    const febEntries = filterLedgerByMonth(entries, "2024-02");
    expect(febEntries).toHaveLength(2);
    expect(febEntries.map((e) => e.id)).toEqual([4, 5]);
  });
});

// ---------------------------------------------------------------------------
// filterLedgerByCategory
// ---------------------------------------------------------------------------
describe("filterLedgerByCategory", () => {
  const entries = transactionsToLedger(sampleTxs);

  it("returns only matching category entries", () => {
    const salesEntries = filterLedgerByCategory(entries, "sales");
    expect(salesEntries).toHaveLength(1);
    expect(salesEntries[0]!.id).toBe(1);
    expect(salesEntries[0]!.category).toBe("sales");
  });

  it("returns empty array for category with no entries", () => {
    const insuranceEntries = filterLedgerByCategory(entries, "insurance");
    expect(insuranceEntries).toHaveLength(0);
  });

  it("returns all entries of a specific category when multiple exist", () => {
    // Add another sales entry to the ledger
    const extraTx: Transaction = makeTx({ id: 6, date: "2024-03-01", credit: 200, category: "sales", balance: null });
    const allTxs = [...sampleTxs, extraTx];
    const allEntries = transactionsToLedger(allTxs);

    const salesEntries = filterLedgerByCategory(allEntries, "sales");
    expect(salesEntries).toHaveLength(2);
    expect(salesEntries.map((e) => e.id)).toEqual([1, 6]);
  });

  it("handles empty ledger", () => {
    const result = filterLedgerByCategory([], "sales");
    expect(result).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// computeLedgerTotals
// ---------------------------------------------------------------------------
describe("computeLedgerTotals", () => {
  it("correct sums for mixed debit/credit entries", () => {
    const entries = transactionsToLedger(sampleTxs);
    // Debits: tx2 = 2000, tx3 = 350.5, tx5 = 1000 -> 3350.5
    // Credits: tx1 = 5000, tx4 = 125.25 -> 5125.25
    const totals = computeLedgerTotals(entries);

    expect(totals.totalDebit).toBeCloseTo(3350.5, 2);
    expect(totals.totalCredit).toBeCloseTo(5125.25, 2);
    expect(totals.netCashflow).toBeCloseTo(5125.25 - 3350.5, 2);
  });

  it("empty input yields zero totals", () => {
    const totals = computeLedgerTotals([]);
    expect(totals.totalDebit).toBe(0);
    expect(totals.totalCredit).toBe(0);
    expect(totals.netCashflow).toBe(0);
  });

  it("only-debit entries produce negative net cashflow", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, debit: 500, balance: null }),
      makeTx({ id: 2, debit: 250, balance: null }),
    ];
    const entries = transactionsToLedger(txs);
    const totals = computeLedgerTotals(entries);
    expect(totals.totalDebit).toBe(750);
    expect(totals.totalCredit).toBe(0);
    expect(totals.netCashflow).toBe(-750);
  });

  it("only-credit entries produce positive net cashflow", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, credit: 1000, balance: null }),
      makeTx({ id: 2, credit: 300, balance: null }),
    ];
    const entries = transactionsToLedger(txs);
    const totals = computeLedgerTotals(entries);
    expect(totals.totalDebit).toBe(0);
    expect(totals.totalCredit).toBe(1300);
    expect(totals.netCashflow).toBe(1300);
  });
});

// ---------------------------------------------------------------------------
// getAvailableMonths
// ---------------------------------------------------------------------------
describe("getAvailableMonths", () => {
  it("returns unique sorted YYYY-MM strings", () => {
    const entries = transactionsToLedger(sampleTxs);
    const months = getAvailableMonths(entries);

    expect(months).toEqual(["2024-01", "2024-02"]);
  });

  it("returns empty array for empty ledger", () => {
    expect(getAvailableMonths([])).toEqual([]);
  });

  it("handles single entry", () => {
    const txs = [makeTx({ id: 1, date: "2024-06-15" })];
    const entries = transactionsToLedger(txs);
    expect(getAvailableMonths(entries)).toEqual(["2024-06"]);
  });

  it("returns sorted when entries are out of order", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2024-12-01" }),
      makeTx({ id: 2, date: "2024-01-15" }),
      makeTx({ id: 3, date: "2024-06-30" }),
    ];
    const entries = transactionsToLedger(txs);
    expect(getAvailableMonths(entries)).toEqual(["2024-01", "2024-06", "2024-12"]);
  });

  it("deduplicates duplicate months", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2024-03-05" }),
      makeTx({ id: 2, date: "2024-03-20" }),
      makeTx({ id: 3, date: "2024-03-15" }),
    ];
    const entries = transactionsToLedger(txs);
    expect(getAvailableMonths(entries)).toEqual(["2024-03"]);
  });
});

// ---------------------------------------------------------------------------
// exportLedgerCsv
// ---------------------------------------------------------------------------
describe("exportLedgerCsv", () => {
  const entries = transactionsToLedger(sampleTxs);

  it("produces valid CSV that Papa can parse back", () => {
    const csv = exportLedgerCsv(entries);
    const parsed = Papa.parse(csv, { header: true, skipEmptyLines: true });
    expect(parsed.errors).toHaveLength(0);
    expect(parsed.data).toHaveLength(5);
  });

  it("contains the correct columns", () => {
    const csv = exportLedgerCsv(entries);
    const parsed = Papa.parse<string[]>(csv, { skipEmptyLines: true });
    const headers = parsed.data[0]!;
    expect(headers).toEqual([
      "date",
      "description",
      "category",
      "debit",
      "credit",
      "balance",
      "notes",
    ]);
  });

  it("uses bilingual category labels, not slugs", () => {
    const csv = exportLedgerCsv(entries);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });

    expect(parsed.data[0]!["category"]).toBe(getCategoryLabel("sales"));
    expect(parsed.data[1]!["category"]).toBe(getCategoryLabel("rent"));
    expect(parsed.data[2]!["category"]).toBe(getCategoryLabel("utilities"));
    expect(parsed.data[3]!["category"]).toBe(getCategoryLabel("other_income"));
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
    const csv = exportLedgerCsv(entries);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });

    for (const row of parsed.data) {
      // balance should always have 2dp when present
      const balance = row["balance"];
      if (balance !== undefined && balance !== "") {
        expect(balance).toMatch(/^\d+\.\d{2}$/);
      }

      // debit shows only when > 0: when present it must be 2dp
      const debit = row["debit"];
      if (debit !== undefined && debit !== "") {
        expect(debit).toMatch(/^\d+\.\d{2}$/);
      }

      // credit shows only when > 0: when present it must be 2dp
      const credit = row["credit"];
      if (credit !== undefined && credit !== "") {
        expect(credit).toMatch(/^\d+\.\d{2}$/);
      }
    }
  });

  it("leaves debit empty for credit-only rows and vice-versa", () => {
    const csv = exportLedgerCsv(entries);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });

    // Row 1 = credit 5000, debit 0 -> debit should be ""
    expect(parsed.data[0]!["debit"]).toBe("");
    expect(parsed.data[0]!["credit"]).toBe("5000.00");

    // Row 2 = debit 2000, credit 0 -> credit should be ""
    expect(parsed.data[1]!["credit"]).toBe("");
    expect(parsed.data[1]!["debit"]).toBe("2000.00");
  });

  it("leaves balance empty when runningBalance is null (manual LedgerEntry)", () => {
    const entry = {
      id: 1,
      date: "2024-01-01",
      description: "Test",
      category: "sales" as CategorySlug,
      debit: 0,
      credit: 100,
      runningBalance: null,
      notes: "",
    };
    const csv = exportLedgerCsv([entry]);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });
    expect(parsed.data[0]!["balance"]).toBe("");
  });

  it("handles notes field with commas (CSV escaping)", () => {
    const tx = makeTx({ id: 1, credit: 50, balance: null });
    const singleEntries = transactionsToLedger([tx]);
    // Manually set a note with a comma
    singleEntries[0]!.notes = "Refund, partial payment";
    const csv = exportLedgerCsv(singleEntries);
    const parsed = Papa.parse<Record<string, string>>(csv, { header: true, skipEmptyLines: true });
    expect(parsed.errors).toHaveLength(0);
    expect(parsed.data[0]!["notes"]).toBe("Refund, partial payment");
  });
});

// ---------------------------------------------------------------------------
// exportLedgerJson
// ---------------------------------------------------------------------------
describe("exportLedgerJson", () => {
  const entries = transactionsToLedger(sampleTxs);
  const totals = computeLedgerTotals(entries);

  it("produces valid JSON with entries and totals", () => {
    const json = exportLedgerJson(entries, totals);
    const parsed = JSON.parse(json);

    expect(parsed).toHaveProperty("entries");
    expect(parsed).toHaveProperty("totals");
    expect(Array.isArray(parsed.entries)).toBe(true);
    expect(parsed.entries).toHaveLength(5);
    expect(parsed.totals.totalDebit).toBeCloseTo(3350.5, 2);
    expect(parsed.totals.totalCredit).toBeCloseTo(5125.25, 2);
    expect(parsed.totals.netCashflow).toBeCloseTo(5125.25 - 3350.5, 2);
  });

  it("preserves individual entry fields in JSON", () => {
    const json = exportLedgerJson(entries, totals);
    const parsed = JSON.parse(json);

    const entry = parsed.entries[0];
    expect(entry.id).toBe(1);
    expect(entry.date).toBe("2024-01-05");
    expect(entry.description).toBe("Customer Invoice #001");
    expect(entry.category).toBe("sales");
    expect(entry.debit).toBe(0);
    expect(entry.credit).toBe(5000);
    expect(entry.runningBalance).toBe(15000);
    expect(entry.notes).toBe("");
  });

  it("handles empty entries with zero totals", () => {
    const json = exportLedgerJson([], computeLedgerTotals([]));
    const parsed = JSON.parse(json);
    expect(parsed.entries).toEqual([]);
    expect(parsed.totals.totalDebit).toBe(0);
    expect(parsed.totals.totalCredit).toBe(0);
    expect(parsed.totals.netCashflow).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Edge cases: empty input
// ---------------------------------------------------------------------------
describe("empty input", () => {
  it("transactionsToLedger → empty array", () => {
    expect(transactionsToLedger([])).toEqual([]);
  });

  it("filterLedgerByMonth → empty array", () => {
    expect(filterLedgerByMonth([], "2024-01")).toEqual([]);
  });

  it("filterLedgerByCategory → empty array", () => {
    expect(filterLedgerByCategory([], "sales")).toEqual([]);
  });

  it("computeLedgerTotals → zero totals", () => {
    const totals = computeLedgerTotals([]);
    expect(totals).toEqual({ totalDebit: 0, totalCredit: 0, netCashflow: 0 });
  });

  it("getAvailableMonths → empty array", () => {
    expect(getAvailableMonths([])).toEqual([]);
  });

  it("exportLedgerCsv → header-only CSV", () => {
    const csv = exportLedgerCsv([]);
    const parsed = Papa.parse(csv, { header: true, skipEmptyLines: true });
    expect(parsed.data).toHaveLength(0);
    // Should still have the header line
    expect(csv).toBe("date,description,category,debit,credit,balance,notes");
  });
});
