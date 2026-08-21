#![no_main]

use libfuzzer_sys::fuzz_target;
use starpdf::image::{
    parse_jpeg_info, AddImageSpec, ImageFormat, RemoveImageSpec, ReplaceImageSpec,
};
use starpdf::mutation::PdfChange;
use starpdf::PdfDocument;

fuzz_target!(|data: &[u8]| {
    if data.len() < 12 {
        return;
    }

    // Branch 1: Fuzz direct JPEG parsing
    let _ = parse_jpeg_info(data);

    // Branch 2: Fuzz Image ID parsing and extraction on arbitrary documents
    let page_idx = data[0] as usize;
    let stream_idx = data[1] as usize;
    let instr_idx = u16::from_le_bytes([data[2], data[3]]) as usize;
    let img_id = format!("img_p{page_idx}_s{stream_idx}_i{instr_idx}");

    let format_choice = match data[4] % 3 {
        0 => ImageFormat::AutoDetect,
        1 => ImageFormat::Jpeg,
        _ => ImageFormat::Flate {
            color_space: if data[5] % 2 == 0 {
                "DeviceRGB"
            } else {
                "DeviceGray"
            }
            .to_string(),
            width: (data[6] as u32).max(1),
            height: (data[7] as u32).max(1),
            bits_per_component: 8,
        },
    };

    let payload = &data[8..];

    if let Ok(mut doc) = PdfDocument::from_bytes(data) {
        // Enumerate images
        if let Ok(images) = doc.enumerate_images(0) {
            for img in images.iter().take(3) {
                // Try replace
                let replace_spec = ReplaceImageSpec {
                    page_index: 0,
                    image_id: img.image_id.clone(),
                    new_image_bytes: payload.to_vec(),
                    format: format_choice.clone(),
                    clone_if_shared: data[5] % 2 == 0,
                };
                if let Ok(plan) = doc.replace_image(&replace_spec) {
                    if let Ok(exported) = doc.export_incremental(&plan) {
                        let _ = PdfDocument::from_bytes(&exported);
                    }
                }

                // Try remove
                let remove_spec = RemoveImageSpec {
                    page_index: 0,
                    image_id: img.image_id.clone(),
                };
                if let Ok(plan) = doc.remove_image(&remove_spec) {
                    if let Ok(exported) = doc.export_incremental(&plan) {
                        let _ = PdfDocument::from_bytes(&exported);
                    }
                }
            }
        }

        // Try add image
        let add_spec = AddImageSpec {
            page_index: 0,
            image_bytes: payload.to_vec(),
            format: format_choice,
            x: (data[8] as f64) * 2.0,
            y: (data[9] as f64) * 2.0,
            width: (data[10] as f64).max(10.0),
            height: (data[11] as f64).max(10.0),
        };
        if let Ok(plan) = doc.add_image(&add_spec) {
            if let Ok(exported) = doc.export_incremental(&plan) {
                let _ = PdfDocument::from_bytes(&exported);
            }
        }

        // Fuzz mutation engine direct change
        let change = PdfChange::RemoveImage {
            spec: RemoveImageSpec {
                page_index: 0,
                image_id: img_id,
            },
        };
        let _ = doc.apply_mutation(&[change]);
    }
});
