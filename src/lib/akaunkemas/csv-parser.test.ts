import { describe, expect, it } from "vitest";
import { detectColumns, parseBankCsv, validateParseResult } from "./csv-parser";

describe("detectColumns", () => {
  it("detects standard English headers (date, description, debit, credit, balance)", () => {
    const result = detectColumns(["Date", "Description", "Debit", "Credit", "Balance"]);
    expect(result.dateCol).toBe("Date");
    expect(result.descCol).toBe("Description");
    expect(result.debitCol).toBe("Debit");
    expect(result.creditCol).toBe("Credit");
    expect(result.balanceCol).toBe("Balance");
    expect(result.amountCol).toBeNull();
  });

  it("detects Malay headers (tarikh, perihal, debit, kredit, baki)", () => {
    const result = detectColumns(["Tarikh", "Perihal", "Debit", "Kredit", "Baki"]);
    expect(result.dateCol).toBe("Tarikh");
    expect(result.descCol).toBe("Perihal");
    expect(result.debitCol).toBe("Debit");
    expect(result.creditCol).toBe("Kredit");
    expect(result.balanceCol).toBe("Baki");
  });

  it("detects signed amount column (no separate debit/credit)", () => {
    const result = detectColumns(["Transaction Date", "Narrative", "Amount", "Balance"]);
    expect(result.dateCol).toBe("Transaction Date");
    expect(result.descCol).toBe("Narrative");
    expect(result.amountCol).toBe("Amount");
    expect(result.debitCol).toBeNull();
    expect(result.creditCol).toBeNull();
  });

  it("returns nulls when nothing matches", () => {
    const result = detectColumns(["Col1", "Col2", "Col3"]);
    expect(result.dateCol).toBeNull();
    expect(result.descCol).toBeNull();
    expect(result.debitCol).toBeNull();
  });

  it("handles empty headers", () => {
    const result = detectColumns([]);
    expect(result.dateCol).toBeNull();
    expect(result.descCol).toBeNull();
  });

  it("detects Malay variant (tarikh, butiran, debit, kredit, baki)", () => {
    const result = detectColumns(["Tarikh", "Butiran", "Debit", "Kredit", "Baki"]);
    expect(result.dateCol).toBe("Tarikh");
    expect(result.descCol).toBe("Butiran");
    expect(result.debitCol).toBe("Debit");
    expect(result.creditCol).toBe("Kredit");
    expect(result.balanceCol).toBe("Baki");
  });

  it("detects Malay single-amount headers (tarikh, butiran, jumlah)", () => {
    const result = detectColumns(["Tarikh", "Butiran", "Jumlah"]);
    expect(result.dateCol).toBe("Tarikh");
    expect(result.descCol).toBe("Butiran");
    expect(result.amountCol).toBe("Jumlah");
    expect(result.debitCol).toBeNull();
    expect(result.creditCol).toBeNull();
  });

  it("detects pengeluaran/pendapatan as Malay debit/credit", () => {
    const result = detectColumns([
      "Tarikh",
      "Perihal",
      "Pengeluaran",
      "Pendapatan",
      "Baki",
    ]);
    expect(result.debitCol).toBe("Pengeluaran");
    expect(result.creditCol).toBe("Pendapatan");
  });
});

