# ToolRakyat

Free practical productivity tools for everyone.

ToolRakyat is an all-in-one productivity tools website (PDF, image, compression, converters, text tools, business generators, QR tools, developer utilities, calculators, and AI-assisted tools with clean provider abstraction).

## Tech Stack

- Next.js App Router
- TypeScript
- Tailwind CSS
- shadcn/ui
- React Hook Form + Zod
- pdf-lib (PDF tools)
- sharp (image tools)
- qrcode (QR tools)
- jszip + archiver (ZIP/compression tools)
- file-type + nanoid (file validation + safe temp workflows)
- Playwright (E2E tests)

## Privacy Model (MVP)

- Files are processed temporarily and are not stored permanently by default.
- Uploaded/processed files are cleaned up after processing.
- Files are never sent to third-party services unless a tool clearly says so (e.g. AI tools requiring provider setup).

## Local Setup

```bash
cd /Users/sallahuddin/Desktop/toolkits/toolrakyat
npm install
npm run dev
```

Open http://localhost:3000

## Commands

```bash
npm run dev
npm run lint
npm run typecheck
npm run build
npm test
npx playwright test
```

## Environment Variables

No environment variables are required for the current MVP tools.

Planned AI provider abstraction:

- `AI_PROVIDER`
- `AI_API_KEY`
- `AI_MODEL`

## Tool Implementation Status (Snapshot)

| Category | Tool | Route | Status |
|---|---|---|---|
| PDF | Merge PDF | `/tools/pdf/merge` | Working |
| PDF | Split PDF | `/tools/pdf/split` | Working |
| PDF | Rotate PDF | `/tools/pdf/rotate` | Working |
| PDF | Delete PDF Pages | `/tools/pdf/delete-pages` | Working |
| PDF | Extract PDF Pages | `/tools/pdf/extract-pages` | Working |
| PDF | Add Page Numbers | `/tools/pdf/page-numbers` | Working |
| PDF | Add Watermark | `/tools/pdf/watermark` | Working |
| PDF | Images to PDF | `/tools/pdf/images-to-pdf` | Working |
| PDF | PDF Metadata Editor | `/tools/pdf/metadata` | Working |
| PDF | Sign PDF | `/tools/pdf/sign` | Working |
| PDF | Reorder PDF Pages | `/tools/pdf/reorder` | Working |
| Image | Compress Image | `/tools/image/compress` | Working |
| Image | Resize Image | `/tools/image/resize` | Working |
| Image | Convert Image | `/tools/image/convert` | Working |
| Image | Rotate Image | `/tools/image/rotate` | Working |
| Image | Flip Image | `/tools/image/flip` | Working |
| Image | Remove Image Metadata | `/tools/image/remove-metadata` | Working |
| Compression | Image Batch Compressor | `/tools/compression/batch-image-compress` | Working |
| Compression | ZIP Creator | `/tools/compression/zip` | Working |
| Compression | JSON Minifier | `/tools/compression/json-minifier` | Working |
| Compression | CSS Minifier | `/tools/compression/css-minifier` | Working |
| Compression | HTML Minifier | `/tools/compression/html-minifier` | Working |
| Compression | ZIP Extractor (Safe-Mode) | `/tools/compression/zip-extract` | Working |
| Converter | CSV to JSON | `/tools/converter/csv-to-json` | Working |
| Converter | JSON to CSV | `/tools/converter/json-to-csv` | Working |
| Converter | Markdown to HTML | `/tools/converter/markdown-to-html` | Working |
| Text | Word Counter | `/tools/text/word-counter` | Working |
| Text | Case Converter | `/tools/text/case-converter` | Working |
| Text | Text Cleaner | `/tools/text/text-cleaner` | Working |
| Text | Slug Generator | `/tools/text/slug-generator` | Working |
| Text | Password Generator | `/tools/text/password-generator` | Working |
| QR | QR Code Generator | `/tools/qr/qr-code-generator` | Working |
| QR | WhatsApp Link Generator | `/tools/qr/whatsapp-link-generator` | Working |
| QR | WiFi QR Generator | `/tools/qr/wifi` | Working |
| QR | vCard QR Generator | `/tools/qr/vcard` | Working |
| QR | Email QR Generator | `/tools/qr/email` | Working |
| QR | SMS QR Generator | `/tools/qr/sms` | Working |
| Business | Invoice Generator | `/tools/business/invoice-generator` | Working |
| Business | Quotation Generator | `/tools/business/quotation-generator` | Working |
| Business | Receipt Generator | `/tools/business/receipt-generator` | Working |
| Business | Delivery Order Generator | `/tools/business/delivery-order-generator` | Working |
| Business | Purchase Order Generator | `/tools/business/purchase-order-generator` | Working |
| Calculator | Loan Calculator | `/tools/calculator/loan` | Working |
| Calculator | Profit Margin Calculator | `/tools/calculator/profit-margin` | Working |
| Calculator | Discount Calculator | `/tools/calculator/discount` | Working |
| Calculator | SST Calculator | `/tools/calculator/sst` | Working |
| Calculator | Age Calculator | `/tools/calculator/age` | Working |
| Calculator | Date Difference Calculator | `/tools/calculator/date-difference` | Working |
| Calculator | BMI Calculator | `/tools/calculator/bmi` | Working |
| Calculator | Compound Interest Calculator | `/tools/calculator/compound-interest` | Working |
| Developer | JSON Formatter | `/tools/developer/json-formatter` | Working |
| Developer | JSON Validator | `/tools/developer/json-validator` | Working |
| Developer | Base64 Encode/Decode | `/tools/developer/base64` | Working |
| Developer | URL Encode/Decode | `/tools/developer/url-encode` | Working |
| Developer | SHA-256 Hash Generator | `/tools/developer/sha-256` | Working |
| Developer | UUID Generator | `/tools/developer/uuid-generator` | Working |
| Developer | HEX/RGB Color Converter | `/tools/developer/color-converter` | Working |

