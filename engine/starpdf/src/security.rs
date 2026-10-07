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

/// Structural recognition is distinct from a qualified decryptor. No revision is
/// currently authorized for opening by this engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardRevision {
    R2,
    R3,
    R4,
    R5,
    R6,
    Unsupported,
}

impl StandardRevision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::R2 => "R2",
            Self::R3 => "R3",
            Self::R4 => "R4",
            Self::R5 => "R5",
            Self::R6 => "R6",
            Self::Unsupported => "UNSUPPORTED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    UserPassword,
    OwnerPassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialRefusal {
    NoEncryption,
    MalformedEncryption,
    UnsupportedHandler,
    UnsupportedRevision,
    DecryptionUnavailable,
}

impl CredentialRefusal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoEncryption => "NO_ENCRYPTION",
            Self::MalformedEncryption => "MALFORMED_ENCRYPTION",
            Self::UnsupportedHandler => "UNSUPPORTED_HANDLER",
            Self::UnsupportedRevision => "UNSUPPORTED_REVISION",
            Self::DecryptionUnavailable => "DECRYPTION_UNAVAILABLE",
        }
    }
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
    pub standard_revision: Option<StandardRevision>,
    pub encryption_dictionary_valid: bool,
    pub encryption_length: Option<i64>,
    pub permissions: EncryptionPermissions,
    pub mutation_allowed: bool,
    pub mutation_reason_code: Option<String>,
    pub signed_mutation_state: SignedMutationState,
    pub document_id_valid: bool,
    /// Greatest end offset among signed ByteRange segments.
    pub signed_revision_end: Option<u64>,
    /// True when a known cross-reference revision begins beyond the signed range.
    pub later_revision_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDecision {
    pub allowed: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentCapabilities {
    pub can_view: CapabilityDecision,
    pub can_edit: CapabilityDecision,
    pub can_extract: CapabilityDecision,
    pub can_annotate: CapabilityDecision,
    pub can_modify_pages: CapabilityDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedExportAssessment {
    pub signed_document: bool,
    pub signed_bytes_preserved: bool,
    pub mutation_appends_after_signed_range: bool,
    pub cryptographic_verification: &'static str,
}

pub fn assess_signed_export(
    original: &[u8],
    exported: &[u8],
    info: &DocumentSecurityInfo,
) -> SignedExportAssessment {
    let signed_document = info.signature_state != SignatureState::Unsigned;
    let ranges_valid =
        info.signature_state == SignatureState::SignedWithByteRange && !info.byte_ranges.is_empty();
    let signed_bytes_preserved = ranges_valid
        && info.byte_ranges.iter().flatten().all(|segment| {
            let Ok(start) = usize::try_from(segment.offset) else {
                return false;
            };
            let Ok(length) = usize::try_from(segment.length) else {
                return false;
            };
            let Some(end) = start.checked_add(length) else {
                return false;
            };
            original
                .get(start..end)
                .is_some_and(|bytes| exported.get(start..end) == Some(bytes))
        });
    let mutation_appends_after_signed_range = signed_bytes_preserved
        && exported.len() > original.len()
        && exported.starts_with(original)
        && info
            .signed_revision_end
            .is_some_and(|end| end <= original.len() as u64);
    SignedExportAssessment {
        signed_document,
        signed_bytes_preserved,
        mutation_appends_after_signed_range,
        cryptographic_verification: "NOT_PERFORMED",
    }
}

impl DocumentSecurityInfo {
    pub fn capabilities(&self, evidence_verified: bool) -> DocumentCapabilities {
        let allowed = |reason| CapabilityDecision {
            allowed: true,
            reason,
        };
        let denied = |reason| CapabilityDecision {
            allowed: false,
            reason,
        };
        if !evidence_verified {
            let decision = denied("INSUFFICIENT_EVIDENCE");
            return DocumentCapabilities {
                can_view: decision.clone(),
                can_edit: decision.clone(),
                can_extract: decision.clone(),
                can_annotate: decision.clone(),
                can_modify_pages: decision,
            };
        }
        let decrypted = self.encryption_state == EncryptionState::NotEncrypted;
        let view = if decrypted {
            allowed("UNENCRYPTED_VERIFIED")
        } else {
            denied("CREDENTIAL_REQUIRED_OR_DECRYPTION_UNAVAILABLE")
        };
        let extract = if self.permissions.copying == Some(false) {
            denied("COPY_PERMISSION_DENIED")
        } else if decrypted {
            allowed("UNENCRYPTED_VERIFIED")
        } else {
            denied("CREDENTIAL_REQUIRED_OR_DECRYPTION_UNAVAILABLE")
        };
        let mutation = if self.permissions.modification == Some(false) {
            denied("MODIFICATION_PERMISSION_DENIED")
        } else if !decrypted {
            denied("CREDENTIAL_REQUIRED_OR_DECRYPTION_UNAVAILABLE")
        } else if self.mutation_allowed {
            allowed("UNENCRYPTED_VERIFIED")
        } else {
            denied("MALFORMED_SIGNATURE_STRUCTURE")
        };
        let annotation = if self.permissions.annotation_and_forms == Some(false) {
            denied("ANNOTATION_PERMISSION_DENIED")
        } else {
            mutation.clone()
        };
        let pages = if self.signature_state != SignatureState::Unsigned {
            denied("SIGNED_PAGE_MUTATION_UNSUPPORTED")
        } else {
            mutation.clone()
        };
        DocumentCapabilities {
            can_view: view,
            can_edit: mutation.clone(),
            can_extract: extract,
            can_annotate: annotation,
            can_modify_pages: pages,
        }
    }
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
        let standard_revision = if encryption_state == EncryptionState::StandardSecurityDetected {
            encryption_revision.map(|revision| match revision {
                2 => StandardRevision::R2,
                3 => StandardRevision::R3,
                4 => StandardRevision::R4,
                5 => StandardRevision::R5,
                6 => StandardRevision::R6,
                _ => StandardRevision::Unsupported,
            })
        } else {
            None
        };
        let encryption_dictionary_valid =
            if encryption_state == EncryptionState::StandardSecurityDetected {
                let encrypt = store.trailer().get("Encrypt").cloned();
                encrypt
                    .and_then(|value| store.resolve_object(&value).ok())
                    .and_then(|value| value.as_dict().cloned())
                    .is_some_and(|dict| {
                        let expected_length = match standard_revision {
                            Some(StandardRevision::R2) => 40,
                            Some(StandardRevision::R3 | StandardRevision::R4) => 128,
                            Some(StandardRevision::R5 | StandardRevision::R6) => 256,
                            _ => return false,
                        };
                        let key_length = encryption_length.unwrap_or(expected_length);
                        let owner_len = dict
                            .get("O")
                            .and_then(PdfObject::as_bytes)
                            .map_or(0, <[u8]>::len);
                        let user_len = dict
                            .get("U")
                            .and_then(PdfObject::as_bytes)
                            .map_or(0, <[u8]>::len);
                        let version_matches = matches!(
                            (standard_revision, encryption_version),
                            (Some(StandardRevision::R2), Some(1))
                                | (Some(StandardRevision::R3), Some(2))
                                | (Some(StandardRevision::R4), Some(4))
                                | (Some(StandardRevision::R5 | StandardRevision::R6), Some(5))
                        );
                        let modern_entries = if matches!(
                            standard_revision,
                            Some(StandardRevision::R5 | StandardRevision::R6)
                        ) {
                            owner_len >= 48
                                && user_len >= 48
                                && dict
                                    .get("OE")
                                    .and_then(PdfObject::as_bytes)
                                    .is_some_and(|value| value.len() == 32)
                                && dict
                                    .get("UE")
                                    .and_then(PdfObject::as_bytes)
                                    .is_some_and(|value| value.len() == 32)
                                && dict
                                    .get("Perms")
                                    .and_then(PdfObject::as_bytes)
                                    .is_some_and(|value| value.len() == 16)
                        } else {
                            owner_len >= 32 && user_len >= 32
                        };
                        permissions.raw.is_some()
                            && document_id_valid
                            && store.trailer().contains_key("ID")
                            && version_matches
                            && key_length == expected_length
                            && modern_entries
                    })
            } else {
                encryption_state == EncryptionState::NotEncrypted
            };
        let signed_revision_end = byte_ranges
            .iter()
            .flat_map(|range| range.iter())
            .filter_map(|segment| segment.offset.checked_add(segment.length))
            .max();
        let later_revision_present = signed_revision_end.is_some_and(|end| {
            store
                .xref()
                .revisions
                .iter()
                .any(|revision| revision.xref_offset >= end)
        });
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
            standard_revision,
            encryption_dictionary_valid,
            encryption_length,
            permissions,
            mutation_allowed,
            mutation_reason_code,
            signed_mutation_state,
            document_id_valid,
            signed_revision_end,
            later_revision_present,
        })
    }

    /// Accepts the credential type without storing or logging the secret. A
    /// decryptor must be qualified before any password can grant access.
    pub fn authorize_credential(
        &self,
        _kind: CredentialKind,
        _password: &[u8],
    ) -> Result<(), CredentialRefusal> {
        Err(match self.encryption_state {
            EncryptionState::NotEncrypted => CredentialRefusal::NoEncryption,
            EncryptionState::MalformedEncryptionDictionary => {
                CredentialRefusal::MalformedEncryption
            }
            EncryptionState::StandardSecurityDetected if !self.encryption_dictionary_valid => {
                CredentialRefusal::MalformedEncryption
            }
            EncryptionState::StandardSecurityDetected
                if self.standard_revision == Some(StandardRevision::Unsupported) =>
            {
                CredentialRefusal::UnsupportedRevision
            }
            EncryptionState::StandardSecurityDetected => CredentialRefusal::DecryptionUnavailable,
            _ => CredentialRefusal::UnsupportedHandler,
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