describe("parseBankCsv", () => {
  it("parses standard debit/credit CSV", () => {
    const csv = `Date,Description,Debit,Credit,Balance\n2024-01-15,Payment to Supplier,1500.00,,8500.00\n2024-01-16,Customer Payment,,3000.00,11500.00`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(2);

    const t1 = result.transactions[0]!;
    expect(t1.date).toBe("2024-01-15");
    expect(t1.description).toBe("Payment to Supplier");
    expect(t1.debit).toBe(1500);
    expect(t1.credit).toBe(0);
    expect(t1.amount).toBe(-1500);
    expect(t1.balance).toBe(8500);
    expect(t1.category).toBe("uncategorised");

    const t2 = result.transactions[1]!;
    expect(t2.credit).toBe(3000);
    expect(t2.amount).toBe(3000);
  });

  it("parses signed amount column", () => {
    const csv = `Date,Description,Amount\n2024-01-15,Payment,-1500.00\n2024-01-16,Deposit,3000.00`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    expect(result.transactions[0]!.debit).toBe(1500);
    expect(result.transactions[0]!.credit).toBe(0);
    expect(result.transactions[1]!.credit).toBe(3000);
    expect(result.transactions[1]!.debit).toBe(0);
  });

  it("handles amounts with commas", () => {
    const csv = `Date,Description,Debit,Credit\n2024-01-15,Payment,"1,500.50",`;
    const result = parseBankCsv(csv);
    expect(result.transactions[0]!.debit).toBe(1500.5);
  });

  it("detects Malay headers automatically", () => {
    const csv = `Tarikh,Perihal,Debit,Kredit,Baki\n2024-01-15,Bayaran Sewa,2000.00,,8000.00`;
    const result = parseBankCsv(csv);
    expect(result.detectedColumns.dateCol).toBe("Tarikh");
    expect(result.detectedColumns.descCol).toBe("Perihal");
    expect(result.transactions).toHaveLength(1);
    expect(result.transactions[0]!.description).toBe("Bayaran Sewa");
  });

  it("returns errors when CSV is empty", () => {
    const result = parseBankCsv("");
    expect(result.errors).toContain("CSV input is empty.");
    expect(result.transactions).toHaveLength(0);
  });

  it("returns a warning when no columns are detected", () => {
    const csv = `Col1,Col2\nval1,val2`;
    const result = parseBankCsv(csv);
    const hasWarning = result.errors.some((e) => e.includes("auto-detect"));
    expect(hasWarning).toBe(true);
  });

  it("skips fully empty rows", () => {
    const csv = `Date,Description,Debit,Credit\n2024-01-15,Payment,100,,\n,,\n2024-01-16,Deposit,,200`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
  });

  it("handles quoted descriptions with embedded commas", () => {
    const csv = `Date,Description,Debit,Credit\n2024-01-15,"Payment, to Supplier Co.",150.50,\n2024-01-16,"Refund, partial",,75.25`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    expect(result.transactions[0]!.description).toBe("Payment, to Supplier Co.");
    expect(result.transactions[0]!.debit).toBe(150.5);
    expect(result.transactions[1]!.description).toBe("Refund, partial");
    expect(result.transactions[1]!.credit).toBe(75.25);
  });

  it("handles negative balance values", () => {
    const csv = `Date,Description,Debit,Credit,Balance\n2024-01-15,Overdraft Fee,50.00,,-200.00\n2024-01-16,Deposit,,500.00,300.00`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    expect(result.transactions[0]!.balance).toBe(-200);
    expect(result.transactions[1]!.balance).toBe(300);
  });

  it("handles very large amounts", () => {
    const csv = `Date,Description,Debit,Credit\n2024-01-15,Raw Material Purchase,"1,500,000.00",\n2024-01-16,Big Sale,,"2,750,000.50"`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    expect(result.transactions[0]!.debit).toBe(1500000);
    expect(result.transactions[1]!.credit).toBe(2750000.5);
  });

  it("handles varying date formats", () => {
    const csv = `Date,Description,Debit,Credit\n15/01/2024,Invoice Payment,100,\n2024-01-16,Another Payment,200,\n16-JAN-2024,Third Payment,50,`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(3);
    // Dates are stored as-is from the CSV (no normalization in parser)
    expect(result.transactions[0]!.date).toBe("15/01/2024");
    expect(result.transactions[1]!.date).toBe("2024-01-16");
    expect(result.transactions[2]!.date).toBe("16-JAN-2024");
  });

  it("handles empty CSV with only headers", () => {
    const csv = `Date,Description,Debit,Credit,Balance\n`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(0);
    expect(result.detectedColumns.dateCol).toBe("Date");
    expect(result.detectedColumns.descCol).toBe("Description");
    // No data rows, so validateParseResult should flag it
    const issues = validateParseResult(result);
    expect(issues.some((i) => i.includes("No transactions"))).toBe(true);
  });

  it("skips metadata rows before actual CSV headers (Maybank-style)", () => {
    const csv = `Bank Statement - Maybank
Account: 1234567890
Statement Period: Jan 2024
Date,Description,Debit,Credit,Balance
2024-01-15,Payment to Supplier,1500.00,,8500.00
2024-01-16,Customer Deposit,,3000.00,11500.00`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(2);
    expect(result.detectedColumns.dateCol).toBe("Date");
    expect(result.detectedColumns.descCol).toBe("Description");
    expect(result.transactions[0]!.debit).toBe(1500);
    expect(result.transactions[0]!.description).toBe("Payment to Supplier");
    expect(result.transactions[1]!.credit).toBe(3000);
    expect(result.transactions[1]!.balance).toBe(11500);
  });

  it("skips metadata rows before Malay CSV headers", () => {
    const csv = `Penyata Bank - CIMB
No Akaun: 9876543210
Tarikh,Butiran,Debit,Kredit,Baki
2024-01-15,Bayaran Sewa,2000.00,,8000.00
2024-01-16,Deposit Pelanggan,,5000.00,13000.00`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(2);
    expect(result.detectedColumns.dateCol).toBe("Tarikh");
    expect(result.detectedColumns.descCol).toBe("Butiran");
    expect(result.transactions[0]!.debit).toBe(2000);
    expect(result.transactions[1]!.credit).toBe(5000);
  });

  it("handles metadata rows with commas before multi-column header", () => {
    const csv = `Bank Name, Maybank Berhad
Account No, 1234567890
Date,Description,Debit,Credit
2024-01-15,Transfer Out,500.00,
2024-01-16,Transfer In,,750.00`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(2);
    expect(result.transactions[0]!.debit).toBe(500);
    expect(result.transactions[1]!.credit).toBe(750);
  });

  it("handles whitespace-only rows interspersed with data", () => {
    const csv = `Date,Description,Debit,Credit
2024-01-15,Payment,100,
   ,   ,   ,
2024-01-16,Deposit,,200
   ,   ,
2024-01-17,Transfer,50,`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(3);
    expect(result.transactions[0]!.debit).toBe(100);
    expect(result.transactions[1]!.credit).toBe(200);
    expect(result.transactions[2]!.debit).toBe(50);
  });

  it("parses RM currency prefix amounts", () => {
    const csv = `Date,Description,Debit,Credit
2024-01-15,Bill payment,"RM1,200.50",
2024-01-16,Sale receipt,,"RM 2,500.00"
2024-01-17,Refund,"RM 150.75",`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(3);
    expect(result.transactions[0]!.debit).toBe(1200.5);
    expect(result.transactions[1]!.credit).toBe(2500);
    expect(result.transactions[2]!.debit).toBe(150.75);
  });

  it("parses parenthesized negative amounts (accounting notation)", () => {
    const csv = `Date,Description,Debit,Credit
2024-01-15,Service charge,"(25.50)",
2024-01-16,Big purchase,"(1,200.00)",`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    // Parenthesized in debit column → treated as a credit (reversal)
    expect(result.transactions[0]!.debit).toBe(0);
    expect(result.transactions[0]!.credit).toBe(25.5);
    expect(result.transactions[1]!.debit).toBe(0);
    expect(result.transactions[1]!.credit).toBe(1200);
  });

  it("parses space-separated thousand amounts", () => {
    const csv = `Date,Description,Debit,Credit
2024-01-15,Payment,"1 200.50",
2024-01-16,Deposit,,"2 500.00"`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    expect(result.transactions[0]!.debit).toBe(1200.5);
    expect(result.transactions[1]!.credit).toBe(2500);
  });

  it("parses negative amounts with minus sign", () => {
    const csv = `Date,Description,Debit,Credit
2024-01-15,Payment,"-1,200.50",
2024-01-16,Fee,-50.00,`;
    const result = parseBankCsv(csv);
    expect(result.transactions).toHaveLength(2);
    // Negative debit → treated as credit (reversal)
    expect(result.transactions[0]!.debit).toBe(0);
    expect(result.transactions[0]!.credit).toBe(1200.5);
    expect(result.transactions[1]!.debit).toBe(0);
    expect(result.transactions[1]!.credit).toBe(50);
  });
});

