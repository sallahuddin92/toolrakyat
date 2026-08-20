#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::filter::{PredictorDecoder, PredictorParams};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let params = PredictorParams {
        predictor: (data[0] % 16) as i32,
        columns: (data[1] as usize % 64) + 1,
        colors: (data[2] as usize % 4) + 1,
        bits_per_component: 8,
    };
    let _ = PredictorDecoder::decode(&data[3..], &params);
});
