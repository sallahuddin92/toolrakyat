use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starpdf::execution::{RenderNodeId, StarNodeId};

/// Two-dimensional bounding box in PDF point space (72 points = 1 inch).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

/// Font metadata associated with a text selection or editing session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontInfo {
    pub name: String,
    pub size: f64,
    pub weight: Option<String>,
    pub style: Option<String>,
}

impl FontInfo {
    pub fn new(name: impl Into<String>, size: f64) -> Self {
        Self {
            name: name.into(),
            size,
            weight: None,
            style: None,
        }
    }
}

/// Serde helper for `StarNodeId`.
pub mod star_node_id_serde {
    use super::*;

    pub fn serialize<S>(val: &StarNodeId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(val.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StarNodeId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        StarNodeId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Serde helper for `Option<RenderNodeId>`.
pub mod option_render_node_id_serde {
    use super::*;

    pub fn serialize<S>(val: &Option<RenderNodeId>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match val {
            Some(v) => serializer.serialize_some(v.as_str()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<RenderNodeId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => RenderNodeId::from_str(&s)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// Authoritative selection state across all selectable PDF entity types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum SelectionState {
    #[default]
    None,

    Text {
        #[serde(with = "star_node_id_serde")]
        star_node_id: StarNodeId,
        #[serde(with = "option_render_node_id_serde")]
        render_node_id: Option<RenderNodeId>,
        text: String,
        bounds: Rect,
        font_info: Option<FontInfo>,
        is_editable: bool,
    },

    Image {
        #[serde(with = "star_node_id_serde")]
        star_node_id: StarNodeId,
        #[serde(with = "option_render_node_id_serde")]
        render_node_id: Option<RenderNodeId>,
        image_ref: String,
        bounds: Rect,
        is_shared: bool,
    },

    Vector {
        #[serde(with = "star_node_id_serde")]
        star_node_id: StarNodeId,
        #[serde(with = "option_render_node_id_serde")]
        render_node_id: Option<RenderNodeId>,
        bounds: Rect,
        stroke: Option<String>,
        fill: Option<String>,
    },

    Annotation {
        annot_ref: String,
        subtype: String,
        contents: Option<String>,
        bounds: Rect,
    },

    Page {
        page_index: u32,
    },
}

impl SelectionState {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Text { .. } => "Text",
            Self::Image { .. } => "Image",
            Self::Vector { .. } => "Vector",
            Self::Annotation { .. } => "Annotation",
            Self::Page { .. } => "Page",
        }
    }

    pub fn star_node_id(&self) -> Option<&StarNodeId> {
        match self {
            Self::Text { star_node_id, .. }
            | Self::Image { star_node_id, .. }
            | Self::Vector { star_node_id, .. } => Some(star_node_id),
            _ => None,
        }
    }

    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Self::Text { bounds, .. }
            | Self::Image { bounds, .. }
            | Self::Vector { bounds, .. }
            | Self::Annotation { bounds, .. } => Some(*bounds),
            _ => None,
        }
    }
}
