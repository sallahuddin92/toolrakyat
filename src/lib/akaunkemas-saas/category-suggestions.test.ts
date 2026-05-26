import { describe, it, expect } from "vitest";
import { suggestCategory, suggestCategories } from "./category-suggestions";
import type { CategorySuggestion } from "./category-suggestions";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function expectCategory(
  description: string,
  expectedSlug: string,
  expectedConfidence: "high" | "medium" | "low",
) {
  const result = suggestCategory(description);
  expect(result.suggestedCategorySlug).toBe(expectedSlug);
  expect(result.confidence).toBe(expectedConfidence);
}

function expectLow(description: string) {
  expectCategory(description, "uncategorised", "low");
}

// ---------------------------------------------------------------------------
// Utilities: Electricity
// ---------------------------------------------------------------------------

describe("suggestCategory — Utilities: Electricity", () => {
  it('matches "TNB"', () => expectCategory("TNB payment", "utilities", "high"));
  it('matches "Tenaga Nasional"', () => expectCategory("Tenaga Nasional Berhad", "utilities", "high"));
  it('matches "electricity bill"', () => expectCategory("Electricity bill May", "utilities", "high"));
  it('matches "bil elektrik"', () => expectCategory("Bil Elektrik Rumah", "utilities", "high"));
  it('matches "SESB"', () => expectCategory("SESB payment", "utilities", "high"));
});

// ---------------------------------------------------------------------------
// Utilities: Water
// ---------------------------------------------------------------------------

describe("suggestCategory — Utilities: Water", () => {
  it('matches "Air Selangor"', () => expectCategory("Air Selangor bill", "utilities", "high"));
  it('matches "PBA"', () => expectCategory("PBA water", "utilities", "high"));
  it('matches "SYABAS"', () => expectCategory("SYABAS", "utilities", "high"));
  it('matches "bil air"', () => expectCategory("Bil Air bulanan", "utilities", "high"));
  it('matches "IWK"', () => expectCategory("IWK Indah Water", "utilities", "high"));
});

// ---------------------------------------------------------------------------
// Utilities: Telco / Internet
// ---------------------------------------------------------------------------

describe("suggestCategory — Utilities: Telco", () => {
  it('matches "Unifi"', () => expectCategory("Unifi bill", "utilities", "high"));
  it('matches "Maxis"', () => expectCategory("Maxis postpaid", "utilities", "high"));
  it('matches "CelcomDigi"', () => expectCategory("CelcomDigi", "utilities", "high"));
  it('matches "TM"', () => expectCategory("TM Unifi", "utilities", "high"));
  it('matches "Astro"', () => expectCategory("Astro subscription", "utilities", "high"));
  it('matches "internet"', () => expectCategory("Internet payment", "utilities", "high"));
});

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

describe("suggestCategory — Transport", () => {
  it('matches "Grab"', () => expectCategory("Grab ride", "transport", "high"));
  it('matches "Petronas"', () => expectCategory("Petronas fuel", "transport", "high"));
  it('matches "Shell"', () => expectCategory("Shell petrol", "transport", "high"));
  it('matches "Touch n Go"', () => expectCategory("Touch n Go reload", "transport", "high"));
  it('matches "toll"', () => expectCategory("Toll payment", "transport", "high"));
  it('matches "LRT"', () => expectCategory("LRT fare", "transport", "high"));
  it('matches "parking"', () => expectCategory("Parking fee", "transport", "high"));
  it('matches "minyak" (BM)', () => expectCategory("Minyak kereta", "transport", "high"));
});

// ---------------------------------------------------------------------------
// Bank Charges
// ---------------------------------------------------------------------------

describe("suggestCategory — Bank Charges", () => {
  it('matches "bank charge"', () => expectCategory("Bank charge", "bank_charges", "high"));
  it('matches "service fee"', () => expectCategory("Service fee", "bank_charges", "high"));
  it('matches "annual fee"', () => expectCategory("Annual fee", "bank_charges", "high"));
  it('matches "caj bank"', () => expectCategory("Caj bank bulanan", "bank_charges", "high"));
  it('matches "caj perkhidmatan"', () => expectCategory("Caj perkhidmatan", "bank_charges", "high"));
});

