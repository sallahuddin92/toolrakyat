import { z } from "zod";

// ---------------------------------------------------------------------------
// 1. User
// ---------------------------------------------------------------------------

export const UserRoleEnum = z.enum(["superadmin", "admin", "user"]);

export const UserSchema = z.object({
  id: z.string(),
  email: z.string().email(),
  name: z.string().min(1),
  passwordHash: z.string(),
  role: UserRoleEnum,
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type User = z.infer<typeof UserSchema>;

export const UserInsertSchema = UserSchema.omit({ id: true, createdAt: true, updatedAt: true });
export type UserInsert = z.infer<typeof UserInsertSchema>;

export const UserUpdateSchema = UserInsertSchema.partial();
export type UserUpdate = z.infer<typeof UserUpdateSchema>;

// ---------------------------------------------------------------------------
// 2. Tenant
// ---------------------------------------------------------------------------

export const TenantPlanEnum = z.enum(["free", "pro"]);

export const TenantSchema = z.object({
  id: z.string(),
  name: z.string().min(1),
  slug: z.string().min(1),
  plan: TenantPlanEnum,
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type Tenant = z.infer<typeof TenantSchema>;

export const TenantInsertSchema = TenantSchema.omit({ id: true, createdAt: true, updatedAt: true });
export type TenantInsert = z.infer<typeof TenantInsertSchema>;

export const TenantUpdateSchema = TenantInsertSchema.partial();
export type TenantUpdate = z.infer<typeof TenantUpdateSchema>;

// ---------------------------------------------------------------------------
// 3. Membership
// ---------------------------------------------------------------------------

export const MembershipRoleEnum = z.enum(["owner", "admin", "staff", "accountant", "viewer"]);

export const MembershipSchema = z.object({
  id: z.string(),
  userId: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  role: MembershipRoleEnum,
  invitedAt: z.string(),
  acceptedAt: z.string().nullable(),
});

export type Membership = z.infer<typeof MembershipSchema>;

export const MembershipInsertSchema = MembershipSchema.omit({ id: true });
export type MembershipInsert = z.infer<typeof MembershipInsertSchema>;

// ---------------------------------------------------------------------------
// 4. Business
// ---------------------------------------------------------------------------

export const BusinessSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  name: z.string().min(1),
  registrationNumber: z.string().optional(),
  address: z.string(),
  phone: z.string(),
  email: z.string(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type Business = z.infer<typeof BusinessSchema>;

export const BusinessInsertSchema = BusinessSchema.omit({ id: true, createdAt: true, updatedAt: true });
export type BusinessInsert = z.infer<typeof BusinessInsertSchema>;

export const BusinessUpdateSchema = BusinessInsertSchema.partial();
export type BusinessUpdate = z.infer<typeof BusinessUpdateSchema>;

// ---------------------------------------------------------------------------
// 5. Category
// ---------------------------------------------------------------------------

export const CategoryTypeEnum = z.enum(["income", "expense"]);

export const CategorySchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  name: z.string().min(1),
  slug: z.string().min(1),
  type: CategoryTypeEnum,
  isDefault: z.boolean(),
  parentId: z.string().optional(),
  createdAt: z.string(),
});

export type Category = z.infer<typeof CategorySchema>;

export const CategoryInsertSchema = CategorySchema.omit({ id: true, createdAt: true });
export type CategoryInsert = z.infer<typeof CategoryInsertSchema>;

export const CategoryUpdateSchema = CategoryInsertSchema.partial();
export type CategoryUpdate = z.infer<typeof CategoryUpdateSchema>;

// ---------------------------------------------------------------------------
// 6. SavedTransaction
// ---------------------------------------------------------------------------

export const TransactionSourceEnum = z.enum(["csv_import", "manual", "receipt_match"]);

export const SavedTransactionSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  date: z.string(),
  description: z.string(),
  debit: z.number(),
  credit: z.number(),
  amount: z.number(),
  balance: z.number().nullable(),
  categorySlug: z.string(),
  isReconciled: z.boolean(),
  notes: z.string(),
  source: TransactionSourceEnum,
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type SavedTransaction = z.infer<typeof SavedTransactionSchema>;

export const SavedTransactionInsertSchema = SavedTransactionSchema.omit({
  id: true,
  createdAt: true,
  updatedAt: true,
});
export type SavedTransactionInsert = z.infer<typeof SavedTransactionInsertSchema>;

export const SavedTransactionUpdateSchema = SavedTransactionInsertSchema.partial();
export type SavedTransactionUpdate = z.infer<typeof SavedTransactionUpdateSchema>;

// ---------------------------------------------------------------------------
// 7. SavedReceipt
// ---------------------------------------------------------------------------

export const SavedReceiptSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  date: z.string(),
  merchant: z.string(),
  amount: z.number(),
  paymentMethod: z.string(),
  categorySlug: z.string(),
  taxAmount: z.number(),
  serviceCharge: z.number(),
  notes: z.string(),
  imageRef: z.string().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type SavedReceipt = z.infer<typeof SavedReceiptSchema>;

export const SavedReceiptInsertSchema = SavedReceiptSchema.omit({
  id: true,
  createdAt: true,
  updatedAt: true,
});
export type SavedReceiptInsert = z.infer<typeof SavedReceiptInsertSchema>;

export const SavedReceiptUpdateSchema = SavedReceiptInsertSchema.partial();
export type SavedReceiptUpdate = z.infer<typeof SavedReceiptUpdateSchema>;

// ---------------------------------------------------------------------------
// 8. ReceiptMatch
// ---------------------------------------------------------------------------

export const MatchTypeEnum = z.enum(["exact", "fuzzy", "manual"]);

export const ReceiptMatchSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  transactionId: z.string(),
  receiptId: z.string(),
  matchType: MatchTypeEnum,
  dateDelta: z.number(),
  amountDelta: z.number(),
  matchedBy: z.string(),
  createdAt: z.string(),
});

export type ReceiptMatch = z.infer<typeof ReceiptMatchSchema>;

export const ReceiptMatchInsertSchema = ReceiptMatchSchema.omit({ id: true, createdAt: true });
export type ReceiptMatchInsert = z.infer<typeof ReceiptMatchInsertSchema>;

// ---------------------------------------------------------------------------
// 9. AccountantPack
// ---------------------------------------------------------------------------

export const AccountantPackStatusEnum = z.enum(["draft", "generated", "sent", "archived"]);

export const AccountantPackSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  label: z.string(),
  periodStart: z.string(),
  periodEnd: z.string(),
  status: AccountantPackStatusEnum,
  notes: z.string(),
  fileUrl: z.string().nullable(),
  generatedAt: z.string().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type AccountantPack = z.infer<typeof AccountantPackSchema>;

export const AccountantPackInsertSchema = AccountantPackSchema.omit({
  id: true,
  createdAt: true,
  updatedAt: true,
});
export type AccountantPackInsert = z.infer<typeof AccountantPackInsertSchema>;

export const AccountantPackUpdateSchema = AccountantPackInsertSchema.partial();
export type AccountantPackUpdate = z.infer<typeof AccountantPackUpdateSchema>;

// ---------------------------------------------------------------------------
// 10. AuditLog
// ---------------------------------------------------------------------------

export const AuditLogSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  businessId: z.string(),
  userId: z.string(),
  eventType: z.string(),
  entityType: z.string(),
  entityId: z.string(),
  summary: z.string(),
  metadata: z.record(z.string(), z.unknown()),
  createdAt: z.string(),
});