describe("validateParseResult", () => {
  it("reports no transactions found", () => {
    const result = parseBankCsv("Date,Description,Debit,Credit\n");
    const issues = validateParseResult(result);
    expect(issues.some((i) => i.includes("No transactions"))).toBe(true);
  });

  it("reports missing dates", () => {
    const csv = `Date,Description,Debit,Credit\n,Payment,100,`;
    const result = parseBankCsv(csv);
    const issues = validateParseResult(result);
    expect(issues.some((i) => i.includes("missing dates"))).toBe(true);
  });

  it("returns empty issues for valid result", () => {
    const csv = `Date,Description,Debit,Credit\n2024-01-15,Payment,100,`;
    const result = parseBankCsv(csv);
    const issues = validateParseResult(result);
    expect(issues).toHaveLength(0);
  });
});

describe("Malaysian bank CSV samples", () => {
  it("parses English Maybank-style CSV (Date, Description, Debit, Credit)", () => {
    const csv = `Date,Description,Debit,Credit
2026-05-01,Sales transfer,,1200.00
2026-05-02,Office rent,500.00,
2026-05-03,TNB Electricity,180.50,
2026-05-04,Bank charge,2.00,
2026-05-05,Owner transfer,,300.00`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(5);
    expect(result.transactions[0]!.credit).toBe(1200);
    expect(result.transactions[0]!.amount).toBe(1200);
    expect(result.transactions[1]!.debit).toBe(500);
    expect(result.transactions[1]!.amount).toBe(-500);
    expect(result.transactions[2]!.debit).toBe(180.5);
    expect(result.transactions[3]!.debit).toBe(2);
    expect(result.transactions[4]!.credit).toBe(300);
  });

  it("parses Malay BSN-style CSV (Tarikh, Butiran, Debit, Kredit)", () => {
    const csv = `Tarikh,Butiran,Debit,Kredit
2026-05-01,Jualan pelanggan,,1500.00
2026-05-02,Sewa kedai,700.00,
2026-05-03,Bil utiliti,220.00,
2026-05-04,Caj bank,3.00,
2026-05-05,Belian stok,450.00,`;
    const result = parseBankCsv(csv);
    expect(result.errors).toHaveLength(0);
    expect(result.transactions).toHaveLength(5);
    expect(result.detectedColumns.dateCol).toBe("Tarikh");
    expect(result.detectedColumns.descCol).toBe("Butiran");
    expect(result.detectedColumns.debitCol).toBe("Debit");
    expect(result.detectedColumns.creditCol).toBe("Kredit");
    expect(result.transactions[0]!.credit).toBe(1500);
    expect(result.transactions[1]!.debit).toBe(700);
    expect(result.transactions[2]!.debit).toBe(220);
    expect(result.transactions[4]!.debit).toBe(450);
  });
});
