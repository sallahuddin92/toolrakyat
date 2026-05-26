import { describe, expect, it, beforeEach } from "vitest";
import {
  createReceipt,
  addReceipt,
  updateReceipt,
  deleteReceipt,
  resetReceiptIdCounter,
} from "./receipts";
import { computeReceiptSummary } from "./receipt-summary";
import { exportReceiptCsv, exportReceiptJson, parseReceiptCsv } from "./receipt-export";
import type { Receipt, CategorySlug, PaymentMethod } from "./types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeReceipt(overrides: Partial<Receipt> = {}): Receipt {
  return createReceipt(overrides);
}

// ---------------------------------------------------------------------------
// createReceipt
// ---------------------------------------------------------------------------

describe("createReceipt", () => {
  beforeEach(() => {
    resetReceiptIdCounter();
  });

  it("returns a receipt with default values", () => {
    const r = createReceipt();
    expect(r.id).toBe(1);
    expect(r.date).toBe(new Date().toISOString().slice(0, 10));
    expect(r.merchant).toBe("");
    expect(r.amount).toBe(0);
    expect(r.paymentMethod).toBe("cash");
    expect(r.category).toBe("uncategorised");
    expect(r.taxAmount).toBe(0);
    expect(r.serviceCharge).toBe(0);
    expect(r.notes).toBe("");
    expect(r.imageRef).toBeNull();
  });

  it("accepts partial overrides", () => {
    const r = createReceipt({
      merchant: "Kedai Ali",
      amount: 150.5,
      paymentMethod: "card" as PaymentMethod,
      category: "purchases" as CategorySlug,
    });
    expect(r.merchant).toBe("Kedai Ali");
    expect(r.amount).toBe(150.5);
    expect(r.paymentMethod).toBe("card");
    expect(r.category).toBe("purchases");
    // defaults still apply for non-overridden fields
    expect(r.notes).toBe("");
    expect(r.imageRef).toBeNull();
  });

  it("increments the ID counter with each call", () => {
    const r1 = createReceipt();
    const r2 = createReceipt();
    const r3 = createReceipt();
    expect(r1.id).toBe(1);
    expect(r2.id).toBe(2);
    expect(r3.id).toBe(3);
  });

  it("overrides the auto-generated ID when specified", () => {
    const r = createReceipt({ id: 99 });
    expect(r.id).toBe(99);
  });
});

// ---------------------------------------------------------------------------
// resetReceiptIdCounter
// ---------------------------------------------------------------------------

