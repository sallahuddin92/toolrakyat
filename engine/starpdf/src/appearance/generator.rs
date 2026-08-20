use std::collections::BTreeMap;

use crate::appearance::checkbox::CheckboxAppearance;
use crate::appearance::choice::ChoiceAppearance;
use crate::appearance::da_parser::DefaultAppearance;
use crate::appearance::radio::RadioAppearance;
use crate::appearance::status::AppearanceStatus;
use crate::appearance::text_field::TextFieldAppearance;
use crate::error::PdfResult;
use crate::forms::field::FieldType;
use crate::syntax::object::PdfObject;

pub struct AppearanceGenerator;

impl AppearanceGenerator {
    /// Generates the `/AP` dictionary entry for a form widget given its field type, value, and properties.
    pub fn generate_widget_ap(
        field_type: &FieldType,
        rect: [f64; 4],
        value_str: &str,
        da: &DefaultAppearance,
        quadding: i32,
        on_state_name: Option<&str>,
        _checked: bool,
    ) -> PdfResult<(PdfObject, AppearanceStatus)> {
        match field_type {
            FieldType::Text { multiline, .. } => {
                let stream = TextFieldAppearance::generate_stream(
                    rect, value_str, da, quadding, *multiline,
                )?;
                let ap_dict = BTreeMap::from([("N".to_string(), PdfObject::Stream(stream))]);
                Ok((
                    PdfObject::Dictionary(ap_dict),
                    AppearanceStatus::AppearanceRegenerated,
                ))
            }
            FieldType::Checkbox => {
                let on_name = on_state_name.unwrap_or("Yes");
                let off_stream = CheckboxAppearance::generate_off_stream(rect)?;
                let on_stream = CheckboxAppearance::generate_on_stream(rect)?;

                let mut n_dict = BTreeMap::new();
                n_dict.insert("Off".to_string(), PdfObject::Stream(off_stream));
                n_dict.insert(on_name.to_string(), PdfObject::Stream(on_stream));

                let ap_dict = BTreeMap::from([("N".to_string(), PdfObject::Dictionary(n_dict))]);
                Ok((
                    PdfObject::Dictionary(ap_dict),
                    AppearanceStatus::AppearanceRegenerated,
                ))
            }
            FieldType::RadioButtonGroup => {
                let on_name = on_state_name.unwrap_or("0");
                let off_stream = RadioAppearance::generate_off_stream(rect)?;
                let on_stream = RadioAppearance::generate_on_stream(rect)?;

                let mut n_dict = BTreeMap::new();
                n_dict.insert("Off".to_string(), PdfObject::Stream(off_stream));
                n_dict.insert(on_name.to_string(), PdfObject::Stream(on_stream));

                let ap_dict = BTreeMap::from([("N".to_string(), PdfObject::Dictionary(n_dict))]);
                Ok((
                    PdfObject::Dictionary(ap_dict),
                    AppearanceStatus::AppearanceRegenerated,
                ))
            }
            FieldType::Choice { .. } => {
                let stream = ChoiceAppearance::generate_stream(rect, value_str, da, quadding)?;
                let ap_dict = BTreeMap::from([("N".to_string(), PdfObject::Stream(stream))]);
                Ok((
                    PdfObject::Dictionary(ap_dict),
                    AppearanceStatus::AppearanceRegenerated,
                ))
            }
            _ => Ok((
                PdfObject::Dictionary(BTreeMap::new()),
                AppearanceStatus::AppearanceUnsupported,
            )),
        }
    }
}
