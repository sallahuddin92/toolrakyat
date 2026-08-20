#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::font::UnicodeCMap;

fuzz_target!(|data: &[u8]| {
    let _ = UnicodeCMap::parse(data);
});
