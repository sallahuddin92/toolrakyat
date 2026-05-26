"use server";

import { getCurrentUser } from "@/lib/auth/dal";
import { createMatchRepository } from "@/lib/akaunkemas-saas/services/receipt-matches-db";
import { revalidatePath } from "next/cache";

const matchRepo = createMatchRepository();

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
// Run Auto-Matching
// ---------------------------------------------------------------------------

export async function runMatching(formData: FormData) {
  const ctx = await getContext();
  const dateWindowDays = parseInt((formData.get("dateWindowDays") as string) || "3", 10);

  try {
    matchRepo.runMatching(ctx.tenantId, ctx.businessId, dateWindowDays);
    revalidatePath("/app/akaunkemas/matching");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to run matching" };
  }
}

// ---------------------------------------------------------------------------
// Add Manual Match
// ---------------------------------------------------------------------------

export async function addManualMatch(formData: FormData) {
  const ctx = await getContext();
  const transactionId = formData.get("transactionId") as string;
  const receiptId = formData.get("receiptId") as string;

  if (!transactionId || !receiptId) {
    return { success: false, error: "Transaction and receipt are required" };
  }

  try {
    matchRepo.addManualMatch(
      ctx.tenantId,
      ctx.businessId,
      transactionId,
      receiptId,
      ctx.userId,
    );
    revalidatePath("/app/akaunkemas/matching");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to add manual match" };
  }
}

// ---------------------------------------------------------------------------
// Remove Match
// ---------------------------------------------------------------------------

export async function removeMatch(formData: FormData) {
  const ctx = await getContext();
  const transactionId = formData.get("transactionId") as string;
  const receiptId = formData.get("receiptId") as string;

  if (!transactionId || !receiptId) {
    return { success: false, error: "Transaction and receipt IDs are required" };
  }

  try {
    matchRepo.deleteMatch(ctx.tenantId, ctx.businessId, transactionId, receiptId);
    revalidatePath("/app/akaunkemas/matching");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to remove match" };
  }
}
