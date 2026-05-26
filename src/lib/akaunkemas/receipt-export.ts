import Papa from "papaparse";
import type {
  Receipt,
  ReceiptParseResult,
  DetectedReceiptColumns,
  CategorySlug,
  PaymentMethod,
} from "./types";
import { getCategoryLabel, CATEGORIES } from "./categories";
import { computeReceiptSummary } from "./receipt-summary";

// ---------------------------------------------------------------------------
// Column detection patterns (English + Malay headers)
// ---------------------------------------------------------------------------

const RECEIPT_COLUMN_PATTERNS: Array<{
  key: keyof DetectedReceiptColumns;
  patterns: RegExp[];
}> = [
  {
    key: "dateCol",
    patterns: [/^date$/i, /^tarikh$/i],
  },
  {
    key: "merchantCol",
    patterns: [/^merchant$/i, /^peniaga$/i, /^vendor$/i, /^payee$/i],
  },
  {
    key: "amountCol",
    patterns: [/^amount$/i, /^jumlah$/i, /^total$/i],
  },
  {
    key: "paymentMethodCol",
    patterns: [/^payment[_\s]?method$/i, /^kaedah[_\s]?bayaran$/i, /^method$/i],
  },
  {
    key: "categoryCol",
    patterns: [/^category$/i, /^kategori$/i],
  },
  {
    key: "taxCol",
    patterns: [/^tax$/i, /^tax[_\s]?amount$/i, /^cukai$/i],
  },
  {
    key: "serviceChargeCol",
    patterns: [
      /^service[_\s]?charge$/i,
      /^caj[_\s]?perkhidmatan$/i,
      /^svc[_\s]?charge$/i,
    ],
  },
  {
    key: "notesCol",
    patterns: [/^notes$/i, /^nota$/i, /^remarks$/i, /^comment$/i],
  },
];

// ---------------------------------------------------------------------------
// Column auto-detection
// ---------------------------------------------------------------------------

/**
 * Auto-detect which CSV columns map to receipt fields.
 * Follows the same pattern as detectColumns in csv-parser.ts.
 */
