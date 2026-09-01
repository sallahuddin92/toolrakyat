use std::collections::BTreeMap;

use crate::appearance::AppearanceStatus;
use crate::content::operator::ContentOperator;
use crate::content::source::{
    decode_content_stream, scan_instruction_sources, write_uncompressed_content,
    ContentInstructionSource,
};
use crate::document::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::image::extractor::ImageExtractor;
use crate::image::jpeg::parse_jpeg_info;
use crate::image::types::{
    AddImageSpec, ImageFormat, ImageXObjectInfo, RemoveImageSpec, ReplaceImageSpec,
};
use crate::mutation::MutationPlan;
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};

pub struct ImageEditor;

impl ImageEditor {
    /// Replaces an existing Image XObject's content stream and dictionary.
    pub fn replace_image(
        store: &mut ObjectStore,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        spec: &ReplaceImageSpec,
    ) -> PdfResult<MutationPlan> {
        let target_image = {
            let mut extractor = ImageExtractor::from_store(store, page_refs);
            let images = extractor.extract_page_images(spec.page_index)?;

            images
                .into_iter()
                .find(|img| img.image_id == spec.image_id)
                .ok_or_else(|| {
                    PdfError::ImageNotFound(format!(
                        "Image with ID '{}' not found on page {}",
                        spec.image_id, spec.page_index
                    ))
                })?
        };

        if target_image.is_nested_form {
            return Err(PdfError::NestedFormXObjectRefusal(format!(
                "Image '{}' is nested inside a Form XObject. Bounded v0.14 policy refuses in-place mutation of nested Form XObjects.",
                spec.image_id
            )));
        }

        let is_shared = {
            let mut extractor = ImageExtractor::from_store(store, page_refs);
            let all_images = extractor.extract_all_images()?;
            all_images
                .iter()
                .filter(|img| img.object_ref == target_image.object_ref)
                .count()
                > 1
        };

        let payload = Self::prepare_image_payload(&spec.new_image_bytes, &spec.format)?;

        let mut modified_objects = BTreeMap::new();

        if is_shared && !spec.clone_if_shared {
            return Err(PdfError::InvalidOperation(
                "SHARED_IMAGE_REPLACEMENT_REFUSED: occurrence isolation requires cloning"
                    .to_string(),
            ));
        }

        if is_shared && spec.clone_if_shared {
            let new_img_num = *next_alloc_obj_num;
            *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
            let new_img_ref = ObjectRef::new(new_img_num, 0);

            let mut dict = BTreeMap::new();
            dict.insert("Type".to_string(), PdfObject::Name("XObject".to_string()));
            dict.insert("Subtype".to_string(), PdfObject::Name("Image".to_string()));
            dict.insert(
                "Width".to_string(),
                PdfObject::Integer(i64::from(payload.width)),
            );
            dict.insert(
                "Height".to_string(),
                PdfObject::Integer(i64::from(payload.height)),
            );
            dict.insert(
                "ColorSpace".to_string(),
                PdfObject::Name(payload.color_space),
            );
            dict.insert(
                "BitsPerComponent".to_string(),
                PdfObject::Integer(i64::from(payload.bits_per_component)),
            );
            if let Some(f) = payload.filter {
                dict.insert("Filter".to_string(), PdfObject::Name(f));
            }
            dict.insert(
                "Length".to_string(),
                PdfObject::Integer(payload.stream_data.len() as i64),
            );

            let new_stream = StreamObject {
                dict,
                data: payload.stream_data,
                stream_offset: 0,
                stream_length: 0,
            };
            modified_objects.insert(new_img_ref, PdfObject::Stream(new_stream));

            // Register a unique resource name and rewrite only the selected invocation. Rebinding
            // the existing name would replace every occurrence on this page.
            let page_ref = page_refs[spec.page_index];
            let page_obj = store.resolve(page_ref)?.clone();
            let mut page_dict =
                page_obj
                    .as_dict()
                    .cloned()
                    .ok_or_else(|| PdfError::TypeMismatch {
                        expected: "dictionary",
                        actual: page_obj.type_name(),
                    })?;

            let mut res_dict = match page_dict.get("Resources") {
                Some(PdfObject::Dictionary(d)) => d.clone(),
                Some(PdfObject::Reference(r)) => {
                    store.resolve(*r)?.as_dict().cloned().unwrap_or_default()
                }
                _ => BTreeMap::new(),
            };

            let mut xo_dict = match res_dict.get("XObject") {
                Some(PdfObject::Dictionary(d)) => d.clone(),
                Some(PdfObject::Reference(r)) => {
                    store.resolve(*r)?.as_dict().cloned().unwrap_or_default()
                }
                _ => BTreeMap::new(),
            };

            let new_resource_name = format!("Im_star_{new_img_num}");
            xo_dict.insert(new_resource_name.clone(), PdfObject::Reference(new_img_ref));
            res_dict.insert("XObject".to_string(), PdfObject::Dictionary(xo_dict));
            page_dict.insert("Resources".to_string(), PdfObject::Dictionary(res_dict));

            let contents_ref =
                Self::get_page_content_stream_ref(&page_dict, target_image.stream_index)?;
            let mut contents_stream = store
                .resolve(contents_ref)?
                .as_stream()
                .cloned()
                .ok_or_else(|| {
                    PdfError::InvalidObject("Resolved object is not a stream".to_string())
                })?;
            let decoded = decode_content_stream(&contents_stream)?;
            let source = Self::locate_image_invocation(&decoded, &target_image)?;
            let replacement = format!("/{new_resource_name} Do");
            let mut updated = Vec::with_capacity(decoded.len() + replacement.len());
            updated.extend_from_slice(&decoded[..source.byte_start]);
            updated.extend_from_slice(replacement.as_bytes());
            updated.extend_from_slice(&decoded[source.byte_end..]);
            write_uncompressed_content(&mut contents_stream, updated);
            let (content_changes, _) = Self::place_content_stream_mutation(
                store,
                page_refs,
                next_alloc_obj_num,
                &mut page_dict,
                contents_ref,
                target_image.stream_index,
                contents_stream,
            )?;
            modified_objects.extend(content_changes);
            modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else {
            let mut existing_stream = store
                .resolve(target_image.object_ref)?
                .as_stream()
                .cloned()
                .ok_or_else(|| {
                    PdfError::InvalidObject("Resolved object is not a stream".to_string())
                })?;

            existing_stream.dict.insert(
                "Width".to_string(),
                PdfObject::Integer(i64::from(payload.width)),
            );
            existing_stream.dict.insert(
                "Height".to_string(),
                PdfObject::Integer(i64::from(payload.height)),
            );
            existing_stream.dict.insert(
                "ColorSpace".to_string(),
                PdfObject::Name(payload.color_space),
            );
            existing_stream.dict.insert(
                "BitsPerComponent".to_string(),
                PdfObject::Integer(i64::from(payload.bits_per_component)),
            );
            if let Some(f) = payload.filter {
                existing_stream
                    .dict
                    .insert("Filter".to_string(), PdfObject::Name(f));
            } else {
                existing_stream.dict.remove("Filter");
            }
            existing_stream.dict.insert(
                "Length".to_string(),
                PdfObject::Integer(payload.stream_data.len() as i64),
            );
            existing_stream.data = payload.stream_data;
            existing_stream.stream_length = existing_stream.data.len();

            modified_objects.insert(target_image.object_ref, PdfObject::Stream(existing_stream));
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    /// Adds a new image to a page at specified coordinates and display size.
    pub fn add_image(
        store: &mut ObjectStore,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        spec: &AddImageSpec,
    ) -> PdfResult<MutationPlan> {
        let payload = Self::prepare_image_payload(&spec.image_bytes, &spec.format)?;

        let mut modified_objects = BTreeMap::new();

        // 1. Create new Image XObject
        let new_img_num = *next_alloc_obj_num;
        *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
        let new_img_ref = ObjectRef::new(new_img_num, 0);

        let mut dict = BTreeMap::new();
        dict.insert("Type".to_string(), PdfObject::Name("XObject".to_string()));
        dict.insert("Subtype".to_string(), PdfObject::Name("Image".to_string()));
        dict.insert(
            "Width".to_string(),
            PdfObject::Integer(i64::from(payload.width)),
        );
        dict.insert(
            "Height".to_string(),
            PdfObject::Integer(i64::from(payload.height)),
        );
        dict.insert(
            "ColorSpace".to_string(),
            PdfObject::Name(payload.color_space),
        );
        dict.insert(
            "BitsPerComponent".to_string(),
            PdfObject::Integer(i64::from(payload.bits_per_component)),
        );
        if let Some(f) = payload.filter {
            dict.insert("Filter".to_string(), PdfObject::Name(f));
        }
        dict.insert(
            "Length".to_string(),
            PdfObject::Integer(payload.stream_data.len() as i64),
        );

        let new_stream = StreamObject {
            dict,
            data: payload.stream_data,
            stream_offset: 0,
            stream_length: 0,
        };
        modified_objects.insert(new_img_ref, PdfObject::Stream(new_stream));

        // 2. Register unique resource name in page /Resources /XObject
        let page_ref = page_refs[spec.page_index];
        let page_obj = store.resolve(page_ref)?.clone();
        let mut page_dict = page_obj
            .as_dict()
            .cloned()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: page_obj.type_name(),
            })?;

        let mut res_dict = match page_dict.get("Resources") {
            Some(PdfObject::Dictionary(d)) => d.clone(),
            Some(PdfObject::Reference(r)) => {
                store.resolve(*r)?.as_dict().cloned().unwrap_or_default()
            }
            _ => BTreeMap::new(),
        };

        let mut xo_dict = match res_dict.get("XObject") {
            Some(PdfObject::Dictionary(d)) => d.clone(),
            Some(PdfObject::Reference(r)) => {
                store.resolve(*r)?.as_dict().cloned().unwrap_or_default()
            }
            _ => BTreeMap::new(),
        };

        let res_name = format!("Im_star_{new_img_num}");
        xo_dict.insert(res_name.clone(), PdfObject::Reference(new_img_ref));
        res_dict.insert("XObject".to_string(), PdfObject::Dictionary(xo_dict));
        page_dict.insert("Resources".to_string(), PdfObject::Dictionary(res_dict));

        // 3. Append drawing operator to page content stream: q {width} 0 0 {height} {x} {y} cm /{res_name} Do Q\n
        let draw_cmd = format!(
            "\nq\n{:.4} 0 0 {:.4} {:.4} {:.4} cm\n/{} Do\nQ\n",
            spec.width, spec.height, spec.x, spec.y, res_name
        );

        let add_stream_ref = ObjectRef::new(*next_alloc_obj_num, 0);
        *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
        let mut stream_dict = BTreeMap::new();
        stream_dict.insert(
            "Length".to_string(),
            PdfObject::Integer(draw_cmd.len() as i64),
        );
        modified_objects.insert(
            add_stream_ref,
            PdfObject::Stream(StreamObject {
                dict: stream_dict,
                data: draw_cmd.into_bytes(),
                stream_offset: 0,
                stream_length: 0,
            }),
        );
        let mut contents = match page_dict.get("Contents").cloned() {
            Some(PdfObject::Reference(reference)) => vec![PdfObject::Reference(reference)],
            Some(PdfObject::Array(items)) => items,
            Some(_) => {
                return Err(PdfError::InvalidOperation(
                    "IMAGE_CONTENT_STREAM_REFUSED: page contents are not indirect streams"
                        .to_string(),
                ));
            }
            None => Vec::new(),
        };
        contents.push(PdfObject::Reference(add_stream_ref));
        page_dict.insert("Contents".to_string(), PdfObject::Array(contents));

        modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    /// Removes an image drawing instruction from a page content stream.
    pub fn remove_image(
        store: &mut ObjectStore,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        spec: &RemoveImageSpec,
    ) -> PdfResult<MutationPlan> {
        let target_image = {
            let mut extractor = ImageExtractor::from_store(store, page_refs);
            let images = extractor.extract_page_images(spec.page_index)?;

            images
                .into_iter()
                .find(|img| img.image_id == spec.image_id)
                .ok_or_else(|| {
                    PdfError::ImageNotFound(format!(
                        "Image with ID '{}' not found on page {}",
                        spec.image_id, spec.page_index
                    ))
                })?
        };

        if target_image.is_nested_form {
            return Err(PdfError::NestedFormXObjectRefusal(format!(
                "Image '{}' is nested inside a Form XObject. Bounded v0.14 policy refuses in-place removal from nested Form XObjects.",
                spec.image_id
            )));
        }

        let page_ref = page_refs[spec.page_index];
        let page_obj = store.resolve(page_ref)?.clone();
        let mut page_dict = page_obj
            .as_dict()
            .cloned()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: page_obj.type_name(),
            })?;

        let contents_ref =
            Self::get_page_content_stream_ref(&page_dict, target_image.stream_index)?;

        let mut stream_obj = store
            .resolve(contents_ref)?
            .as_stream()
            .cloned()
            .ok_or_else(|| {
                PdfError::InvalidObject("Resolved object is not a stream".to_string())
            })?;

        let decoded = decode_content_stream(&stream_obj)?;
        let target = Self::locate_image_invocation(&decoded, &target_image)?;
        let mut updated = Vec::with_capacity(decoded.len() - (target.byte_end - target.byte_start));
        updated.extend_from_slice(&decoded[..target.byte_start]);
        updated.extend_from_slice(&decoded[target.byte_end..]);
        write_uncompressed_content(&mut stream_obj, updated);

        let (mut modified_objects, cloned_stream) = Self::place_content_stream_mutation(
            store,
            page_refs,
            next_alloc_obj_num,
            &mut page_dict,
            contents_ref,
            target_image.stream_index,
            stream_obj,
        )?;
        if cloned_stream {
            modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    /// Moves and/or resizes an existing image occurrence on a page content stream.
    pub fn update_image(
        store: &mut ObjectStore,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        spec: &crate::image::types::UpdateImageSpec,
    ) -> PdfResult<MutationPlan> {
        let target_image = {
            let mut extractor = ImageExtractor::from_store(store, page_refs);
            let images = extractor.extract_page_images(spec.page_index)?;

            images
                .into_iter()
                .find(|img| img.image_id == spec.image_id)
                .ok_or_else(|| {
                    PdfError::ImageNotFound(format!(
                        "Image with ID '{}' not found on page {}",
                        spec.image_id, spec.page_index
                    ))
                })?
        };

        if target_image.is_nested_form {
            return Err(PdfError::NestedFormXObjectRefusal(format!(
                "Image '{}' is nested inside a Form XObject. Bounded policy refuses in-place mutation of nested Form XObjects.",
                spec.image_id
            )));
        }

        let page_ref = page_refs[spec.page_index];
        let page_obj = store.resolve(page_ref)?.clone();
        let mut page_dict = page_obj
            .as_dict()
            .cloned()
            .ok_or_else(|| PdfError::TypeMismatch {
                expected: "dictionary",
                actual: page_obj.type_name(),
            })?;

        let contents_ref =
            Self::get_page_content_stream_ref(&page_dict, target_image.stream_index)?;

        let mut stream_obj = store
            .resolve(contents_ref)?
            .as_stream()
            .cloned()
            .ok_or_else(|| {
                PdfError::InvalidObject("Resolved object is not a stream".to_string())
            })?;

        let decoded = decode_content_stream(&stream_obj)?;
        let instructions = scan_instruction_sources(&decoded)?;
        let target_idx = target_image.instruction_index;
        let target = instructions.get(target_idx).ok_or_else(|| {
            PdfError::ImageNotFound(format!(
                "Image occurrence '{}' no longer resolves to its source instruction",
                target_image.image_id
            ))
        })?;
        if target.resource_name() != Some(target_image.resource_name.as_str()) {
            return Err(PdfError::ImageNotFound(format!(
                "Image occurrence '{}' source identity changed",
                target_image.image_id
            )));
        }
        let is_isolated_block = target_idx >= 2
            && target_idx + 1 < instructions.len()
            && instructions[target_idx - 2].instruction.operator == ContentOperator::Q
            && instructions[target_idx - 1].instruction.operator == ContentOperator::Cm
            && target.instruction.operator == ContentOperator::Do
            && instructions[target_idx + 1].instruction.operator == ContentOperator::QEnd;
        if !is_isolated_block {
            return Err(PdfError::InvalidOperation(format!(
                "IMAGE_TRANSFORM_SOURCE_REFUSED: image '{}' is not controlled by an isolated q/cm/Do/Q block",
                target_image.image_id
            )));
        }
        let cm = &instructions[target_idx - 1];
        let replacement = format!(
            "{:.4} 0 0 {:.4} {:.4} {:.4} cm",
            spec.new_width, spec.new_height, spec.new_x, spec.new_y
        );
        let mut updated = Vec::with_capacity(decoded.len() + replacement.len());
        updated.extend_from_slice(&decoded[..cm.byte_start]);
        updated.extend_from_slice(replacement.as_bytes());
        updated.extend_from_slice(&decoded[cm.byte_end..]);
        write_uncompressed_content(&mut stream_obj, updated);

        let (mut modified_objects, cloned_stream) = Self::place_content_stream_mutation(
            store,
            page_refs,
            next_alloc_obj_num,
            &mut page_dict,
            contents_ref,
            target_image.stream_index,
            stream_obj,
        )?;
        if cloned_stream {
            modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    fn locate_image_invocation(
        decoded: &[u8],
        target: &ImageXObjectInfo,
    ) -> PdfResult<ContentInstructionSource> {
        let source = scan_instruction_sources(decoded)?
            .into_iter()
            .find(|source| source.instruction_index == target.instruction_index)
            .ok_or_else(|| {
                PdfError::ImageNotFound(format!(
                    "Image occurrence '{}' has no matching source instruction",
                    target.image_id
                ))
            })?;
        if source.instruction.operator != ContentOperator::Do
            || source.resource_name() != Some(target.resource_name.as_str())
        {
            return Err(PdfError::ImageNotFound(format!(
                "Image occurrence '{}' no longer matches /{} Do",
                target.image_id, target.resource_name
            )));
        }
        Ok(source)
    }

    fn place_content_stream_mutation(
        store: &mut ObjectStore,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        page_dict: &mut BTreeMap<String, PdfObject>,
        contents_ref: ObjectRef,
        stream_index: usize,
        stream_obj: StreamObject,
    ) -> PdfResult<(BTreeMap<ObjectRef, PdfObject>, bool)> {
        let mut reference_count = 0usize;
        for &other_page_ref in page_refs {
            let other_obj = store.resolve(other_page_ref)?;
            if let Some(other_dict) = other_obj.as_dict() {
                if let Some(contents) = other_dict.get("Contents") {
                    if contents.as_reference() == Some(contents_ref) {
                        reference_count += 1;
                    } else if let Some(items) = contents.as_array() {
                        reference_count += items
                            .iter()
                            .filter(|item| item.as_reference() == Some(contents_ref))
                            .count();
                    }
                }
            }
        }

        let mut modified = BTreeMap::new();
        if reference_count <= 1 {
            modified.insert(contents_ref, PdfObject::Stream(stream_obj));
            return Ok((modified, false));
        }

        let new_stream_ref = ObjectRef::new(*next_alloc_obj_num, 0);
        *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
        modified.insert(new_stream_ref, PdfObject::Stream(stream_obj));
        match page_dict.get_mut("Contents") {
            Some(PdfObject::Reference(reference)) if *reference == contents_ref => {
                *reference = new_stream_ref;
            }
            Some(PdfObject::Array(items)) => {
                let item = items.get_mut(stream_index).ok_or_else(|| {
                    PdfError::InvalidOperation(format!(
                        "IMAGE_CONTENT_STREAM_REFUSED: stream index {stream_index} disappeared"
                    ))
                })?;
                if item.as_reference() != Some(contents_ref) {
                    return Err(PdfError::InvalidOperation(
                        "IMAGE_CONTENT_STREAM_REFUSED: target stream identity changed".to_string(),
                    ));
                }
                *item = PdfObject::Reference(new_stream_ref);
            }
            _ => {
                return Err(PdfError::InvalidOperation(
                    "IMAGE_CONTENT_STREAM_REFUSED: page contents are not an indirect stream"
                        .to_string(),
                ));
            }
        }
        Ok((modified, true))
    }

    /// Resolves the specific content stream reference for a given stream index on a page.
    fn get_page_content_stream_ref(
        page_dict: &BTreeMap<String, PdfObject>,
        stream_index: usize,
    ) -> PdfResult<ObjectRef> {
        if let Some(contents_obj) = page_dict.get("Contents") {
            match contents_obj {
                PdfObject::Reference(r) => {
                    if stream_index == 0 {
                        return Ok(*r);
                    }
                }
                PdfObject::Array(arr) => {
                    if stream_index < arr.len() {
                        if let Some(r) = arr[stream_index].as_reference() {
                            return Ok(r);
                        }
                    }
                }
                _ => {}
            }
        }

        Err(PdfError::TargetTextNotFound(format!(
            "Content stream at index {stream_index} not found on page"
        )))
    }

    /// Prepares raw image bytes into PDF stream parameters.
    fn prepare_image_payload(
        bytes: &[u8],
        format: &ImageFormat,
    ) -> PdfResult<PreparedImagePayload> {
        match format {
            ImageFormat::Jpeg => {
                let info = parse_jpeg_info(bytes)?;
                Ok(PreparedImagePayload {
                    width: info.width,
                    height: info.height,
                    color_space: info.color_space,
                    bits_per_component: u32::from(info.bits_per_component),
                    filter: Some("DCTDecode".to_string()),
                    stream_data: bytes.to_vec(),
                })
            }
            ImageFormat::Flate {
                color_space,
                width,
                height,
                bits_per_component,
            } => {
                let compressed = miniz_oxide::deflate::compress_to_vec_zlib(bytes, 6);
                Ok(PreparedImagePayload {
                    width: *width,
                    height: *height,
                    color_space: color_space.clone(),
                    bits_per_component: *bits_per_component,
                    filter: Some("FlateDecode".to_string()),
                    stream_data: compressed,
                })
            }
            ImageFormat::AutoDetect => {
                if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
                    let info = parse_jpeg_info(bytes)?;
                    Ok(PreparedImagePayload {
                        width: info.width,
                        height: info.height,
                        color_space: info.color_space,
                        bits_per_component: u32::from(info.bits_per_component),
                        filter: Some("DCTDecode".to_string()),
                        stream_data: bytes.to_vec(),
                    })
                } else {
                    Err(PdfError::UnsupportedImageFormat(
                        "AutoDetect format currently requires standard JPEG/JFIF stream bytes"
                            .to_string(),
                    ))
                }
            }
        }
    }
}

struct PreparedImagePayload {
    width: u32,
    height: u32,
    color_space: String,
    bits_per_component: u32,
    filter: Option<String>,
    stream_data: Vec<u8>,
}
