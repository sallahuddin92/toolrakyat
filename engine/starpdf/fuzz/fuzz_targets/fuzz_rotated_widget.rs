#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use starpdf::appearance::rotation::WidgetRotation;
use starpdf::syntax::object::StreamObject;

fuzz_target!(|data: &[u8]| {
    if data.len() < 40 {
        return;
    }
    let number = |offset: usize| {
        f64::from_bits(u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
        ]))
    };
    let degrees = i64::from_le_bytes([
        data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
    ]);
    let Ok(rotation) = WidgetRotation::from_degrees(degrees) else {
        return;
    };
    let mut stream = StreamObject {
        dict: BTreeMap::new(),
        stream_offset: 0,
        stream_length: data.len(),
        data: data.to_vec(),
    };
    let _ = rotation.apply_to_stream(
        [number(0), number(8), number(16), number(24)],
        &mut stream,
    );
});
