pub mod index;
pub mod matcher;
pub mod result;

pub use index::{DocumentSearchIndex, PageSearchIndex};
pub use matcher::TextMatcher;
pub use result::{SearchBoundingBox, SearchOptions, SearchResult};
