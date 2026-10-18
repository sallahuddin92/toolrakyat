/**
 * Flat-Form Affordance Detector (Local-First, No OCR)
 *
 * Implements the 4-level detection hierarchy:
 * LEVEL 1: Native AcroForm Fields (Exact semantics)
 * LEVEL 2: Native PDF Vector Geometry (Lines, Rectangles, Circles)
 * LEVEL 3: Flat Raster Page Analysis (Local contour, aspect ratio, & fill ratio)
 * LEVEL 4: Manual Fallback (Always accessible)
 */

import type { AcroFormField } from "@/lib/pdf/pdf-types";
import type { StarPdfVectorGraphicInfo } from "@/lib/pdf/starpdf-types";
import type { FlatFormCandidate, DetectorOptions } from "./types";

const DEFAULT_OPTIONS: Required<DetectorOptions> = {
  minCheckboxSize: 8,
  maxCheckboxSize: 32,
  minRadioDiameter: 8,
  maxRadioDiameter: 28,
  minTextRegionWidth: 40,
  maxTextRegionWidth: 500,
  minTextRegionHeight: 10,
};


export interface DetectPageOptions {
  pageIndex: number;
  pageWidth: number; // in PDF points
  pageHeight: number; // in PDF points
  acroFields?: AcroFormField[];
  vectorGraphics?: StarPdfVectorGraphicInfo[];
  imageData?: ImageData | null;
  detectorOptions?: DetectorOptions;
}

/**
 * Calculates Intersection over Union (IoU) of two PDF rectangles [x1, y1, x2, y2]
 */
export function calculateIoU(
  rectA: [number, number, number, number],
  rectB: [number, number, number, number],
): number {
  const [ax1, ay1, ax2, ay2] = [
    Math.min(rectA[0], rectA[2]),
    Math.min(rectA[1], rectA[3]),
    Math.max(rectA[0], rectA[2]),
    Math.max(rectA[1], rectA[3]),
  ];
  const [bx1, by1, bx2, by2] = [
    Math.min(rectB[0], rectB[2]),
    Math.min(rectB[1], rectB[3]),
    Math.max(rectB[0], rectB[2]),
    Math.max(rectB[1], rectB[3]),
  ];

  const interX1 = Math.max(ax1, bx1);
  const interY1 = Math.max(ay1, by1);
  const interX2 = Math.min(ax2, bx2);
  const interY2 = Math.min(ay2, by2);

  if (interX2 <= interX1 || interY2 <= interY1) {
    return 0;
  }

  const interArea = (interX2 - interX1) * (interY2 - interY1);
  const areaA = (ax2 - ax1) * (ay2 - ay1);
  const areaB = (bx2 - bx1) * (by2 - by1);
  const unionArea = areaA + areaB - interArea;

  return unionArea <= 0 ? 0 : interArea / unionArea;
}

/**
 * Deduplicates overlapping candidates, prioritizing AcroForm > Vector > Raster and higher confidence.
 */
export function deduplicateCandidates(candidates: FlatFormCandidate[]): FlatFormCandidate[] {
  const sorted = [...candidates].sort((a, b) => {
    const sourcePriority = { acroform: 3, vector: 2, raster: 1 };
    const confPriority = { HIGH: 3, MEDIUM: 2, LOW: 1 };

    const spA = sourcePriority[a.source] || 0;
    const spB = sourcePriority[b.source] || 0;
    if (spA !== spB) return spB - spA;

    const cpA = confPriority[a.confidence] || 0;
    const cpB = confPriority[b.confidence] || 0;
    return cpB - cpA;
  });

  const results: FlatFormCandidate[] = [];

  for (const cand of sorted) {
    const isDuplicate = results.some((existing) => {
      if (existing.pageIndex !== cand.pageIndex) return false;
      const iou = calculateIoU(existing.pdfRect, cand.pdfRect);
      if (iou > 0.35) return true;

      // Center distance check for small marks
      const cx1 = (existing.pdfRect[0] + existing.pdfRect[2]) / 2;
      const cy1 = (existing.pdfRect[1] + existing.pdfRect[3]) / 2;
      const cx2 = (cand.pdfRect[0] + cand.pdfRect[2]) / 2;
      const cy2 = (cand.pdfRect[1] + cand.pdfRect[3]) / 2;
      const dist = Math.hypot(cx1 - cx2, cy1 - cy2);

      const maxDim = Math.max(
        Math.abs(existing.pdfRect[2] - existing.pdfRect[0]),
        Math.abs(existing.pdfRect[3] - existing.pdfRect[1]),
      );

      return dist < maxDim * 0.6;
    });

    if (!isDuplicate) {
      results.push(cand);
    }
  }

  return results;
}

