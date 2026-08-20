#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::manual_let_else,
    clippy::match_same_arms,
    clippy::uninlined_format_args,
    clippy::write_with_newline,
    clippy::needless_continue,
    clippy::checked_conversions,
    clippy::redundant_closure_for_method_calls,
    clippy::redundant_else,
    clippy::module_inception,
    clippy::missing_panics_doc,
    clippy::approx_constant,
    clippy::doc_markdown
)]

pub mod content;
pub mod document;
pub mod error;
pub mod io;
pub mod syntax;
pub mod validate;
pub mod writer;
pub mod xref;

pub use content::{ContentInstruction, ContentOperand, ContentOperator, ContentParser};
pub use document::{ObjectStore, ObjectStoreMetrics, PageTree, PdfDocument};
pub use error::{PdfError, PdfResult};
pub use io::{ByteCursor, ByteSource};
pub use syntax::{Lexer, ObjectRef, Parser, PdfObject, StreamObject, Token};
pub use validate::StructuralValidator;
pub use writer::{MinimalWriter, Serializer};
pub use xref::{XrefEntry, XrefResolver, XrefTable};
