import { PDFDocument, StandardFonts, rgb } from "pdf-lib";
import type { BusinessHeader, MonthlySummary } from "./types";
import { getCategoryLabel } from "./categories";

const A4: [number, number] = [595.28, 841.89];
const MARGIN = 50;
const FONT_SIZE = 10;
const TITLE_SIZE = 18;

const MYR = new Intl.NumberFormat("ms-MY", { style: "currency", currency: "MYR" });

function formatCurrency(n: number): string {
  return MYR.format(Number.isFinite(n) ? n : 0);
}

function formatDate(iso?: string): string {
  if (!iso) return new Date().toISOString().slice(0, 10);
  return iso.slice(0, 10);
}

/**
 * Build aggregate data for PDF export (testable without pdf-lib).
 */
export function buildPdfData(summary: MonthlySummary, periodStart?: string, periodEnd?: string) {
  return {
    title: "AkaunKemas Monthly Summary",
    generatedDate: new Date().toISOString().slice(0, 10),
    periodStart: periodStart ?? "",
    periodEnd: periodEnd ?? "",
    totalIncome: formatCurrency(summary.totalIncome),
    totalExpense: formatCurrency(summary.totalExpense),
    netCashflow: formatCurrency(summary.netCashflow),
    transactionCount: summary.transactionCount,
    categoryRows: summary.categorySummaries.map((cs) => ({
      category: getCategoryLabel(cs.category),
      total: formatCurrency(cs.total),
      count: cs.count,
    })),
  };
}

/**
 * Generate a monthly summary PDF.
 * When businessHeader is provided, includes a business info block at the top.
 */
export async function generateMonthlySummaryPdf(args: {
  summary: MonthlySummary;
  periodStart?: string;
  periodEnd?: string;
  businessHeader?: BusinessHeader;
}): Promise<Uint8Array> {
  const { summary, periodStart, periodEnd, businessHeader } = args;

  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);

  const page = doc.addPage(A4);
  const { width, height } = page.getSize();

  let y = height - MARGIN;

  // Title
  page.drawText("AkaunKemas Monthly Summary", {
    x: MARGIN,
    y,
    size: TITLE_SIZE,
    font: bold,
    color: rgb(0.07, 0.1, 0.2),
  });
  y -= 28;

  // Generated date
  page.drawText(`Generated: ${formatDate()}`, {
    x: MARGIN,
    y,
    size: FONT_SIZE,
    font,
    color: rgb(0.4, 0.4, 0.4),
  });
  y -= 16;

  // Period
  if (periodStart && periodEnd) {
    page.drawText(`Period: ${periodStart} — ${periodEnd}`, {
      x: MARGIN,
      y,
      size: FONT_SIZE,
      font,
      color: rgb(0.2, 0.2, 0.2),
    });
    y -= 16;
  }

  // Business header (optional)
  if (businessHeader?.name) {
    y -= 8;
    page.drawText(businessHeader.name, {
      x: MARGIN,
      y,
      size: 12,
      font: bold,
      color: rgb(0.1, 0.1, 0.1),
    });
    y -= 16;

    const bizLines = [
      businessHeader.registrationNumber ? `Reg: ${businessHeader.registrationNumber}` : "",
      businessHeader.address ?? "",
      businessHeader.phone ? `Phone: ${businessHeader.phone}` : "",
      businessHeader.email ? `Email: ${businessHeader.email}` : "",
      businessHeader.preparedBy ? `Prepared by: ${businessHeader.preparedBy}` : "",
    ].filter(Boolean);

    for (const line of bizLines) {
      page.drawText(line, { x: MARGIN, y, size: FONT_SIZE, font, color: rgb(0.3, 0.3, 0.3) });
      y -= 13;
    }
    y -= 4;
  }

  // Divider
  y -= 6;
  page.drawLine({
    start: { x: MARGIN, y },
    end: { x: width - MARGIN, y },
    thickness: 1,
    color: rgb(0.85, 0.87, 0.9),
  });
  y -= 18;

  // Summary totals section
  page.drawText("Summary", { x: MARGIN, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
  y -= 18;

  const totalItems = [
    ["Total Income", formatCurrency(summary.totalIncome)],
    ["Total Expenses", formatCurrency(summary.totalExpense)],
    ["Net Cashflow", formatCurrency(summary.netCashflow)],
    ["Transaction Count", String(summary.transactionCount)],
  ];

  for (const [label, value] of totalItems) {
    page.drawText(label, { x: MARGIN, y, size: FONT_SIZE, font, color: rgb(0.2, 0.2, 0.2) });
    const vw = font.widthOfTextAtSize(value, FONT_SIZE);
    page.drawText(value, { x: width - MARGIN - vw, y, size: FONT_SIZE, font: bold, color: rgb(0.1, 0.1, 0.1) });
    y -= 15;
  }

  // Divider
  y -= 4;
  page.drawLine({
    start: { x: MARGIN, y },
    end: { x: width - MARGIN, y },
    thickness: 1,
    color: rgb(0.85, 0.87, 0.9),
  });
  y -= 18;

  // Category summary table
  page.drawText("Category Breakdown", { x: MARGIN, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
  y -= 18;

  // Table header
  const colX = { cat: MARGIN, count: MARGIN + 260, total: width - MARGIN };
  page.drawText("Category", { x: colX.cat, y, size: FONT_SIZE, font: bold, color: rgb(0.1, 0.1, 0.1) });
  page.drawText("Count", { x: colX.count, y, size: FONT_SIZE, font: bold, color: rgb(0.1, 0.1, 0.1) });
  page.drawText("Total", { x: colX.total - 60, y, size: FONT_SIZE, font: bold, color: rgb(0.1, 0.1, 0.1) });
  y -= 6;
  page.drawLine({
    start: { x: MARGIN, y },
    end: { x: width - MARGIN, y },
    thickness: 0.5,
    color: rgb(0.85, 0.87, 0.9),
  });
  y -= 14;

  for (const cs of summary.categorySummaries) {
    if (y < 120) {
      const newPage = doc.addPage(A4);
      y = height - MARGIN;
      // Re-draw table header on new page
      newPage.drawText("Category Breakdown (continued)", { x: MARGIN, y, size: 13, font: bold, color: rgb(0.1, 0.1, 0.1) });
      y -= 18;
    }

    const label = getCategoryLabel(cs.category);
    page.drawText(label, { x: colX.cat, y, size: FONT_SIZE, font, color: rgb(0.2, 0.2, 0.2) });
    page.drawText(String(cs.count), { x: colX.count, y, size: FONT_SIZE, font, color: rgb(0.2, 0.2, 0.2) });

    const totalStr = formatCurrency(cs.total);
    const tw = font.widthOfTextAtSize(totalStr, FONT_SIZE);
    page.drawText(totalStr, { x: colX.total - tw, y, size: FONT_SIZE, font, color: rgb(0.2, 0.2, 0.2) });
    y -= 14;
  }

  // Footer disclaimer
  const footerY = 40;
  page.drawLine({
    start: { x: MARGIN, y: footerY + 10 },
    end: { x: width - MARGIN, y: footerY + 10 },
    thickness: 0.5,
    color: rgb(0.85, 0.87, 0.9),
  });
  page.drawText(
    "Generated by AkaunKemas / ToolRakyat. Please review with your accountant before submission.",
    { x: MARGIN, y: footerY - 8, size: 8, font, color: rgb(0.5, 0.5, 0.5) },
  );

  return await doc.save();
}
