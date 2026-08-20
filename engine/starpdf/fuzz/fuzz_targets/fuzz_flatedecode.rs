#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::filter::limits::DecompressLimits;
use starpdf::filter::FlateDecoder;

fuzz_target!(|data: &[u8]| {
    let limits = DecompressLimits::default();
    let _ = FlateDecoder::decode(data, &limits);
});
