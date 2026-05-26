/** Stable English slug for each category. */
export type CategorySlug =
  | "sales"
  | "purchases"
  | "rent"
  | "utilities"
  | "salary"
  | "transport"
  | "marketing"
  | "office_supplies"
  | "professional_fees"
  | "bank_charges"
  | "tax"
  | "insurance"
  | "loan_payment"
  | "transfer"
  | "owner_drawings"
  | "other_income"
  | "other_expense"
  | "uncategorised";

/** A single bank transaction row after parsing. */
export interface Transaction {
  id: number;
  date: string;
  description: string;
  debit: number;
  credit: number;
  amount: number;
  balance: number | null;
  category: CategorySlug;
}

// ---------------------------------------------------------------------------
// Receipt Organizer types
// ---------------------------------------------------------------------------

export type PaymentMethod = "cash" | "card" | "bank_transfer" | "e_wallet" | "cheque" | "other";

/** A manually entered receipt record. */
export interface Receipt {
  id: number;
  date: string;
  merchant: string;
  amount: number;
  paymentMethod: PaymentMethod;
  category: CategorySlug;
  taxAmount: number;
  serviceCharge: number;
  notes: string;
  imageRef: string | null;
}

export interface ReceiptSummary {
  totalAmount: number;
  totalTax: number;
  totalServiceCharge: number;
  receiptCount: number;
  categorySummaries: ReceiptCategorySummary[];
}

export interface ReceiptCategorySummary {
  category: CategorySlug;
  total: number;
  count: number;
}

// ---------------------------------------------------------------------------
// Receipt Matcher types
// ---------------------------------------------------------------------------

export interface MatchResult {
  bankTxId: number;
  receiptId: number;
  matchType: "exact" | "fuzzy" | "manual";
  dateDelta: number;
  amountDelta: number;
}

export interface MatchingReport {
  matched: MatchResult[];
  unmatchedBank: Transaction[];
  unmatchedReceipts: Receipt[];
  dateWindowDays: number;
}

// ---------------------------------------------------------------------------
// Simple Ledger types
// ---------------------------------------------------------------------------

export interface LedgerEntry {
  id: number;
  date: string;
  description: string;
  category: CategorySlug;
  debit: number;
  credit: number;
  runningBalance: number | null;
  notes: string;
}

export interface LedgerTotals {
  totalDebit: number;
  totalCredit: number;
  netCashflow: number;
}

// ---------------------------------------------------------------------------
// Accountant Pack types
// ---------------------------------------------------------------------------

export interface AccountantPackInput {
  transactions: Transaction[];
  receipts: Receipt[];
  unmatchedTransactions?: Transaction[];
  notes?: string;
}

export interface AccountantPackFiles {
  filename: string;
  content: string | Uint8Array;
  mimeType: string;
}

// ---------------------------------------------------------------------------
// CSV column detection types
// ---------------------------------------------------------------------------

export interface DetectedColumns {
  dateCol: string | null;
  descCol: string | null;
  debitCol: string | null;
  creditCol: string | null;
  amountCol: string | null;
  balanceCol: string | null;
}

export interface DetectedReceiptColumns {
  dateCol: string | null;
  merchantCol: string | null;
  amountCol: string | null;
  paymentMethodCol: string | null;
  categoryCol: string | null;
  taxCol: string | null;
  serviceChargeCol: string | null;
  notesCol: string | null;
}

export interface DetectedLedgerColumns {
  dateCol: string | null;
  descCol: string | null;
  debitCol: string | null;
  creditCol: string | null;
  amountCol: string | null;
  categoryCol: string | null;
  balanceCol: string | null;
}

// ---------------------------------------------------------------------------
// Parse result types
// ---------------------------------------------------------------------------

export interface ParseResult {
  transactions: Transaction[];
  detectedColumns: DetectedColumns;
  errors: string[];
  rawRowCount: number;
}

export interface ReceiptParseResult {
  receipts: Receipt[];
  detectedColumns: DetectedReceiptColumns;
  errors: string[];
  rawRowCount: number;
}

export interface LedgerParseResult {
  entries: LedgerEntry[];
  detectedColumns: DetectedLedgerColumns;
  errors: string[];
  rawRowCount: number;
}

// ---------------------------------------------------------------------------
// Summary types
// ---------------------------------------------------------------------------

export interface CategorySummary {
  category: CategorySlug;
  total: number;
  count: number;
}

export interface MonthlySummary {
  totalIncome: number;
  totalExpense: number;
  netCashflow: number;
  transactionCount: number;
  categorySummaries: CategorySummary[];
}

// ---------------------------------------------------------------------------
// PDF types
// ---------------------------------------------------------------------------

export interface BusinessHeader {
  name: string;
  registrationNumber?: string;
  address?: string;
  phone?: string;
  email?: string;
  preparedBy?: string;
}
