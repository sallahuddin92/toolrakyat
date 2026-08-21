#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::appearance::choice::ChoiceAppearance;
use starpdf::appearance::da_parser::DefaultAppearance;
use starpdf::appearance::text_field::{TextFieldAppearance, TextLayoutOptions};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    let text = String::from_utf8_lossy(&data[4..]);
    let rect = [
        0.0,
        0.0,
        f64::from(data[0]).max(1.0),
        f64::from(data[1]).max(1.0),
    ];
    let da = DefaultAppearance::default();
    let comb = usize::from(data[2]).min(64);
    let _ = TextFieldAppearance::generate_stream_with_options(
        rect,
        &text,
        &da,
        i32::from(data[3] % 3),
        TextLayoutOptions {
            multiline: data[3] & 0x80 != 0,
            comb_max_len: (comb > 0).then_some(comb),
        },
    );
    let options: Vec<String> = text.split('\n').take(32).map(str::to_string).collect();
    let selected = if options.is_empty() {
        Vec::new()
    } else {
        vec![0]
    };
    let _ = ChoiceAppearance::generate_list_stream(rect, &options, &selected, 0, &da);
});
