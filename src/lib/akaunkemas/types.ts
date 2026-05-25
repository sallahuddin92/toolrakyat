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
  date: string; // ISO-like YYYY-MM-DD or original string
  description: string;
  debit: number; // amount debited (non-negative)
  credit: number; // amount credited (non-negative)
  amount: number; // signed: credit positive, debit negative
  balance: number | null;
  category: CategorySlug;
}

/** Aggregated summary for a set of transactions. */
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

/** Column mapping after auto-detection. */
export interface DetectedColumns {
  dateCol: string | null;
  descCol: string | null;
  debitCol: string | null;
  creditCol: string | null;
  amountCol: string | null;
  balanceCol: string | null;
}

/** Result of parsing a bank CSV. */
export interface ParseResult {
  transactions: Transaction[];
  detectedColumns: DetectedColumns;
  errors: string[];
  rawRowCount: number;
}

/** Optional business header for PDF export. */
export interface BusinessHeader {
  name: string;
  registrationNumber?: string;
  address?: string;
  phone?: string;
  email?: string;
  preparedBy?: string;
}
