"use server";

import { getCurrentUser } from "@/lib/auth/dal";
import { createDbTransactionService } from "@/lib/akaunkemas-saas/services/transactions-db";
import { parseBankCsv } from "@/lib/akaunkemas/csv-parser";
import { revalidatePath } from "next/cache";
import { z } from "zod";

const txService = createDbTransactionService();

// ---------------------------------------------------------------------------
// Helpers
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
// Create
// ---------------------------------------------------------------------------

export async function createTransaction(formData: FormData) {
  const ctx = await getContext();

  const date = formData.get("date") as string;
  const description = formData.get("description") as string;
  const debit = parseFloat((formData.get("debit") as string) || "0");
  const credit = parseFloat((formData.get("credit") as string) || "0");
  const categorySlug = (formData.get("categorySlug") as string) || "uncategorised";
  const notes = (formData.get("notes") as string) || "";

  try {
    txService.create({
      tenantId: ctx.tenantId,
      businessId: ctx.businessId,
      date,
      description,
      debit,
      credit,
      categorySlug,
      notes,
      balance: null,
      isReconciled: false,
      source: "manual",
      status: "draft",
      importHash: null,
    });
    revalidatePath("/app/akaunkemas/transactions");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to create transaction" };
  }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

const UpdateSchema = z.object({
  id: z.string().min(1),
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
  description: z.string().min(1).max(500).optional(),
  debit: z.number().min(0).optional(),
  credit: z.number().min(0).optional(),
  categorySlug: z.string().min(1).optional(),
  notes: z.string().max(1000).optional(),
  isReconciled: z.boolean().optional(),
  status: z.enum(["draft", "reviewed", "locked"]).optional(),
});

export async function updateTransaction(formData: FormData) {
  const ctx = await getContext();

  const raw = {
    id: formData.get("id") as string,
    date: formData.get("date") as string | undefined,
    description: formData.get("description") as string | undefined,
    debit: formData.get("debit") ? parseFloat(formData.get("debit") as string) : undefined,
    credit: formData.get("credit") ? parseFloat(formData.get("credit") as string) : undefined,
    categorySlug: formData.get("categorySlug") as string | undefined,
    notes: formData.get("notes") as string | undefined,
    status: formData.get("status") as string | undefined,
  };

  // Switched-to-reconciled checkbox pattern
  const isReconciled = formData.get("isReconciled");
  const input: Record<string, unknown> = {};
  if (raw.id) input.id = raw.id;
  if (raw.date) input.date = raw.date;
  if (raw.description !== undefined) input.description = raw.description;
  if (raw.debit !== undefined) input.debit = raw.debit;
  if (raw.credit !== undefined) input.credit = raw.credit;
  if (raw.categorySlug !== undefined) input.categorySlug = raw.categorySlug;
  if (raw.notes !== undefined) input.notes = raw.notes;
  if (isReconciled !== null) input.isReconciled = isReconciled === "true";
  if (raw.status) input.status = raw.status;

  try {
    const parsed = UpdateSchema.parse(input);
    const result = txService.update(ctx.tenantId, ctx.businessId, raw.id, parsed);
    if (!result) {
      return { success: false, error: "Transaction not found or is locked" };
    }
    revalidatePath("/app/akaunkemas/transactions");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to update transaction" };
  }
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

export async function deleteTransaction(formData: FormData) {
  const ctx = await getContext();
  const id = formData.get("id") as string;

  try {
    const success = txService.delete(ctx.tenantId, ctx.businessId, id);
    if (!success) {
      return { success: false, error: "Transaction not found or is locked" };
    }
    revalidatePath("/app/akaunkemas/transactions");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to delete transaction" };
  }
}

// ---------------------------------------------------------------------------
// Bulk CSV Import
// ---------------------------------------------------------------------------

export async function importTransactionsCsv(formData: FormData) {
  const ctx = await getContext();
  const file = formData.get("file") as File | null;

  if (!file) {
    return { success: false, error: "No file provided" };
  }

  // Optional: JSON map of row index → category slug overrides from the review step
  let categoryOverrides: Record<number, string> = {};
  const overridesRaw = formData.get("categoryOverrides") as string | null;
  if (overridesRaw) {
    try {
      categoryOverrides = JSON.parse(overridesRaw);
    } catch {
      // Ignore malformed overrides
    }
  }

  try {
    const text = await file.text();
    const parseResult = parseBankCsv(text);

    if (parseResult.errors.length > 0 && parseResult.transactions.length === 0) {
      return {
        success: false,
        error: parseResult.errors.join("; "),
        detectedColumns: parseResult.detectedColumns,
      };
    }

    let imported = 0;
    let skipped = 0;
    const emptyMatcher = (cs: string) => !cs || cs.trim() === "";

    for (let i = 0; i < parseResult.transactions.length; i++) {
      const tx = parseResult.transactions[i]!;
      const overrideSlug = categoryOverrides[i];

      try {
        txService.create({
          tenantId: ctx.tenantId,
          businessId: ctx.businessId,
          date: tx.date,
          description: tx.description || "(empty)",
          debit: tx.debit,
          credit: tx.credit,
          balance: tx.balance,
          categorySlug: overrideSlug
            ?? (emptyMatcher(tx.category) ? "uncategorised" : tx.category),
          isReconciled: false,
          notes: "",
          source: "csv_import",
          status: "draft",
          importHash: null, // auto-computed by the service
        });
        imported++;
      } catch {
        // Duplicate — skip
        skipped++;
      }
    }

    revalidatePath("/app/akaunkemas/transactions");
    return {
      success: true,
      imported,
      skipped,
      errors: parseResult.errors.length > 0 ? parseResult.errors : undefined,
      detectedColumns: parseResult.detectedColumns,
    };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to import CSV" };
  }
}

// ---------------------------------------------------------------------------
// Bulk category update (for "Apply all suggestions")
// ---------------------------------------------------------------------------

export async function bulkUpdateCategories(
  updates: Array<{ id: string; categorySlug: string }>,
) {
  const ctx = await getContext();

  let updated = 0;
  for (const { id, categorySlug } of updates) {
    try {
      const result = txService.update(ctx.tenantId, ctx.businessId, id, { categorySlug });
      if (result) updated++;
    } catch {
      // skip locked / missing
    }
  }

  revalidatePath("/app/akaunkemas/transactions");
  return { success: true, updated };
}
