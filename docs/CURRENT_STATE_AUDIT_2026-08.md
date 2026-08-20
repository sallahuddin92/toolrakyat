# ToolRakyat Current State Audit

**Date of Audit:** August 20, 2026  
**Auditor:** Senior Full-Stack & Production-Readiness Auditor (Deep Forensics)  
**Target Repository:** `https://github.com/sallahuddin92/toolrakyat`  
**Local Path:** `/Users/sallahuddin/Desktop/Projects/toolrakyat`  
**Commit Head:** `7f685730a8037c5e97b016460588aa80af13707d` (`main`)

---

## 1. Executive Summary

This forensic audit establishes the exact state of the **ToolRakyat** codebase.

The repository contains three primary domains with vastly different levels of maturity:
1. **AkaunKemas (Dual Architecture: Free Client-Side Tools + SQLite/Drizzle SaaS App):** **Substantially Implemented & Architecturally Solid.** The core bookkeeping engine, Malaysian bank CSV parser, bilingual categorisation rules, SQLite persistence, JWT authentication, RBAC, tenant isolation, matching engine, accountant pack generator, and audit logging services are fully functional with **608 passing unit tests**. However, the SaaS frontend is currently blocked by build/typecheck failures from orphaned files and missing configuration constants.
2. **SmartPDF (Advanced PDF Editor):** **Placeholder / Missing.** The documentation in `README.md` describes an intelligence-driven PDF editor with OCR and form filling, and references `docs/SMARTPDF_BETA_CHECKLIST.md` and `/tools/pdf/editor/uat`. Forensics reveal that the checklist and UAT route do not exist, the route `/tools/pdf/editor` displays a "Tool wiring pending" placeholder, and the API endpoint `/api/pdf/[[...path]]` is merely a reverse proxy delegating to a non-existent external backend (`http://localhost:8000`).
3. **ToolRakyat Core Productivity Tools:** **Partially Implemented Shell.** A directory and routing shell exists with 48+ tools registered in `src/lib/tools/registry.ts`, but zero general productivity tool UIs are implemented (only the 5 AkaunKemas tools are implemented). Furthermore, the dynamic route handler `/tools/[category]/[slug]` attempts to import a missing component (`WordCounterTool`), breaking the build.

The immediate blocker preventing the application from building, starting, and running Playwright E2E suites is a small set of missing modules and orphaned files (`src/lib/limits.ts`, `WordCounterTool`, and `UniversalImportClient.tsx`).

---

## 2. Repository State

### Git Forensics
- **Branch:** `main` (tracking `origin/main`, up to date)
- **Commit SHA:** `7f685730a8037c5e97b016460588aa80af13707d`
- **Commit Message:** `refactor(akaunkemas): remove in-memory receipt service from production path`
- **Working Tree:** Clean (no uncommitted modifications prior to database setup)
- **Tags:** None
- **Total Commits:** 13 commits

### Environment & Runtime
- **Node.js:** `v20.20.2`
- **npm:** `10.8.2`
- **Next.js:** `16.2.4` (App Router with Turbopack)
- **React:** `19.2.4`
- **Database Engine:** SQLite (via `better-sqlite3` `^12.10.0` and `drizzle-orm` `^0.45.2`)

---

## 3. Architecture Map