describe("resetReceiptIdCounter", () => {
  it("resets the counter to zero", () => {
    createReceipt();
    createReceipt();
    resetReceiptIdCounter();
    const r = createReceipt();
    expect(r.id).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// addReceipt
// ---------------------------------------------------------------------------

describe("addReceipt", () => {
  it("appends a receipt to the list", () => {
    const r1 = makeReceipt({ merchant: "First" });
    const r2 = makeReceipt({ merchant: "Second" });
    const list = addReceipt([r1], r2);
    expect(list).toHaveLength(2);
    expect(list[0]!.merchant).toBe("First");
    expect(list[1]!.merchant).toBe("Second");
  });

  it("does not mutate the original array", () => {
    const original = [makeReceipt({ merchant: "Original" })];
    const copy = addReceipt(original, makeReceipt({ merchant: "New" }));
    expect(original).toHaveLength(1);
    expect(copy).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// updateReceipt
// ---------------------------------------------------------------------------

describe("updateReceipt", () => {
  it("updates the receipt with the given id", () => {
    const r1 = makeReceipt({ merchant: "Old Name", amount: 100 });
    const r2 = makeReceipt({ merchant: "Keep" });
    const updated = updateReceipt([r1, r2], r1.id, {
      merchant: "New Name",
      amount: 200,
    });
    expect(updated[0]!.merchant).toBe("New Name");
    expect(updated[0]!.amount).toBe(200);
    expect(updated[1]!.merchant).toBe("Keep");
  });

  it("does nothing when id is not found", () => {
    const list = [makeReceipt({ merchant: "Only" })];
    const updated = updateReceipt(list, 999, { merchant: "Ghost" });
    expect(updated).toHaveLength(1);
    expect(updated[0]!.merchant).toBe("Only");
  });

  it("does not mutate the original array", () => {
    const original = [makeReceipt({ merchant: "Original" })];
    const copy = updateReceipt(original, original[0]!.id, { merchant: "Updated" });
    expect(original[0]!.merchant).toBe("Original");
    expect(copy[0]!.merchant).toBe("Updated");
  });
});

// ---------------------------------------------------------------------------
// deleteReceipt
// ---------------------------------------------------------------------------

describe("deleteReceipt", () => {
  it("removes the receipt with the given id", () => {
    const r1 = makeReceipt({ merchant: "Keep" });
    const r2 = makeReceipt({ merchant: "Remove" });
    const result = deleteReceipt([r1, r2], r2.id);
    expect(result).toHaveLength(1);
    expect(result[0]!.merchant).toBe("Keep");
  });

  it("returns the same list when id is not found", () => {
    const list = [makeReceipt({ merchant: "Only" })];
    const result = deleteReceipt(list, 999);
    expect(result).toHaveLength(1);
  });

  it("does not mutate the original array", () => {
    const original = [makeReceipt(), makeReceipt()];
    const copy = deleteReceipt(original, original[0]!.id);
    expect(original).toHaveLength(2);
    expect(copy).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// computeReceiptSummary
// ---------------------------------------------------------------------------

describe("computeReceiptSummary", () => {
  it("computes totals for mixed income and expense receipts", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 5000, category: "sales", taxAmount: 300, serviceCharge: 50 }),
      makeReceipt({ id: 2, amount: 1500, category: "rent", taxAmount: 0, serviceCharge: 0 }),
      makeReceipt({ id: 3, amount: 300, category: "utilities", taxAmount: 15, serviceCharge: 5 }),
    ];
    const summary = computeReceiptSummary(receipts);
    expect(summary.totalAmount).toBe(6800);
    expect(summary.totalTax).toBe(315);
    expect(summary.totalServiceCharge).toBe(55);
    expect(summary.receiptCount).toBe(3);
  });

  it("groups and sums by category", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 1000, category: "sales" }),
      makeReceipt({ id: 2, amount: 500, category: "sales" }),
      makeReceipt({ id: 3, amount: 200, category: "utilities" }),
    ];
    const summary = computeReceiptSummary(receipts);
    const salesSummary = summary.categorySummaries.find((c) => c.category === "sales");
    expect(salesSummary?.total).toBe(1500);
    expect(salesSummary?.count).toBe(2);

    const utilitiesSummary = summary.categorySummaries.find((c) => c.category === "utilities");
    expect(utilitiesSummary?.total).toBe(200);
    expect(utilitiesSummary?.count).toBe(1);
  });

  it("sorts category summaries by total descending", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 100, category: "utilities" }),
      makeReceipt({ id: 2, amount: 5000, category: "sales" }),
      makeReceipt({ id: 3, amount: 1500, category: "rent" }),
    ];
    const summary = computeReceiptSummary(receipts);
    expect(summary.categorySummaries[0]!.category).toBe("sales");
    expect(summary.categorySummaries[1]!.category).toBe("rent");
    expect(summary.categorySummaries[2]!.category).toBe("utilities");
  });

  it("returns zero totals for empty receipt list", () => {
    const summary = computeReceiptSummary([]);
    expect(summary.totalAmount).toBe(0);
    expect(summary.totalTax).toBe(0);
    expect(summary.totalServiceCharge).toBe(0);
    expect(summary.receiptCount).toBe(0);
    expect(summary.categorySummaries).toHaveLength(0);
  });

  it("handles neutral categories (transfer, uncategorised)", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 1000, category: "transfer" }),
      makeReceipt({ id: 2, amount: 500, category: "uncategorised" }),
    ];
    const summary = computeReceiptSummary(receipts);
    expect(summary.totalAmount).toBe(1500);
    expect(summary.categorySummaries).toHaveLength(2);
  });

  it("handles very large amounts", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 1500000, category: "sales", taxAmount: 90000, serviceCharge: 15000 }),
      makeReceipt({ id: 2, amount: 2750000.5, category: "purchases", taxAmount: 165000.03, serviceCharge: 27500 }),
    ];
    const summary = computeReceiptSummary(receipts);
    expect(summary.totalAmount).toBe(4250000.5);
    expect(summary.totalTax).toBe(255000.03);
    expect(summary.totalServiceCharge).toBe(42500);
    expect(summary.receiptCount).toBe(2);
  });

  it("handles mix of income, expense, and neutral categories together", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 10000, category: "sales" }),
      makeReceipt({ id: 2, amount: 3000, category: "rent" }),
      makeReceipt({ id: 3, amount: 200, category: "utilities" }),
      makeReceipt({ id: 4, amount: 500, category: "other_income" }),
      makeReceipt({ id: 5, amount: 1000, category: "transfer" }),
      makeReceipt({ id: 6, amount: 0, category: "uncategorised" }),
    ];
    const summary = computeReceiptSummary(receipts);
    expect(summary.totalAmount).toBe(14700);
    expect(summary.receiptCount).toBe(6);
    expect(summary.categorySummaries).toHaveLength(6);
  });
});

