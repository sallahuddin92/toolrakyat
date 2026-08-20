use std::collections::BTreeSet;

use crate::document::object_store::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{ObjectRef, PdfObject};

pub const MAX_SIGNATURES: usize = 64;
pub const MAX_BYTE_RANGE_VALUES: usize = 16;
pub const MAX_SIGNATURE_CONTENTS_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SECURITY_OBJECTS_INSPECTED: usize = 100_000;
pub const MAX_CRYPT_FILTERS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureState {
    Unsigned,
    SignedPresent,
    SignedWithByteRange,
    SignedStructureMalformed,
}

impl SignatureState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsigned => "UNSIGNED",
            Self::SignedPresent => "SIGNED_PRESENT",
            Self::SignedWithByteRange => "SIGNED_WITH_BYTE_RANGE",
            Self::SignedStructureMalformed => "SIGNED_STRUCTURE_MALFORMED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionState {
    NotEncrypted,
    StandardSecurityDetected,
    PublicKeySecurityDetected,
    UnsupportedEncryption,
    MalformedEncryptionDictionary,
}

impl EncryptionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotEncrypted => "NOT_ENCRYPTED",
            Self::StandardSecurityDetected => "STANDARD_SECURITY_DETECTED",
            Self::PublicKeySecurityDetected => "PUBLIC_KEY_SECURITY_DETECTED",
            Self::UnsupportedEncryption => "UNSUPPORTED_ENCRYPTION",
            Self::MalformedEncryptionDictionary => "MALFORMED_ENCRYPTION_DICTIONARY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedMutationState {
    NotApplicable,
    SignedBytesPreserved,
    PostSignatureRevisionAdded,
    SignatureValidityUnknown,
    MutationRefused,
}