/**
 * Detects form affordances on a page using the 4-level detection hierarchy.
 */
export function detectFlatFormCandidates(options: DetectPageOptions): FlatFormCandidate[] {
  const {
    pageIndex,
    pageWidth,
    pageHeight,
    acroFields = [],
    vectorGraphics = [],
    imageData = null,
    detectorOptions = {},
  } = options;

  const opt = { ...DEFAULT_OPTIONS, ...detectorOptions };
  const rawCandidates: FlatFormCandidate[] = [];

  // -------------------------------------------------------------
  // LEVEL 1: NATIVE ACROFORM FIELDS
  // -------------------------------------------------------------
  for (const field of acroFields) {
    const isPageMatch =
      field.pageNumber !== undefined
        ? field.pageNumber === pageIndex + 1
        : field.pageIndex !== undefined
          ? field.pageIndex === pageIndex
          : true;

    if (isPageMatch && field.rect) {
      const { x, y, width, height } = field.rect;
      const pdfRect: [number, number, number, number] = [x, y, x + width, y + height];

      let type: FlatFormCandidate["type"] = "text-region";
      if (field.type === "checkbox") type = "checkbox";
      else if (field.type === "radio") type = "radio";

      rawCandidates.push({
        id: `cand-acro-${field.name}`,
        type,
        pageIndex,
        pdfRect,
        confidence: "HIGH",
        source: "acroform",
      });
    }
  }

  // -------------------------------------------------------------
  // LEVEL 2: NATIVE PDF VECTOR GEOMETRY
  // -------------------------------------------------------------
  for (const vec of vectorGraphics) {
    if (vec.page_index === pageIndex && vec.bounds) {
      const [x, y, width, height] = vec.bounds;
      if (width <= 0 || height <= 0) continue;

      const aspect = width / height;
      const pdfRect: [number, number, number, number] = [
        x,
        y,
        x + width,
        y + height,
      ];

      // Ignore whole page borders or very large containers
      if (width > pageWidth * 0.9 && height > pageHeight * 0.9) continue;

      // Circular vector / path -> Radio candidate
      const isCircle =
        vec.graphic_type === "Circle" ||
        (vec.graphic_type === "Path" &&
          aspect >= 0.8 &&
          aspect <= 1.25 &&
          width >= opt.minRadioDiameter &&
          width <= opt.maxRadioDiameter);

      if (
        isCircle &&
        aspect >= 0.75 &&
        aspect <= 1.35 &&
        width >= opt.minRadioDiameter &&
        width <= opt.maxRadioDiameter
      ) {
        rawCandidates.push({
          id: `cand-vec-rad-${vec.graphic_id}`,
          type: "radio",
          pageIndex,
          pdfRect,
          confidence: "HIGH",
          source: "vector",
        });
        continue;
      }

      // Small near-square -> Checkbox candidate
      if (
        aspect >= 0.8 &&
        aspect <= 1.25 &&
        width >= opt.minCheckboxSize &&
        width <= opt.maxCheckboxSize &&
        height >= opt.minCheckboxSize &&
        height <= opt.maxCheckboxSize
      ) {
        rawCandidates.push({
          id: `cand-vec-cb-${vec.graphic_id}`,
          type: "checkbox",
          pageIndex,
          pdfRect,
          confidence: "HIGH",
          source: "vector",
        });
        continue;
      }

      // Horizontal line -> text line affordance
      if (
        (vec.graphic_type === "Line" || (height <= 4 && width >= opt.minTextRegionWidth)) &&
        width >= opt.minTextRegionWidth &&
        width <= opt.maxTextRegionWidth
      ) {
        rawCandidates.push({
          id: `cand-vec-line-${vec.graphic_id}`,
          type: "text-region",
          pageIndex,
          pdfRect: [pdfRect[0], pdfRect[1], pdfRect[2], pdfRect[1] + 16],
          confidence: "HIGH",
          source: "vector",
        });
        continue;
      }
    }
  }

  // -------------------------------------------------------------
  // LEVEL 3: FLAT RASTER PAGE LOCAL GEOMETRY ANALYSIS

  // -------------------------------------------------------------
  if (imageData && imageData.width > 0 && imageData.height > 0) {
    const rasterCandidates = analyzeRasterImageData(imageData, pageIndex, pageWidth, pageHeight, opt);
    rawCandidates.push(...rasterCandidates);
  }

  return deduplicateCandidates(rawCandidates);
}

