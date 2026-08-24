use std::collections::BTreeMap;
use std::io::Write;

use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};
use crate::writer::serializer::Serializer;
use crate::xref::table::{XrefEntry, XrefTable};

pub struct IncrementalWriter;

impl IncrementalWriter {
    pub const MAX_INCREMENTAL_OUTPUT_GROWTH: usize = 64 * 1024 * 1024;

    /// Appends modified and new indirect objects to the original PDF byte slice,
    /// generating a standard incremental update xref section and updated trailer.
    pub fn write_update(
        original_bytes: &[u8],
        modified_objects: &BTreeMap<ObjectRef, PdfObject>,
        prev_startxref: usize,
        original_trailer: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Vec<u8>> {
        Self::write_update_internal(
            original_bytes,
            modified_objects,
            prev_startxref,
            original_trailer,
            None,
        )
    }

    /// Writes a clean terminal xref table for a recovered document. Every effective
    /// uncompressed object is re-indexed and the malformed historical `/Prev` chain
    /// is deliberately omitted from the new trailer.
    pub fn write_recovered_update(
        original_bytes: &[u8],
        modified_objects: &BTreeMap<ObjectRef, PdfObject>,
        effective_xref: &XrefTable,
        original_trailer: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Vec<u8>> {
        if effective_xref
            .entries
            .values()
            .any(|entry| matches!(entry, XrefEntry::Compressed { .. }))
        {
            return Self::write_recovered_xref_stream_update(
                original_bytes,
                modified_objects,
                effective_xref,
                original_trailer,
            );
        }
        Self::write_update_internal(
            original_bytes,
            modified_objects,
            0,
            original_trailer,
            Some(effective_xref),
        )
    }

    fn write_update_internal(
        original_bytes: &[u8],
        modified_objects: &BTreeMap<ObjectRef, PdfObject>,
        prev_startxref: usize,
        original_trailer: &BTreeMap<String, PdfObject>,
        recovered_xref: Option<&XrefTable>,
    ) -> PdfResult<Vec<u8>> {
        if modified_objects.is_empty() {
            // Nothing to update, return original bytes unchanged
            return Ok(original_bytes.to_vec());
        }

        let mut output = Vec::with_capacity(original_bytes.len().saturating_add(4096));
        output.extend_from_slice(original_bytes);

        // Ensure newline separation
        if !output.ends_with(b"\n") && !output.ends_with(b"\r") {
            output.push(b'\n');
        }

        // 1. Serialize modified indirect objects and record their byte offsets
        let mut object_offsets: BTreeMap<ObjectRef, usize> = BTreeMap::new();
        let mut max_obj_num = 0u64;

        if let Some(xref) = recovered_xref {
            for (&obj_num, entry) in &xref.entries {
                match *entry {
                    XrefEntry::InUse {
                        byte_offset,
                        generation,
                    } => {
                        let offset = usize::try_from(byte_offset).map_err(|_| {
                            PdfError::InvalidOperation(
                                "Recovered xref object offset exceeds platform range".into(),
                            )
                        })?;
                        if offset >= original_bytes.len() {
                            return Err(PdfError::InvalidOperation(format!(
                                "Recovered xref object {obj_num} points outside the source"
                            )));
                        }
                        object_offsets.insert(ObjectRef::new(obj_num, generation), offset);
                        max_obj_num = max_obj_num.max(obj_num);
                    }
                    XrefEntry::Compressed { .. } => {
                        return Err(PdfError::RecoveredXrefExport(
                            "compressed entry reached the classic recovered writer".into(),
                        ));
                    }
                    XrefEntry::Free { .. } => {}
                }
            }
        }

        for (&obj_ref, obj) in modified_objects {
            if obj_ref.number > 9_999_999_999 {
                return Err(PdfError::InvalidOperation(
                    "PDF object number exceeds the incremental writer limit".into(),
                ));
            }
            max_obj_num = max_obj_num.max(obj_ref.number);
            let offset = output.len();
            object_offsets.insert(obj_ref, offset);

            if let Err(e) = writeln!(output, "{} {} obj", obj_ref.number, obj_ref.generation) {
                return Err(PdfError::InvalidOperation(format!(
                    "Failed to format object header: {e}"
                )));
            }

            if let Err(e) = Serializer::write_object(&mut output, obj) {
                return Err(PdfError::InvalidOperation(format!(
                    "Failed to serialize object {}: {e}",
                    obj_ref.number
                )));
            }

            if let Err(e) = writeln!(output, "\nendobj") {
                return Err(PdfError::InvalidOperation(format!(
                    "Failed to format endobj: {e}"
                )));
            }
            Self::validate_growth(original_bytes.len(), output.len())?;
        }

        // 2. Format xref section with contiguous subsection grouping
        let xref_start_offset = output.len();
        if let Err(e) = writeln!(output, "xref") {
            return Err(PdfError::InvalidOperation(format!(
                "Failed to write xref header: {e}"
            )));
        }

        let entries: Vec<(ObjectRef, usize)> = object_offsets.into_iter().collect();
        let mut i = 0;
        while i < entries.len() {
            let start_num = entries[i].0.number;
            let mut j = i + 1;
            while j < entries.len()
                && entries[j].0.number == entries[j - 1].0.number.saturating_add(1)
            {
                j += 1;
            }

            let subsection_count = j - i;
            if let Err(e) = writeln!(output, "{} {}", start_num, subsection_count) {
                return Err(PdfError::InvalidOperation(format!(
                    "Failed to write xref subsection header: {e}"
                )));
            }

            for k in i..j {
                let (o_ref, offset) = &entries[k];
                // PDF standard 20-byte xref line: 10-digit offset, space, 5-digit gen, space, 'n', space, cr, lf
                if let Err(e) = write!(output, "{:010} {:05} n \r\n", offset, o_ref.generation) {
                    return Err(PdfError::InvalidOperation(format!(
                        "Failed to write xref entry: {e}"
                    )));
                }
            }

            i = j;
        }

        // 3. Construct updated trailer dictionary
        let mut trailer_dict = original_trailer.clone();

        // Calculate updated /Size
        let orig_size = original_trailer
            .get("Size")
            .and_then(|v| v.as_integer())
            .map_or(0, |s| s.max(0) as u64);
        let new_size = orig_size.max(max_obj_num.saturating_add(1));
        trailer_dict.insert("Size".to_string(), PdfObject::Integer(new_size as i64));

        // Add /Prev pointing to original startxref
        if recovered_xref.is_none() && prev_startxref > 0 {
            trailer_dict.insert(
                "Prev".to_string(),
                PdfObject::Integer(prev_startxref as i64),
            );
        } else {
            trailer_dict.remove("Prev");
        }

        // Remove incompatible stream keys if any
        trailer_dict.remove("XRefStm");

        if let Err(e) = writeln!(output, "trailer") {
            return Err(PdfError::InvalidOperation(format!(
                "Failed to write trailer token: {e}"
            )));
        }

        if let Err(e) = Serializer::write_object(&mut output, &PdfObject::Dictionary(trailer_dict))
        {
            return Err(PdfError::InvalidOperation(format!(
                "Failed to serialize trailer dict: {e}"
            )));
        }

        if let Err(e) = writeln!(output, "\nstartxref\n{}\n%%EOF", xref_start_offset) {
            return Err(PdfError::InvalidOperation(format!(
                "Failed to write startxref: {e}"
            )));
        }

        Self::validate_growth(original_bytes.len(), output.len())?;

        Ok(output)
    }

    fn write_recovered_xref_stream_update(
        original_bytes: &[u8],
        modified_objects: &BTreeMap<ObjectRef, PdfObject>,
        effective_xref: &XrefTable,
        original_trailer: &BTreeMap<String, PdfObject>,
    ) -> PdfResult<Vec<u8>> {
        if modified_objects.is_empty() {
            return Ok(original_bytes.to_vec());
        }

        let mut output = Vec::with_capacity(original_bytes.len().saturating_add(4096));
        output.extend_from_slice(original_bytes);
        if !output.ends_with(b"\n") && !output.ends_with(b"\r") {
            output.push(b'\n');
        }

        let mut final_entries = effective_xref.entries.clone();
        match final_entries.get(&0) {
            Some(XrefEntry::Free { .. }) => {}
            None => {
                final_entries.insert(
                    0,
                    XrefEntry::Free {
                        next_free_obj: 0,
                        generation: u16::MAX,
                    },
                );
            }
            Some(_) => {
                return Err(PdfError::RecoveredXrefExport(
                    "object 0 is not a free xref entry".into(),
                ));
            }
        }

        let mut modified_numbers = BTreeMap::<u64, ObjectRef>::new();
        for &reference in modified_objects.keys() {
            if reference.number > 9_999_999_999 {
                return Err(PdfError::RecoveredXrefExport(
                    "PDF object number exceeds the incremental writer limit".into(),
                ));
            }
            if let Some(previous) = modified_numbers.insert(reference.number, reference) {
                return Err(PdfError::RecoveredXrefExport(format!(
                    "conflicting modified identities {previous} and {reference}"
                )));
            }
            if let Some(existing) = final_entries.get(&reference.number) {
                if existing.generation() != reference.generation {
                    return Err(PdfError::RecoveredXrefExport(format!(
                        "modified object {reference} conflicts with effective generation {}",
                        existing.generation()
                    )));
                }
            }
        }

        for (&obj_num, entry) in &final_entries {
            if modified_numbers.contains_key(&obj_num) {
                continue;
            }
            match *entry {
                XrefEntry::InUse { byte_offset, .. } => {
                    let offset = usize::try_from(byte_offset).map_err(|_| {
                        PdfError::RecoveredXrefExport(format!(
                            "object {obj_num} offset exceeds platform range"
                        ))
                    })?;
                    if offset >= original_bytes.len() {
                        return Err(PdfError::RecoveredXrefExport(format!(
                            "object {obj_num} points outside the source"
                        )));
                    }
                }
                XrefEntry::Compressed { stream_obj_num, .. } => {
                    if stream_obj_num == obj_num {
                        return Err(PdfError::RecoveredXrefExport(format!(
                            "compressed object {obj_num} references itself as an object stream"
                        )));
                    }
                    match final_entries.get(&stream_obj_num) {
                        Some(XrefEntry::InUse { generation: 0, .. }) => {}
                        _ => {
                            return Err(PdfError::RecoveredXrefExport(format!(
                                "compressed object {obj_num} references invalid ObjStm {stream_obj_num}"
                            )));
                        }
                    }
                    if modified_numbers.contains_key(&stream_obj_num) {
                        return Err(PdfError::RecoveredXrefExport(format!(
                            "compressed object {obj_num} references modified ObjStm {stream_obj_num}"
                        )));
                    }
                }
                XrefEntry::Free { .. } => {}
            }
        }

        for (&reference, object) in modified_objects {
            let offset = output.len();
            writeln!(output, "{} {} obj", reference.number, reference.generation).map_err(
                |error| {
                    PdfError::RecoveredXrefExport(format!(
                        "failed to format object header: {error}"
                    ))
                },
            )?;
            Serializer::write_object(&mut output, object).map_err(|error| {
                PdfError::RecoveredXrefExport(format!(
                    "failed to serialize object {}: {error}",
                    reference.number
                ))
            })?;
            writeln!(output, "\nendobj").map_err(|error| {
                PdfError::RecoveredXrefExport(format!("failed to format endobj: {error}"))
            })?;
            final_entries.insert(
                reference.number,
                XrefEntry::InUse {
                    byte_offset: u64::try_from(offset).map_err(|_| {
                        PdfError::RecoveredXrefExport("serialized object offset exceeds u64".into())
                    })?,
                    generation: reference.generation,
                },
            );
            Self::validate_growth(original_bytes.len(), output.len())?;
        }

        let original_size = original_trailer
            .get("Size")
            .and_then(PdfObject::as_i64)
            .and_then(|size| u64::try_from(size).ok())
            .unwrap_or(0);
        let highest_obj_num = final_entries.keys().next_back().copied().unwrap_or(0);
        let xref_stream_obj_num = highest_obj_num
            .checked_add(1)
            .map(|next| next.max(original_size))
            .ok_or_else(|| {
                PdfError::RecoveredXrefExport("xref-stream object number overflow".into())
            })?;
        if xref_stream_obj_num > 9_999_999_999 {
            return Err(PdfError::RecoveredXrefExport(
                "xref-stream object number exceeds the writer limit".into(),
            ));
        }

        let xref_stream_offset = output.len();
        final_entries.insert(
            xref_stream_obj_num,
            XrefEntry::InUse {
                byte_offset: u64::try_from(xref_stream_offset).map_err(|_| {
                    PdfError::RecoveredXrefExport("xref-stream offset exceeds u64".into())
                })?,
                generation: 0,
            },
        );

        let index_ranges = Self::xref_index_ranges(&final_entries)?;
        let (widths, stream_data) = Self::serialize_xref_entries(&final_entries, &index_ranges)?;
        let size = xref_stream_obj_num
            .checked_add(1)
            .ok_or_else(|| PdfError::RecoveredXrefExport("xref-stream /Size overflow".into()))?;

        let mut stream_dict = original_trailer.clone();
        for key in [
            "Prev",
            "XRefStm",
            "Type",
            "Length",
            "Filter",
            "DecodeParms",
            "W",
            "Index",
        ] {
            stream_dict.remove(key);
        }
        stream_dict.insert("Type".into(), PdfObject::Name("XRef".into()));
        stream_dict.insert(
            "Size".into(),
            PdfObject::Integer(i64::try_from(size).map_err(|_| {
                PdfError::RecoveredXrefExport("xref-stream /Size exceeds i64".into())
            })?),
        );
        stream_dict.insert(
            "W".into(),
            PdfObject::Array(
                widths
                    .into_iter()
                    .map(|width| PdfObject::Integer(width as i64))
                    .collect(),
            ),
        );
        let mut index = Vec::with_capacity(index_ranges.len().saturating_mul(2));
        for (start, count) in &index_ranges {
            index.push(PdfObject::Integer(i64::try_from(*start).map_err(|_| {
                PdfError::RecoveredXrefExport("/Index start exceeds i64".into())
            })?));
            index.push(PdfObject::Integer(i64::try_from(*count).map_err(|_| {
                PdfError::RecoveredXrefExport("/Index count exceeds i64".into())
            })?));
        }
        stream_dict.insert("Index".into(), PdfObject::Array(index));

        writeln!(output, "{xref_stream_obj_num} 0 obj").map_err(|error| {
            PdfError::RecoveredXrefExport(format!("failed to format xref-stream header: {error}"))
        })?;
        Serializer::write_object(
            &mut output,
            &PdfObject::Stream(StreamObject {
                dict: stream_dict,
                stream_length: stream_data.len(),
                data: stream_data,
                stream_offset: 0,
            }),
        )
        .map_err(|error| {
            PdfError::RecoveredXrefExport(format!(
                "failed to serialize terminal xref stream: {error}"
            ))
        })?;
        writeln!(output, "\nendobj\nstartxref\n{xref_stream_offset}\n%%EOF").map_err(|error| {
            PdfError::RecoveredXrefExport(format!("failed to write startxref: {error}"))
        })?;
        Self::validate_growth(original_bytes.len(), output.len())?;
        Ok(output)
    }

    fn xref_index_ranges(entries: &BTreeMap<u64, XrefEntry>) -> PdfResult<Vec<(u64, usize)>> {
        let mut ranges = Vec::new();
        let mut keys = entries.keys().copied();
        let Some(mut start) = keys.next() else {
            return Err(PdfError::RecoveredXrefExport(
                "effective xref has no entries".into(),
            ));
        };
        let mut previous = start;
        let mut count = 1usize;
        for key in keys {
            if key == previous.saturating_add(1) {
                count = count
                    .checked_add(1)
                    .ok_or_else(|| PdfError::RecoveredXrefExport("/Index count overflow".into()))?;
            } else {
                ranges.push((start, count));
                start = key;
                count = 1;
            }
            previous = key;
        }
        ranges.push((start, count));
        Ok(ranges)
    }

    fn serialize_xref_entries(
        entries: &BTreeMap<u64, XrefEntry>,
        index_ranges: &[(u64, usize)],
    ) -> PdfResult<([usize; 3], Vec<u8>)> {
        let mut max_field2 = 0u64;
        let mut max_field3 = 0u64;
        for entry in entries.values() {
            let (field2, field3) = match *entry {
                XrefEntry::Free {
                    next_free_obj,
                    generation,
                } => (next_free_obj, u64::from(generation)),
                XrefEntry::InUse {
                    byte_offset,
                    generation,
                } => (byte_offset, u64::from(generation)),
                XrefEntry::Compressed {
                    stream_obj_num,
                    index_in_stream,
                } => (stream_obj_num, u64::from(index_in_stream)),
            };
            max_field2 = max_field2.max(field2);
            max_field3 = max_field3.max(field3);
        }
        let widths = [
            1,
            Self::unsigned_width(max_field2),
            Self::unsigned_width(max_field3),
        ];
        let entry_width = widths.iter().sum::<usize>();
        let entry_count = index_ranges.iter().try_fold(0usize, |total, (_, count)| {
            total.checked_add(*count).ok_or_else(|| {
                PdfError::RecoveredXrefExport("xref-stream entry count overflow".into())
            })
        })?;
        let capacity = entry_count.checked_mul(entry_width).ok_or_else(|| {
            PdfError::RecoveredXrefExport("xref-stream byte length overflow".into())
        })?;
        let mut data = Vec::with_capacity(capacity);
        for (start, count) in index_ranges {
            for index in 0..*count {
                let obj_num = start.checked_add(index as u64).ok_or_else(|| {
                    PdfError::RecoveredXrefExport("/Index object number overflow".into())
                })?;
                let entry = entries.get(&obj_num).ok_or_else(|| {
                    PdfError::RecoveredXrefExport(format!(
                        "missing effective entry for indexed object {obj_num}"
                    ))
                })?;
                let (entry_type, field2, field3) = match *entry {
                    XrefEntry::Free {
                        next_free_obj,
                        generation,
                    } => (0, next_free_obj, u64::from(generation)),
                    XrefEntry::InUse {
                        byte_offset,
                        generation,
                    } => (1, byte_offset, u64::from(generation)),
                    XrefEntry::Compressed {
                        stream_obj_num,
                        index_in_stream,
                    } => (2, stream_obj_num, u64::from(index_in_stream)),
                };
                Self::write_big_endian(&mut data, entry_type, widths[0]);
                Self::write_big_endian(&mut data, field2, widths[1]);
                Self::write_big_endian(&mut data, field3, widths[2]);
            }
        }
        Ok((widths, data))
    }

    const fn unsigned_width(value: u64) -> usize {
        let bits = u64::BITS - value.leading_zeros();
        let bytes = bits.div_ceil(8) as usize;
        if bytes == 0 {
            1
        } else {
            bytes
        }
    }

    fn write_big_endian(output: &mut Vec<u8>, value: u64, width: usize) {
        let bytes = value.to_be_bytes();
        output.extend_from_slice(&bytes[bytes.len() - width..]);
    }

    fn validate_growth(original_len: usize, output_len: usize) -> PdfResult<()> {
        let growth = output_len.checked_sub(original_len).ok_or_else(|| {
            PdfError::InvalidOperation("Incremental output length arithmetic underflow".into())
        })?;
        if growth > Self::MAX_INCREMENTAL_OUTPUT_GROWTH {
            return Err(PdfError::InvalidOperation(format!(
                "Incremental output growth exceeds maximum of {} bytes",
                Self::MAX_INCREMENTAL_OUTPUT_GROWTH
            )));
        }
        Ok(())
    }
}
