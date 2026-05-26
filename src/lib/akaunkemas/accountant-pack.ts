import JSZip from "jszip";
import Papa from "papaparse";
import type { AccountantPackFiles, AccountantPackInput, Receipt } from "./types";
import { exportCleanedCsv } from "./export-cleaned-csv";
import { computeMonthlySummary } from "./summary";
import { generateMonthlySummaryPdf } from "./pdf-export";
import { getCategoryLabel } from "./categories";

/**
 * Generate all files for the Accountant Pack.
 * Returns an array of { filename, content, mimeType } objects
 * that represent all the files to include in the accountant pack.
 */
export async function generateAccountantPack(
  input: AccountantPackInput,
): Promise<AccountantPackFiles[]> {
  const files: AccountantPackFiles[] = [];

  // 1. cleaned-transactions.csv
  const cleanedCsv = exportCleanedCsv(input.transactions);
  files.push({
    filename: "cleaned-transactions.csv",
    content: cleanedCsv,
    mimeType: "text/csv",
  });

  // 2. receipt-list.csv
  const receiptCsv = generateReceiptCsv(input.receipts);
  files.push({
    filename: "receipt-list.csv",
    content: receiptCsv,
    mimeType: "text/csv",
  });

  // 3. monthly-summary.json
  const summary = computeMonthlySummary(input.transactions);
  const summaryJson = JSON.stringify(
    {
      totalIncome: summary.totalIncome,
      totalExpense: summary.totalExpense,
      netCashflow: summary.netCashflow,
      transactionCount: summary.transactionCount,
      categoryBreakdown: summary.categorySummaries.map((cs) => ({
        category: getCategoryLabel(cs.category),
        slug: cs.category,
        total: cs.total,
        count: cs.count,
      })),
    },
    null,
    2,
  );
  files.push({
    filename: "monthly-summary.json",
    content: summaryJson,
    mimeType: "application/json",
  });

  // 4. unmatched-transactions.csv (optional)
  if (input.unmatchedTransactions && input.unmatchedTransactions.length > 0) {
    const unmatchedCsv = exportCleanedCsv(input.unmatchedTransactions);
    files.push({
      filename: "unmatched-transactions.csv",
      content: unmatchedCsv,
      mimeType: "text/csv",
    });
  }

  // 5. accountant-summary.pdf
  const periodStart =
    input.transactions.length > 0
      ? input.transactions.reduce((a, b) => (a.date < b.date ? a : b)).date
      : undefined;
  const periodEnd =
    input.transactions.length > 0
      ? input.transactions.reduce((a, b) => (a.date > b.date ? a : b)).date
      : undefined;

  const pdfBytes = await generateMonthlySummaryPdf({
    summary,
    periodStart,
    periodEnd,
  });
  files.push({
    filename: "accountant-summary.pdf",
    content: pdfBytes,
    mimeType: "application/pdf",
  });

  // 6. notes.txt (optional)
  if (input.notes && input.notes.trim().length > 0) {
    files.push({
      filename: "notes.txt",
      content: input.notes.trim(),
      mimeType: "text/plain",
    });
  }

  return files;
}

/**
 * Generate a receipt CSV string from an array of Receipt objects.
 * Columns: date, merchant, amount, category, paymentMethod, taxAmount, serviceCharge, notes
 */
function generateReceiptCsv(receipts: Receipt[]): string {
  if (receipts.length === 0) {
    // Return header-only CSV
    return "date,merchant,amount,category,paymentMethod,taxAmount,serviceCharge,notes";
  }

  const rows = receipts.map((r) => ({
    date: r.date,
    merchant: r.merchant,
    amount: r.amount.toFixed(2),
    category: getCategoryLabel(r.category),
    paymentMethod: r.paymentMethod,
    taxAmount: r.taxAmount.toFixed(2),
    serviceCharge: r.serviceCharge.toFixed(2),
    notes: r.notes,
  }));

  return Papa.unparse(rows);
}

/**
 * Generate a ZIP blob containing all accountant pack files.
 */
export async function generateAccountantPackZip(
  input: AccountantPackInput,
): Promise<Blob> {
  const zip = new JSZip();
  const files = await generateAccountantPack(input);

  for (const file of files) {
    zip.file(file.filename, file.content);
  }

  return zip.generateAsync({ type: "blob" });
}
