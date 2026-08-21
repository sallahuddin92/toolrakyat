use std::collections::{BTreeMap, BTreeSet};

use crate::document::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::page_ops::PageOperationLimits;
use crate::syntax::{ObjectRef, PdfObject};

pub(crate) const INHERITED_PAGE_KEYS: [&str; 7] = [
    "Resources",
    "MediaBox",
    "CropBox",
    "BleedBox",
    "TrimBox",
    "ArtBox",
    "Rotate",
];

pub(crate) fn materialize_page_inheritance(
    store: &mut ObjectStore<'_>,
    page_ref: ObjectRef,
    limits: &PageOperationLimits,
) -> PdfResult<BTreeMap<String, PdfObject>> {
    let page = store.resolve(page_ref)?.clone();
    let mut page_dict = page
        .as_dict()
        .cloned()
        .ok_or_else(|| PdfError::TypeMismatch {
            expected: "page dictionary",
            actual: page.type_name(),
        })?;
    if page_dict.get("Type").and_then(PdfObject::as_name) != Some("Page") {
        return Err(PdfError::PageOperation(format!(
            "object {page_ref} is not a /Page"
        )));
    }

    let missing: Vec<&str> = INHERITED_PAGE_KEYS
        .iter()
        .copied()
        .filter(|key| !page_dict.contains_key(*key))
        .collect();
    if !missing.is_empty() {
        let mut current = page_dict.get("Parent").and_then(PdfObject::as_reference);
        let mut visited = BTreeSet::new();
        let mut depth = 0usize;
        while let Some(reference) = current {
            if depth >= limits.max_dependency_depth {
                return Err(PdfError::PageResourceLimit(format!(
                    "page inheritance depth exceeds {}",
                    limits.max_dependency_depth
                )));
            }
            if !visited.insert(reference) {
                return Err(PdfError::CircularReference(format!(
                    "cycle in page inheritance at {reference}"
                )));
            }
            let parent = store.resolve(reference)?.clone();
            let dict = parent.as_dict().ok_or_else(|| PdfError::TypeMismatch {
                expected: "page-tree ancestor dictionary",
                actual: parent.type_name(),
            })?;
            for key in &missing {
                if !page_dict.contains_key(*key) {
                    if let Some(value) = dict.get(*key) {
                        page_dict.insert((*key).to_string(), value.clone());
                    }
                }
            }
            if missing.iter().all(|key| page_dict.contains_key(*key)) {
                break;
            }
            current = dict.get("Parent").and_then(PdfObject::as_reference);
            depth = depth.checked_add(1).ok_or_else(|| {
                PdfError::PageResourceLimit("page inheritance depth overflow".into())
            })?;
        }
    }

    // Strict geometry policy:
    // A) If /MediaBox exists on page -> use it.
    // B) Else walk valid inherited /Pages ancestors -> already done above.
    // C) Else derive geometry if /CropBox, /TrimBox, /BleedBox, or /ArtBox is present and valid.
    // D) Otherwise fail with typed refusal (no silent Letter assumption).
    if !page_dict.contains_key("MediaBox") {
        for fallback_box_key in ["CropBox", "TrimBox", "BleedBox", "ArtBox"] {
            if let Some(box_obj) = page_dict.get(fallback_box_key).cloned() {
                validate_box(fallback_box_key, &box_obj)?;
                page_dict.insert("MediaBox".to_string(), box_obj);
                break;
            }
        }
    }

    validate_page_geometry(&page_dict, limits)?;
    validate_page_resources(&page_dict, store, limits)?;
    validate_page_annotations(&page_dict, store, limits)?;
    Ok(page_dict)
}

