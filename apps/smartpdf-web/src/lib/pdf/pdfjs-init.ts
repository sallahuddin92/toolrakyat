import type { PDFDocumentProxy, RenderTask } from "pdfjs-dist";

// Polyfill DOMMatrix for safe module evaluation in SSR environments
if (typeof globalThis !== "undefined" && typeof (globalThis as unknown as { DOMMatrix?: unknown }).DOMMatrix === "undefined") {
  (globalThis as unknown as { DOMMatrix: unknown }).DOMMatrix = class DOMMatrix {
    a = 1;
    b = 0;
    c = 0;
    d = 1;
    e = 0;
    f = 0;
    constructor() {}
  };
}

let pdfjsLibCache: typeof import("pdfjs-dist") | null = null;

export async function getPdfjsLib(): Promise<typeof import("pdfjs-dist")> {
  if (pdfjsLibCache) return pdfjsLibCache;

  if (typeof globalThis !== "undefined" && typeof (globalThis as unknown as { DOMMatrix?: unknown }).DOMMatrix === "undefined") {
    (globalThis as unknown as { DOMMatrix: unknown }).DOMMatrix = class DOMMatrix {
      a = 1;
      b = 0;
      c = 0;
      d = 1;
      e = 0;
      f = 0;
      constructor() {}
    };
  }

  // The unminified PDF.js bundle reuses Webpack's __webpack_exports__ name,
  // which collides with Next's eval wrapper in Webpack development builds.
  const lib = await import("pdfjs-dist/build/pdf.min.mjs");
  if (typeof window !== "undefined") {
    lib.GlobalWorkerOptions.workerSrc = "/pdf.worker.min.mjs";
  }

  pdfjsLibCache = lib;
  return lib;
}

export type { PDFDocumentProxy, RenderTask };
