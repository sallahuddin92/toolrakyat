use std::collections::BTreeMap;
use std::io::Write;

use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};
use crate::writer::serializer::Serializer;

pub struct CompleteWriter;

impl CompleteWriter {
    pub const MAX_OBJECTS: usize = 100_000;

    pub fn write(
        version: &str,
        objects: &BTreeMap<ObjectRef, PdfObject>,
        root_ref: ObjectRef,
        trailer_entries: &BTreeMap<String, PdfObject>,
        max_output_bytes: usize,
    ) -> PdfResult<Vec<u8>> {
        if objects.is_empty() || objects.len() > Self::MAX_OBJECTS {
            return Err(PdfError::PageResourceLimit(format!(
                "destination object count must be between 1 and {}",
                Self::MAX_OBJECTS
            )));
        }
        if !objects.contains_key(&root_ref) {
            return Err(PdfError::PageOperation(
                "complete writer root object is missing".into(),
            ));
        }
        let expected_last = u64::try_from(objects.len()).map_err(|_| {
            PdfError::PageResourceLimit("destination object count conversion overflow".into())
        })?;
        for expected in 1..=expected_last {
            if !objects.contains_key(&ObjectRef::new(expected, 0)) {
                return Err(PdfError::PageOperation(format!(
                    "complete writer requires sequential destination object {expected} 0 R"
                )));
            }
        }

        let safe_version = match version {
            "1.0" | "1.1" | "1.2" | "1.3" | "1.4" | "1.5" | "1.6" | "1.7" | "2.0" => version,
            _ => "1.7",
        };
        let mut output = Vec::new();
        write!(output, "%PDF-{safe_version}\n%").map_err(|error| Self::io_error(&error))?;
        output.extend_from_slice(&[0xE2, 0xE3, 0xCF, 0xD3, b'\n']);
        let offset_capacity = objects
            .len()
            .checked_add(1)
            .ok_or_else(|| PdfError::PageResourceLimit("xref offset capacity overflow".into()))?;
        let mut offsets = Vec::with_capacity(offset_capacity);
        offsets.push(0usize);

        for (reference, object) in objects {
            if reference.generation != 0 || reference.number > 9_999_999_999 {
                return Err(PdfError::PageOperation(format!(
                    "unsupported destination object identity {reference}"
                )));
            }
            offsets.push(output.len());
            writeln!(output, "{} 0 obj", reference.number)
                .map_err(|error| Self::io_error(&error))?;
            Serializer::write_object(&mut output, object)
                .map_err(|error| Self::io_error(&error))?;
            output.extend_from_slice(b"\nendobj\n");
            Self::check_output_limit(output.len(), max_output_bytes)?;
        }

        let xref_offset = output.len();
        let xref_count = objects
            .len()
            .checked_add(1)
            .ok_or_else(|| PdfError::PageResourceLimit("xref entry count overflow".into()))?;
        writeln!(output, "xref\n0 {xref_count}").map_err(|error| Self::io_error(&error))?;
        output.extend_from_slice(b"0000000000 65535 f \r\n");
        for offset in offsets.iter().skip(1) {
            let offset_u64 = u64::try_from(*offset).map_err(|_| {
                PdfError::PageResourceLimit("complete writer offset conversion overflow".into())
            })?;
            if offset_u64 > 9_999_999_999 {
                return Err(PdfError::PageResourceLimit(
                    "complete writer offset exceeds classic xref width".into(),
                ));
            }
            write!(output, "{offset:010} 00000 n \r\n").map_err(|error| Self::io_error(&error))?;
        }

        let mut trailer = trailer_entries.clone();
        let size = i64::try_from(xref_count)
            .map_err(|_| PdfError::PageResourceLimit("trailer /Size conversion overflow".into()))?;
        trailer.insert("Size".into(), PdfObject::Integer(size));
        trailer.insert("Root".into(), PdfObject::Reference(root_ref));
        trailer.remove("Prev");
        trailer.remove("XRefStm");
        trailer.remove("Encrypt");
        output.extend_from_slice(b"trailer\n");
        Serializer::write_object(&mut output, &PdfObject::Dictionary(trailer))
            .map_err(|error| Self::io_error(&error))?;
        write!(output, "\nstartxref\n{xref_offset}\n%%EOF\n")
            .map_err(|error| Self::io_error(&error))?;
        Self::check_output_limit(output.len(), max_output_bytes)?;
        Ok(output)
    }

    fn io_error(error: &std::io::Error) -> PdfError {
        PdfError::InvalidOperation(format!("complete PDF serialization failed: {error}"))
    }

    fn check_output_limit(output_len: usize, max_output_bytes: usize) -> PdfResult<()> {
        if output_len > max_output_bytes {
            return Err(PdfError::PageResourceLimit(format!(
                "complete output exceeds {max_output_bytes} bytes"
            )));
        }
        Ok(())
    }
}