fn validate_page_annotations(
    page_dict: &BTreeMap<String, PdfObject>,
    store: &mut ObjectStore<'_>,
    limits: &PageOperationLimits,
) -> PdfResult<()> {
    let Some(annotations) = page_dict.get("Annots") else {
        return Ok(());
    };
    let annotations = store.resolve_object(annotations)?;
    let items = annotations
        .as_array()
        .ok_or_else(|| PdfError::PageOperation("page /Annots must resolve to an array".into()))?;
    if items.len() > limits.max_annotations_per_page {
        return Err(PdfError::PageResourceLimit(format!(
            "page annotations exceed {} entries",
            limits.max_annotations_per_page
        )));
    }
    Ok(())
}

pub(crate) fn validate_blank_geometry(width: f64, height: f64, rotation: i32) -> PdfResult<()> {
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || width > 20_000.0
        || height > 20_000.0
    {
        return Err(PdfError::PageOperation(
            "blank page dimensions must be finite, positive, and at most 20000 points".into(),
        ));
    }
    validate_rotation(rotation)
}

fn validate_page_geometry(
    page_dict: &BTreeMap<String, PdfObject>,
    _limits: &PageOperationLimits,
) -> PdfResult<()> {
    let media_box = page_dict.get("MediaBox").ok_or_else(|| {
        PdfError::PageOperation("page has no direct or inherited /MediaBox".into())
    })?;
    validate_box("MediaBox", media_box)?;
    for key in ["CropBox", "BleedBox", "TrimBox", "ArtBox"] {
        if let Some(value) = page_dict.get(key) {
            validate_box(key, value)?;
        }
    }
    if let Some(rotation) = page_dict.get("Rotate") {
        let value = rotation
            .as_integer()
            .ok_or_else(|| PdfError::PageOperation("page /Rotate must be an integer".into()))?;
        let value = i32::try_from(value)
            .map_err(|_| PdfError::PageOperation("page /Rotate is out of range".into()))?;
        validate_rotation(value)?;
    }
    Ok(())
}

fn validate_box(name: &str, value: &PdfObject) -> PdfResult<()> {
    let values = value
        .as_array()
        .ok_or_else(|| PdfError::PageOperation(format!("page /{name} must be an array")))?;
    if values.len() != 4 {
        return Err(PdfError::PageOperation(format!(
            "page /{name} must contain four numbers"
        )));
    }
    let mut numbers = [0.0; 4];
    for (index, item) in values.iter().enumerate() {
        numbers[index] = item.as_real().ok_or_else(|| {
            PdfError::PageOperation(format!("page /{name} contains a non-number"))
        })?;
        if !numbers[index].is_finite() {
            return Err(PdfError::PageOperation(format!(
                "page /{name} contains a non-finite number"
            )));
        }
    }
    let width = numbers[2] - numbers[0];
    let height = numbers[3] - numbers[1];
    if width <= 0.0 || height <= 0.0 || width > 200_000.0 || height > 200_000.0 {
        return Err(PdfError::PageOperation(format!(
            "page /{name} dimensions are invalid or exceed 200000 points"
        )));
    }
    Ok(())
}

fn validate_rotation(rotation: i32) -> PdfResult<()> {
    if !matches!(rotation, 0 | 90 | 180 | 270) {
        return Err(PdfError::PageOperation(
            "page rotation must be one of 0, 90, 180, or 270".into(),
        ));
    }
    Ok(())
}

fn validate_page_resources(
    page_dict: &BTreeMap<String, PdfObject>,
    store: &mut ObjectStore<'_>,
    limits: &PageOperationLimits,
) -> PdfResult<()> {
    let Some(resources) = page_dict.get("Resources") else {
        return Ok(());
    };
    let resources = store.resolve_object(resources)?;
    let Some(dict) = resources.as_dict() else {
        return Err(PdfError::TypeMismatch {
            expected: "page resources dictionary",
            actual: resources.type_name(),
        });
    };
    if dict.len() > limits.max_resources_per_page {
        return Err(PdfError::PageResourceLimit(format!(
            "page resources exceed {} entries",
            limits.max_resources_per_page
        )));
    }
    Ok(())
}