export type AuditLog = z.infer<typeof AuditLogSchema>;

export const AuditLogInsertSchema = AuditLogSchema.omit({ id: true, createdAt: true });
export type AuditLogInsert = z.infer<typeof AuditLogInsertSchema>;

// ---------------------------------------------------------------------------
// DEFAULT_CATEGORIES — seed data for AkaunKemas
// ---------------------------------------------------------------------------

export const DEFAULT_CATEGORIES = [
  // Income
  { name: "Sales", slug: "sales", type: "income" as const, isDefault: true },
  { name: "Other Income", slug: "other_income", type: "income" as const, isDefault: true },

  // Expense
  { name: "Purchases", slug: "purchases", type: "expense" as const, isDefault: true },
  { name: "Rent", slug: "rent", type: "expense" as const, isDefault: true },
  { name: "Utilities", slug: "utilities", type: "expense" as const, isDefault: true },
  { name: "Salary", slug: "salary", type: "expense" as const, isDefault: true },
  { name: "Transport", slug: "transport", type: "expense" as const, isDefault: true },
  { name: "Marketing", slug: "marketing", type: "expense" as const, isDefault: true },
  { name: "Office Supplies", slug: "office_supplies", type: "expense" as const, isDefault: true },
  { name: "Professional Fees", slug: "professional_fees", type: "expense" as const, isDefault: true },
  { name: "Bank Charges", slug: "bank_charges", type: "expense" as const, isDefault: true },
  { name: "Tax", slug: "tax", type: "expense" as const, isDefault: true },
  { name: "Insurance", slug: "insurance", type: "expense" as const, isDefault: true },
  { name: "Loan Payment", slug: "loan_payment", type: "expense" as const, isDefault: true },
  { name: "Transfer", slug: "transfer", type: "expense" as const, isDefault: true },
  { name: "Owner Drawings", slug: "owner_drawings", type: "expense" as const, isDefault: true },
  { name: "Other Expense", slug: "other_expense", type: "expense" as const, isDefault: true },
  { name: "Uncategorised", slug: "uncategorised", type: "expense" as const, isDefault: true },
] as const;

export type DefaultCategory = (typeof DEFAULT_CATEGORIES)[number];
