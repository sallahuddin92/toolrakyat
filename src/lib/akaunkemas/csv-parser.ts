import Papa from "papaparse";
import type { CategorySlug, DetectedColumns, ParseResult, Transaction } from "./types";

/**
 * Patterns used to auto-detect common bank CSV column names.
 * Each entry maps a column-name pattern to a detector key.
 */
const COLUMN_PATTERNS: Array<{ key: keyof DetectedColumns; patterns: RegExp[] }> = [
  {
    key: "dateCol",
    patterns: [
      /^date$/i,
      /^transaction\s*date$/i,
      /^posting\s*date$/i,
      /^value\s*date$/i,
      /^txn\s*date$/i,
      /^tarikh$/i,
    ],
  },
  {
    key: "descCol",
    patterns: [
      /^description$/i,
      /^desc$/i,
      /^narrative$/i,
      /^particulars$/i,
      /^details$/i,
      /^transaction\s*description$/i,
      /^memo$/i,
      /^remarks$/i,
      /^perihal$/i,
      /^butiran$/i,
    ],
  },
  {
    key: "debitCol",
    patterns: [
      /^debit$/i,
      /^debit\s*amount$/i,
      /^withdrawal$/i,
      /^withdrawals$/i,
      /^money\s*out$/i,
      /^pengeluaran$/i,
    ],
  },
  {
    key: "creditCol",
    patterns: [
      /^credit$/i,
      /^credit\s*amount$/i,
      /^deposit$/i,
      /^deposits$/i,
      /^money\s*in$/i,
      /^pendapatan$/i,
      /^kredit$/i,
    ],
  },
  {
    key: "amountCol",
    patterns: [
      /^amount$/i,
      /^transaction\s*amount$/i,
      /^sum$/i,
      /^value$/i,
      /^jumlah$/i,
    ],
  },
  {
    key: "balanceCol",
    patterns: [
      /^balance$/i,
      /^running\s*balance$/i,
      /^closing\s*balance$/i,
      /^available\s*balance$/i,
      /^baki$/i,
    ],
  },
];

/**
 * Try to auto-detect which CSV columns map to date, description, debit, credit, amount, balance.
 */
export function detectColumns(headers: string[]): DetectedColumns {
  const result: DetectedColumns = {
    dateCol: null,
    descCol: null,
    debitCol: null,
    creditCol: null,
    amountCol: null,
    balanceCol: null,
  };

  for (let i = 0; i < headers.length; i++) {
    const h = (headers[i] ?? "").trim();
    if (!h) continue;

    for (const { key, patterns } of COLUMN_PATTERNS) {
      if (result[key]) continue; // already matched
      if (patterns.some((p) => p.test(h))) {
        result[key] = h;
        break;
      }
    }
  }

  return result;
}

/**
 * Scan the first few lines of CSV text to find the row that looks like
 * it contains column headers. Malaysian bank CSVs often contain metadata
 * rows (bank name, account number, statement period) before data headers.
 */
function findHeaderRowIndex(csvText: string, maxScan: number = 10): number {
  const lines = csvText.split(/\r?\n/);
  for (let i = 0; i < Math.min(lines.length, maxScan); i++) {
    const line = lines[i].trim();
    if (!line) continue;
    // Split on comma (sufficient for header-name detection; headers
    // rarely contain quoted commas.)
    const fields = line.split(",").map((f) => {
      const t = f.trim();
      if (t.startsWith('"') && t.endsWith('"')) return t.slice(1, -1);
      return t;
    });
    const detected = detectColumns(fields);
    // Need at minimum a date column and either a description or amount
    // column to consider this a real header row.
    if (detected.dateCol && (detected.descCol || detected.amountCol)) {
      return i;
    }
  }
  return 0;
}

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

  // Remove comma thousand-separators and space thousand-separators.
  // Space-removal is applied after RM stripping so "RM 1 200.50"
  // becomes "1200.50", not "1 2 0 0.5 0".
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

/**
 * Parse a bank CSV string into typed transactions with auto-detected columns.
 */
