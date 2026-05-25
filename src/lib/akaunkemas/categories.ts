import type { CategorySlug } from "./types";

export interface CategoryDef {
  slug: CategorySlug;
  label: string; // bilingual display
}

export const CATEGORIES: CategoryDef[] = [
  { slug: "sales", label: "Jualan / Sales" },
  { slug: "purchases", label: "Belian / Purchases" },
  { slug: "rent", label: "Sewa / Rent" },
  { slug: "utilities", label: "Utiliti / Utilities" },
  { slug: "salary", label: "Gaji / Salary" },
  { slug: "transport", label: "Pengangkutan / Transport" },
  { slug: "marketing", label: "Pemasaran / Marketing" },
  { slug: "office_supplies", label: "Bekalan Pejabat / Office Supplies" },
  { slug: "professional_fees", label: "Yuran Profesional / Professional Fees" },
  { slug: "bank_charges", label: "Caj Bank / Bank Charges" },
  { slug: "tax", label: "Cukai / Tax" },
  { slug: "insurance", label: "Insurans / Insurance" },
  { slug: "loan_payment", label: "Pinjaman / Loan Payment" },
  { slug: "transfer", label: "Pindahan / Transfer" },
  { slug: "owner_drawings", label: "Ambilan Pemilik / Owner Drawings" },
  { slug: "other_income", label: "Pendapatan Lain / Other Income" },
  { slug: "other_expense", label: "Perbelanjaan Lain / Other Expense" },
  { slug: "uncategorised", label: "Tidak Pasti / Uncategorised" },
];

/** Lookup label by slug. */
export function getCategoryLabel(slug: CategorySlug): string {
  return CATEGORIES.find((c) => c.slug === slug)?.label ?? slug;
}

/** Income-type categories (positive contribution). */
const INCOME_SLUGS: Set<CategorySlug> = new Set([
  "sales",
  "other_income",
]);

/** Expense-type categories (negative contribution). */
const EXPENSE_SLUGS: Set<CategorySlug> = new Set([
  "purchases",
  "rent",
  "utilities",
  "salary",
  "transport",
  "marketing",
  "office_supplies",
  "professional_fees",
  "bank_charges",
  "tax",
  "insurance",
  "loan_payment",
  "other_expense",
  "owner_drawings",
]);

/** Neutral categories (transfers, uncategorised). */
const NEUTRAL_SLUGS: Set<CategorySlug> = new Set([
  "transfer",
  "uncategorised",
]);

export function isIncomeCategory(slug: CategorySlug): boolean {
  return INCOME_SLUGS.has(slug);
}

export function isExpenseCategory(slug: CategorySlug): boolean {
  return EXPENSE_SLUGS.has(slug);
}

export function isNeutralCategory(slug: CategorySlug): boolean {
  return NEUTRAL_SLUGS.has(slug);
}
