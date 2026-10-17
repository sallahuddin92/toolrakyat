use serde::{Deserialize, Serialize};
use starpdf::document::PdfDocument;

use crate::error::{AppCoreError, AppResult};

/// Supported export profiles for document serialization and content extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportProfile {
    StandardPdf,
    IncrementalPdf,
    CanonicalPdf,
    Text,
    Json,
}

impl ExportProfile {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::StandardPdf => "StandardPdf",
            Self::IncrementalPdf => "IncrementalPdf",
            Self::CanonicalPdf => "CanonicalPdf",
            Self::Text => "Text",
            Self::Json => "Json",
        }
    }

    pub const fn default_extension(&self) -> &'static str {
        match self {
            Self::StandardPdf | Self::IncrementalPdf | Self::CanonicalPdf => "pdf",
            Self::Text => "txt",
            Self::Json => "json",
        }
    }

    pub const fn mime_type(&self) -> &'static str {
        match self {
            Self::StandardPdf | Self::IncrementalPdf | Self::CanonicalPdf => "application/pdf",
            Self::Text => "text/plain; charset=utf-8",
            Self::Json => "application/json",
        }
    }
}

/// Dispatches an export request according to the specified `ExportProfile`.
pub fn export_document(bytes: &[u8], profile: ExportProfile) -> AppResult<Vec<u8>> {
    match profile {
        ExportProfile::StandardPdf
        | ExportProfile::IncrementalPdf
        | ExportProfile::CanonicalPdf => {
            // Validate that bytes form a valid PDF document
            let _ = PdfDocument::from_bytes(bytes).map_err(AppCoreError::from)?;
            Ok(bytes.to_vec())
        }
        ExportProfile::Text => {
            let mut doc = PdfDocument::from_bytes(bytes).map_err(AppCoreError::from)?;
            let pages_text = doc.extract_all_text().map_err(AppCoreError::from)?;
            let mut buffer = String::new();
            for (i, page) in pages_text.iter().enumerate() {
                if i > 0 {
                    buffer.push_str("\n\n");
                }
                buffer.push_str(&format!("--- Page {} ---\n", page.page_index + 1));
                for span in &page.spans {
                    buffer.push_str(&span.text);
                    buffer.push(' ');
                }
            }
            Ok(buffer.into_bytes())
        }
        ExportProfile::Json => {
            let mut doc = PdfDocument::from_bytes(bytes).map_err(AppCoreError::from)?;
            let page_count = doc.page_count().map_err(AppCoreError::from)?;
            let version = doc.version().to_string();
            let sec_info = doc.security_info().ok();
            let pages_text = doc.extract_all_text().unwrap_or_default();

            let mut json = String::new();
            json.push('{');
            json.push_str(&format!(
                r#""format":"starpdf_document","version":"{}","page_count":{},"security":{{"is_encrypted":{},"mutation_allowed":{}}},"pages":["#,
                escape_json(&version),
                page_count,
                sec_info.as_ref().map_or(false, |s| s.encryption_state != starpdf::security::EncryptionState::NotEncrypted),
                sec_info.as_ref().map_or(true, |s| s.mutation_allowed),
            ));

            for (p_idx, page) in pages_text.iter().enumerate() {
                if p_idx > 0 {
                    json.push(',');
                }
                json.push_str(&format!(r#"{{"page_index":{},"spans":["#, page.page_index));
                for (s_idx, span) in page.spans.iter().enumerate() {
                    if s_idx > 0 {
                        json.push(',');
                    }
                    json.push_str(&format!(
                        r#"{{"id":"{}","text":"{}","bounds":{{"x":{},"y":{},"w":{},"h":{}}}}}"#,
                        escape_json(&span.span_id),
                        escape_json(&span.text),
                        span.x,
                        span.y,
                        span.width,
                        span.height,
                    ));
                }
                json.push_str("]}");
            }
            json.push_str("]}");

            Ok(json.into_bytes())
        }
    }
}

fn escape_json(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => escaped.push_str(r#"\""#),
            '\\' => escaped.push_str(r"\\"),
            '\n' => escaped.push_str(r"\n"),
            '\r' => escaped.push_str(r"\r"),
            '\t' => escaped.push_str(r"\t"),
            c if c.is_control() => escaped.push_str(&format!(r"\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}
