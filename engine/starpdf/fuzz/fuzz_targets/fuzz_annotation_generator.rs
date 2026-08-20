#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::annotation::{AnnotationGenerator, AnnotationSpec};

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let x1 = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
    let y1 = f64::from_le_bytes([data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15]]);
    let x2 = f64::from_le_bytes([data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23]]);
    let y2 = f64::from_le_bytes([data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31]]);

    let rect = [x1, y1, x2, y2];
    let contents = String::from_utf8_lossy(&data[32..]).to_string();

    let specs = [
        AnnotationSpec::FreeText {
            rect,
            text: contents.clone(),
            font_size: Some(12.0),
            color: Some(vec![0.0, 0.0, 0.0]),
        },
        AnnotationSpec::Square {
            rect,
            stroke_color: Some(vec![1.0, 0.0, 0.0]),
            fill_color: Some(vec![0.9, 0.9, 0.9]),
            border_width: Some(2.0),
        },
        AnnotationSpec::Circle {
            rect,
            stroke_color: Some(vec![0.0, 1.0, 0.0]),
            fill_color: None,
            border_width: Some(1.5),
        },
        AnnotationSpec::Line {
            line_points: rect,
            stroke_color: Some(vec![0.0, 0.0, 1.0]),
            fill_color: None,
            stroke_width: Some(1.0),
            line_endings: Default::default(),
            contents: None,
        },
        AnnotationSpec::Highlight {
            rect,
            quad_points: vec![rect[0], rect[1], rect[2], rect[1], rect[0], rect[3], rect[2], rect[3]],
            color: Some(vec![1.0, 1.0, 0.0]),
        },
        AnnotationSpec::Underline {
            rect,
            quad_points: vec![rect[0], rect[3], rect[2], rect[3], rect[0], rect[1], rect[2], rect[1]],
            color: Some(vec![0.0, 0.0, 1.0]),
        },
        AnnotationSpec::StrikeOut {
            rect,
            quad_points: vec![rect[0], rect[3], rect[2], rect[3], rect[0], rect[1], rect[2], rect[1]],
            color: Some(vec![1.0, 0.0, 0.0]),
        },
        AnnotationSpec::Ink {
            rect,
            ink_list: vec![vec![[rect[0], rect[1]], [rect[2], rect[3]]]],
            stroke_color: Some(vec![0.0]),
            stroke_width: Some(1.0),
        },
        AnnotationSpec::Link {
            rect,
            uri: contents,
        },
    ];

    for spec in &specs {
        let _ = AnnotationGenerator::generate_annotation_objects(spec);
    }
});
