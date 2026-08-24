use std::collections::BTreeMap;
use std::io::Write;

use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};
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
                        return Err(PdfError::InvalidOperation(
                            "XREF_RECOVERED_EXPORT_UNSUPPORTED_COMPRESSED: clean recovered export requires xref-stream serialization"
                                .into(),
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