```
toolrakyat/
├── src/
│   ├── app/
│   │   ├── (marketing)/
│   │   │   ├── page.tsx                     # Landing page
│   │   │   ├── pricing/page.tsx             # Pricing tiers
│   │   │   ├── privacy/page.tsx             # Privacy policy
│   │   │   └── terms/page.tsx               # Terms of service
│   │   ├── tools/
│   │   │   ├── page.tsx                     # Tool directory index
│   │   │   ├── [category]/[slug]/page.tsx   # Dynamic tool route (BROKEN: missing WordCounterTool)
│   │   │   └── akaunkemas/                  # 5 Free Client-Side Public Tools
│   │   │       ├── bank-csv-cleaner/        # Bank CSV cleaner & categoriser (VERIFIED)
│   │   │       ├── receipt-organizer/       # Client-side receipt logger (VERIFIED)
│   │   │       ├── receipt-matcher/         # Client-side bank vs receipt matcher (VERIFIED)
│   │   │       ├── accountant-pack/         # Client-side ZIP/PDF pack generator (VERIFIED)
│   │   │       └── simple-ledger/           # Client-side cashflow ledger (VERIFIED)
│   │   ├── app/akaunkemas/                  # Authenticated SaaS App (SQLite + Drizzle)
│   │   │   ├── (auth)/login/ & register/    # JWT Auth pages & server actions (VERIFIED)
│   │   │   ├── page.tsx                     # SaaS Dashboard (VERIFIED)
│   │   │   ├── transactions/                # Persistent Transaction Ledger & Actions (VERIFIED)
│   │   │   ├── receipts/                    # Persistent Receipt Management & Actions (VERIFIED)
│   │   │   ├── matching/                    # Persistent Auto & Manual Matching (VERIFIED)
│   │   │   ├── accountant-packs/            # Persistent Pack Generator & ZIP Downloads (VERIFIED)
│   │   │   ├── audit-logs/                  # Persistent Audit Logs with Metadata Redaction (VERIFIED)
│   │   │   ├── monthly-review/              # Monthly Reconciliation Overview (VERIFIED)
│   │   │   ├── import-bank-csv/             # 3-Step CSV Import Wizard (VERIFIED)
│   │   │   ├── settings/business/           # Business Settings (PLACEHOLDER: disabled form)
│   │   │   ├── logout/route.ts              # Session deletion route (VERIFIED)
│   │   │   └── _components/                 # Client UI Views
│   │   │       └── UniversalImportClient.tsx # (ORPHANED / UNUSED / TYPE-BROKEN)
│   │   └── api/
│   │       ├── akaunkemas/parse-csv/        # Server-side CSV parse fallback (BROKEN: missing limits)
│   │       └── pdf/[[...path]]/             # Reverse proxy to http://localhost:8000 (PLACEHOLDER)
│   ├── components/                          # UI & Layout components (shadcn/ui + Tailwind v4)
│   ├── lib/
│   │   ├── auth/                            # JWT session (jose), bcrypt hashing, DAL, auth-service
│   │   ├── db/                              # Drizzle SQLite schema (10 tables), migrations, seed
│   │   ├── akaunkemas/                      # Free tool business logic (CSV, PDF, matching, packs)
│   │   ├── akaunkemas-saas/                 # SaaS services (transactions, receipts, matching, packs, audit)
│   │   └── tools/                           # Tool registry, formatters, temp file management, validators
│   └── proxy.ts                             # Next.js route protection & rate limiter (BROKEN: missing limits)
```

---

## 4. Verified Working Features

The following features have been verified by executing unit tests and code inspections:

1. **Malaysian Bank CSV Parsing Engine (`src/lib/akaunkemas/csv-parser.ts`):**
   - Verified across 33 unit tests.
   - Robust column auto-detection for Maybank, CIMB, Public Bank, RHB, Hong Leong, AmBank, Bank Islam, and generic formats.
   - Handles multi-line headers, comma-formatted currency, parenthesized negatives, debit/credit splitting, and Malaysian date formats (`DD/MM/YYYY`, `DD-MM-YYYY`, `YYYY-MM-DD`).
