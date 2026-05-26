import {
  sqliteTable,
  text,
  integer,
  real,
  uniqueIndex,
  index,
} from "drizzle-orm/sqlite-core";

// ---------------------------------------------------------------------------
// Helper: current ISO timestamp
// ---------------------------------------------------------------------------
const ts = () => new Date().toISOString();

// ---------------------------------------------------------------------------
// 1. users
// ---------------------------------------------------------------------------
export const users = sqliteTable("users", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  email: text("email").notNull().unique(),
  name: text("name").notNull(),
  passwordHash: text("password_hash").notNull(),
  createdAt: text("created_at")
    .notNull()
    .$defaultFn(() => ts()),
  updatedAt: text("updated_at")
    .notNull()
    .$defaultFn(() => ts()),
});

export type User = typeof users.$inferSelect;
export type NewUser = typeof users.$inferInsert;

// ---------------------------------------------------------------------------
// 2. tenants
// ---------------------------------------------------------------------------
export const tenants = sqliteTable("tenants", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  name: text("name").notNull(),
  slug: text("slug").notNull().unique(),
  plan: text("plan").notNull().default("free"),
  createdAt: text("created_at")
    .notNull()
    .$defaultFn(() => ts()),
  updatedAt: text("updated_at")
    .notNull()
    .$defaultFn(() => ts()),
});

export type Tenant = typeof tenants.$inferSelect;
export type NewTenant = typeof tenants.$inferInsert;

// ---------------------------------------------------------------------------
// 3. businesses
// ---------------------------------------------------------------------------
export const businesses = sqliteTable("businesses", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  tenantId: text("tenant_id")
    .notNull()
    .references(() => tenants.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  registrationNumber: text("registration_number"),
  address: text("address").notNull().default(""),
  phone: text("phone").notNull().default(""),
  email: text("email").notNull().default(""),
  createdAt: text("created_at")
    .notNull()
    .$defaultFn(() => ts()),
  updatedAt: text("updated_at")
    .notNull()
    .$defaultFn(() => ts()),
});

export type Business = typeof businesses.$inferSelect;
export type NewBusiness = typeof businesses.$inferInsert;

// ---------------------------------------------------------------------------
// 4. memberships
// ---------------------------------------------------------------------------
export const memberships = sqliteTable(
  "memberships",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    userId: text("user_id")
      .notNull()
      .references(() => users.id, { onDelete: "cascade" }),
    tenantId: text("tenant_id")
      .notNull()
      .references(() => tenants.id, { onDelete: "cascade" }),
    businessId: text("business_id")
      .notNull()
      .references(() => businesses.id, { onDelete: "cascade" }),
    role: text("role").notNull().default("staff"),
    invitedAt: text("invited_at")
      .notNull()
      .$defaultFn(() => ts()),
    acceptedAt: text("accepted_at"),
  },
  (table) => ({
    uniqueUserTenantBusiness: uniqueIndex("ux_memberships_user_tenant_business").on(
      table.userId,
      table.tenantId,
      table.businessId,
    ),
  }),
);

export type Membership = typeof memberships.$inferSelect;
export type NewMembership = typeof memberships.$inferInsert;

// ---------------------------------------------------------------------------
// 5. categories
// ---------------------------------------------------------------------------
export const categories = sqliteTable(
  "categories",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    tenantId: text("tenant_id").notNull(),
    businessId: text("business_id").notNull(),
    name: text("name").notNull(),
    slug: text("slug").notNull(),
    type: text("type", { enum: ["income", "expense"] }).notNull(),
    isDefault: integer("is_default").notNull().default(0),
    parentId: text("parent_id"),
    createdAt: text("created_at")
      .notNull()
      .$defaultFn(() => ts()),
  },
  (table) => ({
    tenantBusinessIdx: index("idx_categories_tenant_business").on(
      table.tenantId,
      table.businessId,
    ),
    uniqueTenantBusinessSlug: uniqueIndex(
      "ux_categories_tenant_business_slug",
    ).on(table.tenantId, table.businessId, table.slug),
  }),
);

export type Category = typeof categories.$inferSelect;
export type NewCategory = typeof categories.$inferInsert;

// ---------------------------------------------------------------------------
// 6. transactions
// ---------------------------------------------------------------------------
export const transactions = sqliteTable(
  "transactions",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    tenantId: text("tenant_id").notNull(),
    businessId: text("business_id").notNull(),
    date: text("date").notNull(),
    description: text("description").notNull(),
    debit: real("debit").notNull().default(0),
    credit: real("credit").notNull().default(0),
    amount: real("amount").notNull().default(0),
    balance: real("balance"),
    categorySlug: text("category_slug").notNull().default("uncategorised"),
    isReconciled: integer("is_reconciled").notNull().default(0),
    notes: text("notes").notNull().default(""),
    source: text("source").notNull().default("manual"),
    createdAt: text("created_at")
      .notNull()
      .$defaultFn(() => ts()),
    updatedAt: text("updated_at")
      .notNull()
      .$defaultFn(() => ts()),
  },
  (table) => ({
    tenantBusinessDateIdx: index("idx_transactions_tenant_business_date").on(
      table.tenantId,
      table.businessId,
      table.date,
    ),
    tenantBusinessCategoryIdx: index(
      "idx_transactions_tenant_business_category",
    ).on(table.tenantId, table.businessId, table.categorySlug),
  }),
);

