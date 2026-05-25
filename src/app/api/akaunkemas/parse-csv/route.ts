import { NextRequest, NextResponse } from "next/server";
import { parseBankCsv, validateParseResult } from "@/lib/akaunkemas/csv-parser";
import { LIMITS } from "@/lib/limits";

/**
 * Server-side CSV parsing fallback for large files (>5MB up to 20MB).
 * Accepts JSON body with { csvText: string }.
 * The client reads the file as text and sends the content.
 */
export async function POST(request: NextRequest) {
  try {
    const maxBytes = LIMITS.GLOBAL_MAX_FILE_SIZE_MB * 1024 * 1024;

    // Guard: early reject if content-length header already exceeds limit.
    const contentLength = request.headers.get("content-length");
    if (contentLength && Number(contentLength) > maxBytes) {
      return NextResponse.json(
        { error: `Request body too large. Maximum is ${LIMITS.GLOBAL_MAX_FILE_SIZE_MB}MB.` },
        { status: 413 },
      );
    }

    // Read the raw body as text so we can check its actual size (content-length
    // is not always present, e.g. chunked transfer-encoding).
    let rawBody: string;
    try {
      rawBody = await request.text();
    } catch {
      return NextResponse.json({ error: "Failed to read request body." }, { status: 400 });
    }

    if (rawBody.length > maxBytes) {
      return NextResponse.json(
        { error: `Request body too large. Maximum is ${LIMITS.GLOBAL_MAX_FILE_SIZE_MB}MB.` },
        { status: 413 },
      );
    }

    let body: { csvText?: string };
    try {
      body = JSON.parse(rawBody) as { csvText?: string };
    } catch {
      return NextResponse.json(
        { error: "Invalid JSON body. Expected { csvText: string }." },
        { status: 400 },
      );
    }

    if (typeof body.csvText !== "string") {
      return NextResponse.json(
        { error: "Missing or invalid csvText field." },
        { status: 400 },
      );
    }

    const csvText = body.csvText.trim();
    if (!csvText) {
      return NextResponse.json({ error: "CSV text is empty." }, { status: 400 });
    }

    // Parse in memory
    const result = parseBankCsv(csvText);
    const validationIssues = validateParseResult(result);

    return NextResponse.json({
      success: true,
      transactions: result.transactions,
      detectedColumns: result.detectedColumns,
      errors: [...result.errors, ...validationIssues],
      rawRowCount: result.rawRowCount,
    });
  } catch (error) {
    console.error("AkaunKemas CSV parse error:", error);
    return NextResponse.json(
      { error: "Failed to parse CSV on the server." },
      { status: 500 },
    );
  }
}
