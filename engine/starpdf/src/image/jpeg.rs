use crate::error::{PdfError, PdfResult};

/// Extracted metadata from a JPEG stream header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegInfo {
    pub width: u32,
    pub height: u32,
    pub components: u8,
    pub bits_per_component: u8,
    pub color_space: String,
}

/// Parses JPEG SOI/SOF markers to determine width, height, and color space safely.
pub fn parse_jpeg_info(data: &[u8]) -> PdfResult<JpegInfo> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(PdfError::UnsupportedImageFormat(
            "Data does not start with valid JPEG SOI marker (0xFFD8)".to_string(),
        ));
    }

    let mut offset = 2;
    while offset + 4 <= data.len() {
        if data[offset] != 0xFF {
            // Skip non-marker fill bytes
            offset += 1;
            continue;
        }

        let marker = data[offset + 1];
        offset += 2;

        // Skip standalone markers without length (RST0..RST7, SOI, EOI, TEM)
        if (0xD0..=0xD7).contains(&marker) || marker == 0xD8 || marker == 0xD9 || marker == 0x01 {
            continue;
        }

        if offset + 2 > data.len() {
            break;
        }

        let length = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        if length < 2 || offset + length > data.len() {
            return Err(PdfError::UnsupportedImageFormat(
                "Malformed JPEG marker segment length".to_string(),
            ));
        }

        // Check for Start of Frame markers: SOF0 (0xC0), SOF1 (0xC1), SOF2 (0xC2), SOF3 (0xC3),
        // SOF5..SOF7 (0xC5..0xC7), SOF9..SOF11 (0xC9..0xCB), SOF13..SOF15 (0xCD..0xCF)
        let is_sof = matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        );

        if is_sof {
            let sof_offset = offset + 2; // skip length bytes
            if sof_offset + 6 > data.len() {
                return Err(PdfError::UnsupportedImageFormat(
                    "Truncated JPEG SOF segment".to_string(),
                ));
            }

            let bits_per_component = data[sof_offset];
            let height = u16::from_be_bytes([data[sof_offset + 1], data[sof_offset + 2]]) as u32;
            let width = u16::from_be_bytes([data[sof_offset + 3], data[sof_offset + 4]]) as u32;
            let components = data[sof_offset + 5];

            let color_space = match components {
                1 => "DeviceGray".to_string(),
                3 => "DeviceRGB".to_string(),
                4 => "DeviceCMYK".to_string(),
                other => {
                    return Err(PdfError::UnsupportedImageFormat(format!(
                        "Unsupported number of JPEG color components: {other}"
                    )));
                }
            };

            return Ok(JpegInfo {
                width,
                height,
                components,
                bits_per_component,
                color_space,
            });
        }

        offset += length;
    }

    Err(PdfError::UnsupportedImageFormat(
        "No SOF marker found in JPEG stream".to_string(),
    ))
}
