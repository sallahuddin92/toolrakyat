use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::io::cursor::ByteCursor;
use crate::io::source::ByteSource;
use crate::syntax::object::{ObjectRef, PdfObject};
use crate::syntax::parser::Parser;
use crate::xref::table::XrefTable;

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
    metrics: ObjectStoreMetrics,
}

impl<'a> ObjectStore<'a> {
    pub fn new(source: ByteSource<'a>, xref: XrefTable) -> Self {
        let objects_known = xref.entries.len();
        Self {
            source,
            xref,
            cache: BTreeMap::new(),
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

    /// Resolves an indirect object reference lazily from the PDF byte source.
    pub fn resolve(&mut self, obj_ref: ObjectRef) -> PdfResult<&PdfObject> {
        if self.cache.contains_key(&obj_ref) {
            self.metrics.cache_hits += 1;
            return Ok(self.cache.get(&obj_ref).unwrap());
        }

        let offset = self
            .xref
            .get_offset(obj_ref.number)
            .ok_or(PdfError::ObjectNotFound {
                number: obj_ref.number,
                generation: obj_ref.generation,
            })?;

        if offset >= self.source.len() as u64 {
            return Err(PdfError::ObjectNotFound {
                number: obj_ref.number,
                generation: obj_ref.generation,
            });
        }

        let start_pos = offset as usize;
        let mut cursor = ByteCursor::new(self.source);
        cursor.set_position(start_pos)?;

        let mut parser = Parser::from_cursor(cursor);
        let (parsed_ref, obj) = parser.parse_indirect_object()?;

        if parsed_ref.number != obj_ref.number {
            return Err(PdfError::InvalidSyntax(format!(
                "Object number mismatch at offset {offset}: expected {}, found {}",
                obj_ref.number, parsed_ref.number
            )));
        }

        let end_pos = parser.position();
        self.metrics.bytes_parsed += end_pos.saturating_sub(start_pos);
        self.metrics.objects_resolved += 1;

        self.cache.insert(obj_ref, obj);
        Ok(self.cache.get(&obj_ref).unwrap())
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
