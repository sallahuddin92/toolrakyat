#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::font::subset::TrueTypeSubsetter;

fuzz_target!(|data: &[u8]| {
    let split = data.len().min(32);
    let glyphs: Vec<u16> = data[..split]
        .chunks(2)
        .map(|chunk| if chunk.len() == 2 { u16::from_be_bytes([chunk[0], chunk[1]]) } else { u16::from(chunk[0]) })
        .collect();
    let _ = TrueTypeSubsetter::subset(&data[split..], &glyphs);
});
