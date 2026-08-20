use crate::appearance::da_parser::DefaultAppearance;
use std::collections::BTreeMap;

use crate::appearance::fonts::FontMetricsHelper;
use crate::appearance::text_field::{TextFieldAppearance, MAX_APPEARANCE_BYTES};
use crate::error::{PdfError, PdfResult};
use crate::font::appearance::AppearanceFont;
use crate::syntax::object::{PdfObject, StreamObject};

pub const MAX_LIST_OPTIONS: usize = 5_000;
pub const MAX_MULTI_SELECT_INDEXES: usize = 1_000;

pub struct ChoiceAppearance;

impl ChoiceAppearance {
    /// Generates the normal appearance Form XObject stream for a choice field (combobox/listbox).
    pub fn generate_stream(
        rect: [f64; 4],
        value: &str,
        da: &DefaultAppearance,
        quadding: i32,
    ) -> PdfResult<StreamObject> {
        TextFieldAppearance::generate_stream(rect, value, da, quadding, false)
    }

    pub fn generate_list_stream(
        rect: [f64; 4],
        options: &[String],
        selected_indexes: &[usize],
        top_index: usize,
        da: &DefaultAppearance,
    ) -> PdfResult<StreamObject> {
        Self::generate_list_stream_with_font(rect, options, selected_indexes, top_index, da, None)
    }

    pub fn generate_list_stream_with_font(
        rect: [f64; 4],
        options: &[String],
        selected_indexes: &[usize],
        top_index: usize,
        da: &DefaultAppearance,
        resolved_font: Option<&AppearanceFont>,
    ) -> PdfResult<StreamObject> {
        if options.len() > MAX_LIST_OPTIONS {
            return Err(PdfError::InvalidOperation(format!(
                "List box options exceed maximum of {MAX_LIST_OPTIONS}"
            )));
        }
        if selected_indexes.len() > MAX_MULTI_SELECT_INDEXES
            || selected_indexes.iter().any(|index| *index >= options.len())
        {
            return Err(PdfError::InvalidOperation(
                "List box selection indexes are invalid or exceed the configured limit".into(),
            ));
        }
        if top_index > options.len() {
            return Err(PdfError::InvalidOperation(
                "List box top index is outside the option range".into(),
            ));
        }
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Invalid list box widget dimensions".into(),
            ));
        }
        let font_size = if da.font_size > 0.0 {
            da.font_size
        } else {
            10.0
        };
        let line_height = font_size * 1.25;
        let visible_count = ((height / line_height).ceil() as usize).saturating_add(1);
        let end = top_index.saturating_add(visible_count).min(options.len());
        let font_key = resolved_font.map_or_else(
            || da.font_name.trim_start_matches('/'),
            |font| font.resource_name.as_str(),
        );
        let mut content = Vec::with_capacity(1024);
        content.extend_from_slice(
            format!(
                "/Tx BMC\nq\n0 0 {:.2} {:.2} re\nW\nn\n1 1 1 rg\n0 0 {:.2} {:.2} re\nf\n",
                width, height, width, height
            )
            .as_bytes(),
        );
        for (row, option_index) in (top_index..end).enumerate() {
            let y_bottom = height - (row as f64 + 1.0) * line_height;
            let selected = selected_indexes.binary_search(&option_index).is_ok();
            if selected {
                content.extend_from_slice(
                    format!(
                        "0.153 0.400 0.820 rg\n0 {:.2} {:.2} {:.2} re\nf\n",
                        y_bottom, width, line_height
                    )
                    .as_bytes(),
                );
            }
            let color = if selected {
                "1 1 1 rg".to_string()
            } else {
                da.color.to_fill_ops()
            };
            content.extend_from_slice(
                format!(
                    "{}\nBT\n/{} {:.2} Tf\n1 0 0 1 2 {:.2} Tm\n{} Tj\nET\n",
                    color,
                    font_key,
                    font_size,
                    y_bottom + (line_height - font_size) / 2.0 + font_size * 0.15,
                    TextFieldAppearance::text_operand(&options[option_index], resolved_font,)?
                )
                .as_bytes(),
            );
            if content.len() > MAX_APPEARANCE_BYTES {
                return Err(PdfError::InvalidOperation(
                    "Generated list box appearance exceeds maximum buffer limit".into(),
                ));
            }
        }
        content.extend_from_slice(b"Q\nEMC\n");
        let mut dict = BTreeMap::new();
        dict.insert("Type".to_string(), PdfObject::Name("XObject".to_string()));
        dict.insert("Subtype".to_string(), PdfObject::Name("Form".to_string()));
        dict.insert("FormType".to_string(), PdfObject::Integer(1));
        dict.insert(
            "BBox".to_string(),
            PdfObject::Array(vec![
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(width),
                PdfObject::Real(height),
            ]),
        );
        dict.insert(
            "Length".to_string(),
            PdfObject::Integer(content.len() as i64),
        );
        let resources = resolved_font.map_or_else(
            || FontMetricsHelper::build_font_resource(&da.font_name),
            AppearanceFont::resource_dictionary,
        );
        dict.insert("Resources".to_string(), PdfObject::Dictionary(resources));
        Ok(StreamObject {
            dict,
            stream_offset: 0,
            stream_length: content.len(),
            data: content,
        })
    }
}
