use starpdf::appearance::color::PdfColor;
use starpdf::appearance::da_parser::DefaultAppearance;
use starpdf::appearance::generator::AppearanceGenerator;
use starpdf::appearance::status::AppearanceStatus;
use starpdf::appearance::text_field::TextFieldAppearance;
use starpdf::forms::field::FieldType;
use starpdf::syntax::object::PdfObject;

#[test]
fn test_default_appearance_parser() {
    // 1. Grayscale + font
    let da = DefaultAppearance::parse("/Helv 14 Tf 0.2 g").unwrap();
    assert_eq!(da.font_name, "Helv");
    assert_eq!(da.font_size, 14.0);
    assert_eq!(da.color, PdfColor::Grayscale(0.2));

    // 2. RGB + font
    let da_rgb = DefaultAppearance::parse("0 0.5 1 rg /Helvetica-Bold 18.5 Tf").unwrap();
    assert_eq!(da_rgb.font_name, "Helvetica-Bold");
    assert_eq!(da_rgb.font_size, 18.5);
    assert_eq!(da_rgb.color, PdfColor::Rgb(0.0, 0.5, 1.0));

    // 3. CMYK + font
    let da_cmyk = DefaultAppearance::parse("/ZaDb 10 Tf 0.1 0.2 0.3 0.4 k").unwrap();
    assert_eq!(da_cmyk.font_name, "ZaDb");
    assert_eq!(da_cmyk.font_size, 10.0);
    assert_eq!(da_cmyk.color, PdfColor::Cmyk(0.1, 0.2, 0.3, 0.4));
}

#[test]
fn test_text_field_appearance_alignment_and_clipping() {
    let rect = [50.0, 50.0, 250.0, 80.0];
    let da = DefaultAppearance::parse("/Helv 12 Tf 0 g").unwrap();

    // 1. Left alignment (Q = 0)
    let left_stream =
        TextFieldAppearance::generate_stream(rect, "Left Aligned", &da, 0, false).unwrap();
    let left_data = String::from_utf8_lossy(&left_stream.data);
    assert!(left_data.contains("/Tx BMC"));
    assert!(left_data.contains("W\nn")); // Clipping path
    assert!(left_data.contains("(Left Aligned) Tj"));

    // 2. Center alignment (Q = 1)
    let center_stream =
        TextFieldAppearance::generate_stream(rect, "Centered Text", &da, 1, false).unwrap();
    let center_data = String::from_utf8_lossy(&center_stream.data);
    assert!(center_data.contains("(Centered Text) Tj"));

    // 3. Right alignment (Q = 2)
    let right_stream =
        TextFieldAppearance::generate_stream(rect, "Right Aligned", &da, 2, false).unwrap();
    let right_data = String::from_utf8_lossy(&right_stream.data);
    assert!(right_data.contains("(Right Aligned) Tj"));

    // 4. Multiline text
    let multi_stream =
        TextFieldAppearance::generate_stream(rect, "Line One\nLine Two\nLine Three", &da, 0, true)
            .unwrap();
    let multi_data = String::from_utf8_lossy(&multi_stream.data);
    assert!(multi_data.contains("(Line One) Tj"));
    assert!(multi_data.contains("(Line Two) Tj"));
    assert!(multi_data.contains("(Line Three) Tj"));
}

#[test]
fn test_appearance_generator_checkbox_and_radio() {
    let rect = [10.0, 10.0, 30.0, 30.0];
    let da = DefaultAppearance::default();

    // 1. Checkbox
    let (check_ap, status) = AppearanceGenerator::generate_widget_ap(
        &FieldType::Checkbox,
        rect,
        "Yes",
        &da,
        0,
        Some("Yes"),
        true,
    )
    .unwrap();

    assert_eq!(status, AppearanceStatus::AppearanceRegenerated);
    let dict = check_ap.as_dict().unwrap();
    let n_dict = dict.get("N").unwrap().as_dict().unwrap();
    assert!(n_dict.contains_key("Off"));
    assert!(n_dict.contains_key("Yes"));

    // 2. Radio Button
    let (radio_ap, radio_status) = AppearanceGenerator::generate_widget_ap(
        &FieldType::RadioButtonGroup,
        rect,
        "Male",
        &da,
        0,
        Some("Male"),
        true,
    )
    .unwrap();

    assert_eq!(radio_status, AppearanceStatus::AppearanceRegenerated);
    let r_dict = radio_ap.as_dict().unwrap();
    let r_n = r_dict.get("N").unwrap().as_dict().unwrap();
    assert!(r_n.contains_key("Off"));
    assert!(r_n.contains_key("Male"));
}

#[test]
fn test_choice_appearance_uses_visible_value_and_supported_status_model() {
    let da = DefaultAppearance::parse("/TiRo 13 Tf 0 0 1 rg").unwrap();
    let (choice_ap, choice_status) = AppearanceGenerator::generate_widget_ap(
        &FieldType::Choice {
            combo: false,
            multi_select: true,
        },
        [10.0, 10.0, 210.0, 60.0],
        "Visible list selection",
        &da,
        1,
        None,
        false,
    )
    .unwrap();
    assert_eq!(choice_status.as_str(), "APPEARANCE_REGENERATED");
    let stream = choice_ap
        .as_dict()
        .and_then(|dict| dict.get("N"))
        .and_then(PdfObject::as_stream)
        .unwrap();
    let content = String::from_utf8_lossy(&stream.data);
    assert!(content.contains("(Visible list selection) Tj"));
    assert!(content.contains("/TiRo 13.00 Tf"));

    let (_, unsupported) = AppearanceGenerator::generate_widget_ap(
        &FieldType::Signature,
        [10.0, 10.0, 210.0, 60.0],
        "",
        &da,
        0,
        None,
        false,
    )
    .unwrap();
    assert_eq!(unsupported.as_str(), "APPEARANCE_UNSUPPORTED");
    assert_eq!(AppearanceStatus::ValueUpdated.as_str(), "VALUE_UPDATED");
    assert_eq!(AppearanceStatus::StateUpdated.as_str(), "STATE_UPDATED");
    assert_eq!(
        AppearanceStatus::AppearancePreserved.as_str(),
        "APPEARANCE_PRESERVED"
    );
}
