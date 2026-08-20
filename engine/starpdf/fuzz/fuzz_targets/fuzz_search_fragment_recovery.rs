#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::search::SearchOptions;
use starpdf::text::{PageText, TextSpan};

fuzz_target!(|data: &[u8]| {
    let mut page = PageText::new(0);
    for (index, chunk) in data.chunks(16).take(256).enumerate() {
        if chunk.len() < 16 {
            break;
        }
        page.spans.push(TextSpan::new(
            0,
            String::from_utf8_lossy(&chunk[8..]).into_owned(),
            f64::from(i16::from_le_bytes([chunk[0], chunk[1]])),
            f64::from(i16::from_le_bytes([chunk[2], chunk[3]])),
            f64::from(u16::from_le_bytes([chunk[4], chunk[5]])),
            f64::from(chunk[6].max(1)),
            f64::from(chunk[7] % 4) * 90.0,
            format!("F{index}"),
            f64::from(chunk[6].max(1)),
            1.0,
        ));
    }
    let query = String::from_utf8_lossy(data.get(..32).unwrap_or(data));
    let _ = page.search(&query, &SearchOptions::default());
});
