#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::font::Font;

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or_default() % 6;
    let payload = data.get(1..).unwrap_or_default();
    let (key, subtype) = match selector {
        0 => ("FontFile", None),
        1 => ("FontFile2", None),
        2 => ("FontFile3", Some("Type1C")),
        3 => ("FontFile3", Some("CIDFontType0C")),
        4 => ("FontFile3", Some("OpenType")),
        _ => ("FontFile3", None),
    };
    let _ = Font::detect_font_program(key, subtype, payload);
});
