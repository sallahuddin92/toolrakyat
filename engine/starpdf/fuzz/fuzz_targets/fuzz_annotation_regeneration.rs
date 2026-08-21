#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::annotation::{AnnotationGenerator, AnnotationSpec, LineEndingStyle};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    let points = [
        f64::from(data[0]),
        f64::from(data[1]),
        f64::from(data[2]) + 1.0,
        f64::from(data[3]) + 1.0,
    ];
    let ending = match data[4] % 6 {
        0 => LineEndingStyle::None,
        1 => LineEndingStyle::Square,
        2 => LineEndingStyle::Circle,
        3 => LineEndingStyle::Diamond,
        4 => LineEndingStyle::OpenArrow,
        _ => LineEndingStyle::ClosedArrow,
    };
    let specs = [
        AnnotationSpec::Line {
            line_points: points,
            stroke_color: Some(vec![f64::from(data[5]) / 255.0]),
            fill_color: Some(vec![f64::from(data[6]) / 255.0]),
            stroke_width: Some(f64::from(data[7]).clamp(0.1, 20.0)),
            line_endings: [ending, LineEndingStyle::None],
            contents: Some(String::from_utf8_lossy(&data[8..]).to_string()),
        },
        AnnotationSpec::Highlight {
            rect: [0.0, 0.0, 10.0, 10.0],
            quad_points: data.iter().map(|byte| f64::from(*byte)).collect(),
            color: Some(vec![1.0, 1.0, 0.0]),
        },
    ];
    for spec in specs {
        if let Ok((dictionary, _)) = AnnotationGenerator::generate_annotation_objects(&spec) {
            let _ = AnnotationGenerator::regenerate_from_dictionary(&dictionary);
        }
    }
});
