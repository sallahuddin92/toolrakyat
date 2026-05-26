import type { CategorySlug } from "@/lib/akaunkemas/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type Confidence = "high" | "medium" | "low";

export interface CategorySuggestion {
  suggestedCategorySlug: CategorySlug;
  confidence: Confidence;
  reason: string;
  matchedRule: string;
}

// ---------------------------------------------------------------------------
// Keyword Rules
// ---------------------------------------------------------------------------

interface Rule {
  categorySlug: CategorySlug;
  keywords: string[]; // lowercase, both BM + English
  confidence: Confidence;
}

/**
 * Rules are ordered from most specific → most general.
 * The first matching rule wins.
 */
const RULES: Rule[] = [
  // --- Utilities: Electricity ---
  {
    categorySlug: "utilities",
    keywords: [
      "tnb", "tenaga nasional", "electricity", "elektrik", "bil elektrik",
      "electric bill", "power", "sesb", "sabah electricity",
      "sarawak energy", "seb",
    ],
    confidence: "high",
  },
  // --- Utilities: Water ---
  {
    categorySlug: "utilities",
    keywords: [
      "air selangor", "pba", "syabas", "saJ", "water bill", "bil air",
      "water supply", "bekalan air", "indah water", "iwk",
    ],
    confidence: "high",
  },
  // --- Utilities: Telco / Internet ---
  {
    categorySlug: "utilities",
    keywords: [
      "tm", "unifi", "maxis", "celcom", "digi", "u mobile", "umobile",
      "celcomdigi", "yes 4g", "yes 5g", "time internet", "astro",
      "broadband", "internet", "telekom malaysia", "telco",
    ],
    confidence: "high",
  },
  // --- Transport ---
  {
    categorySlug: "transport",
    keywords: [
      "grab", "petrol", "petronas", "shell", "caltex", "bhp", "petron",
      "minyak", "diesel", "toll", "touch n go", "tng", "parking",
      "parking fee", "lrt", "mrt", "ktm", "komuter", "rapid kl",
      "bas", "teksi", "taxi", "ez auto", "mycar", "airasia ride",
      "indriver",
    ],
    confidence: "high",
  },
  // --- Bank Charges ---
  {
    categorySlug: "bank_charges",
    keywords: [
      "bank charge", "service fee", "annual fee", "caj bank", "caj perkhidmatan",
      "caj tahunan", "monthly fee", "account fee", "atm fee", "overdraft fee",
      "finance charge", "late payment charge", "statement fee",
    ],
    confidence: "high",
  },
  // --- Rent ---
  {
    categorySlug: "rent",
    keywords: [
      "rent", "rental", "sewa", "sewaan", "sewa rumah", "sewa pejabat",
      "sewa bangunan", "sewa premis", "office rental", "tenancy",
      "lease",
    ],
    confidence: "high",
  },
  // --- Salary ---
  {
    categorySlug: "salary",
    keywords: [
      "salary", "payroll", "gaji", "upah", "wages", "honorarium",
      "elaun", "allowance", "bonus", "payslip", "salary payment",
      "gaji bulanan", "pembayaran gaji",
    ],
    confidence: "high",
  },
  // --- Insurance ---
  {
    categorySlug: "insurance",
    keywords: [
      "insurance", "takaful", "insurans", "premium", "prudential",
      "aia", "great eastern", "allianz", "etiqa", "zurich", "tokio marine",
      "liberty", "berjaya sompo", "msig", "lonpac", "takaful malaysia",
      "fwd takaful", "hong leong assurance",
    ],
    confidence: "high",
  },
  // --- Professional Fees (before Tax — "tax agent" must match here first) ---
  {
    categorySlug: "professional_fees",
    keywords: [
      "accountant", "akauntan", "lawyer", "peguam", "legal", "audit",
      "audit fee", "yuran audit", "consultant", "perunding",
      "secretarial", "setiausaha syarikat", "company secretary",
      "professional fee", "yuran profesional", "engagement fee",
      "tax agent", "ejen cukai",
    ],
    confidence: "high",
  },
  // --- Tax ---
  {
    categorySlug: "tax",
    keywords: [
      "lhdn", "lembaga hasil", "tax", "cukai", "income tax",
      "cukai pendapatan", "gst", "sst", "service tax", "cukai perkhidmatan",
      "e-filing", "p cb", "pcb", "potongan cukai", "cp204", "cp 204",
      "cp500", "cp 500",
    ],
    confidence: "high",
  },
  // --- Other Income (before Sales — "komisen" is not direct sales) ---
  {
    categorySlug: "other_income",
    keywords: [
      "dividend", "dividen", "interest earned", "faedah", "refund",
      "rebate", "cashback", "bayaran balik", "reimbursement",
      "commission", "komisen", "royalty", "royalti",
      "other income", "pendapatan lain", "misc credit",
    ],
    confidence: "medium",
  },
  // --- Sales / Income ---
  {
    categorySlug: "sales",
    keywords: [
      "sales transfer", "sales", "customer payment", "pembayaran pelanggan",
      "jualan", "hasil jualan", "revenue", "income", "pendapatan",
      "shopee", "lazada", "tiktok shop", "tiktokshop", "tiktok payout",
      "shopee payout", "lazada payout", "payout", "marketplace",
      "stripe", "paypal", "billplz", "toyyibpay", "senangpay",
      "fpx", "credit card sales", "online payment received",
      "ecommerce", "e-commerce",
    ],
    confidence: "high",
  },
  // --- Loan Payment (before Purchases — "hire purchase" is a loan) ---
  {
    categorySlug: "loan_payment",
    keywords: [
      "loan", "pinjaman", "repayment", "installment", "ansuran",
      "bayaran balik", "hire purchase", "sewa beli", "leasing",
      "bank loan", "personal loan", "business loan", "overdraft",
      "maybank islamic", "cimb loan", "rhb loan", "aeon credit",
    ],
    confidence: "high",
  },
  // --- Purchases ---
  {
    categorySlug: "purchases",
    keywords: [
      "supplier", "purchase", "belian", "stok", "inventory", "barang",
      "wholesale", "borong", "pembelian",
    ],
    confidence: "medium",
  },
  // --- Marketing ---
  {
    categorySlug: "marketing",
    keywords: [
      "canva", "meta ads", "google ads", "facebook ads", "fb ads",
      "instagram ads", "iklan", "advertising", "pemasaran", "marketing",
      "tiktok ads", "billboard", "banner", "seo", "sem", "ad spend",
      "meta business", "ads manager",
    ],
    confidence: "high",
  },
  // --- Office Supplies ---
  {
    categorySlug: "office_supplies",
    keywords: [
      "office", "stationery", "stationary", "printer", "paper", "alat tulis",
      "bekalan pejabat", "kertas", "toner", "ink", "printing", "photocopy",
      "fotostat", "alat tulis", "atk", "popular bookstore", "mph",
      "kinokuniya", "bookstore",
    ],
    confidence: "medium",
  },
  // --- Owner Drawings ---
  {
    categorySlug: "owner_drawings",
    keywords: [
      "owner transfer", "drawing", "ambilan", "ambilan pemilik",
      "owner draw", "own transfer", "pindahan sendiri",
      "owner withdrawal", "personal use",
    ],
    confidence: "high",
  },
  // --- Transfer (between own accounts) ---
  {
    categorySlug: "transfer",
    keywords: [
      "transfer between own", "pindahan antara akaun", "own account",
      "transfer to own", "internal transfer", "pindahan dalaman",
      "between accounts", "self transfer", "own transfer to",
    ],
    confidence: "medium",
  },
];