impl SignedMutationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "NOT_APPLICABLE",
            Self::SignedBytesPreserved => "SIGNED_BYTES_PRESERVED",
            Self::PostSignatureRevisionAdded => "POST_SIGNATURE_REVISION_ADDED",
            Self::SignatureValidityUnknown => "SIGNATURE_VALIDITY_UNKNOWN",
            Self::MutationRefused => "MUTATION_REFUSED",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncryptionPermissions {
    pub raw: Option<i32>,
    pub printing: Option<bool>,
    pub modification: Option<bool>,
    pub copying: Option<bool>,
    pub annotation_and_forms: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteRangeSegment {
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSecurityInfo {
    pub signature_state: SignatureState,
    pub signature_count: usize,
    pub byte_ranges: Vec<Vec<ByteRangeSegment>>,
    pub encryption_state: EncryptionState,
    pub encryption_filter: Option<String>,
    pub encryption_subfilter: Option<String>,
    pub encryption_version: Option<i64>,
    pub encryption_revision: Option<i64>,
    pub encryption_length: Option<i64>,
    pub permissions: EncryptionPermissions,
    pub mutation_allowed: bool,
    pub mutation_reason_code: Option<String>,
    pub signed_mutation_state: SignedMutationState,
    pub document_id_valid: bool,
}

impl DocumentSecurityInfo {
    pub fn inspect(store: &mut ObjectStore<'_>, file_len: usize) -> PdfResult<Self> {
        let (
            encryption_state,
            encryption_filter,
            encryption_subfilter,
            encryption_version,
            encryption_revision,
            encryption_length,
            permissions,
        ) = inspect_encryption(store)?;
        let (signature_state, signature_count, byte_ranges) = inspect_signatures(store, file_len)?;
        let document_id_valid = valid_document_id(store.trailer().get("ID"));
        let (mutation_allowed, mutation_reason_code, signed_mutation_state) =
            if encryption_state != EncryptionState::NotEncrypted {
                (
                    false,
                    Some("ENCRYPTED_DOCUMENT_MUTATION_UNSUPPORTED".to_string()),
                    SignedMutationState::MutationRefused,
                )
            } else if signature_state == SignatureState::SignedStructureMalformed {
                (
                    false,
                    Some("SIGNED_STRUCTURE_MALFORMED".to_string()),
                    SignedMutationState::MutationRefused,
                )
            } else if signature_state == SignatureState::Unsigned {
                (true, None, SignedMutationState::NotApplicable)
            } else {
                (
                    true,
                    Some("SIGNATURE_VALIDITY_UNKNOWN".to_string()),
                    SignedMutationState::SignatureValidityUnknown,
                )
            };
        Ok(Self {
            signature_state,
            signature_count,
            byte_ranges,
            encryption_state,
            encryption_filter,
            encryption_subfilter,
            encryption_version,
            encryption_revision,
            encryption_length,
            permissions,
            mutation_allowed,
            mutation_reason_code,
            signed_mutation_state,
            document_id_valid,
        })
    }
}

pub fn parse_byte_range(object: &PdfObject, file_len: usize) -> PdfResult<Vec<ByteRangeSegment>> {
    let values = object.as_array().ok_or_else(|| {
        PdfError::MalformedSignature("Signature /ByteRange must be an array".into())
    })?;
    if values.is_empty() || values.len() % 2 != 0 || values.len() > MAX_BYTE_RANGE_VALUES {
        return Err(PdfError::MalformedSignature(format!(
            "Signature /ByteRange must contain 2..={MAX_BYTE_RANGE_VALUES} paired values"
        )));
    }
    let mut segments = Vec::with_capacity(values.len() / 2);
    let mut previous_end = 0u64;
    for pair in values.chunks_exact(2) {
        let offset = pair[0].as_i64().ok_or_else(|| {
            PdfError::MalformedSignature("Signature /ByteRange offset is not an integer".into())
        })?;
        let length = pair[1].as_i64().ok_or_else(|| {
            PdfError::MalformedSignature("Signature /ByteRange length is not an integer".into())
        })?;
        let offset = u64::try_from(offset).map_err(|_| {
            PdfError::MalformedSignature("Signature /ByteRange offset is negative".into())
        })?;
        let length = u64::try_from(length).map_err(|_| {
            PdfError::MalformedSignature("Signature /ByteRange length is negative".into())
        })?;
        let end = offset.checked_add(length).ok_or_else(|| {
            PdfError::MalformedSignature("Signature /ByteRange arithmetic overflow".into())
        })?;
        if offset < previous_end {
            return Err(PdfError::MalformedSignature(
                "Signature /ByteRange segments overlap or are unordered".into(),
            ));
        }
        if end > file_len as u64 {
            return Err(PdfError::MalformedSignature(format!(
                "Signature /ByteRange end {end} exceeds file length {file_len}"
            )));
        }
        segments.push(ByteRangeSegment { offset, length });
        previous_end = end;
    }
    Ok(segments)
}

fn inspect_signatures(
    store: &mut ObjectStore<'_>,
    file_len: usize,
) -> PdfResult<(SignatureState, usize, Vec<Vec<ByteRangeSegment>>)> {
    let object_count = store
        .xref()
        .entries
        .values()
        .filter(|entry| entry.is_in_use())
        .count();
    if object_count > MAX_SECURITY_OBJECTS_INSPECTED {
        return Err(PdfError::InvalidOperation(format!(
            "Security object count {object_count} exceeds maximum of {MAX_SECURITY_OBJECTS_INSPECTED}"
        )));
    }
    let refs: Vec<ObjectRef> = store
        .xref()
        .entries
        .iter()
        .filter(|(_, entry)| entry.is_in_use())
        .map(|(number, entry)| ObjectRef::new(*number, entry.generation()))
        .collect();
    let mut signature_refs = BTreeSet::new();
    let mut ranges = Vec::new();
    let mut malformed = false;
    for reference in refs {
        let Ok(object) = store.resolve(reference).cloned() else {
            continue;
        };
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let is_signature_field = dict.get("FT").and_then(PdfObject::as_name) == Some("Sig");
        let is_signature = is_signature_field
            || dict.get("Type").and_then(PdfObject::as_name) == Some("Sig")
            || (dict.contains_key("ByteRange") && dict.contains_key("Contents"));
        if !is_signature {
            continue;
        }
        if signature_refs.len() >= MAX_SIGNATURES {
            return Err(PdfError::InvalidOperation(format!(
                "Signature count exceeds maximum of {MAX_SIGNATURES}"
            )));
        }
        let identity = if is_signature_field {
            dict.get("V")
                .and_then(PdfObject::as_reference)
                .unwrap_or(reference)
        } else {
            reference
        };
        signature_refs.insert(identity);
        if let Some(contents) = dict.get("Contents") {
            let size = match contents {
                PdfObject::String(bytes) => bytes.len(),
                PdfObject::Stream(stream) => stream.data.len(),
                _ => {
                    malformed = true;
                    0
                }
            };
            if size > MAX_SIGNATURE_CONTENTS_BYTES {
                return Err(PdfError::InvalidOperation(format!(
                    "Signature contents exceed maximum of {MAX_SIGNATURE_CONTENTS_BYTES} bytes"
                )));
            }
        }
        if let Some(byte_range) = dict.get("ByteRange") {
            match store.resolve_object(byte_range) {
                Ok(resolved) => match parse_byte_range(&resolved, file_len) {
                    Ok(parsed) => ranges.push(parsed),
                    Err(_) => malformed = true,
                },
                Err(_) => malformed = true,
            }
        }
    }
    let count = signature_refs.len();
    let state = if malformed {
        SignatureState::SignedStructureMalformed
    } else if !ranges.is_empty() {
        SignatureState::SignedWithByteRange
    } else if count > 0 {
        SignatureState::SignedPresent
    } else {
        SignatureState::Unsigned
    };
    Ok((state, count, ranges))
}

#[allow(clippy::type_complexity)]
fn inspect_encryption(
    store: &mut ObjectStore<'_>,
) -> PdfResult<(
    EncryptionState,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    EncryptionPermissions,
)> {
    let Some(encrypt) = store.trailer().get("Encrypt").cloned() else {
        return Ok((
            EncryptionState::NotEncrypted,
            None,
            None,
            None,
            None,
            None,
            EncryptionPermissions::default(),
        ));
    };
    let Ok(encrypt) = store.resolve_object(&encrypt) else {
        return Ok((
            EncryptionState::MalformedEncryptionDictionary,
            None,
            None,
            None,
            None,
            None,
            EncryptionPermissions::default(),
        ));
    };
    let Some(dict) = encrypt.as_dict() else {
        return Ok((
            EncryptionState::MalformedEncryptionDictionary,
            None,
            None,
            None,
            None,
            None,
            EncryptionPermissions::default(),
        ));
    };
    if dict
        .get("CF")
        .and_then(PdfObject::as_dict)
        .is_some_and(|filters| filters.len() > MAX_CRYPT_FILTERS)
    {
        return Err(PdfError::InvalidOperation(format!(
            "Crypt filter count exceeds maximum of {MAX_CRYPT_FILTERS}"
        )));
    }
    let filter = dict
        .get("Filter")
        .and_then(PdfObject::as_name)
        .map(str::to_string);
    let subfilter = dict
        .get("SubFilter")
        .and_then(PdfObject::as_name)
        .map(str::to_string);
    let state = match filter.as_deref() {
        Some("Standard") => EncryptionState::StandardSecurityDetected,
        Some("Adobe.PubSec") => EncryptionState::PublicKeySecurityDetected,
        Some(_) => EncryptionState::UnsupportedEncryption,
        None => EncryptionState::MalformedEncryptionDictionary,
    };
    let raw_permissions = dict
        .get("P")
        .and_then(PdfObject::as_i64)
        .and_then(|value| i32::try_from(value).ok());
    let permissions = raw_permissions.map_or_else(EncryptionPermissions::default, |raw| {
        EncryptionPermissions {
            raw: Some(raw),
            printing: Some((raw & (1 << 2)) != 0),
            modification: Some((raw & (1 << 3)) != 0),
            copying: Some((raw & (1 << 4)) != 0),
            annotation_and_forms: Some((raw & (1 << 5)) != 0),
        }
    });
    Ok((
        state,
        filter,
        subfilter,
        dict.get("V").and_then(PdfObject::as_i64),
        dict.get("R").and_then(PdfObject::as_i64),
        dict.get("Length").and_then(PdfObject::as_i64),
        permissions,
    ))
}

fn valid_document_id(object: Option<&PdfObject>) -> bool {
    let Some(values) = object.and_then(PdfObject::as_array) else {
        return object.is_none();
    };
    values.len() == 2
        && values
            .iter()
            .all(|value| matches!(value, PdfObject::String(_)))
}
