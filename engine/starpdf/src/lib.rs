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
    clippy::let_unit_value,
    clippy::many_single_char_names,
    clippy::unused_self,
    clippy::items_after_statements,
    clippy::manual_is_multiple_of,
    clippy::return_self_not_must_use,
    clippy::too_many_arguments,
    clippy::derivable_impls,
    clippy::unnecessary_wraps,
    clippy::needless_range_loop,
    clippy::doc_markdown
)]

pub mod annotation;
pub mod appearance;
pub mod content;
pub mod document;
pub mod error;
pub mod filter;
pub mod font;
pub mod forms;
pub mod io;
pub mod mutation;
pub mod page_ops;
pub mod search;
pub mod security;
pub mod syntax;
pub mod text;
pub mod validate;
#[cfg(feature = "wasm")]
pub mod wasm;
pub mod writer;
pub mod xref;

pub use annotation::{
    Annotation, AnnotationGenerator, AnnotationParser, AnnotationSpec, AnnotationSubtype,
    AnnotationUpdateSpec,
};
pub use appearance::{AppearanceGenerator, AppearanceStatus, DefaultAppearance, PdfColor};
pub use content::{ContentInstruction, ContentOperand, ContentOperator, ContentParser};
pub use document::{
    DecodedObjectStream, ObjectStore, ObjectStoreMetrics, ObjectStreamReader, PageTree, PdfDocument,
};
pub use error::{PdfError, PdfResult};
pub use filter::{DecompressLimits, FlateDecoder, PredictorDecoder, PredictorParams};
pub use font::{Font, PageResources, SimpleEncoding, UnicodeCMap};
pub use forms::{
    AcroForm, AcroFormParser, ChoiceOption, FieldType, FieldValue, FormField, WidgetAnnotation,
};
pub use io::{ByteCursor, ByteSource};
pub use mutation::{MutationEngine, MutationPlan, PdfChange};
pub use page_ops::{
    DependencyDisposition, DocumentBuilder, DocumentWriteMode, IncrementalPageEditor, PageEdit,
    PageOperationLimits, PageOperationPlan, PageOperationType,
};
pub use search::{
    DocumentSearchIndex, PageSearchIndex, SearchBoundingBox, SearchOptions, SearchResult,
    TextMatcher,
};
pub use syntax::{Lexer, ObjectRef, Parser, PdfObject, StreamObject, Token};
pub use text::{GraphicsState, Matrix2D, PageText, TextExtractor, TextSpan, TextState};
pub use validate::StructuralValidator;
pub use writer::{CompleteWriter, IncrementalWriter, MinimalWriter, Serializer};
pub use xref::{XrefEntry, XrefResolver, XrefStreamParser, XrefTable};
