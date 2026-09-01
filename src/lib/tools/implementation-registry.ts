import type { ToolDefinition } from "./types";

export type ToolImplementationKey = "generic-utility" | "word-counter" | "smartpdf-launcher" | "dedicated-route";

const GENERIC_UTILITY_IDS = new Set([
  "pdf-merge", "pdf-split", "pdf-rotate", "pdf-images-to-pdf", "pdf-delete-pages", "pdf-extract-pages",
  "pdf-page-numbers", "pdf-watermark", "pdf-metadata", "pdf-sign", "pdf-reorder",
  "image-compress", "image-resize", "image-convert", "image-rotate", "image-flip", "image-remove-metadata",
  "compression-batch-image-compress", "zip-create", "compression-json-minifier", "compression-css-minifier",
  "compression-html-minifier", "zip-extract", "converter-csv-to-json", "converter-json-to-csv",
  "converter-markdown-to-html", "text-slug-generator", "text-case-converter", "text-cleaner",
  "text-password-generator", "business-invoice", "business-quotation", "business-receipt",
  "business-delivery-order", "business-purchase-order", "calc-loan", "calc-profit-margin", "calc-discount",
  "calc-sst", "calc-age", "calc-date-difference", "calc-bmi", "calc-compound-interest", "dev-json-formatter",
  "dev-json-validator", "dev-base64", "dev-url-encode", "dev-sha-256", "dev-uuid-generator",
  "dev-color-converter", "qr-qr-code-generator", "qr-whatsapp-link-generator", "qr-wifi", "qr-vcard",
  "qr-email", "qr-sms",
]);

const DEDICATED_ROUTE_IDS = new Set([
  "akaunkemas-bank-csv-cleaner", "akaunkemas-receipt-organizer", "akaunkemas-receipt-matcher",
  "akaunkemas-accountant-pack", "akaunkemas-simple-ledger",
]);

export function getImplementationKey(tool: ToolDefinition): ToolImplementationKey | undefined {
  if (!tool.isImplemented) return undefined;
  if (tool.id === "text-word-counter") return "word-counter";
  if (tool.id === "pdf-editor") return "smartpdf-launcher";
  if (DEDICATED_ROUTE_IDS.has(tool.id)) return "dedicated-route";
  if (GENERIC_UTILITY_IDS.has(tool.id)) return "generic-utility";
  return undefined;
}

export function hasPrimaryAction(tool: ToolDefinition): boolean {
  return getImplementationKey(tool) !== undefined;
}