// ---------------------------------------------------------------------------
// Rent
// ---------------------------------------------------------------------------

describe("suggestCategory — Rent", () => {
  it('matches "rent"', () => expectCategory("Office rent", "rent", "high"));
  it('matches "sewa"', () => expectCategory("Sewa rumah", "rent", "high"));
  it('matches "rental"', () => expectCategory("Rental payment", "rent", "high"));
  it('matches "sewa pejabat"', () => expectCategory("Sewa pejabat", "rent", "high"));
});

// ---------------------------------------------------------------------------
// Salary
// ---------------------------------------------------------------------------

describe("suggestCategory — Salary", () => {
  it('matches "salary"', () => expectCategory("Staff salary", "salary", "high"));
  it('matches "gaji"', () => expectCategory("Gaji bulanan", "salary", "high"));
  it('matches "payroll"', () => expectCategory("Payroll May", "salary", "high"));
  it('matches "bonus"', () => expectCategory("Staff bonus", "salary", "high"));
  it('matches "allowance"', () => expectCategory("Travel allowance", "salary", "high"));
});

// ---------------------------------------------------------------------------
// Tax
// ---------------------------------------------------------------------------

describe("suggestCategory — Tax", () => {
  it('matches "LHDN"', () => expectCategory("LHDN", "tax", "high"));
  it('matches "income tax"', () => expectCategory("Income tax", "tax", "high"));
  it('matches "cukai"', () => expectCategory("Cukai pendapatan", "tax", "high"));
  it('matches "SST"', () => expectCategory("SST payment", "tax", "high"));
  it('matches "PCB"', () => expectCategory("PCB deduction", "tax", "high"));
  it('matches "CP204"', () => expectCategory("CP204 payment", "tax", "high"));
});

// ---------------------------------------------------------------------------
// Insurance
// ---------------------------------------------------------------------------

describe("suggestCategory — Insurance", () => {
  it('matches "insurance"', () => expectCategory("Insurance premium", "insurance", "high"));
  it('matches "Takaful"', () => expectCategory("Takaful", "insurance", "high"));
  it('matches "Prudential"', () => expectCategory("Prudential", "insurance", "high"));
  it('matches "AIA"', () => expectCategory("AIA insurance", "insurance", "high"));
  it('matches "Great Eastern"', () => expectCategory("Great Eastern", "insurance", "high"));
});

// ---------------------------------------------------------------------------
// Sales / Income
// ---------------------------------------------------------------------------

describe("suggestCategory — Sales", () => {
  it('matches "sales"', () => expectCategory("Sales transfer", "sales", "high"));
  it('matches "Shopee"', () => expectCategory("Shopee payout", "sales", "high"));
  it('matches "Lazada"', () => expectCategory("Lazada", "sales", "high"));
  it('matches "TikTok Shop"', () => expectCategory("TikTok Shop", "sales", "high"));
  it('matches "Stripe"', () => expectCategory("Stripe payment", "sales", "high"));
  it('matches "PayPal"', () => expectCategory("PayPal", "sales", "high"));
  it('matches "Billplz"', () => expectCategory("Billplz", "sales", "high"));
  it('matches "revenue"', () => expectCategory("Revenue", "sales", "high"));
  it('matches "jualan"', () => expectCategory("Hasil jualan", "sales", "high"));
});

// ---------------------------------------------------------------------------
// Purchases
// ---------------------------------------------------------------------------

describe("suggestCategory — Purchases", () => {
  it('matches "purchase"', () => expectCategory("Purchase order", "purchases", "medium"));
  it('matches "supplier"', () => expectCategory("Supplier invoice", "purchases", "medium"));
  it('matches "belian"', () => expectCategory("Belian stok", "purchases", "medium"));
  it('matches "stok"', () => expectCategory("Stok barang", "purchases", "medium"));
});