export function parseBankCsv(csvText: string): ParseResult {
  const errors: string[] = [];
  const raw = csvText.trim();
  if (!raw) {
    return {
      transactions: [],
      detectedColumns: { dateCol: null, descCol: null, debitCol: null, creditCol: null, amountCol: null, balanceCol: null },
      errors: ["CSV input is empty."],
      rawRowCount: 0,
    };
  }

  // Many Malaysian bank CSVs include metadata rows (bank name, account
  // number, statement period) before the actual column headers.  Scan
  // for the first row that resembles a header so those rows are not
  // mistaken for data.
  const headerIndex = findHeaderRowIndex(raw);
  let cleanCsv = raw;
  if (headerIndex > 0) {
    const lines = raw.split(/\r?\n/);
    cleanCsv = lines.slice(headerIndex).join("\n");
  }

  const parsed = Papa.parse<Record<string, unknown>>(cleanCsv, {
    header: true,
    skipEmptyLines: true,
    dynamicTyping: false, // we parse amounts ourselves for control
  });

  if (parsed.errors?.length) {
    for (const e of parsed.errors) {
      // Suppress FieldMismatch errors — these typically come from
      // whitespace-only rows or trailing commas and are handled by
      // our row-skipping logic below.
      if (e.type === "FieldMismatch") continue;
      errors.push(`CSV parse error row ${e.row}: ${e.message}`);
    }
  }

  const headers = parsed.meta.fields ?? [];
  const detectedColumns = detectColumns(headers);

  if (!detectedColumns.dateCol && !detectedColumns.descCol) {
    errors.push(
      "Could not auto-detect date or description columns. Please check your CSV headers.",
    );
  }

  const transactions: Transaction[] = [];

  for (let i = 0; i < parsed.data.length; i++) {
    const row = parsed.data[i]!;

    const dateRaw = detectedColumns.dateCol ? row[detectedColumns.dateCol] : "";
    const descRaw = detectedColumns.descCol ? row[detectedColumns.descCol] : "";
    const debitRaw = detectedColumns.debitCol ? row[detectedColumns.debitCol] : undefined;
    const creditRaw = detectedColumns.creditCol ? row[detectedColumns.creditCol] : undefined;
    const amountRaw = detectedColumns.amountCol ? row[detectedColumns.amountCol] : undefined;
    const balanceRaw = detectedColumns.balanceCol ? row[detectedColumns.balanceCol] : undefined;

    const date = parseDate(dateRaw);
    const description = typeof descRaw === "string" ? descRaw.trim() : String(descRaw ?? "").trim();

    if (!date && !description) continue; // skip fully empty rows

    // Determine debit/credit/amount
    let debit = 0;
    let credit = 0;
    let amount = 0;

    if (debitRaw !== undefined || creditRaw !== undefined) {
      // Bank uses separate debit/credit columns
      debit = parseAmount(debitRaw ?? 0);
      credit = parseAmount(creditRaw ?? 0);
      // Negative debit → credit (e.g. parenthesized reversal)
      if (debit < 0) {
        credit = Math.abs(debit);
        debit = 0;
      }
      // Negative credit → debit (e.g. parenthesized reversal)
      if (credit < 0) {
        debit = Math.abs(credit);
        credit = 0;
      }
      amount = credit - debit;
    } else if (amountRaw !== undefined) {
      // Bank uses a single signed amount column
      amount = parseAmount(amountRaw);
      if (amount < 0) {
        debit = Math.abs(amount);
        credit = 0;
      } else {
        credit = amount;
        debit = 0;
      }
    }

    const balance = balanceRaw !== undefined ? parseAmount(balanceRaw) : null;

    transactions.push({
      id: i + 1,
      date,
      description,
      debit,
      credit,
      amount,
      balance,
      category: "uncategorised" as CategorySlug,
    });
  }

  return {
    transactions,
    detectedColumns,
    errors,
    rawRowCount: parsed.data.length,
  };
}

/**
 * Validate that parsing produced usable results.
 */
export function validateParseResult(result: ParseResult): string[] {
  const issues: string[] = [];
  if (result.transactions.length === 0 && result.errors.length === 0) {
    issues.push("No transactions found in the CSV.");
  }
  const noDates = result.transactions.filter((t) => !t.date).length;
  if (noDates > 0) {
    issues.push(`${noDates} row(s) have missing dates.`);
  }
  return issues;
}