2. **Smart Bilingual Categorisation (`src/lib/akaunkemas-saas/category-suggestions.ts`):**
   - Verified across 110 unit tests.
   - Accurately classifies Malaysian merchant names and keywords (e.g. TNB, Syabas, Petronas, Shell, Touch 'n Go, Shopee, KWSP, PERKESO, LHDN, Gaji) in Malay and English.
   - Returns confidence scores and human-readable reasoning.
3. **Transaction & Receipt Matching Engine (`src/lib/akaunkemas/receipt-matcher.ts`):**
   - Verified across 37 unit tests.
   - Exact amount + exact date matching, exact amount + fuzzy date window (±3 days), and manual override matching.
4. **Accountant Pack Generation & ZIP Packing (`src/lib/akaunkemas/accountant-pack.ts`, `pdf-export.ts`):**
   - Verified across 17 accountant pack tests and 6 PDF export tests.
   - Generates formatted financial summaries in PDF using `pdf-lib` and bundles CSV ledgers and audit records into a clean ZIP using `jszip`.
5. **Database Schema & Migrations (`src/lib/db/`):**
   - Successfully executed `npm run db:migrate` and `npm run db:seed`.
   - 10 tables created cleanly: `users`, `tenants`, `businesses`, `memberships`, `categories`, `transactions`, `receipts`, `receipt_matches`, `accountant_packs`, `audit_logs`.
   - Seed script creates demo tenant, business, admin user (`demo@akaunkemas.my` / `demo1234`), and default income/expense categories.
6. **SaaS Backend Services & Repositories (`src/lib/akaunkemas-saas/services/`):**
   - `transactions-db.ts`: Verified (27 unit tests) — CRUD, CSV hash deduplication, locked status prevention.
   - `receipts-db.ts`: Verified (23 unit tests) — CRUD, categorization, totals aggregation.
   - `receipt-matches-db.ts`: Verified (19 unit tests) — Auto-match execution, manual match management, unmatched counts.
   - `accountant-packs-db.ts`: Verified (11 unit tests) — Pack generation, status transitions (`draft` -> `generated` -> `sent` -> `archived`).
   - `audit-db.ts`: Verified (25 unit tests) — Tenant isolation, event filtering, and strict metadata redaction (passwords, tokens, raw file buffers).
7. **Authentication & RBAC Logic (`src/lib/auth/`, `src/lib/akaunkemas-saas/rbac.ts`):**
   - Verified across 90 unit tests (49 RBAC tests, 41 server RBAC tests).
   - Role hierarchy (`owner` > `admin` > `staff` > `accountant` > `viewer`).
   - Tenant & business context verification.

---

## 5. Implemented but Not Fully Verified

1. **AkaunKemas SaaS UI Server Actions (`src/app/app/akaunkemas/**/actions.ts`):**
   - Server actions for transaction CRUD, receipt CRUD, matching, pack generation, and CSV import are written and wired to the database services.
   - Cannot be end-to-end verified via browser/Playwright until the Next.js build errors are resolved.
2. **AkaunKemas Free Client-Side Tool Pages (`src/app/tools/akaunkemas/*`):**
   - Complete client components exist for Bank CSV Cleaner, Receipt Organizer, Receipt Matcher, Accountant Pack, and Simple Ledger.
   - Core libraries are verified with unit tests, but full UI interaction requires web server startup.

---

## 6. Partial Implementations

1. **`src/app/app/akaunkemas/settings/business/page.tsx`:**
   - Displays a business profile form, but all input fields and the submit button have the `disabled` attribute hardcoded, and no server action is attached.
2. **`src/app/app/akaunkemas/import-bank-csv/page.tsx`:**
   - Implements a complete 3-step import wizard, but contains an internal subcomponent (`StepIndicator`) declared inside render that fails ESLint (`react-hooks/static-components`).
3. **`src/proxy.ts` Rate Limiting:**
   - Implements an in-memory token-bucket rate limiter for `/api/tools/*` routes, but imports missing `@/lib/limits`.

---

## 7. Broken Features / Regressions

1. **Missing Module `src/lib/limits.ts` (CRITICAL P0):**
   - `src/lib/tools/file-validation.ts`, `src/lib/tools/temp-files.ts`, `src/proxy.ts`, and `src/app/api/akaunkemas/parse-csv/route.ts` all import `LIMITS` from `limits.ts`.
   - Because `limits.ts` was not checked into git, `npm test` fails 1 suite (`file-validation.test.ts`), `npm run typecheck` fails 4 files, and `npm run build` fails immediately.
2. **Missing Component `WordCounterTool` (CRITICAL P0):**
   - `src/app/tools/[category]/[slug]/page.tsx` imports `@/components/tools/implementations/text/WordCounterTool`.
   - The file does not exist in the repository, crashing `npm run typecheck` and `npm run build`.
3. **Orphaned / Broken Component `UniversalImportClient.tsx` (HIGH P1):**
   - `src/app/app/akaunkemas/_components/UniversalImportClient.tsx` attempts to import from `@/lib/akaunkemas-saas/import/import-pipeline` (which does not exist) and contains 13 implicit `any` type errors and 9 lint errors.
   - Note: This component is never imported by any page or layout.

---

## 8. Missing Features

1. **SmartPDF Advanced PDF Editor Engine (`/tools/pdf/editor`):**
   - No PDF canvas renderer, annotation layer, text replacement engine, OCR integration, or AcroForm filling UI exists in the repository.
   - The API route `/api/pdf/[[...path]]` merely forwards HTTP requests to an unconfigured external service at `http://localhost:8000`.
   - Documentation claims (`docs/SMARTPDF_BETA_CHECKLIST.md`, `/tools/pdf/editor/uat`) are completely missing.
2. **Productivity Tools UI Implementations (40+ Tools):**
   - All tools under PDF (Merge, Split, Rotate, Sign, Watermark, etc.), Image (Compress, Resize, Convert, Flip), Compression (ZIP, Minifiers), Converters, Calculators, and Business Generators have entries in `registry.ts` marked as `isImplemented: true`, but have no actual UI components. They render `<ToolPlaceholder>`.
3. **Database File Directory Creation:**
   - SQLite attempts to open `./data/akaunkemas.db`, but the `./data` directory is gitignored and not auto-created on initial checkout, requiring manual `mkdir -p data` before running migrations.

---

## 9. AkaunKemas Capability Matrix

| Capability | Exists | Verified | Persistence | Tests | Issues | Status |
|---|---|---|---|---|---|---|
| **User Registration** | Yes | Yes (Unit) | SQLite (`users`, `tenants`, `businesses`, `memberships`) | Unit (Auth Service) | Auto-login redirect requires build fix | Implemented |
| **User Login & Session** | Yes | Yes (Unit) | JWT Cookie (`ak_session`) | Unit (Auth/Session) | Works with hardcoded dev secret fallback | Implemented |
| **RBAC Authorization** | Yes | Yes (Unit) | SQLite (`memberships.role`) | 90 tests | Full role hierarchy enforced | Implemented & Verified |
| **Tenant / Business Isolation** | Yes | Yes (Unit) | SQLite (`tenant_id`, `business_id` columns & indexes) | Unit tests in all services | Strict WHERE clauses on all DB queries | Implemented & Verified |
| **Bank CSV Parser (MY Banks)** | Yes | Yes (Unit/E2E) | Client-side / In-memory | 33 unit tests | Handles Maybank, CIMB, RHB, Public Bank, etc. | Implemented & Verified |
| **Bilingual Categorisation** | Yes | Yes (Unit) | In-memory rules | 110 unit tests | BM & EN rules with confidence scores | Implemented & Verified |
| **Transaction CRUD** | Yes | Yes (Unit) | SQLite (`transactions`) | 27 unit tests | Hash deduplication, locked status check | Implemented & Verified |
| **Receipt CRUD** | Yes | Yes (Unit) | SQLite (`receipts`) | 23 unit tests | Aggregations by category and totals | Implemented & Verified |
| **Auto & Manual Matching** | Yes | Yes (Unit) | SQLite (`receipt_matches`) | 19 unit tests | Exact + fuzzy date window (±3d) + manual | Implemented & Verified |
| **Accountant Pack Generation** | Yes | Yes (Unit) | SQLite (`accountant_packs`) + ZIP/PDF | 17 pack + 6 PDF tests | Full PDF ledger & ZIP generation | Implemented & Verified |
| **Audit Logging** | Yes | Yes (Unit) | SQLite (`audit_logs`) | 25 unit tests | Redacts passwords, tokens, raw CSVs | Implemented & Verified |
| **Monthly Review Page** | Yes | Yes (Code) | SQLite Queries | Code audit | Live financial aggregation | Implemented |
| **CSV Import 3-Step Wizard** | Yes | Partial | SQLite via Server Actions | Code audit | Lint error (`StepIndicator` inside render) | Partially Implemented |
| **Business Profile Settings** | Yes | No | None | None | Form inputs & save button are disabled | Placeholder / Mock |
| **Universal Import Client** | Yes | No | None | None | Broken imports & type errors; unreferenced | Broken / Orphaned |

---

## 10. SmartPDF Capability Matrix

| Capability | Exists | Verified | Test Coverage | Limitations | Status |
|---|---|---|---|---|---|
| **Route `/tools/pdf/editor`** | Yes (Route) | Verified (Mock) | None | Renders `ToolPlaceholder` ("Tool wiring pending") | Placeholder |
| **Native PDF Text Editing** | No | No | None | Not implemented in frontend or backend | Missing |
| **PDF Annotation / Canvas Overlay** | No | No | None | No canvas or overlay rendering pipeline | Missing |
| **AcroForm Detection & Filling** | No | No | None | No AcroForm parser or interactive widgets | Missing |
| **PDF Scanned Document OCR** | No | No | None | `tesseract.js` is in `package.json`, but unreferenced | Missing |
| **Table Extraction Engine** | No | No | None | Not implemented | Missing |
| **Export Editable / Flattened** | No | No | None | Not implemented | Missing |
| **Backend Reverse Proxy** | Yes | Verified (Code) | None | Forwards to `http://localhost:8000` (no backend in repo) | Broken / Stub |
| **UAT Route (`/tools/pdf/editor/uat`)** | No | Verified (Missing) | None | Documented in README, does not exist | Missing |
| **UAT Checklist (`SMARTPDF_BETA_CHECKLIST.md`)** | No | Verified (Missing) | None | Documented in README, does not exist | Missing |

---

## 11. Test Results

| Test Category | Total Count | Passed | Failed | Skipped / Blocked | Status |
|---|---|---|---|---|---|
| **ESLint (`npm run lint`)** | 23 errors | — | 23 errors | 0 | **FAIL** |
| **TypeScript (`npm run typecheck`)** | 21 errors | — | 21 errors across 5 files | 0 | **FAIL** |
| **Unit & Integration Tests (`npm test`)** | 21 test files (608 tests) | 20 files (608 tests) | 1 file (import error) | 0 | **FAIL (1 Suite)** |
| **Next.js Production Build (`npm run build`)** | 3 build errors | 0 | 3 errors (Turbopack resolution) | 0 | **FAIL** |
| **Playwright E2E (`npx playwright test`)** | 2 spec files | 0 | 0 | Blocked (No web server build) | **BLOCKED** |

### Detailed Failure Analysis

#### 1. TypeScript & Build Failure: Missing `@/lib/limits`
- **Failing Files:** `src/lib/tools/file-validation.ts`, `src/lib/tools/temp-files.ts`, `src/proxy.ts`, `src/app/api/akaunkemas/parse-csv/route.ts`
- **Error:** `error TS2307: Cannot find module '@/lib/limits'`
- **Root Cause:** `src/lib/limits.ts` was referenced by 4 files during previous feature additions but was never committed.
- **Severity:** **P0 Critical**

#### 2. TypeScript & Build Failure: Missing `WordCounterTool`
- **Failing File:** `src/app/tools/[category]/[slug]/page.tsx:8`
- **Error:** `error TS2307: Cannot find module '@/components/tools/implementations/text/WordCounterTool'`
- **Root Cause:** Dynamic route directly imported a specific text tool implementation that was never created.
- **Severity:** **P0 Critical**

#### 3. TypeScript Failure: `UniversalImportClient.tsx`
- **Failing File:** `src/app/app/akaunkemas/_components/UniversalImportClient.tsx`
- **Error:** `Cannot find module '@/lib/akaunkemas-saas/import/import-pipeline'` + 13 implicit any errors.
- **Root Cause:** Abandoned / unreferenced prototype component committed with unresolved imports.
- **Severity:** **P1 High**

#### 4. ESLint Failure: `StepIndicator` Inside Render
- **Failing File:** `src/app/app/akaunkemas/import-bank-csv/page.tsx:327`
- **Error:** `react-hooks/static-components: Cannot create components during render`
- **Root Cause:** Function component `StepIndicator` declared inside `ImportBankCsvPage` body instead of module scope.
- **Severity:** **P1 High**

---

## 12. Security Findings

| ID | Finding | Severity | Description | Recommended Mitigation |
|---|---|---|---|---|
| **SEC-01** | Insecure Fallback Secret in Auth | **P1 High** | `src/lib/auth/session.ts` falls back to hardcoded string `"dev-secret-akaunkemas-do-not-use-in-production"` if `AUTH_SECRET` is unset. | Enforce that `AUTH_SECRET` throws an error in production environments if not set in `process.env`. |
| **SEC-02** | Unvalidated Backend Proxy | **P2 Medium** | `src/app/api/pdf/[[...path]]/route.ts` proxies arbitrary path segments and query strings to `BACKEND_URL` without route whitelisting or authentication checks. | Add path sanitization, restrict allowed endpoints, and require session authentication. |
| **SEC-03** | In-Memory Rate Limiting | **P2 Medium** | Rate limiting in `src/proxy.ts` is backed by an in-memory `Map`. | Document multi-node limitation (requires Redis or Upstash for distributed deployments). |
| **SEC-04** | Metadata Leakage Protection | **P3 Low** (Positive) | `audit-db.ts` aggressively sanitizes metadata by redacting passwords, tokens, hashes, raw CSV content, and file buffers. | Preserved as exemplary pattern. |

---

## 13. Data Integrity Findings

1. **Transaction Immutability on Locked Status:**
   - Verified: `transactions-db.ts` rejects updates and deletions when transaction status is `"locked"`.
2. **Duplicate CSV Import Prevention:**
   - Verified: SHA-256 hash computed on `businessId|date|description|amount` prevents double-importing identical statement rows.
3. **Database Foreign Keys & Constraints:**
   - Verified: `better-sqlite3` enforces `PRAGMA foreign_keys = ON` and `PRAGMA journal_mode = WAL` on initialization. Unique indexes guard user emails, tenant slugs, and category slugs.

---

## 14. UX / Workflow Findings

1. **AkaunKemas Free Tools vs SaaS Integration:**
   - The free client-side tools (`/tools/akaunkemas/*`) operate entirely in-browser and download files locally.
   - The SaaS application (`/app/akaunkemas/*`) provides persistent tracking and multi-tenant management.
   - Both workflows are clean and complementary, but the navigation between free tools and SaaS app could be surfaced more clearly in the header.
2. **Business Settings Page Inaction:**
   - Visiting `/app/akaunkemas/settings/business` presents a disabled form without explanation. A message explaining that business profile editing is coming in the next release should be displayed if editing is not yet wired.

---

## 15. Technical Debt

1. **Orphaned `UniversalImportClient.tsx`:** 697 lines of unreferenced code causing TypeScript and lint failures.
2. **Missing `WordCounterTool`:** Hardcoded special-case in `[category]/[slug]/page.tsx` instead of using the generic registry lookup.
3. **In-Memory Fixtures in Tests:** `__fixtures__/receipts-memory.ts` has unused variables that trigger linter warnings.

---

## 16. Git / Repository Risks

1. **README Claims Divergence:** The README claims 48+ working tools and a full SmartPDF editor with UAT checklist, which risks confusing users and contributors.
2. **Missing Directory Initialization:** `./data` is not tracked in git and not created automatically by `migrate.ts`, causing fresh checkouts to fail on `npm run db:setup` unless `mkdir -p data` is run first.

---

## 17. Production Readiness Score

### 1. ToolRakyat Core: `25/100`
- **Deductions (-75):**
  - -40: 48 of 48 non-AkaunKemas tools are placeholders with no UI or processing logic.
  - -20: Dynamic route is broken by non-existent `WordCounterTool` import.
  - -15: File validation and temp file utilities fail due to missing `limits.ts`.

### 2. SmartPDF: `0/100`
- **Deductions (-100):**
  - -60: No PDF editor engine, OCR, form detection, or canvas overlay exists.
  - -20: API route is a dead proxy to a non-existent external server (`http://localhost:8000`).
  - -20: Documented UAT routes and checklist files do not exist.

### 3. AkaunKemas: `85/100`
- **Deductions (-15):**
  - -5: Orphaned `UniversalImportClient.tsx` and `StepIndicator` lint issues break build/typecheck.
  - -5: Business settings page is a disabled mockup.
  - -5: Production auth secret fallback warning.
- **Strengths (+85):**
  - Complete, robust, verified core banking/accounting engine with 608 passing unit tests, Drizzle SQLite schema, migrations, RBAC, tenant isolation, and audit logging.

### Overall Repository Readiness: `40/100`

---

## 18. Remaining Work

### P0 — Must Fix Immediately (To Restore Green Build & CI)
1. **Create `src/lib/limits.ts`:**
   Define `LIMITS` constants (`GLOBAL_MAX_FILE_SIZE_MB: 20`, `RATE_LIMIT_MAX: 20`, `RATE_LIMIT_BURST: 10`, `TEMP_FILE_MAX_AGE_MINUTES: 30`, `CLIENT_CSV_MAX_SIZE_BYTES: 5 * 1024 * 1024`).
2. **Fix Dynamic Route `src/app/tools/[category]/[slug]/page.tsx`:**
   Remove the hardcoded import of non-existent `WordCounterTool` or replace it with a clean placeholder component until the tool is implemented.
3. **Resolve `UniversalImportClient.tsx`:**
   Either remove the orphaned unreferenced file or stub its missing import pipeline.
4. **Fix ESLint in `import-bank-csv/page.tsx`:**
   Move `StepIndicator` outside the render function to comply with React 19 rules, and clean up unused variables.
5. **Ensure `data/` Directory Creation in `src/lib/db/migrate.ts` and `src/lib/db/index.ts`:**
   Add `fs.mkdirSync("./data", { recursive: true })` before initializing SQLite.

### P1 — Required Before Beta (AkaunKemas Focus)
1. Wire up `BusinessSettingsPage` (`/app/akaunkemas/settings/business`) server action to update `businesses` table in SQLite.
2. Enforce strict `AUTH_SECRET` check in production (disallow hardcoded dev fallback).
3. Run and verify full Playwright E2E test suite (`akaunkemas-bank-csv-cleaner.spec.ts`, `akaunkemas-full-workflow.spec.ts`).
4. Update `README.md` to accurately reflect the real state of features (clarify that AkaunKemas is the active product, SmartPDF is pending backend integration).

### P2 — Required Before Production
1. Implement Redis/Upstash rate limiting adapter for multi-instance deployments.
2. Implement file storage provider (e.g. S3 / local persistent volume) for Accountant Pack file downloads and receipt image attachments.
3. Add health check endpoint (`/api/health`).

### P3 — Future Product Expansions
1. SmartPDF: Implement client-side PDF manipulation using `pdf-lib` + `pdfjs-dist` or build the dedicated Python processing microservice.
2. ToolRakyat Core: Implement actual UI and handlers for top requested productivity tools (PDF Merge/Split, Image Compress).

---

## 19. Recommended Next Development Slice

### Bounded Next Slice: **"Zero-Defect Build Restoration & AkaunKemas Green Pipeline"**

- **Exact Problem:**
  The project cannot run `next build` or `playwright test` due to missing `src/lib/limits.ts`, missing `WordCounterTool`, orphaned `UniversalImportClient.tsx`, and ESLint errors. This blocks all ongoing development and testing of AkaunKemas.
- **Why It Has Highest Priority:**
  Fixing this slice immediately unblocks the entire test suite, Next.js build, and Playwright E2E tests for AkaunKemas without architectural changes or rewrites.
- **Files Likely Affected:**
  - `src/lib/limits.ts` (NEW)
  - `src/app/tools/[category]/[slug]/page.tsx` (MODIFY)
  - `src/app/app/akaunkemas/import-bank-csv/page.tsx` (MODIFY - fix `StepIndicator` & unused vars)
  - `src/app/app/akaunkemas/_components/UniversalImportClient.tsx` (DELETE or FIX)
  - `src/lib/db/index.ts` & `src/lib/db/migrate.ts` (MODIFY - auto `mkdir -p data`)
  - `src/lib/akaunkemas-saas/services/__fixtures__/receipts-memory.ts` (MODIFY - clean unused vars)
- **Tests Required:**
  - `npm run lint` -> 0 errors
  - `npm run typecheck` -> 0 errors
  - `npm test` -> 21/21 suites passing (608+ tests)
  - `npm run build` -> Successful production build
  - `npx playwright test` -> Green E2E test execution
- **Acceptance Criteria:**
  - `npm run lint`, `npm run typecheck`, `npm test`, `npm run build`, and `npx playwright test` all exit with code 0.

---

## 20. Exact Commands to Reproduce Audit

```bash
# 1. Check Git Status & History
git status --short --branch
git remote -v
git log --oneline --decorate -20
git branch -a
git tag

# 2. Dependency Check & Install
node -v
npm -v
npm ci

# 3. Static Analysis & Build Verification
npm run lint
npm run typecheck
npm test
npm run build

# 4. Database Setup & Migration Check
mkdir -p data
npm run db:migrate
npm run db:seed

# 5. Playwright E2E Execution (requires successful build)
npx playwright test
```

---

*Audit completed strictly in accordance with production-readiness, QA engineering, and software archaeology guidelines.*
