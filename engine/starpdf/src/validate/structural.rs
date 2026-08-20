use crate::document::document::PdfDocument;
use crate::error::{PdfError, PdfResult};

pub struct StructuralValidator;

impl StructuralValidator {
    /// Validates basic structural soundness of a parsed PDF document.
    pub fn validate(doc: &mut PdfDocument<'_>) -> PdfResult<()> {
        // 1. Verify trailer
        let trailer = doc.trailer();
        if !trailer.contains_key("Root") {
            return Err(PdfError::InvalidSyntax(
                "Trailer missing /Root entry".into(),
            ));
        }

        // 2. Verify page count is positive
        let pages = doc.page_count()?;
        if pages == 0 {
            return Err(PdfError::InvalidSyntax("Document contains 0 pages".into()));
        }

        // 3. Verify each page can be resolved
        for i in 0..pages {
            let _ = doc.page_dict(i)?;
        }

        Ok(())
    }
}
