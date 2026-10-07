//! Phase 0 contracts. No production mutation or renderer uses this shadow layer.
pub mod coordinates;
pub mod delta;
pub mod evidence;
pub mod hash;
pub mod identity;
pub mod renderer;
pub mod state;

pub use coordinates::*;
pub use delta::*;
pub use evidence::*;
pub use hash::*;
pub use identity::*;
pub use renderer::*;
pub use state::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    InvalidIdentity,
    InvalidCoordinates,
    InvalidHashInput,
    InsufficientEvidence,
    StaleRevision,
    AmbiguousObjectState,
    InvalidDelta,
    IncompleteClosure,
    ForbiddenDependency,
    UnexplainedChange,
    UnsupportedOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractRefusal {
    pub code: RefusalCode,
    pub reason: String,
}

impl ContractRefusal {
    pub fn new(code: RefusalCode, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for ContractRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.reason)
    }
}

impl std::error::Error for ContractRefusal {}

pub type ContractResult<T> = Result<T, ContractRefusal>;
