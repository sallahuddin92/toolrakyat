import { describe, expect, it } from "vitest";
import {
  addManualMatch,
  amountsMatch,
  daysBetween,
  matchReceiptsToTransactions,
  parseDateStr,
  recomputeMatches,
  removeMatch,
} from "./receipt-matcher";
import type { Receipt, Transaction } from "./types";

// ---------------------------------------------------------------------------
// Helpers to create test fixtures
// ---------------------------------------------------------------------------

function makeTx(overrides: Partial<Transaction> = {}): Transaction {
  return {
    id: 1,
    date: "2023-05-15",
    description: "Test transaction",
    debit: 0,
    credit: 0,
    amount: 100,
    balance: null,
    category: "uncategorised",
    ...overrides,
  };
}

function makeReceipt(overrides: Partial<Receipt> = {}): Receipt {
  return {
    id: 1,
    date: "2023-05-15",
    merchant: "Test Merchant",
    amount: 100,
    paymentMethod: "bank_transfer",
    category: "uncategorised",
    taxAmount: 0,
    serviceCharge: 0,
    notes: "",
    imageRef: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// parseDateStr
// ---------------------------------------------------------------------------

describe("parseDateStr", () => {
  it("parses YYYY-MM-DD format", () => {
    const d = parseDateStr("2023-05-15");
    expect(d.getFullYear()).toBe(2023);
    expect(d.getMonth()).toBe(4); // zero-indexed
    expect(d.getDate()).toBe(15);
  });

  it("parses DD/MM/YYYY format", () => {
    const d = parseDateStr("15/05/2023");
    expect(d.getFullYear()).toBe(2023);
    expect(d.getMonth()).toBe(4);
    expect(d.getDate()).toBe(15);
  });

  it("parses DD-MM-YYYY format", () => {
    const d = parseDateStr("15-05-2023");
    expect(d.getFullYear()).toBe(2023);
    expect(d.getMonth()).toBe(4);
    expect(d.getDate()).toBe(15);
  });

  it("handles single digit days and months in DD/MM/YYYY", () => {
    const d = parseDateStr("05/03/2023");
    expect(d.getFullYear()).toBe(2023);
    expect(d.getMonth()).toBe(2);
    expect(d.getDate()).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// daysBetween
// ---------------------------------------------------------------------------

describe("daysBetween", () => {
  it("returns 0 for same date", () => {
    expect(daysBetween(new Date(2023, 4, 15), new Date(2023, 4, 15))).toBe(0);
  });

  it("returns correct number of days for different dates", () => {
    expect(daysBetween(new Date(2023, 4, 15), new Date(2023, 4, 18))).toBe(3);
  });

  it("is symmetric (order does not matter)", () => {
    const a = new Date(2023, 4, 10);
    const b = new Date(2023, 4, 20);
    expect(daysBetween(a, b)).toBe(daysBetween(b, a));
  });

  it("crosses month boundaries correctly", () => {
    expect(daysBetween(new Date(2023, 4, 31), new Date(2023, 5, 2))).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// amountsMatch
// ---------------------------------------------------------------------------

describe("amountsMatch", () => {
  it("matches exact same amounts", () => {
    expect(amountsMatch(100, 100)).toBe(true);
  });

  it("matches absolute values (negative vs positive)", () => {
    expect(amountsMatch(-100, 100)).toBe(true);
    expect(amountsMatch(100, -100)).toBe(true);
    expect(amountsMatch(-100, -100)).toBe(true);
  });

  it("rejects different amounts", () => {
    expect(amountsMatch(100, 200)).toBe(false);
  });

  it("accepts amounts within tolerance", () => {
    expect(amountsMatch(100.005, 100, 0.01)).toBe(true);
    expect(amountsMatch(100, 100.005, 0.01)).toBe(true);
  });

  it("rejects amounts outside tolerance", () => {
    expect(amountsMatch(100.02, 100, 0.01)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// matchReceiptsToTransactions
// ---------------------------------------------------------------------------

describe("matchReceiptsToTransactions", () => {
  // --- exact match scenarios ---

  it("matches two transactions with two receipts exactly", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
      makeTx({ id: 2, date: "2023-05-16", amount: 250 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
      makeReceipt({ id: 2, date: "2023-05-16", amount: 250 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(2);
    expect(report.matched[0].matchType).toBe("exact");
    expect(report.matched[1].matchType).toBe("exact");
    expect(report.unmatchedBank).toHaveLength(0);
    expect(report.unmatchedReceipts).toHaveLength(0);
  });

  it("matches within date window with correct dateDelta", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-17", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    expect(report.matched[0].dateDelta).toBe(2);
    expect(report.matched[0].amountDelta).toBe(0);
  });

  it("leaves items unmatched when date is outside window", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-20", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(0);
    expect(report.unmatchedBank).toHaveLength(1);
    expect(report.unmatchedReceipts).toHaveLength(1);
  });

  it("leaves items unmatched when amounts differ", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 200 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(0);
    expect(report.unmatchedBank).toHaveLength(1);
    expect(report.unmatchedReceipts).toHaveLength(1);
  });

  it("does not match the same receipt to two transactions (no duplicates)", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
      makeTx({ id: 2, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    expect(report.unmatchedBank).toHaveLength(1);
    expect(report.unmatchedReceipts).toHaveLength(0);
  });

  it("does not match the same transaction to two receipts (no duplicates)", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
      makeReceipt({ id: 2, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    expect(report.unmatchedBank).toHaveLength(0);
    expect(report.unmatchedReceipts).toHaveLength(1);
  });

  it("handles empty inputs gracefully", () => {
    const report = matchReceiptsToTransactions([], [], 3);

    expect(report.matched).toHaveLength(0);
    expect(report.unmatchedBank).toHaveLength(0);
    expect(report.unmatchedReceipts).toHaveLength(0);
    expect(report.dateWindowDays).toBe(3);
  });

  // --- edge cases ---

  it("matches negative bank transaction amounts (debit) to positive receipt amounts", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: -100 }), // debit
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    expect(report.matched[0].matchType).toBe("exact");
  });

  // --- fuzzy matching ---

  it("fuzzy-matches amounts within 0.01 tolerance", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100.005 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    expect(report.matched[0].matchType).toBe("fuzzy");
    expect(report.matched[0].amountDelta).toBeLessThanOrEqual(0.01);
  });

  it("exact match takes priority over fuzzy match", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100.005 }),
      makeReceipt({ id: 2, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    // Should prefer the exact amount match (receipt 2) over the fuzzy one
    expect(report.matched[0].receiptId).toBe(2);
    expect(report.matched[0].matchType).toBe("exact");
  });

  // --- closest date preference ---

  it("prefers closest date when multiple receipt candidates exist", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-18", amount: 100 }), // 3 days
      makeReceipt({ id: 2, date: "2023-05-16", amount: 100 }), // 1 day
      makeReceipt({ id: 3, date: "2023-05-19", amount: 100 }), // 4 days (out of window for default 3)
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(report.matched).toHaveLength(1);
    // Should pick receipt 2 (only 1 day apart)
    expect(report.matched[0].receiptId).toBe(2);
    expect(report.matched[0].dateDelta).toBe(1);
    // Receipt 3 is outside the window so should be unmatched
    expect(report.unmatchedReceipts).toHaveLength(2);
  });

  // --- dateWindowDays parameter ---

  it("respects custom dateWindowDays", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-22", amount: 100 }), // 7 days
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 7);

    expect(report.matched).toHaveLength(1);
    expect(report.dateWindowDays).toBe(7);
  });

  it("applies default dateWindowDays of 3 when not specified", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts);

    expect(report.dateWindowDays).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// addManualMatch
// ---------------------------------------------------------------------------

describe("addManualMatch", () => {
  it("adds a manual match between unmatched items", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-20", amount: 100 }), // outside 3-day window
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    expect(report.matched).toHaveLength(0); // not auto-matched

    const updated = addManualMatch(report, 1, 1, txs, receipts);

    expect(updated.matched).toHaveLength(1);
    expect(updated.matched[0].matchType).toBe("manual");
    expect(updated.matched[0].bankTxId).toBe(1);
    expect(updated.matched[0].receiptId).toBe(1);
    expect(updated.unmatchedBank).toHaveLength(0);
    expect(updated.unmatchedReceipts).toHaveLength(0);
  });

  it("does not mutate the original report", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-20", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    addManualMatch(report, 1, 1, txs, receipts);

    // Original should be unchanged
    expect(report.matched).toHaveLength(0);
    expect(report.unmatchedBank).toHaveLength(1);
  });

  it("throws when bankTxId is not in unmatched list", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
      makeTx({ id: 2, date: "2023-05-15", amount: 200 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    // Tx1 is auto-matched, Tx2 is unmatched
    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(() =>
      addManualMatch(report, 1, 1, txs, receipts),
    ).toThrow("not in the unmatched list");
  });
});

// ---------------------------------------------------------------------------
// removeMatch
// ---------------------------------------------------------------------------

describe("removeMatch", () => {
  it("removes an auto-matched result and returns items to unmatched", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    expect(report.matched).toHaveLength(1);

    const updated = removeMatch(report, 1, 1, txs, receipts);

    expect(updated.matched).toHaveLength(0);
    expect(updated.unmatchedBank).toHaveLength(1);
    expect(updated.unmatchedBank[0].id).toBe(1);
    expect(updated.unmatchedReceipts).toHaveLength(1);
    expect(updated.unmatchedReceipts[0].id).toBe(1);
  });

  it("does not mutate the original report", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    removeMatch(report, 1, 1, txs, receipts);

    // Original should still have the match
    expect(report.matched).toHaveLength(1);
  });

  it("throws when match is not found", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);

    expect(() => removeMatch(report, 99, 99, txs, receipts)).toThrow(
      "No match found",
    );
  });
});

// ---------------------------------------------------------------------------
// recomputeMatches
// ---------------------------------------------------------------------------

describe("recomputeMatches", () => {
  it("re-matches items after manual unmatch", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    expect(report.matched).toHaveLength(1);

    // Remove the match
    const afterRemove = removeMatch(report, 1, 1, txs, receipts);
    expect(afterRemove.matched).toHaveLength(0);

    // Recompute — should re-match the items
    const recomputed = recomputeMatches(afterRemove, txs, receipts);

    expect(recomputed.matched).toHaveLength(1);
    expect(recomputed.matched[0].matchType).toBe("exact");
  });

  it("preserves existing matches when recomputing", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
      makeTx({ id: 2, date: "2023-05-15", amount: 200 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
      makeReceipt({ id: 2, date: "2023-05-15", amount: 200 }),
    ];

    // Manually match tx1-r1 first
    const report = matchReceiptsToTransactions(txs, receipts, 3);
    expect(report.matched).toHaveLength(2);

    // Remove tx2 match
    let updated = removeMatch(report, 2, 2, txs, receipts);
    expect(updated.matched).toHaveLength(1); // only tx1-r1 remains

    // Recompute — tx2-r2 should be re-matched, tx1-r1 stays
    updated = recomputeMatches(updated, txs, receipts);

    expect(updated.matched).toHaveLength(2);
    expect(updated.matched.some((m) => m.bankTxId === 1 && m.receiptId === 1)).toBe(true);
    expect(updated.matched.some((m) => m.bankTxId === 2 && m.receiptId === 2)).toBe(true);
  });

  it("returns the same results when nothing new to match", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-15", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    const recomputed = recomputeMatches(report, txs, receipts);

    expect(recomputed.matched).toHaveLength(1);
    expect(recomputed.unmatchedBank).toHaveLength(0);
    expect(recomputed.unmatchedReceipts).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Combined manual + auto scenarios
// ---------------------------------------------------------------------------

describe("mixed manual and automatic matching", () => {
  it("supports addManualMatch followed by removeMatch", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-20", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    expect(report.matched).toHaveLength(0); // outside window

    const withManual = addManualMatch(report, 1, 1, txs, receipts);
    expect(withManual.matched).toHaveLength(1);
    expect(withManual.matched[0].matchType).toBe("manual");

    const afterRemove = removeMatch(withManual, 1, 1, txs, receipts);
    expect(afterRemove.matched).toHaveLength(0);
    expect(afterRemove.unmatchedBank).toHaveLength(1);
    expect(afterRemove.unmatchedReceipts).toHaveLength(1);
  });

  it("does not let recompute override manual matches", () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2023-05-15", amount: 100 }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, date: "2023-05-20", amount: 100 }),
    ];

    const report = matchReceiptsToTransactions(txs, receipts, 3);
    const withManual = addManualMatch(report, 1, 1, txs, receipts);
    const recomputed = recomputeMatches(withManual, txs, receipts);

    expect(recomputed.matched).toHaveLength(1);
    expect(recomputed.matched[0].matchType).toBe("manual");
  });
});
