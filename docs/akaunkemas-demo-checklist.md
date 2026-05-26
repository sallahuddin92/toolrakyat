# AkaunKemas Phase 1 — Demo Checklist

## Route

```
http://localhost:3000/tools/akaunkemas/bank-csv-cleaner
```

## Pre-flight

- [ ] Run `npm run dev`
- [ ] Confirm the page loads with title "Bank CSV Cleaner" and description text

---

## 1. English CSV Upload

- [ ] Click **Choose CSV file**
- [ ] Select `scratch/akaunkemas-sample-english.csv`
- [ ] Confirm:
  - Parsing indicator appears briefly ("Parsing CSV...")
  - Status line: "Detected 5 transactions from 5 rows. (Client-side)"

### Expected Transactions Table

| # | Date | Description | Debit | Credit | Amount | Category |
|---|------|-------------|-------|--------|--------|----------|
| 1 | 2026-05-01 | Sales transfer | - | 1200.00 | 1200.00 | Tidak Pasti / Uncategorised |
| 2 | 2026-05-02 | Office rent | 500.00 | - | -500.00 | Tidak Pasti / Uncategorised |
| 3 | 2026-05-03 | TNB Electricity | 180.50 | - | -180.50 | Tidak Pasti / Uncategorised |
| 4 | 2026-05-04 | Bank charge | 2.00 | - | -2.00 | Tidak Pasti / Uncategorised |
| 5 | 2026-05-05 | Owner transfer | - | 300.00 | 300.00 | Tidak Pasti / Uncategorised |

### Expected Initial Summary (all uncategorised = neutral)

| Field | Value |
|-------|-------|
| Total Income | RM 0.00 |
| Total Expenses | RM 0.00 |
| Net Cashflow | RM 0.00 |
| Transactions | 5 |

- [ ] Category breakdown shows "Tidak Pasti / Uncategorised (5)"

---

## 2. Malay CSV Upload (Re-upload Warning)

- [ ] Click **Choose CSV file** again
- [ ] Confirm browser `confirm()` dialog appears: "Uploading a new file will replace all current transactions and category assignments. Continue?"
- [ ] Click **Cancel** — transactions should remain unchanged
- [ ] Click **Choose CSV file** again, click **OK**, select `scratch/akaunkemas-sample-malay.csv`
- [ ] Confirm:
  - Detected columns show Malay headers (Tarikh, Butiran, Debit, Kredit, Baki)
  - 5 transactions parsed
  - Descriptions are in Malay (Jualan pelanggan, Sewa kedai, etc.)

### Expected Summary (Malay CSV, all uncategorised)

| Field | Value |
|-------|-------|
| Total Income | RM 0.00 |
| Total Expenses | RM 0.00 |
| Net Cashflow | RM 0.00 |
| Transactions | 5 |

---

## 3. Category Selection

Re-upload `akaunkemas-sample-english.csv`. Then:

- [ ] Row 1 (Sales transfer, +1200.00) → change to **Jualan / Sales**
  - Total Income should update to **RM 1,200.00**
  - Net Cashflow should update to **RM 1,200.00**
- [ ] Row 2 (Office rent, -500.00) → change to **Sewa / Rent**
  - Total Expenses should update to **RM 500.00**
  - Net Cashflow should update to **RM 700.00**
- [ ] Row 3 (TNB Electricity, -180.50) → change to **Utiliti / Utilities**
  - Total Expenses should update to **RM 680.50**
- [ ] Row 4 (Bank charge, -2.00) → change to **Caj Bank / Bank Charges**
  - Total Expenses should update to **RM 682.50**
- [ ] Row 5 (Owner transfer, +300.00) → change to **Pendapatan Lain / Other Income**
  - Total Income should update to **RM 1,500.00**
  - Net Cashflow should update to **RM 817.50**

### Final Summary After Categorisation

| Field | Value |
|-------|-------|
| Total Income | RM 1,500.00 |
| Total Expenses | RM 682.50 |
| Net Cashflow | RM 817.50 |
| Transactions | 5 |

Category breakdown should show 5 categories with correct totals and counts.

---

## 4. Cleaned CSV Export

- [ ] Click **Cleaned CSV** button
- [ ] Open the downloaded `akaunkemas-cleaned.csv`
- [ ] Confirm:
  - Columns: `date, description, debit, credit, amount, balance, category`
  - Category column uses bilingual labels (e.g. "Jualan / Sales"), not slugs
  - Debit column is empty for credit-only rows; Credit column empty for debit-only rows
  - All amounts formatted to 2 decimal places

---

## 5. Monthly Summary PDF Export

- [ ] Click **Monthly Summary PDF** button
- [ ] Open the downloaded `akaunkemas-monthly-summary.pdf`
- [ ] Confirm:
  - Title: "AkaunKemas Monthly Summary"
  - Generated date shown
  - Period shows first/last transaction dates
  - Summary section: Total Income, Total Expenses, Net Cashflow, Transaction Count
  - Category Breakdown table with all 5 categories
  - Footer disclaimer present
  - PDF starts with `%PDF-` header (valid PDF)

### PDF with Business Header

- [ ] Check **Add business header to PDF**
- [ ] Fill in business details
- [ ] Click **Monthly Summary PDF** again
- [ ] Confirm the PDF includes business name, reg number, address, phone, email, prepared by

---

## 6. Summary JSON Export

- [ ] Click **Summary JSON** button
- [ ] Open the downloaded `akaunkemas-summary.json`
- [ ] Confirm valid JSON structure:
  - `summary.totalIncome`, `summary.totalExpense`, `summary.netCashflow`, `summary.transactionCount`
  - `categoryBreakdown` array with `category`, `slug`, `total`, `count` per entry

---

## 7. Large File / Edge Cases

- [ ] Upload a CSV with metadata headers (bank name, account number rows before column headers)
  - Confirm the parser skips metadata and finds the real header row
- [ ] Upload a CSV with `RM` currency prefix amounts
  - Confirm values parse correctly (e.g. `RM1,200.50` → 1200.50)
- [ ] Upload a CSV with parenthesized negatives `(25.50)`
  - Confirm these are treated as reversals (column-swap)
- [ ] Upload an empty CSV
  - Confirm error: "CSV input is empty."
- [ ] Upload a non-CSV file (e.g. `.txt`)
  - Confirm error: "Invalid file type."

---

## Known Limitations (Phase 1)

- No database persistence — transactions live in-memory only, lost on page refresh
- No user accounts or authentication
- Table shows first 100 transactions only; exports include all parsed transactions
- Files >5MB use server-side parsing (requires network)
- 25MB hard file size limit
- PDF export uses client-side pdf-lib (no server-side PDF generation)
- Category assignments are not persisted between uploads
- Date format is stored as-is from CSV; no normalization to ISO

---

## Release Status

**Phase 1 — Complete.** All gates passing:

- [x] Lint: 0 errors
- [x] TypeScript: clean
- [x] Unit tests: 145 passed (28 files)
- [x] E2E tests: 5 passed
- [x] Build: compiled successfully
- [x] Commit: `eb4ee85` on `main`
