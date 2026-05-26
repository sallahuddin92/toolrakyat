import { getCurrentUser } from "@/lib/auth/dal";
import { createPackService } from "@/lib/akaunkemas-saas/services/accountant-packs-db";
import { AccountantPacksClient } from "../_components/AccountantPacksClient";

export const dynamic = "force-dynamic";

export default async function AccountantPacksPage() {
  const session = await getCurrentUser();
  const { tenantId, businessId } = session;

  const packService = createPackService();
  const packs = packService.list(tenantId, businessId);

  const packRows = packs.map((p) => ({
    id: p.id,
    label: p.label,
    periodStart: p.periodStart,
    periodEnd: p.periodEnd,
    status: p.status,
    notes: p.notes,
    generatedAt: p.generatedAt,
    createdAt: p.createdAt,
  }));

  return <AccountantPacksClient initialPacks={packRows} />;
}
