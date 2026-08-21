import type { PdfRect, PixelRect } from "./types";

export interface PageDimensions {
  width: number;
  height: number;
  rotation?: number;
}

/**
 * Converts a PDF user-space bounding box (origin bottom-left in standard PDF)
 * to screen/canvas CSS pixel bounding box (origin top-left).
 * Correctly accounts for page dimensions, zoom scale, and page rotation (0, 90, 180, 270 deg).
 */
export function convertPdfRectToPixels(
  rect: PdfRect,
  page: PageDimensions,
  scale: number,
  rotation = 0,
): PixelRect {
  const rot = ((rotation % 360) + 360) % 360;
  let left = 0;
  let top = 0;
  let w = rect.width * scale;
  let h = rect.height * scale;

  if (rot === 0) {
    left = rect.x * scale;
    top = (page.height - (rect.y + rect.height)) * scale;
  } else if (rot === 90) {
    left = rect.y * scale;
    top = rect.x * scale;
    w = rect.height * scale;
    h = rect.width * scale;
  } else if (rot === 180) {
    left = (page.width - (rect.x + rect.width)) * scale;
    top = rect.y * scale;
  } else if (rot === 270) {
    left = (page.height - (rect.y + rect.height)) * scale;
    top = (page.width - (rect.x + rect.width)) * scale;
    w = rect.height * scale;
    h = rect.width * scale;
  }

  return {
    left: Math.max(0, left),
    top: Math.max(0, top),
    width: Math.max(4, w),
    height: Math.max(4, h),
  };
}

/**
 * Checks if a point (in PDF or pixel coordinates) is contained within a rectangle.
 */
export function isPointInRect(
  x: number,
  y: number,
  rect: { x: number; y: number; width: number; height: number },
): boolean {
  return (
    x >= rect.x &&
    x <= rect.x + rect.width &&
    y >= rect.y &&
    y <= rect.y + rect.height
  );
}
