import { z } from "zod";
// CategorySlug type used implicitly via transaction source enum

// ---------------------------------------------------------------------------
// SavedTransaction interface
// ---------------------------------------------------------------------------

export interface SavedTransaction {
  id: string;
  tenantId: string;
  businessId: string;
  date: string;           // "YYYY-MM-DD"
  description: string;
  debit: number;
  credit: number;
  amount: number;          // net: credit - debit
  balance: number | null;
  categorySlug: string;
  isReconciled: boolean;
  notes: string;
  source: "csv_import" | "manual" | "receipt_match";
  status: "draft" | "reviewed" | "locked";
  importHash?: string | null;
  createdAt: Date;
  updatedAt: Date;
}

// ---------------------------------------------------------------------------
// CreateTransactionInput — Zod schema & type
// ---------------------------------------------------------------------------

export const CreateTransactionInputSchema = z.object({
  tenantId: z.string().min(1),
  businessId: z.string().min(1),
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  description: z.string().min(1).max(500),
  debit: z.number().min(0).default(0),
  credit: z.number().min(0).default(0),
  balance: z.number().nullable().default(null),
  categorySlug: z.string().min(1).default("uncategorised"),
  isReconciled: z.boolean().default(false),
  notes: z.string().max(1000).default(""),
  source: z.enum(["csv_import", "manual", "receipt_match"]).default("manual"),
  status: z.enum(["draft", "reviewed", "locked"]).default("draft"),
  importHash: z.string().nullable().default(null),
});

export type CreateTransactionInput = z.infer<typeof CreateTransactionInputSchema>;

// ---------------------------------------------------------------------------
// UpdateTransactionInput — partial, omitting tenantId / businessId
// ---------------------------------------------------------------------------

export const UpdateTransactionInputSchema = CreateTransactionInputSchema.partial().omit({
  tenantId: true,
  businessId: true,
});

// ---------------------------------------------------------------------------
// TransactionService interface
// ---------------------------------------------------------------------------

export interface TransactionService {
  create(input: CreateTransactionInput): SavedTransaction;
  bulkCreate(
    tenantId: string,
    businessId: string,
    inputs: Omit<CreateTransactionInput, "tenantId" | "businessId">[],
  ): SavedTransaction[];
  getById(
    tenantId: string,
    businessId: string,
    id: string,
  ): SavedTransaction | undefined;
  list(
    tenantId: string,
    businessId: string,
    options?: {
      categorySlug?: string;
      isReconciled?: boolean;
      source?: string;
      status?: "draft" | "reviewed" | "locked";
      dateFrom?: string;
      dateTo?: string;
      search?: string;
      limit?: number;
      offset?: number;
    },
  ): SavedTransaction[];
  update(
    tenantId: string,
    businessId: string,
    id: string,
    input: z.infer<typeof UpdateTransactionInputSchema>,
  ): SavedTransaction | undefined;
  delete(tenantId: string, businessId: string, id: string): boolean;
  count(tenantId: string, businessId: string): number;
  countUncategorised(tenantId: string, businessId: string): number;
}

// ---------------------------------------------------------------------------
// Helper — generate a UUID v4 (using crypto.randomUUID where available)
// ---------------------------------------------------------------------------

function generateId(): string {
  // crypto.randomUUID is available in Node >=19 and all modern browsers
  return crypto.randomUUID();
}

// ---------------------------------------------------------------------------
// Factory — createTransactionService
// ---------------------------------------------------------------------------

