#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::mutation::ContentStreamEditor;
use starpdf::mutation::PdfChange;
use starpdf::mutation::TextEditTarget;
use starpdf::PdfDocument;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    // Branch 1: Fuzz direct content stream operand mutation & span ID parsing
    let page_idx = data[0] as usize;
    let stream_idx = data[1] as usize;
    let instr_idx = u16::from_le_bytes([data[2], data[3]]) as usize;
    let op_idx = data[4] as usize;
    let payload = &data[8..];

    let span_id_str = format!("p{page_idx}_s{stream_idx}_i{instr_idx}_o{op_idx}");
    let _ = TextEditTarget::from_span_id(&span_id_str);

    let fuzz_span_str = String::from_utf8_lossy(&data[..data.len().min(64)]);
    let _ = TextEditTarget::from_span_id(&fuzz_span_str);

    let target = TextEditTarget::new(page_idx, stream_idx, instr_idx, op_idx);

    let replacement_bytes = &data[5..data.len().min(32)];
    let _ = ContentStreamEditor::replace_in_stream(payload, &target, replacement_bytes);

    // Branch 2: Fuzz full PdfDocument existing text replacement on arbitrary or valid PDF data
    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        let replacement = String::from_utf8_lossy(payload);

        // Try targeting extracted spans
        if let Ok(page_text) = doc.extract_page_text(0) {
            for span in page_text.spans.iter().take(3) {
                if let Ok(plan) = doc.replace_text_span(0, &span.span_id, &replacement) {
                    if let Ok(exported) = doc.export_incremental(&plan) {
                        let _ = PdfDocument::from_bytes(&exported);
                    }
                }
            }
        }

        // Try direct mutation change
        let change = PdfChange::ReplaceText {
            page_index: page_idx,
            target,
            replacement: replacement.to_string(),
        };
        let _ = doc.apply_mutation(&[change]);
    }
});
