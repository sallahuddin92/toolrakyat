import { z } from "zod";

// ---------------------------------------------------------------------------
// 1. SavedReceipt
// ---------------------------------------------------------------------------

export interface SavedReceipt {
  id: string;
  tenantId: string;
  businessId: string;
  date: string; // "YYYY-MM-DD"
  merchant: string;
  amount: number;
  paymentMethod: "cash" | "card" | "bank_transfer" | "e_wallet" | "cheque" | "other";
  categorySlug: string;
  taxAmount: number;
  serviceCharge: number;
  notes: string;
  imageRef: string | null;
  status: "draft" | "reviewed";
  createdAt: Date;
  updatedAt: Date;
}

// ---------------------------------------------------------------------------
// 2. Create Receipt Input Schema
// ---------------------------------------------------------------------------

export const CreateReceiptInputSchema = z.object({
  tenantId: z.string().min(1),
  businessId: z.string().min(1),
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  merchant: z.string().min(1).max(200),
  amount: z.number().positive(),
  paymentMethod: z
    .enum(["cash", "card", "bank_transfer", "e_wallet", "cheque", "other"])
    .default("cash"),
  categorySlug: z.string().min(1).default("uncategorised"),
  taxAmount: z.number().min(0).default(0),
  serviceCharge: z.number().min(0).default(0),
  notes: z.string().max(1000).default(""),
  imageRef: z.string().nullable().default(null),
  status: z.enum(["draft", "reviewed"]).default("draft"),
});

export type CreateReceiptInput = z.infer<typeof CreateReceiptInputSchema>;

// ---------------------------------------------------------------------------
// 3. Update Receipt Input Schema
// ---------------------------------------------------------------------------

export const UpdateReceiptInputSchema = CreateReceiptInputSchema.partial().omit({
  tenantId: true,
  businessId: true,
});

export type UpdateReceiptInput = z.infer<typeof UpdateReceiptInputSchema>;

// ---------------------------------------------------------------------------
// 4. ReceiptService Interface
// ---------------------------------------------------------------------------

export interface ReceiptService {
  create(input: CreateReceiptInput): SavedReceipt;
  getById(tenantId: string, businessId: string, id: string): SavedReceipt | undefined;
  list(
    tenantId: string,
    businessId: string,
    options?: {
      categorySlug?: string;
      paymentMethod?: string;
      merchant?: string; // search, case-insensitive substring
      dateFrom?: string;
      dateTo?: string;
      status?: "draft" | "reviewed";
      limit?: number;
      offset?: number;
    },
  ): SavedReceipt[];
  update(
    tenantId: string,
    businessId: string,
    id: string,
    input: z.infer<typeof UpdateReceiptInputSchema>,
  ): SavedReceipt | undefined;
  delete(tenantId: string, businessId: string, id: string): boolean;
  count(tenantId: string, businessId: string): number;
  getTotalAmount(tenantId: string, businessId: string): number;
  getByCategory(
    tenantId: string,
    businessId: string,
  ): { categorySlug: string; total: number; count: number }[];
}
// The in-memory factory `createReceiptService()` has been moved to
// __fixtures__/receipts-memory.ts for test use.
//
// For production, use createDbReceiptService() from ./receipts-db.