export function detectReceiptColumns(headers: string[]): DetectedReceiptColumns {
  const result: DetectedReceiptColumns = {
    dateCol: null,
    merchantCol: null,
    amountCol: null,
    paymentMethodCol: null,
    categoryCol: null,
    taxCol: null,
    serviceChargeCol: null,
    notesCol: null,
  };

  for (let i = 0; i < headers.length; i++) {
    const h = (headers[i] ?? "").trim();
    if (!h) continue;

    for (const { key, patterns } of RECEIPT_COLUMN_PATTERNS) {
      if (result[key]) continue; // already matched
      if (patterns.some((p) => p.test(h))) {
        result[key] = h;
        break;
      }
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Value parsers (mirrors csv-parser.ts helpers)
// ---------------------------------------------------------------------------

function parseAmount(v: unknown): number {
  if (typeof v === "number") return v;
  if (typeof v !== "string") return 0;

  let s = v.trim();
  if (!s) return 0;

  // Detect parenthesized negatives: "(1,200.50)" → -1200.50
  const isParensNeg = s.startsWith("(") && s.endsWith(")");
  if (isParensNeg) s = s.slice(1, -1);

  // Strip optional "RM" currency prefix (with or without trailing space)
  s = s.replace(/^RM\s*/i, "");

  // Remove comma thousand-separators and space thousand-separators
  s = s.replace(/[, ]/g, "");

  const n = Number(s);
  if (!Number.isFinite(n)) return 0;
  return isParensNeg ? -Math.abs(n) : n;
}

function parseDate(v: unknown): string {
  if (typeof v === "string") return v.trim();
  if (typeof v === "number") return String(v);
  if (v instanceof Date) return v.toISOString().slice(0, 10);
  return "";
}

function parsePaymentMethod(v: unknown): PaymentMethod {
  if (typeof v !== "string") return "other";
  const s = v.trim().toLowerCase();
  const map: Record<string, PaymentMethod> = {
    cash: "cash",
    tunai: "cash",
    card: "card",
    kad: "card",
    bank_transfer: "bank_transfer",
    banktransfer: "bank_transfer",
    pindahan_bank: "bank_transfer",
    e_wallet: "e_wallet",
    ewallet: "e_wallet",
    e_dompet: "e_wallet",
    cheque: "cheque",
    cek: "cheque",
    other: "other",
    lain: "other",
  };
  return map[s] ?? "other";
}

/**
 * Reverse-lookup a category slug from its bilingual label or raw slug.
 */
function parseCategoryLabel(v: unknown): CategorySlug {
  if (typeof v !== "string") return "uncategorised";
  const s = v.trim();

  // Try matching against known slugs directly
  const slugSet = new Set(CATEGORIES.map((c) => c.slug));
  if (slugSet.has(s as CategorySlug)) return s as CategorySlug;

  // Try matching against bilingual labels
  for (const cat of CATEGORIES) {
    if (cat.label === s) return cat.slug;
  }

  return "uncategorised";
}

// ---------------------------------------------------------------------------
// CSV export
// ---------------------------------------------------------------------------

/**
 * Export receipts to a CSV string.
 *
 * Columns: date, merchant, amount, paymentMethod, category,
 *          taxAmount, serviceCharge, notes.
 * Category is written as the bilingual label from getCategoryLabel.
 * Monetary values are formatted to 2 decimal places.
 */
export function exportReceiptCsv(receipts: Receipt[]): string {
  const rows = receipts.map((r) => ({
    date: r.date,
    merchant: r.merchant,
    amount: r.amount.toFixed(2),
    paymentMethod: r.paymentMethod,
    category: getCategoryLabel(r.category),
    taxAmount: r.taxAmount.toFixed(2),
    serviceCharge: r.serviceCharge.toFixed(2),
    notes: r.notes,
  }));

  return Papa.unparse(rows);
}

// ---------------------------------------------------------------------------
// JSON export
// ---------------------------------------------------------------------------

/**
 * Export receipts as a JSON string containing both the receipt array
 * and a pre-computed summary.
 */
export function exportReceiptJson(receipts: Receipt[]): string {
  const summary = computeReceiptSummary(receipts);
  return JSON.stringify({ receipts, summary }, null, 2);
}

// ---------------------------------------------------------------------------
// CSV parsing
// ---------------------------------------------------------------------------

/**
 * Parse a receipt CSV string into typed receipts with auto-detected columns.
 *
 * Supports English headers (date, merchant, amount, payment_method, category,
 * tax, service_charge, notes) and Malay headers (tarikh, peniaga, jumlah,
 * kaedah_bayaran, kategori, cukai, caj_perkhidmatan, nota).
 */
export function parseReceiptCsv(csvText: string): ReceiptParseResult {
  const errors: string[] = [];
  const raw = csvText.trim();

  if (!raw) {
    return {
      receipts: [],
      detectedColumns: {
        dateCol: null,
        merchantCol: null,
        amountCol: null,
        paymentMethodCol: null,
        categoryCol: null,
        taxCol: null,
        serviceChargeCol: null,
        notesCol: null,
      },
      errors: ["CSV input is empty."],
      rawRowCount: 0,
    };
  }

  const parsed = Papa.parse<Record<string, unknown>>(raw, {
    header: true,
    skipEmptyLines: true,
    dynamicTyping: false, // we parse amounts ourselves for control
  });

  if (parsed.errors?.length) {
    for (const e of parsed.errors) {
      // Suppress FieldMismatch errors — these typically come from
      // whitespace-only rows or trailing commas.
      if (e.type === "FieldMismatch") continue;
      errors.push(`CSV parse error row ${e.row}: ${e.message}`);
    }
  }

  const headers = parsed.meta.fields ?? [];
  const detectedColumns = detectReceiptColumns(headers);

  if (!detectedColumns.dateCol && !detectedColumns.merchantCol) {
    errors.push(
      "Could not auto-detect date or merchant columns. Please check your CSV headers.",
    );
  }

  const receipts: Receipt[] = [];

  for (let i = 0; i < parsed.data.length; i++) {
    const row = parsed.data[i]!;

    const dateRaw = detectedColumns.dateCol ? row[detectedColumns.dateCol] : "";
    const merchantRaw = detectedColumns.merchantCol ? row[detectedColumns.merchantCol] : "";
    const amountRaw = detectedColumns.amountCol ? row[detectedColumns.amountCol] : 0;
    const paymentMethodRaw = detectedColumns.paymentMethodCol
      ? row[detectedColumns.paymentMethodCol]
      : "other";
    const categoryRaw = detectedColumns.categoryCol ? row[detectedColumns.categoryCol] : "";
    const taxRaw = detectedColumns.taxCol ? row[detectedColumns.taxCol] : 0;
    const serviceChargeRaw = detectedColumns.serviceChargeCol
      ? row[detectedColumns.serviceChargeCol]
      : 0;
    const notesRaw = detectedColumns.notesCol ? row[detectedColumns.notesCol] : "";

    const date = parseDate(dateRaw);
    const merchant = typeof merchantRaw === "string" ? merchantRaw.trim() : String(merchantRaw ?? "").trim();

    if (!date && !merchant) continue; // skip fully empty rows

    const amount = parseAmount(amountRaw);
    const paymentMethod = parsePaymentMethod(paymentMethodRaw);
    const category = parseCategoryLabel(categoryRaw);
    const taxAmount = parseAmount(taxRaw);
    const serviceCharge = parseAmount(serviceChargeRaw);
    const notes = typeof notesRaw === "string" ? notesRaw.trim() : String(notesRaw ?? "").trim();

    receipts.push({
      id: i + 1,
      date,
      merchant,
      amount,
      paymentMethod,
      category,
      taxAmount,
      serviceCharge,
      notes,
      imageRef: null,
    });
  }

  return {
    receipts,
    detectedColumns,
    errors,
    rawRowCount: parsed.data.length,
  };
}
