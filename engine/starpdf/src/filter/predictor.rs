use crate::error::{PdfError, PdfResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictorParams {
    pub predictor: i32,
    pub columns: usize,
    pub colors: usize,
    pub bits_per_component: usize,
}

impl Default for PredictorParams {
    fn default() -> Self {
        Self {
            predictor: 1,
            columns: 1,
            colors: 1,
            bits_per_component: 8,
        }
    }
}

pub struct PredictorDecoder;

impl PredictorDecoder {
    pub fn decode(input: &[u8], params: &PredictorParams) -> PdfResult<Vec<u8>> {
        if params.predictor <= 1 {
            return Ok(input.to_vec());
        }

        let columns = params.columns.max(1);
        let colors = params.colors.max(1);
        let bpc = params.bits_per_component.max(1);

        let bpp = (colors * bpc).div_ceil(8).max(1);
        let row_bytes = (columns * colors * bpc).div_ceil(8);

        if row_bytes == 0 {
            return Ok(input.to_vec());
        }

        if params.predictor == 2 {
            // TIFF Predictor 2
            return Ok(Self::decode_tiff(input, row_bytes, bpp));
        }

        if (10..=15).contains(&params.predictor) {
            // PNG Predictors
            return Self::decode_png(input, row_bytes, bpp, params.predictor);
        }

        Err(PdfError::MalformedInput(format!(
            "Unsupported predictor algorithm: {}",
            params.predictor
        )))
    }

    fn decode_tiff(input: &[u8], row_bytes: usize, bpp: usize) -> Vec<u8> {
        let mut output = Vec::with_capacity(input.len());
        for chunk in input.chunks(row_bytes) {
            let mut row = chunk.to_vec();
            for i in bpp..row.len() {
                row[i] = row[i].wrapping_add(row[i - bpp]);
            }
            output.extend_from_slice(&row);
        }
        output
    }

    fn decode_png(
        input: &[u8],
        row_bytes: usize,
        bpp: usize,
        predictor_code: i32,
    ) -> PdfResult<Vec<u8>> {
        let stride = row_bytes + 1; // 1 tag byte + row_bytes data
        let total_rows = input.len().div_ceil(stride);
        let mut output = Vec::with_capacity(total_rows * row_bytes);

        let mut prev_row = vec![0u8; row_bytes];
        let mut current_row = vec![0u8; row_bytes];

        for chunk in input.chunks(stride) {
            if chunk.is_empty() {
                break;
            }

            let filter_type = if predictor_code == 15 {
                chunk[0]
            } else {
                (predictor_code - 10) as u8
            };

            let data = if chunk.len() > 1 { &chunk[1..] } else { &[] };
            let copy_len = data.len().min(row_bytes);

            current_row.fill(0);
            current_row[..copy_len].copy_from_slice(&data[..copy_len]);

            match filter_type {
                0 => {
                    // None: raw bytes
                }
                1 => {
                    // Sub: Left
                    for i in 0..row_bytes {
                        let left = if i >= bpp { current_row[i - bpp] } else { 0 };
                        current_row[i] = current_row[i].wrapping_add(left);
                    }
                }
                2 => {
                    // Up: Above
                    for i in 0..row_bytes {
                        let above = prev_row[i];
                        current_row[i] = current_row[i].wrapping_add(above);
                    }
                }
                3 => {
                    // Average: floor((Left + Above) / 2)
                    for i in 0..row_bytes {
                        let left = if i >= bpp {
                            current_row[i - bpp] as u16
                        } else {
                            0
                        };
                        let above = prev_row[i] as u16;
                        let avg = u16::midpoint(left, above) as u8;
                        current_row[i] = current_row[i].wrapping_add(avg);
                    }
                }
                4 => {
                    // Paeth
                    for i in 0..row_bytes {
                        let a = if i >= bpp { current_row[i - bpp] } else { 0 };
                        let b = prev_row[i];
                        let c = if i >= bpp { prev_row[i - bpp] } else { 0 };
                        let p = paeth_predictor(a, b, c);
                        current_row[i] = current_row[i].wrapping_add(p);
                    }
                }
                other => {
                    return Err(PdfError::MalformedInput(format!(
                        "Invalid PNG predictor filter type {other}"
                    )));
                }
            }

            output.extend_from_slice(&current_row);
            prev_row.copy_from_slice(&current_row);
        }

        Ok(output)
    }
}

#[inline]
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a_i = i32::from(a);
    let b_i = i32::from(b);
    let c_i = i32::from(c);

    let p = a_i + b_i - c_i;
    let pa = (p - a_i).abs();
    let pb = (p - b_i).abs();
    let pc = (p - c_i).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}
