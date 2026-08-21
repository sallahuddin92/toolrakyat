#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::appearance::text_field::TextFieldAppearance;
use starpdf::appearance::DefaultAppearance;

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let x1 = f64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let y1 = f64::from_le_bytes([
        data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
    ]);
    let x2 = f64::from_le_bytes([
        data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
    ]);
    let y2 = f64::from_le_bytes([
        data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
    ]);

    let rect = [x1, y1, x2, y2];
    let text = String::from_utf8_lossy(&data[32..]);

    let da = DefaultAppearance::default();
    let _ = TextFieldAppearance::generate_stream(rect, &text, &da, 0, false);
    let _ = TextFieldAppearance::generate_stream(rect, &text, &da, 1, true);
    let _ = TextFieldAppearance::generate_stream(rect, &text, &da, 2, false);
});
