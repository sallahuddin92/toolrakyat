use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ContentOperand {
    Integer(i64),
    Real(f64),
    Name(String),
    String(Vec<u8>),
    Array(Vec<ContentOperand>),
    Dict(BTreeMap<String, ContentOperand>),
}

impl ContentOperand {
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Real(r) => Some(*r),
            Self::Integer(i) => Some(*i as f64),
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
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(b) => std::str::from_utf8(b).ok(),
            Self::Name(n) => Some(n.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::String(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(n) => Some(n.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_array(&self) -> Option<&[ContentOperand]> {
        match self {
            Self::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }
}
