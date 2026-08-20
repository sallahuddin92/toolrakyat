use std::collections::BTreeMap;

use crate::document::{PageTree, PdfDocument};
use crate::error::{PdfError, PdfResult};
use crate::page_ops::inherited::{materialize_page_inheritance, validate_blank_geometry};
use crate::page_ops::{PageEdit, PageOperationLimits, PageOperationPlan};
use crate::security::{EncryptionState, SignatureState};
use crate::syntax::{ObjectRef, PdfObject};
use crate::validate::StructuralValidator;
use crate::writer::IncrementalWriter;

enum PlannedPage {
    Existing(ObjectRef),
    Blank {
        reference: ObjectRef,
        width: f64,
        height: f64,
        rotation: i32,
    },
}

pub struct IncrementalPageEditor;

impl IncrementalPageEditor {
    pub fn apply(
        document: &mut PdfDocument<'_>,
        plan: &PageOperationPlan,
        limits: &PageOperationLimits,
    ) -> PdfResult<Vec<u8>> {
        Self::ensure_allowed(document)?;
        if plan.edits.is_empty() {
            return Ok(document.source().as_bytes().to_vec());
        }
        let root_pages_ref = document.root_pages_ref();
        let page_refs = PageTree::validate_and_collect(document.store_mut(), root_pages_ref)?;
        let mut pages: Vec<PlannedPage> = page_refs
            .iter()
            .copied()
            .map(PlannedPage::Existing)
            .collect();
        let mut next_object = document
            .store()
            .xref()
            .entries
            .keys()
            .copied()
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| PdfError::PageResourceLimit("object number overflow".into()))?;
        let mut changed = false;

        for edit in &plan.edits {
            match *edit {
                PageEdit::DeletePage { index } => {
                    if pages.len() <= 1 {
                        return Err(PdfError::PageOperation(
                            "deleting the last page is refused; StarPDF documents must retain at least one page"
                                .into(),
                        ));
                    }
                    let removed = pages.get(index).ok_or(PdfError::PageNotFound(index))?;
                    let PlannedPage::Existing(removed_ref) = removed else {
                        pages.remove(index);
                        changed = true;
                        continue;
                    };
                    Self::ensure_page_has_no_widgets(document, *removed_ref)?;
                    Self::ensure_catalog_has_no_page_navigation(document)?;
                    pages.remove(index);
                    changed = true;
                }
                PageEdit::MovePage {
                    from_index,
                    to_index,
                } => {
                    if from_index >= pages.len() {
                        return Err(PdfError::PageNotFound(from_index));
                    }
                    if to_index >= pages.len() {
                        return Err(PdfError::PageNotFound(to_index));
                    }
                    if from_index != to_index {
                        let page = pages.remove(from_index);
                        pages.insert(to_index, page);
                        changed = true;
                    }
                }
                PageEdit::InsertBlankPage {
                    index,
                    width,
                    height,
                    rotation,
                } => {
                    if index > pages.len() {
                        return Err(PdfError::PageNotFound(index));
                    }
                    validate_blank_geometry(width, height, rotation)?;
                    pages.insert(
                        index,
                        PlannedPage::Blank {
                            reference: ObjectRef::new(next_object, 0),
                            width,
                            height,
                            rotation,
                        },
                    );
                    next_object = next_object.checked_add(1).ok_or_else(|| {
                        PdfError::PageResourceLimit("object number overflow".into())
                    })?;
                    changed = true;
                }
                PageEdit::DuplicatePage { .. } => {
                    return Err(PdfError::UnsupportedPageDependency(
                        "duplicate-page planning requires the complete-document remapping path"
                            .into(),
                    ));
                }
            }
            if pages.len() > limits.max_total_pages {
                return Err(PdfError::PageResourceLimit(format!(
                    "page count exceeds {}",
                    limits.max_total_pages
                )));
            }
        }

