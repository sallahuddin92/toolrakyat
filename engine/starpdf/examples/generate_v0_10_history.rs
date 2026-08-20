use std::error::Error;
use std::path::PathBuf;

use starpdf::annotation::AnnotationSpec;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let input = PathBuf::from(arguments.next().ok_or("missing input PDF path")?);
    let output = PathBuf::from(arguments.next().ok_or("missing output PDF path")?);
    if arguments.next().is_some() {
        return Err("usage: generate_v0_10_history INPUT_PDF OUTPUT_PDF".into());
    }

    let original = std::fs::read(input)?;
    let first = {
        let mut document = PdfDocument::from_bytes(&original)?;
        let field = document
            .form_fields()?
            .into_iter()
            .find(|field| field.fully_qualified_name == "shared.contact")
            .ok_or("shared.contact field not found")?;
        document.mutate_and_export(&[PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: "StarPDF incremental revision one".into(),
        }])?
    };
    let second = {
        let mut document = PdfDocument::from_bytes(&first)?;
        document.mutate_and_export(&[PdfChange::AddAnnotation {
            page_index: 0,
            spec: AnnotationSpec::Square {
                rect: [400.0, 600.0, 470.0, 660.0],
                stroke_color: Some(vec![0.1, 0.35, 0.75]),
                fill_color: Some(vec![0.8, 0.9, 1.0]),
                border_width: Some(2.0),
            },
        }])?
    };
    if !first.starts_with(&original) || !second.starts_with(&first) {
        return Err("incremental prefix preservation failed".into());
    }
    std::fs::write(output, second)?;
    Ok(())
}