export function createTransactionService(): TransactionService {
  const store = new Map<string, SavedTransaction>();

  // ---- create -----------------------------------------------------------

  function create(input: CreateTransactionInput): SavedTransaction {
    const parsed = CreateTransactionInputSchema.parse(input);

    const id = generateId();
    const now = new Date();
    const amount = parsed.credit - parsed.debit;

    const record: SavedTransaction = {
      id,
      tenantId: parsed.tenantId,
      businessId: parsed.businessId,
      date: parsed.date,
      description: parsed.description,
      debit: parsed.debit,
      credit: parsed.credit,
      amount,
      balance: parsed.balance,
      categorySlug: parsed.categorySlug,
      isReconciled: parsed.isReconciled,
      notes: parsed.notes,
      source: parsed.source,
      status: parsed.status,
      importHash: parsed.importHash,
      createdAt: now,
      updatedAt: now,
    };

    store.set(id, record);
    return record;
  }

  // ---- bulkCreate -------------------------------------------------------

  function bulkCreate(
    tenantId: string,
    businessId: string,
    inputs: Omit<CreateTransactionInput, "tenantId" | "businessId">[],
  ): SavedTransaction[] {
    return inputs.map((input) =>
      create({ ...input, tenantId, businessId }),
    );
  }

  // ---- getById ----------------------------------------------------------

  function getById(
    tenantId: string,
    businessId: string,
    id: string,
  ): SavedTransaction | undefined {
    const record = store.get(id);
    if (!record) return undefined;

    // Tenant isolation
    if (record.tenantId !== tenantId) return undefined;
    if (record.businessId !== businessId) return undefined;

    return record;
  }

  // ---- list -------------------------------------------------------------

  function list(
    tenantId: string,
    businessId: string,
    options?: {
      categorySlug?: string;
      isReconciled?: boolean;
      source?: string;
      status?: "draft" | "reviewed" | "locked";
      dateFrom?: string;
      dateTo?: string;
      search?: string;
      limit?: number;
      offset?: number;
    },
  ): SavedTransaction[] {
    // Filter by tenant+business first
    const scoped = Array.from(store.values()).filter(
      (r) => r.tenantId === tenantId && r.businessId === businessId,
    );

    // Apply optional filters
    let results = scoped.filter((r) => {
      if (options?.categorySlug !== undefined && r.categorySlug !== options.categorySlug) return false;
      if (options?.isReconciled !== undefined && r.isReconciled !== options.isReconciled) return false;
      if (options?.source !== undefined && r.source !== options.source) return false;
      if (options?.status !== undefined && r.status !== options.status) return false;
      if (options?.dateFrom !== undefined && r.date < options.dateFrom) return false;
      if (options?.dateTo !== undefined && r.date > options.dateTo) return false;
      if (options?.search !== undefined) {
        const s = options.search.toLowerCase();
        if (!r.description.toLowerCase().includes(s)) return false;
      }
      return true;
    });

    // Sort by date descending
    results.sort((a, b) => b.date.localeCompare(a.date));

    // Apply offset
    const offset = options?.offset ?? 0;
    if (offset > 0) {
      results = results.slice(offset);
    }

    // Apply limit
    if (options?.limit !== undefined) {
      results = results.slice(0, options.limit);
    }

    return results;
  }

  // ---- update -----------------------------------------------------------

  function update(
    tenantId: string,
    businessId: string,
    id: string,
    input: z.infer<typeof UpdateTransactionInputSchema>,
  ): SavedTransaction | undefined {
    const record = store.get(id);
    if (!record) return undefined;

    // Tenant isolation
    if (record.tenantId !== tenantId) return undefined;
    if (record.businessId !== businessId) return undefined;

    // Validate the update input
    const parsed = UpdateTransactionInputSchema.parse(input);
    const raw = input as Record<string, unknown>;

    // For fields with defaults (debit=0, credit=0, balance=null, notes=""),
    // Zod applies the default even when the field is not in the input.
    // Use `in` checks to only merge fields that were explicitly provided.
    const newDebit = "debit" in raw ? parsed.debit! : record.debit;
    const newCredit = "credit" in raw ? parsed.credit! : record.credit;
    const newBalance = "balance" in raw ? parsed.balance! : record.balance;
    const newNotes = "notes" in raw ? parsed.notes! : record.notes;

    const updated: SavedTransaction = {
      ...record,
      date: parsed.date ?? record.date,
      description: parsed.description ?? record.description,
      debit: newDebit,
      credit: newCredit,
      amount: newCredit - newDebit,
      balance: newBalance,
      categorySlug: parsed.categorySlug ?? record.categorySlug,
      isReconciled: parsed.isReconciled ?? record.isReconciled,
      notes: newNotes,
      source: parsed.source ?? record.source,
      status: parsed.status ?? record.status,
      updatedAt: new Date(),
    };

    store.set(id, updated);
    return updated;
  }

  // ---- delete -----------------------------------------------------------

  function _delete(
    tenantId: string,
    businessId: string,
    id: string,
  ): boolean {
    const record = store.get(id);
    if (!record) return false;

    // Tenant isolation
    if (record.tenantId !== tenantId) return false;
    if (record.businessId !== businessId) return false;

    return store.delete(id);
  }

  // ---- count ------------------------------------------------------------

  function count(tenantId: string, businessId: string): number {
    let c = 0;
    for (const r of store.values()) {
      if (r.tenantId === tenantId && r.businessId === businessId) {
        c++;
      }
    }
    return c;
  }

  // ---- countUncategorised -----------------------------------------------

  function countUncategorised(tenantId: string, businessId: string): number {
    let c = 0;
    for (const r of store.values()) {
      if (
        r.tenantId === tenantId &&
        r.businessId === businessId &&
        r.categorySlug === "uncategorised"
      ) {
        c++;
      }
    }
    return c;
  }

  // ---- return the service -----------------------------------------------

  return {
    create,
    bulkCreate,
    getById,
    list,
    update,
    delete: _delete,
    count,
    countUncategorised,
  };
}
