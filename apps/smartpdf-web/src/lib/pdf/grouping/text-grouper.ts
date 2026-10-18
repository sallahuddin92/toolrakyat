import type { StarPdfTextSpan } from "../starpdf-types";
import type { HumanTextGroup, TextGroupingOptions } from "./types";

const DEFAULT_OPTIONS: Required<TextGroupingOptions> = {
  maxBaselineDiff: 2.0,
  maxHorizontalGapMultiplier: 1.6,
  maxFontSizeDiff: 1.5,
  granularity: "word",
};




function normalizeFont(name?: string): string {
  if (!name) return "";
  return name.replace(/^[A-Z]{6}\+/, "").replace(/-\d+$/, "");
}

function isSameFont(a: StarPdfTextSpan, b: StarPdfTextSpan): boolean {
  if (a.font_name === b.font_name) return true;
  if (a.font_base_name && b.font_base_name && a.font_base_name === b.font_base_name) return true;
  const normA = normalizeFont(a.font_base_name || a.font_name);
  const normB = normalizeFont(b.font_base_name || b.font_name);
  return normA === normB && normA.length > 0;
}

/**
 * Groups raw extracted StarPDF text spans into human-scale text words/runs
 * while preserving 100% exact underlying source span provenance.
 *
 * Invariants:
 * - Never groups across different pages, orientations, columns, or table cells.
 * - Single-span and multi-span words with proven same-font, same-stream safety
 *   are classified as EDITABLE_ATOMIC.
 * - Incompatible multi-span groups safely refuse mutation as READ_ONLY_REFUSAL.
 */