// ---------------------------------------------------------------------------
// Marketing
// ---------------------------------------------------------------------------

describe("suggestCategory — Marketing", () => {
  it('matches "Meta Ads"', () => expectCategory("Meta Ads", "marketing", "high"));
  it('matches "Google Ads"', () => expectCategory("Google Ads", "marketing", "high"));
  it('matches "Facebook Ads"', () => expectCategory("Facebook Ads", "marketing", "high"));
  it('matches "Canva"', () => expectCategory("Canva Pro", "marketing", "high"));
  it('matches "iklan"', () => expectCategory("Iklan FB", "marketing", "high"));
  it('matches "SEO"', () => expectCategory("SEO services", "marketing", "high"));
});

// ---------------------------------------------------------------------------
// Office Supplies
// ---------------------------------------------------------------------------

describe("suggestCategory — Office Supplies", () => {
  it('matches "stationery"', () => expectCategory("Stationery", "office_supplies", "medium"));
  it('matches "printer"', () => expectCategory("Printer ink", "office_supplies", "medium"));
  it('matches "alat tulis"', () => expectCategory("Alat tulis", "office_supplies", "medium"));
  it('matches "toner"', () => expectCategory("Toner cartridge", "office_supplies", "medium"));
  it('matches "Popular bookstore"', () => expectCategory("Popular Bookstore", "office_supplies", "medium"));
});

// ---------------------------------------------------------------------------
// Professional Fees
// ---------------------------------------------------------------------------

describe("suggestCategory — Professional Fees", () => {
  it('matches "accountant"', () => expectCategory("Accountant fee", "professional_fees", "high"));
  it('matches "lawyer"', () => expectCategory("Lawyer", "professional_fees", "high"));
  it('matches "audit fee"', () => expectCategory("Audit fee", "professional_fees", "high"));
  it('matches "company secretary"', () => expectCategory("Company secretary", "professional_fees", "high"));
  it('matches "tax agent"', () => expectCategory("Tax agent", "professional_fees", "high"));
  it('matches "yuran profesional"', () => expectCategory("Yuran profesional", "professional_fees", "high"));
});

// ---------------------------------------------------------------------------
// Loan Payment
// ---------------------------------------------------------------------------

describe("suggestCategory — Loan Payment", () => {
  it('matches "loan"', () => expectCategory("Business loan", "loan_payment", "high"));
  it('matches "pinjaman"', () => expectCategory("Pinjaman bank", "loan_payment", "high"));
  it('matches "installment"', () => expectCategory("Installment", "loan_payment", "high"));
  it('matches "ansuran"', () => expectCategory("Ansuran bulanan", "loan_payment", "high"));
  it('matches "hire purchase"', () => expectCategory("Hire purchase", "loan_payment", "high"));
});

// ---------------------------------------------------------------------------
// Owner Drawings
// ---------------------------------------------------------------------------

describe("suggestCategory — Owner Drawings", () => {
  it('matches "owner transfer"', () => expectCategory("Owner transfer", "owner_drawings", "high"));
  it('matches "drawing"', () => expectCategory("Drawing", "owner_drawings", "high"));
  it('matches "ambilan"', () => expectCategory("Ambilan pemilik", "owner_drawings", "high"));
  it('matches "owner withdrawal"', () => expectCategory("Owner withdrawal", "owner_drawings", "high"));
});

// ---------------------------------------------------------------------------
// Transfer (own accounts)
// ---------------------------------------------------------------------------

describe("suggestCategory — Transfer", () => {
  it('matches "transfer between own"', () => expectCategory("Transfer between own accounts", "transfer", "medium"));
  it('matches "own account"', () => expectCategory("Own account transfer", "transfer", "medium"));
  it('matches "internal transfer"', () => expectCategory("Internal transfer", "transfer", "medium"));
});

