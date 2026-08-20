pub mod generator;
pub mod parser;
pub mod types;

pub use generator::AnnotationGenerator;
pub use parser::AnnotationParser;
pub use types::{
    Annotation, AnnotationSpec, AnnotationSubtype, AnnotationUpdateSpec, LineEndingStyle,
};
