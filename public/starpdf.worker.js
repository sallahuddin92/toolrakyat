/**
 * StarPDF Web Worker Bridge
 * Runs StarPDF WASM engine off the main browser thread.
 */

/* eslint-disable no-restricted-globals */
import initWasm, {
  starpdf_add_annotation,
  starpdf_close,
  starpdf_create_minimal_pdf,
  starpdf_delete_page,
  starpdf_duplicate_page,
  starpdf_export_incremental,
  starpdf_extract_all_text,
  starpdf_extract_page_text,
  starpdf_extract_pages,
  starpdf_get_annotations,
  starpdf_get_appearance_status,
  starpdf_get_glyph_mapping_quality,
  starpdf_get_form_fields,
  starpdf_get_info,
  starpdf_get_page_count,
  starpdf_get_security_info,
  starpdf_open,
  starpdf_insert_blank_page,
  starpdf_insert_imported_page,
  starpdf_merge_documents,
  starpdf_merge_selected,
  starpdf_move_page,
  starpdf_remove_annotation,
  starpdf_replace_text,
  starpdf_get_text_editability,
  starpdf_search,
  starpdf_split_document,
  starpdf_set_checkbox,
  starpdf_set_choice,
  starpdf_set_choice_values,
  starpdf_set_radio,
  starpdf_set_text_field,
  starpdf_update_annotation,
  starpdf_validate,
  starpdf_version,
} from "/starpdf_wasm/starpdf.js";

let initialized = false;

