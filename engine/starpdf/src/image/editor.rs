use std::collections::BTreeMap;

use crate::appearance::AppearanceStatus;
use crate::content::operand::ContentOperand;
use crate::content::operator::{ContentInstruction, ContentOperator};
use crate::content::parser::ContentParser;
use crate::document::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::image::extractor::ImageExtractor;
use crate::image::jpeg::parse_jpeg_info;
use crate::image::types::{AddImageSpec, ImageFormat, RemoveImageSpec, ReplaceImageSpec};
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

            // Update page's /Resources /XObject to point to new_img_ref
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

            xo_dict.insert(
                target_image.resource_name.clone(),
                PdfObject::Reference(new_img_ref),
            );
            res_dict.insert("XObject".to_string(), PdfObject::Dictionary(xo_dict));
            page_dict.insert("Resources".to_string(), PdfObject::Dictionary(res_dict));
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

        let contents_obj = page_dict.get("Contents").cloned();
        match contents_obj {
            Some(PdfObject::Reference(contents_ref)) => {
                let mut stream_obj = store
                    .resolve(contents_ref)?
                    .as_stream()
                    .cloned()
                    .ok_or_else(|| {
                        PdfError::InvalidObject("Contents object is not a stream".to_string())
                    })?;

                let decoded = FlateDecoder::decode(&stream_obj.data, &DecompressLimits::default())
                    .unwrap_or_else(|_| stream_obj.data.clone());

                let mut updated_data = decoded;
                updated_data.extend_from_slice(draw_cmd.as_bytes());

                stream_obj.data = updated_data;
                stream_obj.stream_length = stream_obj.data.len();
                stream_obj.dict.remove("Filter");
                stream_obj.dict.insert(
                    "Length".to_string(),
                    PdfObject::Integer(stream_obj.data.len() as i64),
                );

                modified_objects.insert(contents_ref, PdfObject::Stream(stream_obj));
            }
            Some(PdfObject::Array(mut arr)) => {
                let add_stream_num = *next_alloc_obj_num;
                *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
                let add_stream_ref = ObjectRef::new(add_stream_num, 0);

                let mut s_dict = BTreeMap::new();
                s_dict.insert(
                    "Length".to_string(),
                    PdfObject::Integer(draw_cmd.len() as i64),
                );
                let add_stream = StreamObject {
                    dict: s_dict,
                    data: draw_cmd.into_bytes(),
                    stream_offset: 0,
                    stream_length: 0,
                };
                modified_objects.insert(add_stream_ref, PdfObject::Stream(add_stream));

                arr.push(PdfObject::Reference(add_stream_ref));
                page_dict.insert("Contents".to_string(), PdfObject::Array(arr));
            }
            _ => {
                let add_stream_num = *next_alloc_obj_num;
                *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
                let add_stream_ref = ObjectRef::new(add_stream_num, 0);

                let mut s_dict = BTreeMap::new();
                s_dict.insert(
                    "Length".to_string(),
                    PdfObject::Integer(draw_cmd.len() as i64),
                );
                let add_stream = StreamObject {
                    dict: s_dict,
                    data: draw_cmd.into_bytes(),
                    stream_offset: 0,
                    stream_length: 0,
                };
                modified_objects.insert(add_stream_ref, PdfObject::Stream(add_stream));
                page_dict.insert("Contents".to_string(), PdfObject::Reference(add_stream_ref));
            }
        }

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
        let page_dict = page_obj
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

        let decoded = FlateDecoder::decode(&stream_obj.data, &DecompressLimits::default())
            .unwrap_or_else(|_| stream_obj.data.clone());

        let mut parser = ContentParser::from_bytes(&decoded);
        let instructions = parser.parse_instructions()?;

        let mut filtered_instructions = Vec::new();
        let target_idx = target_image.instruction_index;

        let is_isolated_block = target_idx >= 2
            && target_idx + 1 < instructions.len()
            && instructions[target_idx - 2].operator == ContentOperator::Q
            && instructions[target_idx - 1].operator == ContentOperator::Cm
            && instructions[target_idx].operator == ContentOperator::Do
            && instructions[target_idx + 1].operator == ContentOperator::QEnd;

        for (idx, instr) in instructions.into_iter().enumerate() {
            if is_isolated_block
                && (idx == target_idx - 2
                    || idx == target_idx - 1
                    || idx == target_idx
                    || idx == target_idx + 1)
            {
                continue;
            }
            if !is_isolated_block && idx == target_idx {
                continue;
            }
            filtered_instructions.push(instr);
        }

        let mut new_stream_bytes = Vec::new();
        for instr in filtered_instructions {
            Self::serialize_instruction(&mut new_stream_bytes, &instr)?;
        }

        stream_obj.data = new_stream_bytes;
        stream_obj.stream_length = stream_obj.data.len();
        stream_obj.dict.remove("Filter");
        stream_obj.dict.insert(
            "Length".to_string(),
            PdfObject::Integer(stream_obj.data.len() as i64),
        );

        let mut modified_objects = BTreeMap::new();
        modified_objects.insert(contents_ref, PdfObject::Stream(stream_obj));

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

        let decoded = FlateDecoder::decode(&stream_obj.data, &DecompressLimits::default())
            .unwrap_or_else(|_| stream_obj.data.clone());

        let mut parser = ContentParser::from_bytes(&decoded);
        let instructions = parser.parse_instructions()?;

        let mut updated_instructions = Vec::with_capacity(instructions.len() + 4);
        let target_idx = target_image.instruction_index;

        let is_isolated_block = target_idx >= 2
            && target_idx + 1 < instructions.len()
            && instructions[target_idx - 2].operator == ContentOperator::Q
            && instructions[target_idx - 1].operator == ContentOperator::Cm
            && instructions[target_idx].operator == ContentOperator::Do
            && instructions[target_idx + 1].operator == ContentOperator::QEnd;

        for (idx, instr) in instructions.into_iter().enumerate() {
            if is_isolated_block {
                if idx == target_idx - 2 {
                    updated_instructions.push(ContentInstruction::new(vec![], ContentOperator::Q));
                } else if idx == target_idx - 1 {
                    updated_instructions.push(ContentInstruction::new(
                        vec![
                            ContentOperand::Real(spec.new_width),
                            ContentOperand::Real(0.0),
                            ContentOperand::Real(0.0),
                            ContentOperand::Real(spec.new_height),
                            ContentOperand::Real(spec.new_x),
                            ContentOperand::Real(spec.new_y),
                        ],
                        ContentOperator::Cm,
                    ));
                } else if idx == target_idx {
                    updated_instructions.push(instr);
                } else if idx == target_idx + 1 {
                    updated_instructions
                        .push(ContentInstruction::new(vec![], ContentOperator::QEnd));
                } else {
                    updated_instructions.push(instr);
                }
            } else if idx == target_idx {
                updated_instructions.push(ContentInstruction::new(vec![], ContentOperator::Q));
                updated_instructions.push(ContentInstruction::new(
                    vec![
                        ContentOperand::Real(spec.new_width),
                        ContentOperand::Real(0.0),
                        ContentOperand::Real(0.0),
                        ContentOperand::Real(spec.new_height),
                        ContentOperand::Real(spec.new_x),
                        ContentOperand::Real(spec.new_y),
                    ],
                    ContentOperator::Cm,
                ));
                updated_instructions.push(instr);
                updated_instructions.push(ContentInstruction::new(vec![], ContentOperator::QEnd));
            } else {
                updated_instructions.push(instr);
            }
        }

        let mut new_stream_bytes = Vec::new();
        for instr in updated_instructions {
            Self::serialize_instruction(&mut new_stream_bytes, &instr)?;
        }

        stream_obj.data = new_stream_bytes;
        stream_obj.stream_length = stream_obj.data.len();
        stream_obj.dict.remove("Filter");
        stream_obj.dict.insert(
            "Length".to_string(),
            PdfObject::Integer(stream_obj.data.len() as i64),
        );

        let mut modified_objects = BTreeMap::new();

        // Check if stream is shared with other pages
        let mut count_referencing_pages = 0;
        for &other_page_ref in page_refs {
            if let Ok(other_obj) = store.resolve(other_page_ref) {
                if let Some(other_dict) = other_obj.as_dict() {
                    if let Some(c) = other_dict.get("Contents") {
                        if let Some(cr) = c.as_reference() {
                            if cr == contents_ref {
                                count_referencing_pages += 1;
                            }
                        } else if let Some(arr) = c.as_array() {
                            for item in arr {
                                if item.as_reference() == Some(contents_ref) {
                                    count_referencing_pages += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        if count_referencing_pages > 1 {
            let new_stream_num = *next_alloc_obj_num;
            *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
            let new_stream_ref = ObjectRef::new(new_stream_num, 0);

            modified_objects.insert(new_stream_ref, PdfObject::Stream(stream_obj));

            if let Some(contents_entry) = page_dict.get_mut("Contents") {
                match contents_entry {
                    PdfObject::Reference(_) => {
                        *contents_entry = PdfObject::Reference(new_stream_ref);
                    }
                    PdfObject::Array(arr) => {
                        if target_image.stream_index < arr.len() {
                            arr[target_image.stream_index] = PdfObject::Reference(new_stream_ref);
                        }
                    }
                    _ => {}
                }
            }
            modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else {
            modified_objects.insert(contents_ref, PdfObject::Stream(stream_obj));
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
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

    /// Serializes a ContentInstruction to bytes.
    fn serialize_instruction<W: std::io::Write>(
        writer: &mut W,
        instr: &ContentInstruction,
    ) -> PdfResult<()> {
        for (i, op) in instr.operands.iter().enumerate() {
            if i > 0 {
                write!(writer, " ").map_err(|e| PdfError::Serialization(e.to_string()))?;
            }
            match op {
                ContentOperand::Integer(n) => {
                    write!(writer, "{n}").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::Real(r) => {
                    if r.fract() == 0.0 {
                        write!(writer, "{:.1}", r)
                            .map_err(|e| PdfError::Serialization(e.to_string()))?;
                    } else {
                        write!(writer, "{:.4}", r)
                            .map_err(|e| PdfError::Serialization(e.to_string()))?;
                    }
                }
                ContentOperand::Name(n) => {
                    write!(writer, "/{n}").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::String(s) => {
                    let s_str = String::from_utf8_lossy(s);
                    write!(writer, "({s_str})")
                        .map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::Array(arr) => {
                    write!(writer, "[").map_err(|e| PdfError::Serialization(e.to_string()))?;
                    for (j, item) in arr.iter().enumerate() {
                        if j > 0 {
                            write!(writer, " ")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?;
                        }
                        match item {
                            ContentOperand::Integer(n) => {
                                write!(writer, "{n}")
                                    .map_err(|e| PdfError::Serialization(e.to_string()))?;
                            }
                            ContentOperand::Real(r) => {
                                write!(writer, "{:.4}", r)
                                    .map_err(|e| PdfError::Serialization(e.to_string()))?;
                            }
                            ContentOperand::String(s) => {
                                let s_str = String::from_utf8_lossy(s);
                                write!(writer, "({s_str})")
                                    .map_err(|e| PdfError::Serialization(e.to_string()))?;
                            }
                            _ => {}
                        }
                    }
                    write!(writer, "]").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::Dict(_) => {
                    write!(writer, "<< >>").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
            }
        }

        if !instr.operands.is_empty() {
            write!(writer, " ").map_err(|e| PdfError::Serialization(e.to_string()))?;
        }

        let op_str = instr.operator.as_str();
        writeln!(writer, "{op_str}").map_err(|e| PdfError::Serialization(e.to_string()))?;
        Ok(())
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
