#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::syntax::Lexer;

fuzz_target!(|data: &[u8]| {
    let mut lexer = Lexer::from_bytes(data);
    while let Ok(Some(_)) = lexer.next_token() {
        // Safe iteration, no panic allowed
    }
});