// ---------------------------------------------------------------------------
// Other Income
// ---------------------------------------------------------------------------

describe("suggestCategory — Other Income", () => {
  it('matches "dividend"', () => expectCategory("Dividend", "other_income", "medium"));
  it('matches "refund"', () => expectCategory("Refund", "other_income", "medium"));
  it('matches "cashback"', () => expectCategory("Cashback", "other_income", "medium"));
  it('matches "commission"', () => expectCategory("Commission earned", "other_income", "medium"));
  it('matches "komisen"', () => expectCategory("Komisen jualan", "other_income", "medium"));
});

// ---------------------------------------------------------------------------
// Case Insensitivity
// ---------------------------------------------------------------------------

describe("suggestCategory — case insensitivity", () => {
  it("matches lowercase", () => expectCategory("tnb", "utilities", "high"));
  it("matches UPPERCASE", () => expectCategory("TNB", "utilities", "high"));
  it("matches MixedCase", () => expectCategory("Tnb Payment", "utilities", "high"));
  it("matches all caps input", () => expectCategory("GRAB RIDE TO WORK", "transport", "high"));
});

// ---------------------------------------------------------------------------
// Empty / whitespace
// ---------------------------------------------------------------------------

describe("suggestCategory — empty description", () => {
  it("returns uncategorised for empty string", () => {
    const r = suggestCategory("");
    expect(r.suggestedCategorySlug).toBe("uncategorised");
    expect(r.confidence).toBe("low");
    expect(r.matchedRule).toBe("fallback_empty");
  });

  it("returns uncategorised for whitespace-only", () => {
    const r = suggestCategory("   ");
    expect(r.suggestedCategorySlug).toBe("uncategorised");
    expect(r.confidence).toBe("low");
    expect(r.matchedRule).toBe("fallback_empty");
  });
});

// ---------------------------------------------------------------------------
// No match → uncategorised
// ---------------------------------------------------------------------------

describe("suggestCategory — no match fallback", () => {
  it("returns uncategorised for unknown description", () => {
    const r = suggestCategory("XYZ random text");
    expect(r.suggestedCategorySlug).toBe("uncategorised");
    expect(r.confidence).toBe("low");
    expect(r.matchedRule).toBe("fallback_no_match");
  });

  it("returns uncategorised for generic unrecognised payment", () => {
    expectLow("Miscellaneous debit");
  });
});

// ---------------------------------------------------------------------------
// First-match-wins ordering
// ---------------------------------------------------------------------------

describe("suggestCategory — first match wins", () => {
  it("owner transfer matches owner_drawings not transfer (more specific first)", () => {
    expectCategory("Owner transfer", "owner_drawings", "high");
  });

  it("transfer between own matches transfer category", () => {
    expectCategory("Transfer between own", "transfer", "medium");
  });
});

// ---------------------------------------------------------------------------
// suggestCategories batch
// ---------------------------------------------------------------------------

describe("suggestCategories", () => {
  it("returns array of suggestions for multiple descriptions", () => {
    const results = suggestCategories([
      "TNB payment May",
      "Grab ride",
      "Unknown thing",
    ]);
    expect(results).toHaveLength(3);
    expect(results[0].suggestedCategorySlug).toBe("utilities");
    expect(results[1].suggestedCategorySlug).toBe("transport");
    expect(results[2].suggestedCategorySlug).toBe("uncategorised");
  });

  it("returns empty array for empty input", () => {
    expect(suggestCategories([])).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Reason and matchedRule fields
// ---------------------------------------------------------------------------

describe("suggestCategory — reason and matchedRule", () => {
  it("includes keyword in reason", () => {
    const r = suggestCategory("TNB");
    expect(r.reason).toContain("tnb");
    expect(r.matchedRule).toBe("keyword:tnb");
  });

  it("fallback includes descriptive reason", () => {
    const r = suggestCategory("???");
    expect(r.reason).toBe("No matching rule — please review");
  });
});