/**
 * Analyzes rendered page pixels to detect candidate checkboxes, radio rings, and text lines.
 */
export function analyzeRasterImageData(
  imageData: ImageData,
  pageIndex: number,
  pageWidth: number,
  pageHeight: number,
  opt: Required<DetectorOptions>,
): FlatFormCandidate[] {
  const { width: imgW, height: imgH, data } = imageData;
  const scaleX = pageWidth / imgW;
  const scaleY = pageHeight / imgH;

  const candidates: FlatFormCandidate[] = [];

  // Helper to test if a pixel is dark (luminance < 140)
  const isDark = (x: number, y: number): boolean => {
    if (x < 0 || x >= imgW || y < 0 || y >= imgH) return false;
    const idx = (y * imgW + x) * 4;
    const r = data[idx];
    const g = data[idx + 1];
    const b = data[idx + 2];
    const a = data[idx + 3];
    if (a < 128) return false;
    const lum = 0.299 * r + 0.587 * g + 0.114 * b;
    return lum < 140;
  };

  // Convert pixel coords (top-left origin) to PDF points (bottom-left origin)
  const toPdfRect = (
    px: number,
    py: number,
    pw: number,
    ph: number,
  ): [number, number, number, number] => {
    const x1 = px * scaleX;
    const y2 = pageHeight - py * scaleY;
    const x2 = (px + pw) * scaleX;
    const y1 = pageHeight - (py + ph) * scaleY;
    return [x1, y1, x2, y2];
  };

  // Grid sweep with stride for bounding box discovery
  const stride = Math.max(2, Math.floor(imgW / 400));
  const visited = new Uint8Array(imgW * imgH);

  for (let y = stride; y < imgH - stride; y += stride) {
    for (let x = stride; x < imgW - stride; x += stride) {
      if (visited[y * imgW + x]) continue;

      if (isDark(x, y)) {
        // Fast horizontal run
        let right = x;
        while (right < imgW && isDark(right, y) && right - x < Math.floor(300 / scaleX)) {
          right++;
        }
        const runLen = right - x;

        // Trace vertical run
        let down = y;
        while (down < imgH && isDark(x, down) && down - y < Math.floor(300 / scaleY)) {
          down++;
        }
        const vRunLen = down - y;

        const wPt = runLen * scaleX;
        const hPt = vRunLen * scaleY;

        // Check for closed box candidate
        if (
          wPt >= opt.minCheckboxSize &&
          wPt <= opt.maxCheckboxSize &&
          hPt >= opt.minCheckboxSize &&
          hPt <= opt.maxCheckboxSize
        ) {
          const aspect = wPt / hPt;
          if (aspect >= 0.75 && aspect <= 1.35) {
            // Verify opposite corners have dark border
            const oppX = right - 1;
            const oppY = down - 1;
            const hasOppH = isDark(oppX, down - 1) || isDark(x + Math.floor(runLen / 2), down - 1);
            const hasOppV = isDark(right - 1, oppY) || isDark(right - 1, y + Math.floor(vRunLen / 2));

            // Interior fill check: checkbox should be mostly empty
            let interiorDark = 0;
            const sampleCount = 9;
            for (let sy = 1; sy <= 3; sy++) {
              for (let sx = 1; sx <= 3; sx++) {
                const sampleX = x + Math.floor((runLen * sx) / 4);
                const sampleY = y + Math.floor((vRunLen * sy) / 4);
                if (isDark(sampleX, sampleY)) interiorDark++;
              }
            }

            const fillRatio = interiorDark / sampleCount;
            if (fillRatio < 0.45 && (hasOppH || hasOppV)) {
              candidates.push({
                id: `cand-rast-cb-${x}-${y}`,
                type: "checkbox",
                pageIndex,
                pdfRect: toPdfRect(x, y, runLen, vRunLen),
                confidence: fillRatio < 0.25 ? "HIGH" : "MEDIUM",
                source: "raster",
              });

              // Mark region visited
              for (let vy = y; vy < down; vy += stride) {
                for (let vx = x; vx < right; vx += stride) {
                  visited[vy * imgW + vx] = 1;
                }
              }
              continue;
            }
          }
        }

        // Check for horizontal blank text entry line
        if (wPt >= opt.minTextRegionWidth && wPt <= pageWidth * 0.85 && hPt <= 6) {
          candidates.push({
            id: `cand-rast-txt-${x}-${y}`,
            type: "text-region",
            pageIndex,
            pdfRect: toPdfRect(x, y - Math.floor(14 / scaleY), runLen, Math.floor(18 / scaleY)),
            confidence: "MEDIUM",
            source: "raster",
          });
        }
      }
    }
  }

  return candidates;
}

