import { describe, expect, it } from "vitest";
import { generateAccountantPack, generateAccountantPackZip } from "./accountant-pack";
import type { Transaction, Receipt, CategorySlug, AccountantPackFiles } from "./types";

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

function makeReceipt(overrides: Partial<Receipt> = {}): Receipt {
  return {
    id: 1,
    date: "2024-01-15",
    merchant: "Test Merchant",
    amount: 100.0,
    paymentMethod: "card",
    category: "office_supplies",
    taxAmount: 6.0,
    serviceCharge: 0,
    notes: "",
    imageRef: null,
    ...overrides,
  };
}

async function filesToMap(files: AccountantPackFiles[]): Promise<Map<string, string>> {
  const map = new Map<string, string>();
  for (const f of files) {
    const content = typeof f.content === "string" ? f.content : new TextDecoder().decode(f.content);
    map.set(f.filename, content);
  }
  return map;
}

describe("generateAccountantPack", () => {
  it("returns correct file names with all options provided", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: -1500, debit: 1500, category: "rent" }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, amount: 100, category: "office_supplies" }),
    ];
    const unmatched: Transaction[] = [
      makeTx({ id: 99, amount: -250, debit: 250, category: "uncategorised" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts,
      unmatchedTransactions: unmatched,
      notes: "Test notes for accountant.",
    });

    const names = files.map((f) => f.filename).sort();
    expect(names).toEqual([
      "accountant-summary.pdf",
      "cleaned-transactions.csv",
      "monthly-summary.json",
      "notes.txt",
      "receipt-list.csv",
      "unmatched-transactions.csv",
    ]);
  });

  it("cleaned-transactions.csv has correct content", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, date: "2024-01-15", amount: 5000, credit: 5000, description: "Client payment", category: "sales" }),
      makeTx({ id: 2, date: "2024-01-20", amount: -1500, debit: 1500, description: "Office rent", category: "rent" }),
    ];

    const files = await generateAccountantPack({ transactions: txs, receipts: [] });
    const map = await filesToMap(files);
    const csv = map.get("cleaned-transactions.csv");
    expect(csv).toBeDefined();

    // Check CSV has header + 2 data rows
    const lines = csv!.split("\n").filter((l) => l.trim());
    expect(lines.length).toBeGreaterThanOrEqual(2); // header + at least 1 row

    // Check for key data
    expect(csv).toContain("Client payment");
    expect(csv).toContain("Office rent");
    expect(csv).toContain("Jualan / Sales");
    expect(csv).toContain("Sewa / Rent");
  });

  it("monthly-summary.json has correct structure", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: -1500, debit: 1500, category: "rent" }),
      makeTx({ id: 3, amount: -200, debit: 200, category: "utilities" }),
    ];

    const files = await generateAccountantPack({ transactions: txs, receipts: [] });
    const map = await filesToMap(files);
    const jsonStr = map.get("monthly-summary.json");
    expect(jsonStr).toBeDefined();

    const json = JSON.parse(jsonStr!);
    expect(json.totalIncome).toBe(5000);
    expect(json.totalExpense).toBe(1700);
    expect(json.netCashflow).toBe(3300);
    expect(json.transactionCount).toBe(3);
    expect(Array.isArray(json.categoryBreakdown)).toBe(true);
    expect(json.categoryBreakdown.length).toBeGreaterThan(0);
  });

  it("receipt-list.csv has correct headers and data", async () => {
    const txs: Transaction[] = [];
    const receipts: Receipt[] = [
      makeReceipt({
        id: 1,
        date: "2024-01-15",
        merchant: "Stationery World",
        amount: 100.0,
        paymentMethod: "card",
        category: "office_supplies",
        taxAmount: 6.0,
        serviceCharge: 0,
        notes: "Pens and paper",
      }),
    ];

    const files = await generateAccountantPack({ transactions: txs, receipts });
    const map = await filesToMap(files);
    const csv = map.get("receipt-list.csv");
    expect(csv).toBeDefined();

    // Check headers
    expect(csv).toContain("date");
    expect(csv).toContain("merchant");
    expect(csv).toContain("amount");
    expect(csv).toContain("category");
    expect(csv).toContain("paymentMethod");
    expect(csv).toContain("taxAmount");
    expect(csv).toContain("serviceCharge");
    expect(csv).toContain("notes");

    // Check data
    expect(csv).toContain("Stationery World");
    expect(csv).toContain("Bekalan Pejabat / Office Supplies");
    expect(csv).toContain("Pens and paper");
  });

  it("PDF file is present with correct mime type", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    ];

    const files = await generateAccountantPack({ transactions: txs, receipts: [] });
    const pdf = files.find((f) => f.filename === "accountant-summary.pdf");
    expect(pdf).toBeDefined();
    expect(pdf!.mimeType).toBe("application/pdf");
    expect(pdf!.content).toBeInstanceOf(Uint8Array);
    expect((pdf!.content as Uint8Array).length).toBeGreaterThan(0);
  });

  it("notes.txt included when notes provided", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 100, credit: 100, category: "sales" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts: [],
      notes: "Please review category assignments for rent transactions.",
    });

    const map = await filesToMap(files);
    const notes = map.get("notes.txt");
    expect(notes).toBe("Please review category assignments for rent transactions.");
  });

  it("notes.txt not included when notes empty", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 100, credit: 100, category: "sales" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts: [],
    });

    const hasNotes = files.some((f) => f.filename === "notes.txt");
    expect(hasNotes).toBe(false);
  });

  it("notes.txt not included when notes is whitespace only", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 100, credit: 100, category: "sales" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts: [],
      notes: "   ",
    });

    const hasNotes = files.some((f) => f.filename === "notes.txt");
    expect(hasNotes).toBe(false);
  });

  it("unmatched-transactions.csv included when unmatched provided", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    ];
    const unmatched: Transaction[] = [
      makeTx({ id: 99, date: "2024-01-30", amount: -250, debit: 250, description: "Unknown charge", category: "uncategorised" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts: [],
      unmatchedTransactions: unmatched,
    });

    const map = await filesToMap(files);
    const csv = map.get("unmatched-transactions.csv");
    expect(csv).toBeDefined();
    expect(csv).toContain("Unknown charge");
  });

  it("unmatched CSV not included when unmatched empty array", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts: [],
      unmatchedTransactions: [],
    });

    const hasUnmatched = files.some((f) => f.filename === "unmatched-transactions.csv");
    expect(hasUnmatched).toBe(false);
  });

  it("unmatched CSV not included when unmatchedTransactions undefined", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    ];

    const files = await generateAccountantPack({
      transactions: txs,
      receipts: [],
    });

    const hasUnmatched = files.some((f) => f.filename === "unmatched-transactions.csv");
    expect(hasUnmatched).toBe(false);
  });

  it("empty receipts produces receipt CSV with only headers", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 100, credit: 100, category: "sales" }),
    ];

    const files = await generateAccountantPack({ transactions: txs, receipts: [] });
    const map = await filesToMap(files);
    const csv = map.get("receipt-list.csv");
    expect(csv).toBeDefined();

    const lines = csv!.split("\n").filter((l) => l.trim());
    // With papaparse, empty data produces just the header line
    expect(lines.length).toBe(1);
    expect(lines[0]).toContain("date");
  });

  it("handles empty transactions array gracefully", async () => {
    const files = await generateAccountantPack({ transactions: [], receipts: [] });
    const map = await filesToMap(files);

    const jsonStr = map.get("monthly-summary.json");
    expect(jsonStr).toBeDefined();
    const json = JSON.parse(jsonStr!);
    expect(json.totalIncome).toBe(0);
    expect(json.totalExpense).toBe(0);
    expect(json.transactionCount).toBe(0);

    const pdf = files.find((f) => f.filename === "accountant-summary.pdf");
    expect(pdf).toBeDefined();
    expect((pdf!.content as Uint8Array).length).toBeGreaterThan(0);
  });

  it("receipt CSV uses bilingual category labels", async () => {
    const txs: Transaction[] = [];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1, category: "sales", merchant: "Shop A" }),
      makeReceipt({ id: 2, category: "rent", merchant: "Landlord" }),
    ];

    const files = await generateAccountantPack({ transactions: txs, receipts });
    const map = await filesToMap(files);
    const csv = map.get("receipt-list.csv");
    expect(csv).toContain("Jualan / Sales");
    expect(csv).toContain("Sewa / Rent");
  });
});

