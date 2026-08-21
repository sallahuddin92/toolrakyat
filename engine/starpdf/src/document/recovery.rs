use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
pub enum RecoveryKind {
    None,
    XrefRecovered,
    StreamLengthReconciled,
    OptionalEntryDefaulted,
    ProducerCompatibilityPath,
    UnsupportedStructure,
    MalformedDocument,
}

impl RecoveryKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::XrefRecovered => "XREF_RECOVERED",
            Self::StreamLengthReconciled => "STREAM_LENGTH_RECONCILED",
            Self::OptionalEntryDefaulted => "OPTIONAL_ENTRY_DEFAULTED",
            Self::ProducerCompatibilityPath => "PRODUCER_COMPATIBILITY_PATH",
            Self::UnsupportedStructure => "UNSUPPORTED_STRUCTURE",
            Self::MalformedDocument => "MALFORMED_DOCUMENT",
        }
    }
}

impl fmt::Display for RecoveryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
pub struct RecoveryEvent {
    pub kind: RecoveryKind,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
pub struct RecoveryTracker {
    pub events: Vec<RecoveryEvent>,
}

impl RecoveryTracker {
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, kind: RecoveryKind, description: impl Into<String>) {
        self.events.push(RecoveryEvent {
            kind,
            description: description.into(),
        });
    }

    pub fn primary_status(&self) -> RecoveryKind {
        if self.events.is_empty() {
            return RecoveryKind::None;
        }

        // Return highest severity encountered
        if self
            .events
            .iter()
            .any(|e| e.kind == RecoveryKind::MalformedDocument)
        {
            return RecoveryKind::MalformedDocument;
        }
        if self
            .events
            .iter()
            .any(|e| e.kind == RecoveryKind::UnsupportedStructure)
        {
            return RecoveryKind::UnsupportedStructure;
        }
        if self
            .events
            .iter()
            .any(|e| e.kind == RecoveryKind::XrefRecovered)
        {
            return RecoveryKind::XrefRecovered;
        }
        if self
            .events
            .iter()
            .any(|e| e.kind == RecoveryKind::StreamLengthReconciled)
        {
            return RecoveryKind::StreamLengthReconciled;
        }
        if self
            .events
            .iter()
            .any(|e| e.kind == RecoveryKind::ProducerCompatibilityPath)
        {
            return RecoveryKind::ProducerCompatibilityPath;
        }
        if self
            .events
            .iter()
            .any(|e| e.kind == RecoveryKind::OptionalEntryDefaulted)
        {
            return RecoveryKind::OptionalEntryDefaulted;
        }

        RecoveryKind::None
    }
}
