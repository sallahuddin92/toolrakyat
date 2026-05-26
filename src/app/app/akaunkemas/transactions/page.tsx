import { getCurrentUser } from "@/lib/auth/dal";
import { createDbTransactionService } from "@/lib/akaunkemas-saas/services/transactions-db";
import { TransactionsClient } from "../_components/TransactionsClient";

export const dynamic = "force-dynamic";

export default async function TransactionsPage() {
  const session = await getCurrentUser();
  const txService = createDbTransactionService();

  const rows = txService.list(session.tenantId, session.businessId, {
    limit: 200,
  });

  const initialTransactions = rows.map((tx) => ({
    ...tx,
    createdAt: tx.createdAt,
    updatedAt: tx.updatedAt,
  }));

  return <TransactionsClient initialTransactions={initialTransactions} />;
}