describe("generateAccountantPackZip", () => {
  it("returns a Blob", async () => {
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    ];

    const blob = await generateAccountantPackZip({
      transactions: txs,
      receipts: [],
    });

    expect(blob).toBeInstanceOf(Blob);
    expect(blob.size).toBeGreaterThan(0);
    expect(blob.type).toBe("application/zip");
  });

  it("ZIP contains all expected files", async () => {
    const JSZip = (await import("jszip")).default;
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
      makeTx({ id: 2, amount: -1500, debit: 1500, category: "rent" }),
    ];
    const receipts: Receipt[] = [
      makeReceipt({ id: 1 }),
    ];

    const blob = await generateAccountantPackZip({
      transactions: txs,
      receipts,
      notes: "Test",
    });

    const arrayBuffer = await blob.arrayBuffer();
    const zip = await JSZip.loadAsync(arrayBuffer);
    const fileNames = Object.keys(zip.files);

    expect(fileNames).toContain("cleaned-transactions.csv");
    expect(fileNames).toContain("receipt-list.csv");
    expect(fileNames).toContain("monthly-summary.json");
    expect(fileNames).toContain("accountant-summary.pdf");
    expect(fileNames).toContain("notes.txt");
  });

  it("ZIP excludes optional files when not provided", async () => {
    const JSZip = (await import("jszip")).default;
    const txs: Transaction[] = [
      makeTx({ id: 1, amount: 5000, credit: 5000, category: "sales" }),
    ];

    const blob = await generateAccountantPackZip({
      transactions: txs,
      receipts: [],
    });

    const arrayBuffer = await blob.arrayBuffer();
    const zip = await JSZip.loadAsync(arrayBuffer);
    const fileNames = Object.keys(zip.files);

    expect(fileNames).not.toContain("notes.txt");
    expect(fileNames).not.toContain("unmatched-transactions.csv");
  });
});
