use serde::{Deserialize, Serialize};

use crate::selection::{FontInfo, Rect};

/// Manages an in-place draft text editing session on a selectable text node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEditingSession {
    pub target_node: String,
    pub page_index: u32,
    pub original_text: String,
    pub draft_text: String,
    pub font_info: Option<FontInfo>,
    pub bounds: Option<Rect>,
    pub is_active: bool,
}

impl TextEditingSession {
    pub fn new(
        target_node: impl Into<String>,
        page_index: u32,
        original_text: impl Into<String>,
        font_info: Option<FontInfo>,
        bounds: Option<Rect>,
    ) -> Self {
        let orig = original_text.into();
        Self {
            target_node: target_node.into(),
            page_index,
            draft_text: orig.clone(),
            original_text: orig,
            font_info,
            bounds,
            is_active: true,
        }
    }

    pub fn update_draft(&mut self, text: impl Into<String>) {
        self.draft_text = text.into();
    }

    pub fn is_dirty(&self) -> bool {
        self.draft_text != self.original_text
    }

    pub fn commit(&mut self) -> (String, String) {
        self.is_active = false;
        (self.target_node.clone(), self.draft_text.clone())
    }

    pub fn cancel(&mut self) {
        self.draft_text = self.original_text.clone();
        self.is_active = false;
    }

    pub fn target_node(&self) -> &str {
        &self.target_node
    }

    pub fn page_index(&self) -> u32 {
        self.page_index
    }

    pub fn original_text(&self) -> &str {
        &self.original_text
    }

    pub fn draft_text(&self) -> &str {
        &self.draft_text
    }

    pub fn font_info(&self) -> Option<&FontInfo> {
        self.font_info.as_ref()
    }

    pub fn bounds(&self) -> Option<Rect> {
        self.bounds
    }
}
