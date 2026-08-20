#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::search::SearchOptions;
#[cfg(feature = "wasm")]
use crate::validate::StructuralValidator;
#[cfg(feature = "wasm")]
use crate::wasm::dto::{
    WasmDocumentInfo, WasmPageText, WasmSearchBoundingBox, WasmSearchResult, WasmTextSpan,
};
#[cfg(feature = "wasm")]
use crate::wasm::registry::REGISTRY;
#[cfg(feature = "wasm")]
use crate::writer::MinimalWriter;

#[cfg(feature = "wasm")]
fn to_js_error<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_version() -> String {
    "0.5.0".to_string()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_open(bytes: &[u8]) -> Result<u32, JsValue> {
    REGISTRY.insert(bytes.to_vec()).map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_info(handle: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let page_count = doc.page_count()?;
            let is_valid = StructuralValidator::validate(doc).is_ok();
            let info = WasmDocumentInfo {
                page_count,
                pdf_version: "1.7".to_string(),
                is_valid,
            };
            serde_wasm_bindgen::to_value(&info)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_get_page_count(handle: u32) -> Result<u32, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let count = doc.page_count()?;
            Ok(count as u32)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_extract_page_text(handle: u32, page_index: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let page_text = doc.extract_page_text(page_index as usize)?;
            let spans = page_text
                .spans
                .iter()
                .map(|s| WasmTextSpan {
                    page_index: s.page_index,
                    text: s.text.clone(),
                    x: s.x,
                    y: s.y,
                    width: s.width,
                    height: s.height,
                    rotation: s.rotation,
                    font_name: s.font_name.clone(),
                    font_size: s.font_size,
                    confidence: s.confidence,
                })
                .collect();

            let result = WasmPageText {
                page_index: page_text.page_index,
                plain_text: page_text.plain_text(),
                spans,
            };

            serde_wasm_bindgen::to_value(&result)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_extract_all_text(handle: u32) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let pages = doc.extract_all_text()?;
            let wasm_pages: Vec<WasmPageText> = pages
                .into_iter()
                .map(|p| {
                    let plain = p.plain_text();
                    let spans = p
                        .spans
                        .into_iter()
                        .map(|s| WasmTextSpan {
                            page_index: s.page_index,
                            text: s.text,
                            x: s.x,
                            y: s.y,
                            width: s.width,
                            height: s.height,
                            rotation: s.rotation,
                            font_name: s.font_name,
                            font_size: s.font_size,
                            confidence: s.confidence,
                        })
                        .collect();

                    WasmPageText {
                        page_index: p.page_index,
                        plain_text: plain,
                        spans,
                    }
                })
                .collect();

            serde_wasm_bindgen::to_value(&wasm_pages)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_search(handle: u32, query: &str, case_sensitive: bool) -> Result<JsValue, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            let options = SearchOptions { case_sensitive };
            let hits = doc.search(query, &options)?;
            let wasm_hits: Vec<WasmSearchResult> = hits
                .into_iter()
                .map(|h| {
                    let boxes = h
                        .boxes
                        .into_iter()
                        .map(|b| WasmSearchBoundingBox {
                            page_index: b.page_index,
                            x: b.x,
                            y: b.y,
                            width: b.width,
                            height: b.height,
                            rotation: b.rotation,
                        })
                        .collect();

                    WasmSearchResult {
                        page_index: h.page_index,
                        matched_text: h.matched_text,
                        start_span_index: h.start_span_index,
                        end_span_index: h.end_span_index,
                        boxes,
                        confidence: h.confidence,
                    }
                })
                .collect();

            serde_wasm_bindgen::to_value(&wasm_hits)
                .map_err(|e| crate::error::PdfError::InvalidOperation(e.to_string()))
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_validate(handle: u32) -> Result<bool, JsValue> {
    REGISTRY
        .with_doc(handle, |doc| {
            StructuralValidator::validate(doc)?;
            Ok(true)
        })
        .map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_close(handle: u32) -> Result<bool, JsValue> {
    REGISTRY.close(handle).map_err(to_js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn starpdf_create_minimal_pdf(text: &str) -> Result<Vec<u8>, JsValue> {
    MinimalWriter::create_minimal_pdf(text).map_err(to_js_error)
}
