pub mod resolver;
pub mod stream;
pub mod table;

pub use resolver::XrefResolver;
pub use stream::XrefStreamParser;
pub use table::{XrefEntry, XrefStatus, XrefTable};
