"use server";

import { getCurrentUser } from "@/lib/auth/dal";
import { createPackService } from "@/lib/akaunkemas-saas/services/accountant-packs-db";
import { revalidatePath } from "next/cache";

const packService = createPackService();

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
// Generate Pack
// ---------------------------------------------------------------------------

export async function generatePack(formData: FormData) {
  const ctx = await getContext();

  const label = formData.get("label") as string;
  const periodStart = formData.get("periodStart") as string;
  const periodEnd = formData.get("periodEnd") as string;
  const notes = (formData.get("notes") as string) || "";

  if (!label || !periodStart || !periodEnd) {
    return { success: false, error: "Label, period start, and period end are required" };
  }

  try {
    const result = await packService.generate(
      ctx.tenantId,
      ctx.businessId,
      { label, periodStart, periodEnd, notes },
      ctx.userId,
    );

    revalidatePath("/app/akaunkemas/accountant-packs");
    return { success: true, packId: result.pack.id };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to generate pack" };
  }
}

// ---------------------------------------------------------------------------
// Update Pack Status
// ---------------------------------------------------------------------------

export async function updatePackStatus(formData: FormData) {
  const ctx = await getContext();
  const id = formData.get("id") as string;
  const status = formData.get("status") as string;

  if (!id || !status) return { success: false, error: "Pack ID and status are required" };

  try {
    const result = packService.updateStatus(
      ctx.tenantId,
      ctx.businessId,
      id,
      status as "draft" | "generated" | "sent" | "archived",
    );
    if (!result) return { success: false, error: "Pack not found" };
    revalidatePath("/app/akaunkemas/accountant-packs");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to update pack status" };
  }
}

// ---------------------------------------------------------------------------
// Delete Pack
// ---------------------------------------------------------------------------

export async function deletePack(formData: FormData) {
  const ctx = await getContext();
  const id = formData.get("id") as string;

  if (!id) return { success: false, error: "Pack ID is required" };

  try {
    const success = packService.delete(ctx.tenantId, ctx.businessId, id);
    if (!success) return { success: false, error: "Pack not found" };
    revalidatePath("/app/akaunkemas/accountant-packs");
    return { success: true };
  } catch (e) {
    return { success: false, error: e instanceof Error ? e.message : "Failed to delete pack" };
  }
}
