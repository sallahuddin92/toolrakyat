use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectRef {
    pub number: u64,
    pub generation: u16,
}

impl ObjectRef {
    #[inline]
    pub const fn new(number: u64, generation: u16) -> Self {
        Self { number, generation }
    }
}

impl fmt::Display for ObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.number, self.generation)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamObject {
    pub dict: BTreeMap<String, PdfObject>,
    pub data: Vec<u8>,
    pub stream_offset: usize,
    pub stream_length: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    Null,
    Bool(bool),
    Integer(i64),
    Real(f64),
    Name(String),
    String(Vec<u8>),
    Array(Vec<PdfObject>),
    Dictionary(BTreeMap<String, PdfObject>),
    Stream(StreamObject),
    Reference(ObjectRef),
}

impl PdfObject {
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    #[inline]
    pub fn as_integer(&self) -> Option<i64> {
        self.as_i64()
    }

    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Real(r) => Some(*r),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    #[inline]
    pub fn as_real(&self) -> Option<f64> {
        self.as_f64()
    }

    #[inline]
    pub fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(n) => Some(n.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::String(bytes) => Some(bytes.as_slice()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(bytes) => std::str::from_utf8(bytes).ok(),
            Self::Name(n) => Some(n.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_string_lossy(&self) -> Option<String> {
        match self {
            Self::String(bytes) => {
                if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                    let u16_units: Vec<u16> = bytes[2..]
                        .chunks_exact(2)
                        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                        .collect();
                    Some(String::from_utf16_lossy(&u16_units))
                } else if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
                    let u16_units: Vec<u16> = bytes[2..]
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();
                    Some(String::from_utf16_lossy(&u16_units))
                } else {
                    Some(String::from_utf8_lossy(bytes).to_string())
                }
            }
            Self::Name(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            Self::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<PdfObject>> {
        match self {
            Self::Array(arr) => Some(arr),
            _ => None,
        }
    }

    #[inline]
    pub fn as_dict(&self) -> Option<&BTreeMap<String, PdfObject>> {
        match self {
            Self::Dictionary(dict) => Some(dict),
            Self::Stream(stream) => Some(&stream.dict),
            _ => None,
        }
    }

    #[inline]
    pub fn as_dict_mut(&mut self) -> Option<&mut BTreeMap<String, PdfObject>> {
        match self {
            Self::Dictionary(dict) => Some(dict),
            Self::Stream(stream) => Some(&mut stream.dict),
            _ => None,
        }
    }

    #[inline]
    pub fn as_stream(&self) -> Option<&StreamObject> {
        match self {
            Self::Stream(s) => Some(s),
            _ => None,
        }
    }

    #[inline]
    pub fn as_reference(&self) -> Option<ObjectRef> {
        match self {
            Self::Reference(r) => Some(*r),
            _ => None,
        }
    }

    #[inline]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Real(_) => "real",
            Self::Name(_) => "name",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Dictionary(_) => "dictionary",
            Self::Stream(_) => "stream",
            Self::Reference(_) => "reference",
        }
    }
}
