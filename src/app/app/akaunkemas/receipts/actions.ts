"use server";

import { getCurrentUser } from "@/lib/auth/dal";
import { createDbReceiptService } from "@/lib/akaunkemas-saas/services/receipts-db";
import { revalidatePath } from "next/cache";
import { z } from "zod";

const receiptService = createDbReceiptService();

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

export async function createReceipt(formData: FormData) {
  const ctx = await getContext();

  try {
    receiptService.create({
      tenantId: ctx.tenantId,
      businessId: ctx.businessId,
      date: formData.get("date") as string,
      merchant: formData.get("merchant") as string,
      amount: parseFloat(formData.get("amount") as string),
      paymentMethod: (formData.get("paymentMethod") as string || "cash") as
        | "cash" | "card" | "bank_transfer" | "e_wallet" | "cheque" | "other",
      categorySlug: (formData.get("categorySlug") as string) || "uncategorised",
      taxAmount: parseFloat((formData.get("taxAmount") as string) || "0"),
      serviceCharge: parseFloat((formData.get("serviceCharge") as string) || "0"),
      notes: (formData.get("notes") as string) || "",
      imageRef: null,
      status: "draft",
    });
    revalidatePath("/app/akaunkemas/receipts");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to create receipt" };
  }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

const UpdateReceiptSchema = z.object({
  id: z.string().min(1),
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
  merchant: z.string().min(1).max(200).optional(),
  amount: z.number().positive().optional(),
  paymentMethod: z.enum(["cash", "card", "bank_transfer", "e_wallet", "cheque", "other"]).optional(),
  categorySlug: z.string().min(1).optional(),
  taxAmount: z.number().min(0).optional(),
  serviceCharge: z.number().min(0).optional(),
  notes: z.string().max(1000).optional(),
  status: z.enum(["draft", "reviewed"]).optional(),
});

export async function updateReceipt(formData: FormData) {
  const ctx = await getContext();

  const input: Record<string, unknown> = { id: formData.get("id") as string };

  const stringFields = ["date", "merchant", "categorySlug", "paymentMethod", "notes", "status"] as const;
  for (const field of stringFields) {
    const val = formData.get(field) as string | null;
    if (val) input[field] = val;
  }

  const numberFields = ["amount", "taxAmount", "serviceCharge"] as const;
  for (const field of numberFields) {
    const val = formData.get(field) as string | null;
    if (val) input[field] = parseFloat(val);
  }

  try {
    const parsed = UpdateReceiptSchema.parse(input);
    const result = receiptService.update(ctx.tenantId, ctx.businessId, parsed.id, parsed);
    if (!result) {
      return { success: false, error: "Receipt not found" };
    }
    revalidatePath("/app/akaunkemas/receipts");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to update receipt" };
  }
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

export async function deleteReceipt(formData: FormData) {
  const ctx = await getContext();
  const id = formData.get("id") as string;

  try {
    const success = receiptService.delete(ctx.tenantId, ctx.businessId, id);
    if (!success) {
      return { success: false, error: "Receipt not found" };
    }
    revalidatePath("/app/akaunkemas/receipts");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to delete receipt" };
  }
}
