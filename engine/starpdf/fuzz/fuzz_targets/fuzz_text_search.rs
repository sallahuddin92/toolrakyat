#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::search::{SearchOptions, TextMatcher};
use starpdf::text::{PageText, TextSpan};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let query_len = (data[0] as usize % 16) + 1;
    if data.len() <= query_len {
        return;
    }

    let query_str = String::from_utf8_lossy(&data[1..=query_len]);
    let body_str = String::from_utf8_lossy(&data[query_len + 1..]);

    let mut page = PageText::new(0);
    page.spans.push(TextSpan::new(
        0,
        body_str.into_owned(),
        50.0,
        700.0,
        200.0,
        12.0,
        0.0,
        "/F1".into(),
        12.0,
        1.0,
    ));

    let _ = TextMatcher::search_page(&page, &query_str, &SearchOptions::default());
});