function classifyError(err) {
  const message = err instanceof Error ? err.message : String(err);
  if (/handle/i.test(message)) return { code: "INVALID_HANDLE", message };
  if (/limit|maximum|exceed/i.test(message)) return { code: "RESOURCE_LIMIT", message };
  if (/ENCRYPTED_DOCUMENT/i.test(message)) return { code: "ENCRYPTED_DOCUMENT", message };
  if (/SIGNATURE|SIGNED_/i.test(message)) return { code: "SIGNED_DOCUMENT", message };
  if (/PARTIAL_FIELD|UNSUPPORTED_PAGE|EXCLUDED_PAGE/i.test(message)) {
    return { code: "UNSUPPORTED", message };
  }
  if (/unsupported/i.test(message)) return { code: "UNSUPPORTED", message };
  if (/parse|syntax|header|xref|PDF/i.test(message)) return { code: "INVALID_PDF", message };
  return { code: "ENGINE_ERROR", message };
}

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
      case "securityInfo": {
        const securityInfo = starpdf_get_security_info(req.handle);
        self.postMessage({ type: "securityInfo", id: req.id, success: true, securityInfo });
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
      case "getFormFields": {
        const fields = starpdf_get_form_fields(req.handle);
        self.postMessage({ type: "getFormFields", id: req.id, success: true, fields });
        break;
      }
      case "getAnnotations": {
        const annotations = starpdf_get_annotations(req.handle, req.pageIndex);
        self.postMessage({ type: "getAnnotations", id: req.id, success: true, annotations });
        break;
      }
      case "setTextField": {
        starpdf_set_text_field(req.handle, BigInt(req.objectNum), req.objectGen, req.value);
        self.postMessage({ type: "setTextField", id: req.id, success: true });
        break;
      }
      case "setCheckbox": {
        starpdf_set_checkbox(req.handle, BigInt(req.objectNum), req.objectGen, req.checked);
        self.postMessage({ type: "setCheckbox", id: req.id, success: true });
        break;
      }
      case "setRadio": {
        starpdf_set_radio(
          req.handle,
          BigInt(req.parentNum),
          req.parentGen,
          BigInt(req.widgetNum),
          req.widgetGen,
          req.onState
        );
        self.postMessage({ type: "setRadio", id: req.id, success: true });
        break;
      }
      case "setChoice": {
        starpdf_set_choice(req.handle, BigInt(req.objectNum), req.objectGen, req.value);
        self.postMessage({ type: "setChoice", id: req.id, success: true });
        break;
      }
      case "setChoiceValues": {
        starpdf_set_choice_values(req.handle, BigInt(req.objectNum), req.objectGen, req.values);
        self.postMessage({ type: "setChoiceValues", id: req.id, success: true });
        break;
      }
      case "addAnnotation": {
        starpdf_add_annotation(req.handle, req.pageIndex, req.input);
        self.postMessage({ type: "addAnnotation", id: req.id, success: true });
        break;
      }
      case "updateAnnotation": {
        starpdf_update_annotation(req.handle, BigInt(req.objectNum), req.objectGen, req.input);
        self.postMessage({ type: "updateAnnotation", id: req.id, success: true });
        break;
      }
      case "removeAnnotation": {
        starpdf_remove_annotation(req.handle, req.pageIndex, BigInt(req.objectNum), req.objectGen);
        self.postMessage({ type: "removeAnnotation", id: req.id, success: true });
        break;
      }
      case "replaceText": {
        const result = starpdf_replace_text(req.handle, req.pageIndex, req.spanId, req.newText);
        self.postMessage({ type: "replaceText", id: req.id, success: true, result });
        break;
      }
      case "getTextEditability": {
        const span = starpdf_get_text_editability(req.handle, req.pageIndex, req.spanId);
        self.postMessage({ type: "getTextEditability", id: req.id, success: true, span });
        break;
      }
      case "exportIncremental": {
        const bytes = starpdf_export_incremental(req.handle);
        self.postMessage({ type: "exportIncremental", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "deletePage": {
        const bytes = starpdf_delete_page(req.handle, req.pageIndex);
        self.postMessage({ type: "deletePage", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "movePage": {
        const bytes = starpdf_move_page(req.handle, req.fromIndex, req.toIndex);
        self.postMessage({ type: "movePage", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "duplicatePage": {
        const bytes = starpdf_duplicate_page(req.handle, req.pageIndex, req.destinationIndex);
        self.postMessage({ type: "duplicatePage", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "insertBlankPage": {
        const bytes = starpdf_insert_blank_page(
          req.handle,
          req.pageIndex,
          req.width,
          req.height,
          req.rotation
        );
        self.postMessage({ type: "insertBlankPage", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "extractPages": {
        const bytes = starpdf_extract_pages(req.handle, req.pageIndices);
        self.postMessage({ type: "extractPages", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "insertImportedPage": {
        const bytes = starpdf_insert_imported_page(
          req.handle,
          new Uint8Array(req.buffer),
          req.importedPageIndex,
          req.destinationIndex
        );
        self.postMessage({ type: "insertImportedPage", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "mergeDocuments": {
        const documents = req.buffers.map((buffer) => new Uint8Array(buffer));
        const bytes = req.pageSources
          ? starpdf_merge_selected(
              documents,
              req.pageSources.map((source) => [source.documentIndex, source.pageIndex])
            )
          : starpdf_merge_documents(documents);
        self.postMessage({ type: "mergeDocuments", id: req.id, success: true, bytes }, [bytes.buffer]);
        break;
      }
      case "splitDocument": {
        const outputs = starpdf_split_document(
          req.handle,
          req.ranges.map((range) => [range.start, range.endExclusive])
        ).map((output) => new Uint8Array(output));
        self.postMessage(
          { type: "splitDocument", id: req.id, success: true, outputs },
          outputs.map((output) => output.buffer)
        );
        break;
      }
      case "getAppearanceStatus": {
        const status = starpdf_get_appearance_status(req.handle);
        self.postMessage({ type: "getAppearanceStatus", id: req.id, success: true, status });
        break;
      }
      case "getGlyphMappingQuality": {
        const quality = starpdf_get_glyph_mapping_quality(req.handle);
        self.postMessage({ type: "getGlyphMappingQuality", id: req.id, success: true, quality });
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
    const failure = classifyError(err);
    self.postMessage({
      type: "error",
      id: req.id,
      success: false,
      error: failure.message,
      code: failure.code,
    });
  }
};