// ---------------------------------------------------------------------------
// exportReceiptCsv
// ---------------------------------------------------------------------------

describe("exportReceiptCsv", () => {
  it("exports receipts to CSV with expected headers", () => {
    const receipts: Receipt[] = [
      makeReceipt({
        id: 1,
        date: "2024-01-15",
        merchant: "Kedai Ali",
        amount: 150.5,
        paymentMethod: "cash" as PaymentMethod,
        category: "purchases" as CategorySlug,
        taxAmount: 9.03,
        serviceCharge: 5,
        notes: "Stationery",
      }),
    ];
    const csv = exportReceiptCsv(receipts);
    expect(csv).toContain("date");
    expect(csv).toContain("merchant");
    expect(csv).toContain("amount");
    expect(csv).toContain("paymentMethod");
    expect(csv).toContain("category");
    expect(csv).toContain("taxAmount");
    expect(csv).toContain("serviceCharge");
    expect(csv).toContain("notes");
    expect(csv).toContain("2024-01-15");
    expect(csv).toContain("Kedai Ali");
    expect(csv).toContain("150.50");
    expect(csv).toContain("9.03");
    expect(csv).toContain("5.00");
    expect(csv).toContain("Stationery");
  });

  it("uses bilingual category labels", () => {
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, category: "sales" as CategorySlug }),
    ];
    const csv = exportReceiptCsv(receipts);
    expect(csv).toContain("Jualan / Sales");
  });

  it("handles empty receipt list", () => {
    const csv = exportReceiptCsv([]);
    // Papa.unparse with an empty array returns an empty string
    expect(csv).toBe("");
  });
});

// ---------------------------------------------------------------------------
// exportReceiptCsv round-trip
// ---------------------------------------------------------------------------

describe("CSV round-trip", () => {
  it("parses what was exported", () => {
    const receipts: Receipt[] = [
      makeReceipt({
        id: 1,
        date: "2024-01-15",
        merchant: "Kedai Ali",
        amount: 150.5,
        paymentMethod: "cash" as PaymentMethod,
        category: "purchases" as CategorySlug,
        taxAmount: 9.03,
        serviceCharge: 5,
        notes: "Stationery",
      }),
      makeReceipt({
        id: 2,
        date: "2024-03-20",
        merchant: "Restoran Maju",
        amount: 89.9,
        paymentMethod: "card" as PaymentMethod,
        category: "utilities" as CategorySlug,
        taxAmount: 5.39,
        serviceCharge: 8.99,
        notes: "Team lunch",
      }),
    ];

    const csv = exportReceiptCsv(receipts);
    const result = parseReceiptCsv(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.receipts).toHaveLength(2);

    const r1 = result.receipts[0]!;
    expect(r1.date).toBe("2024-01-15");
    expect(r1.merchant).toBe("Kedai Ali");
    expect(r1.amount).toBe(150.5);
    expect(r1.paymentMethod).toBe("cash");
    expect(r1.category).toBe("purchases");
    expect(r1.taxAmount).toBe(9.03);
    expect(r1.serviceCharge).toBe(5);
    expect(r1.notes).toBe("Stationery");

    const r2 = result.receipts[1]!;
    expect(r2.merchant).toBe("Restoran Maju");
    expect(r2.amount).toBe(89.9);
    expect(r2.category).toBe("utilities");
  });
});

