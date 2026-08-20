/**
 * StarPDF Web Worker Bridge
 * Runs StarPDF WASM engine off the main browser thread.
 */

/* eslint-disable no-restricted-globals */
import initWasm, {
  starpdf_close,
  starpdf_create_minimal_pdf,
  starpdf_extract_all_text,
  starpdf_extract_page_text,
  starpdf_get_info,
  starpdf_get_page_count,
  starpdf_open,
  starpdf_search,
  starpdf_validate,
  starpdf_version,
} from "/starpdf_wasm/starpdf.js";

let initialized = false;

async function ensureInit(wasmUrl = "/starpdf_wasm/starpdf_bg.wasm") {
  if (!initialized) {
    await initWasm({ module_or_path: wasmUrl });
    initialized = true;
  }
}

self.onmessage = async (event) => {
  const req = event.data;
  if (!req || !req.type || !req.id) return;

  try {
    await ensureInit(req.wasmUrl);

    switch (req.type) {
      case "init": {
        const version = starpdf_version();
        self.postMessage({ type: "init", id: req.id, success: true, version });
        break;
      }
      case "open": {
        const bytes = new Uint8Array(req.buffer);
        const handle = starpdf_open(bytes);
        self.postMessage({ type: "open", id: req.id, success: true, handle });
        break;
      }
      case "info": {
        const info = starpdf_get_info(req.handle);
        self.postMessage({ type: "info", id: req.id, success: true, info });
        break;
      }
      case "pageCount": {
        const pageCount = starpdf_get_page_count(req.handle);
        self.postMessage({ type: "pageCount", id: req.id, success: true, pageCount });
        break;
      }
      case "extractPage": {
        const pageText = starpdf_extract_page_text(req.handle, req.pageIndex);
        self.postMessage({ type: "extractPage", id: req.id, success: true, pageText });
        break;
      }
      case "extractAll": {
        const pages = starpdf_extract_all_text(req.handle);
        self.postMessage({ type: "extractAll", id: req.id, success: true, pages });
        break;
      }
      case "search": {
        const results = starpdf_search(req.handle, req.query, req.caseSensitive);
        self.postMessage({ type: "search", id: req.id, success: true, results });
        break;
      }
      case "validate": {
        const isValid = starpdf_validate(req.handle);
        self.postMessage({ type: "validate", id: req.id, success: true, isValid });
        break;
      }
      case "close": {
        starpdf_close(req.handle);
        self.postMessage({ type: "close", id: req.id, success: true });
        break;
      }
      case "createMinimal": {
        const bytes = starpdf_create_minimal_pdf(req.text);
        self.postMessage({ type: "createMinimal", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      default:
        self.postMessage({
          type: "error",
          id: req.id,
          success: false,
          error: `Unknown request type: ${req.type}`,
        });
    }
  } catch (err) {
    self.postMessage({
      type: "error",
      id: req.id,
      success: false,
      error: err instanceof Error ? err.message : String(err),
    });
  }
};
