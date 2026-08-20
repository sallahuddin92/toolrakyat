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

        let w0 = usize::try_from(
            w_arr[0]
                .as_i64()
                .ok_or_else(|| PdfError::InvalidXref("Invalid /W[0] width".into()))?,
        )
        .map_err(|_| PdfError::InvalidXref("Negative /W[0] width".into()))?;
        let w1 = usize::try_from(
            w_arr[1]
                .as_i64()
                .ok_or_else(|| PdfError::InvalidXref("Invalid /W[1] width".into()))?,
        )
        .map_err(|_| PdfError::InvalidXref("Negative /W[1] width".into()))?;
        let w2 = usize::try_from(
            w_arr[2]
                .as_i64()
                .ok_or_else(|| PdfError::InvalidXref("Invalid /W[2] width".into()))?,
        )
        .map_err(|_| PdfError::InvalidXref("Negative /W[2] width".into()))?;

        let entry_len = w0
            .checked_add(w1)
            .and_then(|width| width.checked_add(w2))
            .ok_or_else(|| PdfError::InvalidXref("XRef stream entry width overflow".into()))?;
        if entry_len == 0 || entry_len > 32 || w0 > 8 || w1 > 8 || w2 > 8 {
            return Err(PdfError::InvalidXref(format!(
                "Invalid XRef stream field widths [{w0}, {w1}, {w2}] ({entry_len} bytes total)"
            )));
        }

        // 3. Read /Size
        let size = usize::try_from(
            dict.get("Size")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| PdfError::InvalidXref("XRef stream missing /Size".into()))?,
        )
        .map_err(|_| PdfError::InvalidXref("Negative XRef stream /Size".into()))?;

        if size > limits.max_xref_entries {
            return Err(PdfError::InvalidXref(format!(
                "XRef stream /Size {size} exceeds security limit {}",
                limits.max_xref_entries
            )));
        }

        // 4. Read /Index [[start, count], ...] or default [0, Size]
        let mut index_ranges = Vec::new();
        let mut indexed_entries = 0usize;
        if let Some(idx_arr) = dict.get("Index").and_then(|v| v.as_array()) {
            if idx_arr.len() % 2 != 0 {
                return Err(PdfError::InvalidXref(
                    "XRef stream /Index array must contain an even number of integers".into(),
                ));
            }
            for chunk in idx_arr.chunks(2) {
                let start =
                    u64::try_from(chunk[0].as_i64().ok_or_else(|| {
                        PdfError::InvalidXref("Invalid /Index start object".into())
                    })?)
                    .map_err(|_| PdfError::InvalidXref("Negative /Index start object".into()))?;
                let count =
                    usize::try_from(chunk[1].as_i64().ok_or_else(|| {
                        PdfError::InvalidXref("Invalid /Index entry count".into())
                    })?)
                    .map_err(|_| PdfError::InvalidXref("Negative /Index entry count".into()))?;
                if count > limits.max_xref_entries {
                    return Err(PdfError::InvalidXref(format!(
                        "XRef stream /Index count {count} exceeds security limit {}",
                        limits.max_xref_entries
                    )));
                }
                indexed_entries = indexed_entries.checked_add(count).ok_or_else(|| {
                    PdfError::InvalidXref("XRef stream /Index total count overflow".into())
                })?;
                if indexed_entries > limits.max_xref_entries {
                    return Err(PdfError::InvalidXref(format!(
                        "XRef stream /Index total {indexed_entries} exceeds security limit {}",
                        limits.max_xref_entries
                    )));
                }
                let end = start.checked_add(count as u64).ok_or_else(|| {
                    PdfError::InvalidXref("XRef stream /Index object range overflow".into())
                })?;
                if end > size as u64 {
                    return Err(PdfError::InvalidXref(format!(
                        "XRef stream /Index range {start}..{end} exceeds /Size {size}"
                    )));
                }
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
            let predictor =
                i32::try_from(parms.get("Predictor").and_then(|v| v.as_i64()).unwrap_or(1))
                    .map_err(|_| PdfError::InvalidXref("XRef predictor exceeds i32".into()))?;

            if predictor > 1 {
                let columns = usize::try_from(
                    parms
                        .get("Columns")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(entry_len as i64),
                )
                .map_err(|_| PdfError::InvalidXref("Negative XRef predictor columns".into()))?;

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
        let mut offset = 0usize;
        for (start_obj, count) in index_ranges {
            for i in 0..count {
                let obj_num = start_obj.checked_add(i as u64).ok_or_else(|| {
                    PdfError::InvalidXref("Object number overflow in xref stream".into())
                })?;

                if offset
                    .checked_add(entry_len)
                    .is_none_or(|end| end > decoded_data.len())
                {
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
                            generation: u16::try_from(field3).map_err(|_| {
                                PdfError::InvalidXref(format!(
                                    "Free entry generation {field3} exceeds u16"
                                ))
                            })?,
                        });
                    }
                    1 => {
                        // Type 1: In-use uncompressed entry (byte_offset, generation)
                        table.entries.entry(obj_num).or_insert(XrefEntry::InUse {
                            byte_offset: field2,
                            generation: u16::try_from(field3).map_err(|_| {
                                PdfError::InvalidXref(format!(
                                    "In-use entry generation {field3} exceeds u16"
                                ))
                            })?,
                        });
                    }
                    2 => {
                        // Type 2: Compressed entry in object stream (stream_obj_num, index_in_stream)
                        table
                            .entries
                            .entry(obj_num)
                            .or_insert(XrefEntry::Compressed {
                                stream_obj_num: field2,
                                index_in_stream: u32::try_from(field3).map_err(|_| {
                                    PdfError::InvalidXref(format!(
                                        "Object stream index {field3} exceeds u32"
                                    ))
                                })?,
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