export type Transaction = typeof transactions.$inferSelect;
export type NewTransaction = typeof transactions.$inferInsert;

// ---------------------------------------------------------------------------
// 7. receipts
// ---------------------------------------------------------------------------
export const receipts = sqliteTable(
  "receipts",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    tenantId: text("tenant_id").notNull(),
    businessId: text("business_id").notNull(),
    date: text("date").notNull(),
    merchant: text("merchant").notNull(),
    amount: real("amount").notNull(),
    paymentMethod: text("payment_method").notNull().default("cash"),
    categorySlug: text("category_slug").notNull().default("uncategorised"),
    taxAmount: real("tax_amount").notNull().default(0),
    serviceCharge: real("service_charge").notNull().default(0),
    notes: text("notes").notNull().default(""),
    imageRef: text("image_ref"),
    createdAt: text("created_at")
      .notNull()
      .$defaultFn(() => ts()),
    updatedAt: text("updated_at")
      .notNull()
      .$defaultFn(() => ts()),
  },
  (table) => ({
    tenantBusinessDateIdx: index("idx_receipts_tenant_business_date").on(
      table.tenantId,
      table.businessId,
      table.date,
    ),
  }),
);

export type Receipt = typeof receipts.$inferSelect;
export type NewReceipt = typeof receipts.$inferInsert;

// ---------------------------------------------------------------------------
// 8. receipt_matches
// ---------------------------------------------------------------------------
export const receiptMatches = sqliteTable(
  "receipt_matches",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    tenantId: text("tenant_id").notNull(),
    businessId: text("business_id").notNull(),
    transactionId: text("transaction_id").notNull(),
    receiptId: text("receipt_id").notNull(),
    matchType: text("match_type", {
      enum: ["exact", "fuzzy", "manual"],
    }).notNull(),
    dateDelta: integer("date_delta").notNull().default(0),
    amountDelta: real("amount_delta").notNull().default(0),
    matchedBy: text("matched_by").notNull(),
    createdAt: text("created_at")
      .notNull()
      .$defaultFn(() => ts()),
  },
  (table) => ({
    tenantBusinessIdx: index("idx_receipt_matches_tenant_business").on(
      table.tenantId,
      table.businessId,
    ),
    transactionIdx: index("idx_receipt_matches_transaction").on(
      table.transactionId,
    ),
    receiptIdx: index("idx_receipt_matches_receipt").on(table.receiptId),
  }),
);

export type ReceiptMatch = typeof receiptMatches.$inferSelect;
export type NewReceiptMatch = typeof receiptMatches.$inferInsert;

// ---------------------------------------------------------------------------
// 9. accountant_packs
// ---------------------------------------------------------------------------
export const accountantPacks = sqliteTable(
  "accountant_packs",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    tenantId: text("tenant_id").notNull(),
    businessId: text("business_id").notNull(),
    label: text("label").notNull(),
    periodStart: text("period_start").notNull(),
    periodEnd: text("period_end").notNull(),
    status: text("status", {
      enum: ["draft", "generated", "sent", "archived"],
    })
      .notNull()
      .default("draft"),
    notes: text("notes").notNull().default(""),
    fileUrl: text("file_url"),
    generatedAt: text("generated_at"),
    createdAt: text("created_at")
      .notNull()
      .$defaultFn(() => ts()),
    updatedAt: text("updated_at")
      .notNull()
      .$defaultFn(() => ts()),
  },
  (table) => ({
    tenantBusinessStatusIdx: index("idx_accountant_packs_tenant_business_status").on(
      table.tenantId,
      table.businessId,
      table.status,
    ),
  }),
);

export type AccountantPack = typeof accountantPacks.$inferSelect;
export type NewAccountantPack = typeof accountantPacks.$inferInsert;

// ---------------------------------------------------------------------------
// 10. audit_logs
// ---------------------------------------------------------------------------
export const auditLogs = sqliteTable(
  "audit_logs",
  {
    id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
    tenantId: text("tenant_id").notNull(),
    businessId: text("business_id").notNull(),
    userId: text("user_id").notNull(),
    eventType: text("event_type").notNull(),
    entityType: text("entity_type").notNull(),
    entityId: text("entity_id").notNull(),
    summary: text("summary").notNull(),
    metadata: text("metadata").notNull().default("{}"),
    createdAt: text("created_at")
      .notNull()
      .$defaultFn(() => ts()),
  },
  (table) => ({
    tenantBusinessCreatedIdx: index("idx_audit_logs_tenant_business_created").on(
      table.tenantId,
      table.businessId,
      table.createdAt,
    ),
    tenantBusinessEventIdx: index("idx_audit_logs_tenant_business_event").on(
      table.tenantId,
      table.businessId,
      table.eventType,
    ),
  }),
);

export type AuditLog = typeof auditLogs.$inferSelect;
export type NewAuditLog = typeof auditLogs.$inferInsert;
