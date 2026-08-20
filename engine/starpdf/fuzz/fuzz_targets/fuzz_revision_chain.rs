#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::io::source::ByteSource;
use starpdf::xref::XrefResolver;

fuzz_target!(|data: &[u8]| {
    let _ = XrefResolver::load_xref_and_trailer(ByteSource::new(data));
});
