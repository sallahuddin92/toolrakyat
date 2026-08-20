#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::syntax::Parser;

fuzz_target!(|data: &[u8]| {
    let mut parser = Parser::from_bytes(data);
    let _ = parser.parse_object();
    let mut parser2 = Parser::from_bytes(data);
    let _ = parser2.parse_indirect_object();
});
