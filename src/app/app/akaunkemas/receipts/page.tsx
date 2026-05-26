import { getCurrentUser } from "@/lib/auth/dal";
import { createDbReceiptService } from "@/lib/akaunkemas-saas/services/receipts-db";
import { ReceiptsClient } from "../_components/ReceiptsClient";

export const dynamic = "force-dynamic";

export default async function ReceiptsPage() {
  const session = await getCurrentUser();
  const receiptService = createDbReceiptService();

  const rows = receiptService.list(session.tenantId, session.businessId, {
    limit: 200,
  });

  const initialReceipts = rows.map((r) => ({
    ...r,
    createdAt: r.createdAt,
    updatedAt: r.updatedAt,
  }));

  return <ReceiptsClient initialReceipts={initialReceipts} />;
}
