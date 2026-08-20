use crate::error::{PdfError, PdfResult};
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::filter::predictor::{PredictorDecoder, PredictorParams};
use crate::syntax::object::StreamObject;
use crate::xref::table::{XrefEntry, XrefTable};

pub struct XrefStreamParser;

impl XrefStreamParser {
    pub fn parse_into_table(
        stream: &StreamObject,
        table: &mut XrefTable,
        limits: &DecompressLimits,
    ) -> PdfResult<()> {
        let dict = &stream.dict;

        // 1. Validate /Type /XRef (or implicit for xref streams)
        if let Some(t) = dict.get("Type").and_then(|v| v.as_name()) {
            if t != "XRef" {
                return Err(PdfError::InvalidXref(format!(
                    "Expected /Type /XRef for xref stream, found /{t}"
                )));
            }
        }

        // 2. Read /W [w0, w1, w2]
        let w_arr = dict
            .get("W")
            .and_then(|v| v.as_array())
            .ok_or_else(|| PdfError::InvalidXref("XRef stream missing /W array".into()))?;

        if w_arr.len() != 3 {
            return Err(PdfError::InvalidXref(format!(
                "XRef stream /W array must have 3 elements, found {}",
                w_arr.len()
            )));
        }

        let w0 = w_arr[0]
            .as_i64()
            .ok_or_else(|| PdfError::InvalidXref("Invalid /W[0] width".into()))?
            as usize;
        let w1 = w_arr[1]
            .as_i64()
            .ok_or_else(|| PdfError::InvalidXref("Invalid /W[1] width".into()))?
            as usize;
        let w2 = w_arr[2]
            .as_i64()
            .ok_or_else(|| PdfError::InvalidXref("Invalid /W[2] width".into()))?
            as usize;

        let entry_len = w0 + w1 + w2;
        if entry_len == 0 || entry_len > 32 {
            return Err(PdfError::InvalidXref(format!(
                "Invalid XRef stream entry width: {entry_len} bytes"
            )));
        }

        // 3. Read /Size
        let size = dict
            .get("Size")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| PdfError::InvalidXref("XRef stream missing /Size".into()))?
            as usize;

        if size > limits.max_xref_entries {
            return Err(PdfError::InvalidXref(format!(
                "XRef stream /Size {size} exceeds security limit {}",
                limits.max_xref_entries
            )));
        }

        // 4. Read /Index [[start, count], ...] or default [0, Size]
        let mut index_ranges = Vec::new();
        if let Some(idx_arr) = dict.get("Index").and_then(|v| v.as_array()) {
            if idx_arr.len() % 2 != 0 {
                return Err(PdfError::InvalidXref(
                    "XRef stream /Index array must contain an even number of integers".into(),
                ));
            }
            for chunk in idx_arr.chunks(2) {
                let start = chunk[0]
                    .as_i64()
                    .ok_or_else(|| PdfError::InvalidXref("Invalid /Index start object".into()))?
                    as u64;
                let count = chunk[1]
                    .as_i64()
                    .ok_or_else(|| PdfError::InvalidXref("Invalid /Index entry count".into()))?
                    as usize;
                index_ranges.push((start, count));
            }
        } else {
            index_ranges.push((0, size));
        }

        // 5. Decompress stream data if encoded with /FlateDecode
        let mut decoded_data = stream.data.clone();
        if let Some(filter) = dict.get("Filter").and_then(|v| v.as_name()) {
            if filter == "FlateDecode" {
                decoded_data = FlateDecoder::decode(&stream.data, limits)?;
            }
        }

        // 6. Apply PNG/TIFF Predictor if specified in /DecodeParms
        if let Some(parms) = dict.get("DecodeParms").and_then(|v| v.as_dict()) {
            let predictor = parms.get("Predictor").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

            if predictor > 1 {
                let columns = parms
                    .get("Columns")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(entry_len as i64) as usize;

                let predictor_params = PredictorParams {
                    predictor,
                    columns,
                    colors: 1,
                    bits_per_component: 8,
                };
                decoded_data = PredictorDecoder::decode(&decoded_data, &predictor_params)?;
            }
        }

        // 7. Parse XRef stream entries
        let mut offset = 0;
        for (start_obj, count) in index_ranges {
            for i in 0..count {
                let obj_num = start_obj.checked_add(i as u64).ok_or_else(|| {
                    PdfError::InvalidXref("Object number overflow in xref stream".into())
                })?;

                if offset + entry_len > decoded_data.len() {
                    return Err(PdfError::InvalidXref(format!(
                        "Unexpected end of decoded xref stream at byte offset {offset}"
                    )));
                }

                let type_val = if w0 > 0 {
                    read_big_endian_uint(&decoded_data[offset..offset + w0])
                } else {
                    1 // Default entry type is 1 (in use) when w0 is 0
                };
                offset += w0;

                let field2 = if w1 > 0 {
                    read_big_endian_uint(&decoded_data[offset..offset + w1])
                } else {
                    0
                };
                offset += w1;

                let field3 = if w2 > 0 {
                    read_big_endian_uint(&decoded_data[offset..offset + w2])
                } else {
                    0
                };
                offset += w2;

                match type_val {
                    0 => {
                        // Type 0: Free entry (next_free_obj, generation)
                        table.entries.entry(obj_num).or_insert(XrefEntry::Free {
                            next_free_obj: field2,
                            generation: field3 as u16,
                        });
                    }
                    1 => {
                        // Type 1: In-use uncompressed entry (byte_offset, generation)
                        table.entries.entry(obj_num).or_insert(XrefEntry::InUse {
                            byte_offset: field2,
                            generation: field3 as u16,
                        });
                    }
                    2 => {
                        // Type 2: Compressed entry in object stream (stream_obj_num, index_in_stream)
                        table
                            .entries
                            .entry(obj_num)
                            .or_insert(XrefEntry::Compressed {
                                stream_obj_num: field2,
                                index_in_stream: field3 as u32,
                            });
                    }
                    other => {
                        return Err(PdfError::InvalidXref(format!(
                            "Invalid XRef stream entry type {other} for object {obj_num}"
                        )));
                    }
                }
            }
        }

        // 8. Merge trailer entries from xref stream dictionary into table.trailer
        for (k, v) in dict {
            if k != "Type"
                && k != "Length"
                && k != "Filter"
                && k != "DecodeParms"
                && k != "W"
                && k != "Index"
            {
                table.trailer.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }

        Ok(())
    }
}

#[inline]
fn read_big_endian_uint(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for &b in bytes {
        val = (val << 8) | (b as u64);
    }
    val
}