// ---------------------------------------------------------------------------
// exportReceiptJson
// ---------------------------------------------------------------------------

describe("exportReceiptJson", () => {
  it("exports JSON with receipts array and summary", () => {
    const receipts: Receipt[] = [
      makeReceipt({
        id: 1,
        date: "2024-01-15",
        merchant: "Kedai Ali",
        amount: 150,
        paymentMethod: "cash" as PaymentMethod,
        category: "purchases" as CategorySlug,
        taxAmount: 9,
        serviceCharge: 5,
        notes: "",
      }),
    ];
    const json = exportReceiptJson(receipts);
    const parsed = JSON.parse(json);

    expect(parsed).toHaveProperty("receipts");
    expect(parsed).toHaveProperty("summary");
    expect(parsed.receipts).toHaveLength(1);
    expect(parsed.receipts[0].merchant).toBe("Kedai Ali");
    expect(parsed.summary.totalAmount).toBe(150);
    expect(parsed.summary.totalTax).toBe(9);
    expect(parsed.summary.totalServiceCharge).toBe(5);
    expect(parsed.summary.receiptCount).toBe(1);
  });

  it("handles empty receipt list", () => {
    const json = exportReceiptJson([]);
    const parsed = JSON.parse(json);
    expect(parsed.receipts).toHaveLength(0);
    expect(parsed.summary.receiptCount).toBe(0);
    expect(parsed.summary.totalAmount).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// parseReceiptCsv
// ---------------------------------------------------------------------------

describe("parseReceiptCsv", () => {
  it("parses CSV with English headers", () => {
    const csv = `date,merchant,amount,payment_method,category,tax,service_charge,notes
2024-01-15,Kedai Ali,150.50,cash,purchases,9.03,5.00,Stationery
2024-01-16,Pasar Mini,45.00,card,office_supplies,0,0,`;
    const result = parseReceiptCsv(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.receipts).toHaveLength(2);

    const r1 = result.receipts[0]!;
    expect(r1.date).toBe("2024-01-15");
    expect(r1.merchant).toBe("Kedai Ali");
    expect(r1.amount).toBe(150.5);
    expect(r1.paymentMethod).toBe("cash");
    expect(r1.category).toBe("purchases");
    expect(r1.taxAmount).toBe(9.03);
    expect(r1.serviceCharge).toBe(5);
    expect(r1.notes).toBe("Stationery");

    const r2 = result.receipts[1]!;
    expect(r2.merchant).toBe("Pasar Mini");
    expect(r2.amount).toBe(45);
    expect(r2.paymentMethod).toBe("card");
    expect(r2.category).toBe("office_supplies");
    expect(r2.taxAmount).toBe(0);
    expect(r2.serviceCharge).toBe(0);
    expect(r2.notes).toBe("");
  });

  it("parses CSV with Malay headers", () => {
    const csv = `tarikh,peniaga,jumlah,kaedah_bayaran,kategori,cukai,caj_perkhidmatan,nota
2024-01-15,Kedai Ali,150.50,tunai,sales,9.03,5.00,Alat tulis
2024-01-16,Pasar Mini,45.00,kad,purchases,0,0,Bekalan dapur`;
    const result = parseReceiptCsv(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.receipts).toHaveLength(2);

    expect(result.detectedColumns.dateCol).toBe("tarikh");
    expect(result.detectedColumns.merchantCol).toBe("peniaga");
    expect(result.detectedColumns.amountCol).toBe("jumlah");
    expect(result.detectedColumns.paymentMethodCol).toBe("kaedah_bayaran");
    expect(result.detectedColumns.categoryCol).toBe("kategori");
    expect(result.detectedColumns.taxCol).toBe("cukai");
    expect(result.detectedColumns.serviceChargeCol).toBe("caj_perkhidmatan");
    expect(result.detectedColumns.notesCol).toBe("nota");

    const r1 = result.receipts[0]!;
    expect(r1.date).toBe("2024-01-15");
    expect(r1.merchant).toBe("Kedai Ali");
    expect(r1.amount).toBe(150.5);
    expect(r1.paymentMethod).toBe("cash"); // tunai maps to cash
    expect(r1.category).toBe("sales");
    expect(r1.taxAmount).toBe(9.03);
    expect(r1.serviceCharge).toBe(5);
    expect(r1.notes).toBe("Alat tulis");

    const r2 = result.receipts[1]!;
    expect(r2.merchant).toBe("Pasar Mini");
    expect(r2.paymentMethod).toBe("card"); // kad maps to card
    expect(r2.category).toBe("purchases");
  });

  it("handles bilingual category labels on re-import", () => {
    const csv = `date,merchant,amount,paymentMethod,category,taxAmount,serviceCharge,notes
2024-01-15,Kedai Ali,150.50,cash,Jualan / Sales,9.03,5.00,`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts).toHaveLength(1);
    expect(result.receipts[0]!.category).toBe("sales");
  });

  it("returns errors when CSV is empty", () => {
    const result = parseReceiptCsv("");
    expect(result.errors).toContain("CSV input is empty.");
    expect(result.receipts).toHaveLength(0);
  });

  it("returns a warning when no columns are detected", () => {
    const csv = `Col1,Col2\nval1,val2`;
    const result = parseReceiptCsv(csv);
    const hasWarning = result.errors.some((e) => e.includes("auto-detect"));
    expect(hasWarning).toBe(true);
  });

  it("skips fully empty rows", () => {
    const csv = `date,merchant,amount\n2024-01-15,Shop,100\n,\n2024-01-16,Another,200`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts).toHaveLength(2);
  });

  it("parses amounts with commas", () => {
    const csv = `date,merchant,amount\n2024-01-15,Purchase,"1,500.50"`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts[0]!.amount).toBe(1500.5);
  });

  it("parses RM currency prefix amounts", () => {
    const csv = `date,merchant,amount\n2024-01-15,Purchase,"RM1,200.50"\n2024-01-16,Sale,"RM 2,500.00"`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts).toHaveLength(2);
    expect(result.receipts[0]!.amount).toBe(1200.5);
    expect(result.receipts[1]!.amount).toBe(2500);
  });

  it("handles empty CSV with only headers", () => {
    const csv = `date,merchant,amount,payment_method,category,tax,service_charge,notes\n`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts).toHaveLength(0);
    expect(result.detectedColumns.dateCol).toBe("date");
    expect(result.detectedColumns.merchantCol).toBe("merchant");
  });

  it("defaults unknown payment methods to other", () => {
    const csv = `date,merchant,amount,payment_method\n2024-01-15,Shop,100,unknown_method`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts[0]!.paymentMethod).toBe("other");
  });

  it("defaults unknown categories to uncategorised", () => {
    const csv = `date,merchant,amount,category\n2024-01-15,Shop,100,unknown_category`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts[0]!.category).toBe("uncategorised");
  });

  it("handles quoted fields with embedded commas", () => {
    const csv = `date,merchant,amount,notes\n2024-01-15,"Kedai, Ali & Sons",150.50,"Pens, pencils, and paper"`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts).toHaveLength(1);
    expect(result.receipts[0]!.merchant).toBe("Kedai, Ali & Sons");
    expect(result.receipts[0]!.notes).toBe("Pens, pencils, and paper");
  });

  it("handles negative amount values", () => {
    const csv = `date,merchant,amount\n2024-01-15,Refund,-50.00`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts[0]!.amount).toBe(-50);
  });

  it("handles varying date formats", () => {
    const csv = `date,merchant,amount\n15/01/2024,Invoice,100\n2024-01-16,Another,200\n16-JAN-2024,Third,50`;
    const result = parseReceiptCsv(csv);
    expect(result.receipts).toHaveLength(3);
    expect(result.receipts[0]!.date).toBe("15/01/2024");
    expect(result.receipts[1]!.date).toBe("2024-01-16");
    expect(result.receipts[2]!.date).toBe("16-JAN-2024");
  });
});
