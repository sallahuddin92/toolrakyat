"use server";

import { getCurrentUser } from "@/lib/auth/dal";
import { createDbTransactionService } from "@/lib/akaunkemas-saas/services/transactions-db";
import { createDbReceiptService } from "@/lib/akaunkemas-saas/services/receipts-db";
import { revalidatePath } from "next/cache";
import crypto from "node:crypto";

const txService = createDbTransactionService();
const receiptService = createDbReceiptService();

// ---------------------------------------------------------------------------
// Context helper
// ---------------------------------------------------------------------------

async function getContext() {
  const session = await getCurrentUser();
  return {
    tenantId: session.tenantId,
    businessId: session.businessId,
    userId: session.userId,
  };
}

// ---------------------------------------------------------------------------
// Save bank transactions (CSV or XLSX)
// ---------------------------------------------------------------------------

export interface SaveBankTransactionInput {
  date: string;
  description: string;
  debit: number;
  credit: number;
  balance: number | null;
  categorySlug: string;
}

export async function saveBankTransactions(
  transactions: SaveBankTransactionInput[],
) {
  const ctx = await getContext();

  let imported = 0;
  let skipped = 0;

  for (const tx of transactions) {
    try {
      txService.create({
        tenantId: ctx.tenantId,
        businessId: ctx.businessId,
        date: tx.date,
        description: tx.description || "(empty)",
        debit: tx.debit,
        credit: tx.credit,
        balance: tx.balance,
        categorySlug: tx.categorySlug || "uncategorised",
        isReconciled: false,
        notes: "",
        source: "csv_import",
        status: "draft",
        importHash: null,
      });
      imported++;
    } catch {
      // Duplicate — skip
      skipped++;
    }
  }

  revalidatePath("/app/akaunkemas/transactions");
  revalidatePath("/app/akaunkemas");
  return { success: true, imported, skipped };
}

// ---------------------------------------------------------------------------
// Save receipt rows
// ---------------------------------------------------------------------------

export interface SaveReceiptInput {
  date: string;
  merchant: string;
  amount: number;
  paymentMethod?: string;
  categorySlug?: string;
  taxAmount?: number;
  serviceCharge?: number;
  notes?: string;
}

export async function saveReceipts(receipts: SaveReceiptInput[]) {
  const ctx = await getContext();

  let imported = 0;
  let skipped = 0;

  for (const r of receipts) {
    try {
      receiptService.create({
        tenantId: ctx.tenantId,
        businessId: ctx.businessId,
        date: r.date,
        merchant: r.merchant || "(empty)",
        amount: r.amount,
        paymentMethod: (r.paymentMethod as "cash" | "card" | "bank_transfer" | "e_wallet" | "cheque" | "other") || "other",
        categorySlug: r.categorySlug || "uncategorised",
        taxAmount: r.taxAmount ?? 0,
        serviceCharge: r.serviceCharge ?? 0,
        notes: r.notes ?? "",
        imageRef: null,
        status: "draft",
      });
      imported++;
    } catch {
      skipped++;
    }
  }

  revalidatePath("/app/akaunkemas/receipts");
  revalidatePath("/app/akaunkemas");
  return { success: true, imported, skipped };
}

// ---------------------------------------------------------------------------
// Save manual entry (PDF/image fallback — receipt or supporting document)
// ---------------------------------------------------------------------------

export interface SaveManualDocumentInput {
  documentType: "receipt" | "supporting_document" | "bank_statement_pending";
  date?: string;
  merchant?: string;
  amount?: number;
  paymentMethod?: string;
  categorySlug?: string;
  notes?: string;
  fileName: string;
  fileSizeBytes: number;
}

export async function saveManualDocument(input: SaveManualDocumentInput) {
  const ctx = await getContext();

  if (input.documentType === "receipt" && input.merchant && input.amount && input.amount > 0) {
    // Save as receipt
    receiptService.create({
      tenantId: ctx.tenantId,
      businessId: ctx.businessId,
      date: input.date ?? new Date().toISOString().slice(0, 10),
      merchant: input.merchant,
      amount: input.amount,
      paymentMethod: (input.paymentMethod as "cash" | "card" | "bank_transfer" | "e_wallet" | "cheque" | "other") || "other",
      categorySlug: input.categorySlug || "uncategorised",
      taxAmount: 0,
      serviceCharge: 0,
      notes: input.notes ?? `Imported from file: ${input.fileName}`,
      imageRef: null,
      status: "draft",
    });
    revalidatePath("/app/akaunkemas/receipts");
    return { success: true, savedAs: "receipt" };
  }

  // For supporting documents and other types — store as a receipt with notes
  // (no dedicated table for arbitrary files in v1)
  receiptService.create({
    tenantId: ctx.tenantId,
    businessId: ctx.businessId,
    date: input.date ?? new Date().toISOString().slice(0, 10),
    merchant: input.merchant || `Document: ${input.fileName}`,
    amount: input.amount || 0,
    paymentMethod: "other",
    categorySlug: "uncategorised",
    taxAmount: 0,
    serviceCharge: 0,
    notes: [
      input.notes,
      `Source file: ${input.fileName} (${(input.fileSizeBytes / 1024).toFixed(1)} KB)`,
      `Document type: ${input.documentType}`,
    ].filter(Boolean).join(" | "),
    imageRef: null,
    status: "draft",
  });

  revalidatePath("/app/akaunkemas/receipts");
  return { success: true, savedAs: "supporting_document" };
}

// ---------------------------------------------------------------------------
// Generate a unique import session ID for audit tracking
// ---------------------------------------------------------------------------

export async function generateImportId(): Promise<string> {
  return `import_${crypto.randomUUID()}`;
}
