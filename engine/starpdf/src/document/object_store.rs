use std::collections::BTreeMap;

use crate::document::object_stream::{DecodedObjectStream, ObjectStreamReader};
use crate::error::{PdfError, PdfResult};
use crate::filter::limits::DecompressLimits;
use crate::io::cursor::ByteCursor;
use crate::io::source::ByteSource;
use crate::syntax::object::{ObjectRef, PdfObject};
use crate::syntax::parser::Parser;
use crate::xref::table::{XrefEntry, XrefTable};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjectStoreMetrics {
    pub objects_known: usize,
    pub objects_resolved: usize,
    pub cache_hits: usize,
    pub bytes_parsed: usize,
}

pub struct ObjectStore<'a> {
    source: ByteSource<'a>,
    xref: XrefTable,
    cache: BTreeMap<ObjectRef, PdfObject>,
    decoded_obj_streams: BTreeMap<u64, DecodedObjectStream>,
    resolving_stack: Vec<ObjectRef>,
    limits: DecompressLimits,
    metrics: ObjectStoreMetrics,
}

impl<'a> ObjectStore<'a> {
    pub fn new(source: ByteSource<'a>, xref: XrefTable) -> Self {
        Self::new_with_limits(source, xref, DecompressLimits::default())
    }

    pub fn new_with_limits(
        source: ByteSource<'a>,
        xref: XrefTable,
        limits: DecompressLimits,
    ) -> Self {
        let objects_known = xref.entries.len();
        Self {
            source,
            xref,
            cache: BTreeMap::new(),
            decoded_obj_streams: BTreeMap::new(),
            resolving_stack: Vec::new(),
            limits,
            metrics: ObjectStoreMetrics {
                objects_known,
                objects_resolved: 0,
                cache_hits: 0,
                bytes_parsed: 0,
            },
        }
    }

    pub fn xref(&self) -> &XrefTable {
        &self.xref
    }

    pub fn trailer(&self) -> &BTreeMap<String, PdfObject> {
        &self.xref.trailer
    }

    pub fn metrics(&self) -> ObjectStoreMetrics {
        self.metrics
    }

    pub fn is_cached(&self, obj_ref: ObjectRef) -> bool {
        self.cache.contains_key(&obj_ref)
    }

    pub fn get_cached(&self, obj_ref: ObjectRef) -> Option<&PdfObject> {
        self.cache.get(&obj_ref)
    }

    /// Resolves an indirect object reference lazily (supporting classic offsets and modern object streams).
    pub fn resolve(&mut self, obj_ref: ObjectRef) -> PdfResult<&PdfObject> {
        if self.cache.contains_key(&obj_ref) {
            self.metrics.cache_hits += 1;
            return Ok(self.cache.get(&obj_ref).unwrap());
        }

        if self.resolving_stack.contains(&obj_ref) {
            return Err(PdfError::CircularReference(format!(
                "Circular reference detected while resolving object {obj_ref}"
            )));
        }

        self.resolving_stack.push(obj_ref);

        let entry =
            self.xref
                .get_entry(obj_ref.number)
                .copied()
                .ok_or(PdfError::ObjectNotFound {
                    number: obj_ref.number,
                    generation: obj_ref.generation,
                })?;

        let resolution_result = match entry {
            XrefEntry::InUse { byte_offset, .. } => {
                self.resolve_uncompressed_object(obj_ref, byte_offset)
            }
            XrefEntry::Compressed {
                stream_obj_num,
                index_in_stream,
            } => self.resolve_compressed_object(obj_ref, stream_obj_num, index_in_stream),
            XrefEntry::Free { .. } => Err(PdfError::ObjectNotFound {
                number: obj_ref.number,
                generation: obj_ref.generation,
            }),
        };

        self.resolving_stack.pop();

        let obj = resolution_result?;
        self.cache.insert(obj_ref, obj);
        self.metrics.objects_resolved += 1;

        Ok(self.cache.get(&obj_ref).unwrap())
    }

    fn resolve_uncompressed_object(
        &mut self,
        obj_ref: ObjectRef,
        byte_offset: u64,
    ) -> PdfResult<PdfObject> {
        if byte_offset >= self.source.len() as u64 {
            return Err(PdfError::ObjectNotFound {
                number: obj_ref.number,
                generation: obj_ref.generation,
            });
        }

        let start_pos = byte_offset as usize;
        let mut cursor = ByteCursor::new(self.source);
        cursor.set_position(start_pos)?;

        let mut parser = Parser::from_cursor(cursor);
        let (parsed_ref, obj) = parser.parse_indirect_object()?;

        if parsed_ref.number != obj_ref.number {
            return Err(PdfError::InvalidSyntax(format!(
                "Object number mismatch at offset {byte_offset}: expected {}, found {}",
                obj_ref.number, parsed_ref.number
            )));
        }

        let end_pos = parser.position();
        self.metrics.bytes_parsed += end_pos.saturating_sub(start_pos);

        Ok(obj)
    }

    fn resolve_compressed_object(
        &mut self,
        _obj_ref: ObjectRef,
        stream_obj_num: u64,
        index_in_stream: u32,
    ) -> PdfResult<PdfObject> {
        // Ensure object stream is loaded and decoded
        if !self.decoded_obj_streams.contains_key(&stream_obj_num) {
            let stream_ref = ObjectRef::new(stream_obj_num, 0);
            let stream_obj = self.resolve(stream_ref)?.clone();

            let stream_data = stream_obj.as_stream().ok_or_else(|| {
                PdfError::InvalidSyntax(format!(
                    "Object {stream_obj_num} referenced as ObjStm is not a stream"
                ))
            })?;

            let decoded_stream = ObjectStreamReader::decode_stream(stream_data, &self.limits)?;
            self.decoded_obj_streams
                .insert(stream_obj_num, decoded_stream);
        }

        let decoded = self.decoded_obj_streams.get(&stream_obj_num).unwrap();
        ObjectStreamReader::extract_object(decoded, index_in_stream)
    }

    /// If `obj` is an indirect reference, resolves it; otherwise returns the object itself.
    pub fn resolve_object(&mut self, obj: &PdfObject) -> PdfResult<PdfObject> {
        match obj {
            PdfObject::Reference(r) => {
                let resolved = self.resolve(*r)?;
                Ok(resolved.clone())
            }
            other => Ok(other.clone()),
        }
    }
}
