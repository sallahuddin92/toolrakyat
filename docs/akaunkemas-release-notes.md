# AkaunKemas Release Notes

## Status

AkaunKemas full workflow is implemented and verified.

## Routes

- /tools/akaunkemas
- /tools/akaunkemas/bank-csv-cleaner
- /tools/akaunkemas/receipt-organizer
- /tools/akaunkemas/simple-ledger
- /tools/akaunkemas/accountant-pack
- /tools/akaunkemas/receipt-matcher

## Features

### Bank CSV Cleaner
- Upload bank CSV
- Auto-detect Malaysian/English bank columns
- Categorise transactions
- Export cleaned CSV, JSON, and PDF summary

### Receipt Organizer
- Add and manage receipt records
- Categorise receipts
- Export receipt CSV, JSON, and PDF

### Simple Ledger
- Import cleaned CSV
- Filter by month/category
- Show income, expense, and net totals
- Export ledger CSV, JSON, and PDF

### Accountant Pack
- Combine cleaned bank CSV and receipt records
- Generate accountant-ready package

### Receipt Matcher
- Upload bank transactions and receipt records
- Match by amount/date
- Review matched and unmatched items
- Export reports

## Verification

- Lint: 0 errors
- TypeScript: clean
- AkaunKemas unit tests: 200 passed
- AkaunKemas E2E tests: 19 passed
- Production build: successful

## Known limitations

- No database or login yet
- No OCR yet
- No cloud sync yet
- Receipt matching is deterministic, not AI-based
- Uploaded files are processed locally/in-memory and not persisted
