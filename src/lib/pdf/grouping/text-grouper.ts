import type { StarPdfTextSpan } from "../starpdf-types";
import type { HumanTextGroup, TextGroupingOptions } from "./types";

const DEFAULT_OPTIONS: Required<TextGroupingOptions> = {
  maxBaselineDiff: 2.0,
  maxHorizontalGapMultiplier: 1.8,
  maxFontSizeDiff: 1.5,
};

/**
 * Groups raw extracted StarPDF text spans into human-scale text runs/words
 * while preserving 100% exact underlying source span provenance.
 *
 * Invariants:
 * - Never groups across different pages or rotations.
 * - Never groups across multi-column gaps or table cell boundaries.
 * - Single-span groups retain atomic editability status.
 * - Multi-span visual groups are marked GROUP_SELECTION_ONLY to guarantee safety.
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
    // Sort spans top-to-bottom, left-to-right
    // In PDF coordinates, top of page has larger Y.
    const sorted = [...bucketSpans].sort((a, b) => {
      // If baselines are significantly different, higher Y comes first (PDF coords)
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

        // Add space if there is a gap between consecutive spans in same group
        if (i > 0) {
          const prev = currentGroupSpans[i - 1];
          const gap = s.x - (prev.x + prev.width);
          const spaceEst = prev.font_size * 0.25;
          if (gap >= spaceEst * 0.8 && !textParts[textParts.length - 1]?.endsWith(" ") && !s.text.startsWith(" ")) {
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
        // Multi-span visual group
        is_editable = false;
        editability = "GROUP_SELECTION_ONLY";
        refusal_reason = "This text selection spans multiple PDF text operations and can't yet be safely rewritten as one native edit.";
        detailed_reason = `This visual text run consists of ${currentGroupSpans.length} separate PDF content stream operations.`;
      }

      const id = isSingle
        ? `group-${singleSpan.span_id}`
        : `group-p${pageIndex}-${Math.round(minX)}-${Math.round(minY)}-${currentGroupSpans.length}`;

      groups.push({
        id,
        pageIndex,
        text: combinedText,
        type: isSingle ? "word" : "run",
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
      const sameFont = span.font_name === prev.font_name;

      // Estimate expected spacing
      const avgFontSize = (span.font_size + prev.font_size) / 2;
      const spaceWidth = avgFontSize * 0.28;
      const maxAllowedGap = Math.max(12.0, spaceWidth * opts.maxHorizontalGapMultiplier);

      const gap = span.x - (prev.x + prev.width);
      // Gaps between 0 and maxAllowedGap (or slight overlap from kerning) are groupable
      const adjacentHorizontally = gap >= -2.0 && gap <= maxAllowedGap;

      if (sameBaseline && sameFontSize && sameFont && adjacentHorizontally) {
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