        if !changed {
            return Ok(document.source().as_bytes().to_vec());
        }
        let root_pages_ref = document.root_pages_ref();
        let root = document.store_mut().resolve(root_pages_ref)?.clone();
        let mut root_dict = root
            .as_dict()
            .cloned()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "root /Pages dictionary",
                actual: root.type_name(),
            })?;
        root_dict.insert("Type".into(), PdfObject::Name("Pages".into()));
        root_dict.insert(
            "Count".into(),
            PdfObject::Integer(i64::try_from(pages.len()).map_err(|_| {
                PdfError::PageResourceLimit("page count conversion overflow".into())
            })?),
        );
        root_dict.insert(
            "Kids".into(),
            PdfObject::Array(
                pages
                    .iter()
                    .map(|page| match page {
                        PlannedPage::Existing(reference) | PlannedPage::Blank { reference, .. } => {
                            PdfObject::Reference(*reference)
                        }
                    })
                    .collect(),
            ),
        );
        root_dict.remove("Parent");

        let mut modified = BTreeMap::new();
        modified.insert(root_pages_ref, PdfObject::Dictionary(root_dict));
        for page in pages {
            match page {
                PlannedPage::Existing(reference) => {
                    let mut dict =
                        materialize_page_inheritance(document.store_mut(), reference, limits)?;
                    dict.insert("Parent".into(), PdfObject::Reference(root_pages_ref));
                    modified.insert(reference, PdfObject::Dictionary(dict));
                }
                PlannedPage::Blank {
                    reference,
                    width,
                    height,
                    rotation,
                } => {
                    let mut dict = BTreeMap::from([
                        ("Type".into(), PdfObject::Name("Page".into())),
                        ("Parent".into(), PdfObject::Reference(root_pages_ref)),
                        (
                            "MediaBox".into(),
                            PdfObject::Array(vec![
                                PdfObject::Integer(0),
                                PdfObject::Integer(0),
                                PdfObject::Real(width),
                                PdfObject::Real(height),
                            ]),
                        ),
                        ("Resources".into(), PdfObject::Dictionary(BTreeMap::new())),
                    ]);
                    if rotation != 0 {
                        dict.insert("Rotate".into(), PdfObject::Integer(i64::from(rotation)));
                    }
                    modified.insert(reference, PdfObject::Dictionary(dict));
                }
            }
        }

        let output = IncrementalWriter::write_update(
            document.source().as_bytes(),
            &modified,
            document.store().xref().startxref_offset as usize,
            document.trailer(),
        )?;
        if output.len() > limits.max_output_bytes {
            return Err(PdfError::PageResourceLimit(format!(
                "page-operation output exceeds {} bytes",
                limits.max_output_bytes
            )));
        }
        Self::verify_output(&output, modified_page_count(&modified, root_pages_ref)?)?;
        Ok(output)
    }

    fn ensure_allowed(document: &mut PdfDocument<'_>) -> PdfResult<()> {
        let security = document.security_info()?;
        if security.encryption_state != EncryptionState::NotEncrypted {
            return Err(PdfError::EncryptedDocumentUnsupported(
                "page operations do not rewrite encrypted object graphs".into(),
            ));
        }
        if security.signature_state != SignatureState::Unsigned {
            return Err(PdfError::SignatureMutationUnsupported(
                "page operations are refused for structurally signed documents".into(),
            ));
        }
        Ok(())
    }

    fn ensure_page_has_no_widgets(
        document: &mut PdfDocument<'_>,
        page_ref: ObjectRef,
    ) -> PdfResult<()> {
        let page = document.store_mut().resolve(page_ref)?.clone();
        let Some(dict) = page.as_dict() else {
            return Err(PdfError::TypeMismatch {
                expected: "page dictionary",
                actual: page.type_name(),
            });
        };
        let Some(annotations) = dict.get("Annots") else {
            return Ok(());
        };
        let annotations = document.store_mut().resolve_object(annotations)?;
        let Some(items) = annotations.as_array() else {
            return Err(PdfError::PageOperation(
                "page /Annots must resolve to an array".into(),
            ));
        };
        for item in items {
            let annotation = document.store_mut().resolve_object(item)?;
            if annotation
                .as_dict()
                .and_then(|entry| entry.get("Subtype"))
                .and_then(PdfObject::as_name)
                == Some("Widget")
            {
                return Err(PdfError::PartialFieldImport(
                    "deleting a page containing a form widget would create a partial field graph"
                        .into(),
                ));
            }
        }
        Ok(())
    }

    fn ensure_catalog_has_no_page_navigation(document: &mut PdfDocument<'_>) -> PdfResult<()> {
        let catalog_ref = document.catalog_ref();
        let catalog = document.store_mut().resolve(catalog_ref)?.clone();
        let Some(dict) = catalog.as_dict() else {
            return Err(PdfError::TypeMismatch {
                expected: "catalog dictionary",
                actual: catalog.type_name(),
            });
        };
        if ["Names", "Outlines", "OpenAction"]
            .iter()
            .any(|key| dict.contains_key(*key))
        {
            return Err(PdfError::UnsupportedPageDependency(
                "delete-page refuses catalogs with named destinations, outlines, or open actions because an excluded-page target cannot be proven absent"
                    .into(),
            ));
        }
        Ok(())
    }

    fn verify_output(bytes: &[u8], expected_pages: usize) -> PdfResult<()> {
        let mut reopened = PdfDocument::from_bytes(bytes)?;
        StructuralValidator::validate(&mut reopened)?;
        let actual = reopened.page_count()?;
        if actual != expected_pages {
            return Err(PdfError::PageOperation(format!(
                "reopened output has {actual} pages, expected {expected_pages}"
            )));
        }
        Ok(())
    }
}

fn modified_page_count(
    modified: &BTreeMap<ObjectRef, PdfObject>,
    root_pages_ref: ObjectRef,
) -> PdfResult<usize> {
    modified
        .get(&root_pages_ref)
        .and_then(PdfObject::as_dict)
        .and_then(|dict| dict.get("Count"))
        .and_then(PdfObject::as_integer)
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| PdfError::PageOperation("planned root /Count is invalid".into()))
}
