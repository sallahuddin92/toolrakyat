#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::content::ContentParser;

fuzz_target!(|data: &[u8]| {
    let mut parser = ContentParser::from_bytes(data);
    let _ = parser.parse_instructions();
});