// ---------------------------------------------------------------------------
// Core Function
// ---------------------------------------------------------------------------

/**
 * Suggest a category for a transaction or receipt based on its
 * description / merchant name.
 *
 * - Matches BM and English keywords (case-insensitive).
 * - Returns confidence: "high", "medium", or "low".
 * - "low" confidence means no keyword matched — suggests "uncategorised".
 */
export function suggestCategory(description: string): CategorySuggestion {
  const search = description.toLowerCase().trim();

  if (!search) {
    return {
      suggestedCategorySlug: "uncategorised",
      confidence: "low",
      reason: "Empty description — please review manually",
      matchedRule: "fallback_empty",
    };
  }

  for (const rule of RULES) {
    for (const keyword of rule.keywords) {
      if (search.includes(keyword)) {
        return {
          suggestedCategorySlug: rule.categorySlug,
          confidence: rule.confidence,
          reason: `Matched keyword: "${keyword}"`,
          matchedRule: `keyword:${keyword}`,
        };
      }
    }
  }

  return {
    suggestedCategorySlug: "uncategorised",
    confidence: "low",
    reason: "No matching rule — please review",
    matchedRule: "fallback_no_match",
  };
}

/**
 * Suggest categories for multiple descriptions at once.
 */
export function suggestCategories(
  descriptions: string[],
): CategorySuggestion[] {
  return descriptions.map((d) => suggestCategory(d));
}
