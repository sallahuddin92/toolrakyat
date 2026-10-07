use super::{ContractRefusal, ContractResult, RefusalCode};
use crate::syntax::object::PdfObject;
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

/// Canonical object value, excluding parser offsets and allocation identity.
pub fn object_content_hash(object: &PdfObject) -> ContractResult<ContentHash> {
    let mut bytes = Vec::new();
    encode_object(object, &mut bytes)?;
    SemanticHashInput {
        domain: HashDomain::PdfObject,
        fields: BTreeMap::from([("value".into(), bytes)]),
    }
    .content_hash()
}

fn field(bytes: &[u8], output: &mut Vec<u8>) {
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn encode_object(object: &PdfObject, output: &mut Vec<u8>) -> ContractResult<()> {
    match object {
        PdfObject::Null => output.push(0),
        PdfObject::Bool(value) => {
            output.push(1);
            output.push(u8::from(*value));
        }
        PdfObject::Integer(value) => {
            output.push(2);
            output.extend_from_slice(&value.to_be_bytes());
        }
        PdfObject::Real(value) => {
            output.push(3);
            output.extend_from_slice(&semantic_number_bytes(*value)?);
        }
        PdfObject::Name(value) => {
            output.push(4);
            field(value.as_bytes(), output);
        }
        PdfObject::String(value) => {
            output.push(5);
            field(value, output);
        }
        PdfObject::Array(values) => {
            output.push(6);
            output.extend_from_slice(&(values.len() as u64).to_be_bytes());
            for value in values {
                encode_object(value, output)?;
            }
        }
        PdfObject::Dictionary(values) => {
            output.push(7);
            encode_dictionary(values, output)?;
        }
        PdfObject::Stream(stream) => {
            output.push(8);
            encode_dictionary(&stream.dict, output)?;
            field(&stream.data, output);
        }
        PdfObject::Reference(reference) => {
            output.push(9);
            output.extend_from_slice(&reference.number.to_be_bytes());
            output.extend_from_slice(&reference.generation.to_be_bytes());
        }
    }
    Ok(())
}

fn encode_dictionary(
    values: &BTreeMap<String, PdfObject>,
    output: &mut Vec<u8>,
) -> ContractResult<()> {
    output.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for (key, value) in values {
        field(key.as_bytes(), output);
        encode_object(value, output)?;
    }
    Ok(())
}

pub fn font_content_hash(fields: BTreeMap<String, Vec<u8>>) -> ContractResult<ContentHash> {
    SemanticHashInput {
        domain: HashDomain::Font,
        fields,
    }
    .content_hash()
}
pub fn image_content_hash(fields: BTreeMap<String, Vec<u8>>) -> ContractResult<ContentHash> {
    SemanticHashInput {
        domain: HashDomain::Image,
        fields,
    }
    .content_hash()
}
pub fn text_run_content_hash(fields: BTreeMap<String, Vec<u8>>) -> ContractResult<ContentHash> {
    SemanticHashInput {
        domain: HashDomain::TextRun,
        fields,
    }
    .content_hash()
}
pub fn annotation_content_hash(fields: BTreeMap<String, Vec<u8>>) -> ContractResult<ContentHash> {
    SemanticHashInput {
        domain: HashDomain::Annotation,
        fields,
    }
    .content_hash()
}