export function groupTextSpans(
  spans: StarPdfTextSpan[],
  options?: TextGroupingOptions,
): HumanTextGroup[] {
  if (!spans || spans.length === 0) {
    return [];
  }

  const opts = { ...DEFAULT_OPTIONS, ...options };
  const groups: HumanTextGroup[] = [];

  // Group spans by (pageIndex, rotation) first
  const pageBuckets = new Map<string, StarPdfTextSpan[]>();
  for (const span of spans) {
    if (!span.text || span.text.trim().length === 0) {
      continue;
    }
    const key = `${span.page_index}_${Math.round(span.rotation || 0)}`;
    const bucket = pageBuckets.get(key);
    if (bucket) {
      bucket.push(span);
    } else {
      pageBuckets.set(key, [span]);
    }
  }

  for (const bucketSpans of pageBuckets.values()) {
    // Sort spans top-to-bottom, left-to-right (PDF coordinate space: higher Y is higher on page)
    const sorted = [...bucketSpans].sort((a, b) => {
      if (Math.abs(a.y - b.y) > opts.maxBaselineDiff) {
        return b.y - a.y;
      }
      return a.x - b.x;
    });

    let currentGroupSpans: StarPdfTextSpan[] = [];

    const flushCurrentGroup = () => {
      if (currentGroupSpans.length === 0) return;

      const first = currentGroupSpans[0];
      const pageIndex = first.page_index;
      const rotation = first.rotation || 0;
      const fontSize = first.font_size;
      const fontName = first.font_name;

      // Compute bounding box containing all spans in group
      let minX = Infinity;
      let minY = Infinity;
      let maxX = -Infinity;
      let maxY = -Infinity;
      const textParts: string[] = [];

      for (let i = 0; i < currentGroupSpans.length; i++) {
        const s = currentGroupSpans[i];
        minX = Math.min(minX, s.x);
        minY = Math.min(minY, s.y);
        maxX = Math.max(maxX, s.x + s.width);
        maxY = Math.max(maxY, s.y + s.height);

        // Add space if there is a true word gap between consecutive spans in same group
        if (i > 0) {
          const prev = currentGroupSpans[i - 1];
          const gap = s.x - (prev.x + prev.width);
          const spaceEst = prev.font_size * 0.28;
          if (gap >= spaceEst * 1.6 && !textParts[textParts.length - 1]?.endsWith(" ") && !s.text.startsWith(" ")) {
            textParts.push(" ");
          }
        }


        textParts.push(s.text);
      }

      const combinedText = textParts.join("").trim();
      const isSingle = currentGroupSpans.length === 1;
      const singleSpan = currentGroupSpans[0];

      let is_editable = false;
      let editability: HumanTextGroup["editability"] = "READ_ONLY_REFUSAL";
      let refusal_reason: string | undefined;
      let detailed_reason: string | undefined;

      if (isSingle) {
        if (singleSpan.is_editable) {
          is_editable = true;
          editability = "EDITABLE_ATOMIC";
        } else {
          is_editable = false;
          editability = "READ_ONLY_REFUSAL";
          refusal_reason = singleSpan.refusal_reason || "This text can't be safely edited in place.";
          detailed_reason = singleSpan.refusal_reason;
        }
      } else {
        // Multi-span group evaluation
        const allSpansEditable = currentGroupSpans.every((s) => s.is_editable);
        const allSameFont = currentGroupSpans.every((s) => isSameFont(s, first));
        const allSameStream = currentGroupSpans.every((s) => s.stream_index === first.stream_index);

        if (allSpansEditable && allSameFont && allSameStream) {
          is_editable = true;
          editability = "EDITABLE_ATOMIC";
        } else {
          is_editable = false;
          editability = "READ_ONLY_REFUSAL";
          refusal_reason = "This text can't be safely edited in place.";
          if (!allSameFont) {
            detailed_reason = "This text spans multiple different font resources that cannot be rewritten together.";
          } else if (!allSameStream) {
            detailed_reason = "This text spans multiple content streams that cannot be modified in a single text transaction.";
          } else {
            detailed_reason = "This text is built from multiple PDF text operations that cannot be safely rewritten together.";
          }
        }
      }

      const id = isSingle
        ? `group-${singleSpan.span_id}`
        : `group-p${pageIndex}-${Math.round(minX)}-${Math.round(minY)}-${currentGroupSpans.length}`;

      groups.push({
        id,
        pageIndex,
        text: combinedText,
        type: opts.granularity === "run" ? "run" : "word",
        x: minX,
        y: minY,
        width: Math.max(1, maxX - minX),
        height: Math.max(1, maxY - minY),
        rotation,
        fontSize,
        fontName,
        is_editable,
        editability,
        refusal_reason,
        detailed_reason,
        sourceSpans: currentGroupSpans,
        primarySpanId: singleSpan.span_id,
      });

      currentGroupSpans = [];
    };

    for (const span of sorted) {
      if (currentGroupSpans.length === 0) {
        currentGroupSpans.push(span);
        continue;
      }

      const prev = currentGroupSpans[currentGroupSpans.length - 1];

      // Check baseline proximity
      const sameBaseline = Math.abs(span.y - prev.y) <= opts.maxBaselineDiff;
      // Check font size proximity
      const sameFontSize = Math.abs(span.font_size - prev.font_size) <= opts.maxFontSizeDiff;
      // Check font resource name
      const sameFont = isSameFont(span, prev);

      // Estimate expected spacing
      const avgFontSize = (span.font_size + prev.font_size) / 2;
      const spaceWidth = avgFontSize * 0.28;

      // Intra-word adjacency: span starts after prev.x, with gap between prev.x + prev.width and span.x <= spaceWidth * 0.45
      // Or in cases where estimated width overlaps, span.x is within prev.x .. prev.x + prev.width
      const isWhitespace = prev.text.endsWith(" ") || span.text.startsWith(" ");
      const withinHorizontalBounds =
        span.x >= prev.x && span.x <= prev.x + Math.max(prev.width, spaceWidth * 2) + spaceWidth * opts.maxHorizontalGapMultiplier;

      const gap = span.x - (prev.x + prev.width);
      const isWordContinuation = (gap <= spaceWidth * opts.maxHorizontalGapMultiplier || span.x <= prev.x + prev.width + 1.0) && withinHorizontalBounds;

      if (sameBaseline && sameFontSize && sameFont && isWordContinuation && !isWhitespace) {
        currentGroupSpans.push(span);
      } else {
        flushCurrentGroup();
        currentGroupSpans.push(span);
      }
    }


    flushCurrentGroup();
  }

  return groups;
}

