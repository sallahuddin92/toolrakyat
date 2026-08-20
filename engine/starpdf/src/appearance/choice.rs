use crate::appearance::da_parser::DefaultAppearance;
use crate::appearance::text_field::TextFieldAppearance;
use crate::error::PdfResult;
use crate::syntax::object::StreamObject;

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
}