/**
 * Computes an auto-centered mark position and size for placing a check, cross, or radio inside a candidate.
 */
export function computeAutoCenteredMark(
  candidateRect: [number, number, number, number],
  markType: "check" | "cross" | "radio" | "text",
): {
  x: number;
  y: number;
  width: number;
  height: number;
  fontSize: number;
} {
  const [x1, y1, x2, y2] = [
    Math.min(candidateRect[0], candidateRect[2]),
    Math.min(candidateRect[1], candidateRect[3]),
    Math.max(candidateRect[0], candidateRect[2]),
    Math.max(candidateRect[1], candidateRect[3]),
  ];

  const candW = x2 - x1;
  const candH = y2 - y1;
  const centerX = x1 + candW / 2;
  const centerY = y1 + candH / 2;

  if (markType === "check" || markType === "cross") {
    const markSize = Math.max(10, Math.min(candW, candH) * 0.85);
    const fontSize = Math.round(markSize);
    return {
      x: centerX - markSize / 2,
      y: centerY - markSize / 2,
      width: markSize,
      height: markSize,
      fontSize,
    };
  }

  if (markType === "radio") {
    const markSize = Math.max(6, Math.min(candW, candH) * 0.6);
    return {
      x: centerX - markSize / 2,
      y: centerY - markSize / 2,
      width: markSize,
      height: markSize,
      fontSize: Math.round(markSize),
    };
  }

  // Text Region default
  const fontSize = 12;
  return {
    x: x1 + 2,
    y: y1 + Math.max(2, (candH - fontSize) / 2),
    width: Math.max(40, candW - 4),
    height: Math.max(16, candH),
    fontSize,
  };
}
