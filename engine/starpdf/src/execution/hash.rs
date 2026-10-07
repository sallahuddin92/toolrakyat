use super::{ContractRefusal, ContractResult, RefusalCode};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn to_hex(self) -> String {
        super::identity::hex(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HashDomain {
    Font = 1,
    Image = 2,
    TextRun = 3,
    Annotation = 4,
    PdfObject = 5,
}

/// Fields describe semantics, never allocation addresses, revision counters or UI state.
/// Names are ASCII and sorted; values are exact bytes under the documented v1 profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticHashInput {
    pub domain: HashDomain,
    pub fields: BTreeMap<String, Vec<u8>>,
}

impl SemanticHashInput {
    pub fn canonical_bytes(&self) -> ContractResult<Vec<u8>> {
        if self
            .fields
            .keys()
            .any(|key| key.is_empty() || !key.is_ascii())
        {
            return Err(ContractRefusal::new(
                RefusalCode::InvalidHashInput,
                "field names must be nonempty ASCII",
            ));
        }
        let mut bytes = b"StarPDF.semantic.v1\0".to_vec();
        bytes.push(self.domain as u8);
        bytes.extend_from_slice(&(self.fields.len() as u64).to_be_bytes());
        for (name, value) in &self.fields {
            bytes.extend_from_slice(&(name.len() as u64).to_be_bytes());
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
            bytes.extend_from_slice(value);
        }
        Ok(bytes)
    }
    pub fn content_hash(&self) -> ContractResult<ContentHash> {
        Ok(ContentHash(Sha256::digest(self.canonical_bytes()?).into()))
    }
}

/// Finite binary64 big-endian; normalize negative zero. No locale/string rounding.
pub fn semantic_number_bytes(value: f64) -> ContractResult<[u8; 8]> {
    if !value.is_finite() {
        return Err(ContractRefusal::new(
            RefusalCode::InvalidHashInput,
            "semantic numbers must be finite",
        ));
    }
    Ok((if value == 0.0 { 0.0 } else { value }).to_be_bytes())
}
