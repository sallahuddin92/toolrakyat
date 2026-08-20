#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::{PageRange, PageSource, PdfDocument};

fuzz_target!(|data: &[u8]| {
    if data.len() < 6 {
        return;
    }
    let mode = data[0] % 4;
    let payload = &data[5..];
    let requested = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
    match mode {
        0 | 2 => {
            if payload.len() < 2 {
                return;
            }
            let split = requested.min(payload.len());
            if split == 0 || split == payload.len() {
                return;
            }
            let first = &payload[..split];
            let second = &payload[split..];
            let output = if mode == 0 {
                PdfDocument::merge_documents(&[first, second])
            } else {
                PdfDocument::merge_selected(
                    &[first, second],
                    &[
                        PageSource::new(1, 0),
                        PageSource::new(0, 0),
                        PageSource::new(1, 0),
                    ],
                )
            };
            if let Ok(bytes) = output {
                let _ = PdfDocument::from_bytes(&bytes);
            }
        }
        1 => {
            if let Ok(mut document) = PdfDocument::from_bytes(payload) {
                let page_count = document.page_count().unwrap_or(0);
                if page_count > 0 {
                    let end = requested.saturating_add(1).min(page_count);
                    if end > 0 {
                        if let Ok(outputs) = document.split_document(&[PageRange::new(0, end)]) {
                            for output in outputs {
                                let _ = PdfDocument::from_bytes(&output);
                            }
                        }
                    }
                }
            }
        }
        _ => {
            let split = requested.min(payload.len());
            if split == 0 || split == payload.len() {
                return;
            }
            let first = &payload[..split];
            let second = &payload[split..];
            if let (Ok(mut primary), Ok(imported)) = (
                PdfDocument::from_bytes(first),
                PdfDocument::from_bytes(second),
            ) {
                if let Ok(bytes) = primary.insert_page_from(&imported, 0, 0) {
                    let _ = PdfDocument::from_bytes(&bytes);
                }
            }
        }
    }
});