## Known Limitations (Honest)

- Some tools require external binaries or external providers (OCR, background removal, some conversions). These will be shipped as UI + adapter placeholders until properly supported.
- Large file uploads may be constrained by runtime memory limits depending on deployment platform; ToolRakyat enforces per-tool max file sizes and validates file types.
- PDF OCR, PDF protect/unlock, and deep PDF compression are not implemented yet.

## PDF Page Range Syntax

Some PDF tools accept page ranges using 1-based page numbers:

- Single page: `1`
- Range: `1-3`
- Mixed: `1-3,5,7-9`

Notes:
- Page numbers must be between 1 and the total page count.
- Duplicates are removed.
- Order is preserved where practical (e.g. `3,1,2` stays in that order).

## PDF Rotate Notes

- Rotation adds to any existing page rotation (e.g. rotating a page already rotated 90° by 90° results in 180°).
- You can rotate all pages or specify custom pages/ranges using the syntax above.

## SmartPDF Manual UAT

SmartPDF is the “Advanced PDF Editor” at `/tools/pdf/editor`.

Run this manual UAT before beta sign-off and before production releases.

Required document set:
- Normal digital PDF (text-based, non-form)
- Scanned PDF (image-only)
- AcroForm PDF (true widgets)
- Visual non-AcroForm form (drawn boxes/labels, no widgets)
- Invoice PDF
- Table-heavy report PDF
- Image-heavy PDF
- Encrypted PDF (user password and/or owner restrictions)
- Corrupted PDF (broken xref / partial download)
- Large PDF (high page count or large file size)

For each document type, verify this flow:
- Upload: status reaches `ready` or a visible error is shown (`error`) with clear message
- Analyze: document preview loads, navigation works, warnings appear when applicable
- Edit/Fill: perform at least one edit or form interaction appropriate to the file type
- Export Editable: download and verify editable-mode result
- Export Flattened: download and verify flattened-mode result
- Re-open Result: re-open/re-upload exported outputs and confirm expected behavior

Detailed checklist and beta readiness report template:
- [docs/SMARTPDF_BETA_CHECKLIST.md](docs/SMARTPDF_BETA_CHECKLIST.md)

Optional: there is also a UI checklist page at `/tools/pdf/editor/uat`.

## Production Hardening & Security

ToolRakyat is designed for high-traffic production environments with the following security measures:

- **IP-Based Rate Limiting**: Global limit of 20 requests per minute with a burst allowance of 10.
- **File Size Enforcement**: 
  - Global hard limit: 20MB (configurable via `MAX_FILE_SIZE_MB`).
  - Per-tool specific limits (validated on both client and server).
- **Temporary File Lifecycle**:
  - All files are processed in isolated workspaces in `/tmp/toolrakyat`.
  - Automatic cleanup of workspaces older than 30 minutes.
- **ZIP Extraction Guards**:
  - Prevents "ZipBomb" attacks via file count (max 100) and total extracted size (max 100MB) limits.
  - Mitigates "ZipSlip" (path traversal) by sanitizing all entry names.
  - Rejects nested archives within ZIPs.
- **Standardized Error Handling**: Detailed logs are kept on the server, while users receive generic, safe error messages to prevent information leakage.
- **Privacy-First**: No data is permanently stored. Processing is stateless and ephemeral.

## Deployment

Recommended: Vercel or any Node.js compatible host.

1. Copy `.env.example` to `.env`.
2. Configure `MAX_FILE_SIZE_MB` and `RATE_LIMIT` variables.
3. Run `npm run build`.
4. Deploy.

## Roadmap

- Implement the shared tool framework components for all tools (Dropzone, processing states, result cards, errors).
- Add more PDF tools (OCR, protect/unlock, deep compression).
- Expand image tools (crop UI, favicon generator).
- Add more compression utilities.
- Add more calculators and developer utilities.
- Expand Playwright E2E coverage for all major flows.
