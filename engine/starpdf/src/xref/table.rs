use crate::syntax::object::PdfObject;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefEntry {
    Free {
        next_free_obj: u64,
        generation: u16,
    },
    InUse {
        byte_offset: u64,
        generation: u16,
    },
    Compressed {
        stream_obj_num: u64,
        index_in_stream: u32,
    },
}

impl XrefEntry {
    #[inline]
    pub const fn is_in_use(&self) -> bool {
        matches!(self, Self::InUse { .. } | Self::Compressed { .. })
    }

    #[inline]
    pub const fn byte_offset(&self) -> Option<u64> {
        match self {
            Self::InUse { byte_offset, .. } => Some(*byte_offset),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct XrefTable {
    pub entries: BTreeMap<u64, XrefEntry>,
    pub trailer: BTreeMap<String, PdfObject>,
    pub startxref_offset: u64,
}

impl XrefTable {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get_entry(&self, obj_num: u64) -> Option<&XrefEntry> {
        self.entries.get(&obj_num)
    }

    #[inline]
    pub fn get_offset(&self, obj_num: u64) -> Option<u64> {
        self.entries
            .get(&obj_num)
            .and_then(|entry| entry.byte_offset())
    }

    #[inline]
    pub fn insert_in_use(&mut self, obj_num: u64, byte_offset: u64, generation: u16) {
        self.entries.insert(
            obj_num,
            XrefEntry::InUse {
                byte_offset,
                generation,
            },
        );
    }

    #[inline]
    pub fn insert_free(&mut self, obj_num: u64, next_free_obj: u64, generation: u16) {
        self.entries.insert(
            obj_num,
            XrefEntry::Free {
                next_free_obj,
                generation,
            },
        );
    }
}
