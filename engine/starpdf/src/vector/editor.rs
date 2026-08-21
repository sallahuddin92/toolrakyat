use std::collections::BTreeMap;
use std::io::Write;

use crate::appearance::status::AppearanceStatus;
use crate::content::operand::ContentOperand;
use crate::content::operator::{ContentInstruction, ContentOperator};
use crate::content::parser::ContentParser;
use crate::document::ObjectStore;
use crate::error::{PdfError, PdfResult};
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::mutation::MutationPlan;
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};
use crate::vector::extractor::VectorExtractor;
use crate::vector::types::{
    AddVectorGraphicSpec, DeleteVectorGraphicSpec, UpdateVectorGraphicSpec, VectorColor,
    VectorGeometry,
};

pub struct VectorEditor;

impl VectorEditor {
    /// Mutates an existing vector graphic object in-place or with shared content stream cloning.
    pub fn update_graphic(
        store: &mut ObjectStore<'_>,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        spec: &UpdateVectorGraphicSpec,
    ) -> PdfResult<MutationPlan> {
        let mut extractor = VectorExtractor::from_store(store, page_refs);
        let all_graphics = extractor.extract_all_graphics()?;

        let target_graphic = all_graphics
            .iter()
            .find(|g| g.graphic_id == spec.graphic_id && g.page_index == spec.page_index)
            .cloned()
            .ok_or_else(|| {
                PdfError::VectorGraphicNotFound(format!(
                    "Vector graphic '{}' not found on page {}",
                    spec.graphic_id, spec.page_index
                ))
            })?;

        if !target_graphic.editability.is_editable() {
            return Err(PdfError::VectorEditRefused(
                target_graphic
                    .editability
                    .reason()
                    .unwrap_or_else(|| "Vector graphic is not editable".to_string()),
            ));
        }

        let page_ref = page_refs[spec.page_index];
        let mut modified_objects = BTreeMap::new();

        // 1. Resolve content stream
        let page_obj = store.resolve(page_ref)?;
        let contents_entry = page_obj
            .as_dict()
            .and_then(|d| d.get("Contents"))
            .cloned()
            .ok_or_else(|| {
                PdfError::VectorGraphicNotFound(format!(
                    "Contents entry not found on page {}",
                    spec.page_index
                ))
            })?;

        let (target_stream_ref, is_array) = match &contents_entry {
            PdfObject::Reference(r) => (*r, false),
            PdfObject::Array(arr) => {
                if target_graphic.stream_index < arr.len() {
                    if let PdfObject::Reference(r) = &arr[target_graphic.stream_index] {
                        (*r, true)
                    } else {
                        return Err(PdfError::InvalidObject(
                            "Contents array entry is not a reference".to_string(),
                        ));
                    }
                } else {
                    return Err(PdfError::PageNotFound(spec.page_index));
                }
            }
            _ => {
                return Err(PdfError::InvalidObject(
                    "Contents entry is not a reference or array".to_string(),
                ));
            }
        };

        // 2. Extract and decompress stream instructions
        let (mut stream_dict, raw_bytes, was_flate) = {
            let stream_obj = store.resolve(target_stream_ref)?;
            if let Some(stream) = stream_obj.as_stream() {
                let is_flate =
                    stream.dict.get("Filter").and_then(|f| f.as_name()) == Some("FlateDecode");
                let data = if is_flate {
                    let limits = DecompressLimits::default();
                    FlateDecoder::decode(&stream.data, &limits)?
                } else {
                    stream.data.clone()
                };
                (stream.dict.clone(), data, is_flate)
            } else {
                return Err(PdfError::InvalidObject(
                    "Target content stream is not a stream object".to_string(),
                ));
            }
        };

        let mut parser = ContentParser::from_bytes(&raw_bytes);
        let mut instructions = parser.parse_instructions()?;

        // 3. Compute replacement instructions
        let new_geom = spec
            .new_geometry
            .as_ref()
            .unwrap_or(&target_graphic.geometry);
        let new_line_width = spec.new_line_width.unwrap_or(target_graphic.line_width);
        let new_stroke_color = spec
            .new_stroke_color
            .as_ref()
            .unwrap_or(&target_graphic.stroke_color);
        let new_fill_color = spec
            .new_fill_color
            .as_ref()
            .unwrap_or(&target_graphic.fill_color);
        let new_is_stroked = spec.new_is_stroked.unwrap_or(target_graphic.is_stroked);
        let new_is_filled = spec.new_is_filled.unwrap_or(target_graphic.is_filled);

        let replacement = Self::generate_graphic_instructions(
            new_geom,
            new_stroke_color.as_ref(),
            new_fill_color.as_ref(),
            new_line_width,
            new_is_stroked,
            new_is_filled,
        )?;

        // 4. Splice replacement instructions into range
        let start_idx = target_graphic.start_instruction_index;
        let end_idx = target_graphic.end_instruction_index;

        if start_idx > end_idx || end_idx >= instructions.len() {
            return Err(PdfError::InvalidOperation(
                "Instruction range out of bounds".to_string(),
            ));
        }

        instructions.splice(start_idx..=end_idx, replacement);

        // 5. Serialize instructions to bytes
        let mut writer = Vec::new();
        for inst in &instructions {
            Self::write_instruction(&mut writer, inst)?;
        }

        let final_data = if was_flate {
            miniz_oxide::deflate::compress_to_vec_zlib(&writer, 6)
        } else {
            writer
        };

        stream_dict.insert(
            "Length".to_string(),
            PdfObject::Integer(final_data.len() as i64),
        );

        let new_stream = StreamObject {
            dict: stream_dict,
            data: final_data,
            stream_offset: 0,
            stream_length: 0,
        };

        // 6. Handle shared content stream cloning
        if target_graphic.is_shared && spec.clone_if_shared {
            let cloned_stream_num = *next_alloc_obj_num;
            *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
            let cloned_stream_ref = ObjectRef::new(cloned_stream_num, 0);

            modified_objects.insert(cloned_stream_ref, PdfObject::Stream(new_stream));

            let mut page_dict =
                store.resolve(page_ref)?.as_dict().cloned().ok_or_else(|| {
                    PdfError::InvalidObject("Page is not a dictionary".to_string())
                })?;

            if is_array {
                if let Some(PdfObject::Array(ref mut arr)) = page_dict.get_mut("Contents") {
                    if target_graphic.stream_index < arr.len() {
                        arr[target_graphic.stream_index] = PdfObject::Reference(cloned_stream_ref);
                    }
                }
            } else {
                page_dict.insert(
                    "Contents".to_string(),
                    PdfObject::Reference(cloned_stream_ref),
                );
            }

            modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else {
            modified_objects.insert(target_stream_ref, PdfObject::Stream(new_stream));
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    /// Adds a new vector graphic (Rectangle or Line) to a page.
    pub fn add_graphic(
        store: &mut ObjectStore<'_>,
        page_refs: &[ObjectRef],
        spec: &AddVectorGraphicSpec,
    ) -> PdfResult<MutationPlan> {
        if spec.page_index >= page_refs.len() {
            return Err(PdfError::PageNotFound(spec.page_index));
        }

        let page_ref = page_refs[spec.page_index];
        let mut modified_objects = BTreeMap::new();

        let page_obj = store.resolve(page_ref)?;
        let contents_entry = page_obj
            .as_dict()
            .and_then(|d| d.get("Contents"))
            .cloned()
            .ok_or_else(|| {
                PdfError::InvalidObject(format!(
                    "Contents entry not found on page {}",
                    spec.page_index
                ))
            })?;

        let target_stream_ref = match &contents_entry {
            PdfObject::Reference(r) => *r,
            PdfObject::Array(arr) => {
                if let Some(PdfObject::Reference(r)) = arr.last() {
                    *r
                } else {
                    return Err(PdfError::InvalidObject(
                        "Last Contents array entry is not a reference".to_string(),
                    ));
                }
            }
            _ => {
                return Err(PdfError::InvalidObject(
                    "Contents entry is not a reference or array".to_string(),
                ));
            }
        };

        let (mut stream_dict, raw_bytes, was_flate) = {
            let stream_obj = store.resolve(target_stream_ref)?;
            if let Some(stream) = stream_obj.as_stream() {
                let is_flate =
                    stream.dict.get("Filter").and_then(|f| f.as_name()) == Some("FlateDecode");
                let data = if is_flate {
                    let limits = DecompressLimits::default();
                    FlateDecoder::decode(&stream.data, &limits)?
                } else {
                    stream.data.clone()
                };
                (stream.dict.clone(), data, is_flate)
            } else {
                return Err(PdfError::InvalidObject(
                    "Target content stream is not a stream object".to_string(),
                ));
            }
        };

        // Format isolated graphics block: q \n ... \n Q
        let mut append_bytes = Vec::new();
        writeln!(append_bytes, "\nq").map_err(|e| PdfError::Serialization(e.to_string()))?;

        let instructions = Self::generate_graphic_instructions(
            &spec.geometry,
            spec.stroke_color.as_ref(),
            spec.fill_color.as_ref(),
            spec.line_width,
            spec.is_stroked,
            spec.is_filled,
        )?;

        for inst in &instructions {
            Self::write_instruction(&mut append_bytes, inst)?;
        }
        writeln!(append_bytes, "Q\n").map_err(|e| PdfError::Serialization(e.to_string()))?;

        let mut combined_bytes = raw_bytes;
        combined_bytes.extend(append_bytes);

        let final_data = if was_flate {
            miniz_oxide::deflate::compress_to_vec_zlib(&combined_bytes, 6)
        } else {
            combined_bytes
        };

        stream_dict.insert(
            "Length".to_string(),
            PdfObject::Integer(final_data.len() as i64),
        );

        let new_stream = StreamObject {
            dict: stream_dict,
            data: final_data,
            stream_offset: 0,
            stream_length: 0,
        };

        modified_objects.insert(target_stream_ref, PdfObject::Stream(new_stream));

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    /// Deletes an existing vector graphic from a page.
    pub fn delete_graphic(
        store: &mut ObjectStore<'_>,
        page_refs: &[ObjectRef],
        next_alloc_obj_num: &mut u64,
        spec: &DeleteVectorGraphicSpec,
    ) -> PdfResult<MutationPlan> {
        let mut extractor = VectorExtractor::from_store(store, page_refs);
        let all_graphics = extractor.extract_all_graphics()?;

        let target_graphic = all_graphics
            .iter()
            .find(|g| g.graphic_id == spec.graphic_id && g.page_index == spec.page_index)
            .cloned()
            .ok_or_else(|| {
                PdfError::VectorGraphicNotFound(format!(
                    "Vector graphic '{}' not found on page {}",
                    spec.graphic_id, spec.page_index
                ))
            })?;

        let page_ref = page_refs[spec.page_index];
        let mut modified_objects = BTreeMap::new();

        let page_obj = store.resolve(page_ref)?;
        let contents_entry = page_obj
            .as_dict()
            .and_then(|d| d.get("Contents"))
            .cloned()
            .ok_or_else(|| {
                PdfError::VectorGraphicNotFound(format!(
                    "Contents entry not found on page {}",
                    spec.page_index
                ))
            })?;

        let (target_stream_ref, is_array) = match &contents_entry {
            PdfObject::Reference(r) => (*r, false),
            PdfObject::Array(arr) => {
                if target_graphic.stream_index < arr.len() {
                    if let PdfObject::Reference(r) = &arr[target_graphic.stream_index] {
                        (*r, true)
                    } else {
                        return Err(PdfError::InvalidObject(
                            "Contents array entry is not a reference".to_string(),
                        ));
                    }
                } else {
                    return Err(PdfError::PageNotFound(spec.page_index));
                }
            }
            _ => {
                return Err(PdfError::InvalidObject(
                    "Contents entry is not a reference or array".to_string(),
                ));
            }
        };

        let (mut stream_dict, raw_bytes, was_flate) = {
            let stream_obj = store.resolve(target_stream_ref)?;
            if let Some(stream) = stream_obj.as_stream() {
                let is_flate =
                    stream.dict.get("Filter").and_then(|f| f.as_name()) == Some("FlateDecode");
                let data = if is_flate {
                    let limits = DecompressLimits::default();
                    FlateDecoder::decode(&stream.data, &limits)?
                } else {
                    stream.data.clone()
                };
                (stream.dict.clone(), data, is_flate)
            } else {
                return Err(PdfError::InvalidObject(
                    "Target content stream is not a stream object".to_string(),
                ));
            }
        };

        let mut parser = ContentParser::from_bytes(&raw_bytes);
        let mut instructions = parser.parse_instructions()?;

        let (del_start, del_end) = if target_graphic.has_isolated_q
            && target_graphic.start_instruction_index > 0
            && target_graphic.end_instruction_index + 1 < instructions.len()
        {
            (
                target_graphic.start_instruction_index - 1,
                target_graphic.end_instruction_index + 1,
            )
        } else {
            (
                target_graphic.start_instruction_index,
                target_graphic.end_instruction_index,
            )
        };

        if del_start <= del_end && del_end < instructions.len() {
            instructions.drain(del_start..=del_end);
        }

        let mut writer = Vec::new();
        for inst in &instructions {
            Self::write_instruction(&mut writer, inst)?;
        }

        let final_data = if was_flate {
            miniz_oxide::deflate::compress_to_vec_zlib(&writer, 6)
        } else {
            writer
        };

        stream_dict.insert(
            "Length".to_string(),
            PdfObject::Integer(final_data.len() as i64),
        );

        let new_stream = StreamObject {
            dict: stream_dict,
            data: final_data,
            stream_offset: 0,
            stream_length: 0,
        };

        if target_graphic.is_shared && spec.clone_if_shared {
            let cloned_stream_num = *next_alloc_obj_num;
            *next_alloc_obj_num = next_alloc_obj_num.saturating_add(1);
            let cloned_stream_ref = ObjectRef::new(cloned_stream_num, 0);

            modified_objects.insert(cloned_stream_ref, PdfObject::Stream(new_stream));

            let mut page_dict =
                store.resolve(page_ref)?.as_dict().cloned().ok_or_else(|| {
                    PdfError::InvalidObject("Page is not a dictionary".to_string())
                })?;

            if is_array {
                if let Some(PdfObject::Array(ref mut arr)) = page_dict.get_mut("Contents") {
                    if target_graphic.stream_index < arr.len() {
                        arr[target_graphic.stream_index] = PdfObject::Reference(cloned_stream_ref);
                    }
                }
            } else {
                page_dict.insert(
                    "Contents".to_string(),
                    PdfObject::Reference(cloned_stream_ref),
                );
            }

            modified_objects.insert(page_ref, PdfObject::Dictionary(page_dict));
        } else {
            modified_objects.insert(target_stream_ref, PdfObject::Stream(new_stream));
        }

        Ok(MutationPlan {
            modified_objects,
            appearance_status: AppearanceStatus::ValueUpdated,
            glyph_mapping_quality: None,
            layout_policy_result: None,
        })
    }

    fn generate_graphic_instructions(
        geometry: &VectorGeometry,
        stroke_color: Option<&VectorColor>,
        fill_color: Option<&VectorColor>,
        line_width: f64,
        is_stroked: bool,
        is_filled: bool,
    ) -> PdfResult<Vec<ContentInstruction>> {
        let mut instructions = Vec::new();

        // Line width
        if is_stroked && line_width > 0.0 {
            instructions.push(ContentInstruction::new(
                vec![ContentOperand::Real(line_width)],
                ContentOperator::LineWidth,
            ));
        }

        // Colors
        if is_stroked {
            if let Some(sc) = stroke_color {
                match sc {
                    VectorColor::Gray(g) => {
                        instructions.push(ContentInstruction::new(
                            vec![ContentOperand::Real(*g)],
                            ContentOperator::GStroke,
                        ));
                    }
                    VectorColor::Rgb(r, g, b) => {
                        instructions.push(ContentInstruction::new(
                            vec![
                                ContentOperand::Real(*r),
                                ContentOperand::Real(*g),
                                ContentOperand::Real(*b),
                            ],
                            ContentOperator::RGStroke,
                        ));
                    }
                    VectorColor::Cmyk(c, m, y, k) => {
                        instructions.push(ContentInstruction::new(
                            vec![
                                ContentOperand::Real(*c),
                                ContentOperand::Real(*m),
                                ContentOperand::Real(*y),
                                ContentOperand::Real(*k),
                            ],
                            ContentOperator::KStroke,
                        ));
                    }
                }
            }
        }

        if is_filled {
            if let Some(fc) = fill_color {
                match fc {
                    VectorColor::Gray(g) => {
                        instructions.push(ContentInstruction::new(
                            vec![ContentOperand::Real(*g)],
                            ContentOperator::GFill,
                        ));
                    }
                    VectorColor::Rgb(r, g, b) => {
                        instructions.push(ContentInstruction::new(
                            vec![
                                ContentOperand::Real(*r),
                                ContentOperand::Real(*g),
                                ContentOperand::Real(*b),
                            ],
                            ContentOperator::RGFill,
                        ));
                    }
                    VectorColor::Cmyk(c, m, y, k) => {
                        instructions.push(ContentInstruction::new(
                            vec![
                                ContentOperand::Real(*c),
                                ContentOperand::Real(*m),
                                ContentOperand::Real(*y),
                                ContentOperand::Real(*k),
                            ],
                            ContentOperator::KFill,
                        ));
                    }
                }
            }
        }

        // Geometry
        match geometry {
            VectorGeometry::Rectangle {
                x,
                y,
                width,
                height,
            } => {
                instructions.push(ContentInstruction::new(
                    vec![
                        ContentOperand::Real(*x),
                        ContentOperand::Real(*y),
                        ContentOperand::Real(*width),
                        ContentOperand::Real(*height),
                    ],
                    ContentOperator::Re,
                ));
            }
            VectorGeometry::Line { x1, y1, x2, y2 } => {
                instructions.push(ContentInstruction::new(
                    vec![ContentOperand::Real(*x1), ContentOperand::Real(*y1)],
                    ContentOperator::M,
                ));
                instructions.push(ContentInstruction::new(
                    vec![ContentOperand::Real(*x2), ContentOperand::Real(*y2)],
                    ContentOperator::L,
                ));
            }
            VectorGeometry::Path { points, closed } => {
                for (i, &(px, py)) in points.iter().enumerate() {
                    if i == 0 {
                        instructions.push(ContentInstruction::new(
                            vec![ContentOperand::Real(px), ContentOperand::Real(py)],
                            ContentOperator::M,
                        ));
                    } else {
                        instructions.push(ContentInstruction::new(
                            vec![ContentOperand::Real(px), ContentOperand::Real(py)],
                            ContentOperator::L,
                        ));
                    }
                }
                if *closed {
                    instructions.push(ContentInstruction::new(Vec::new(), ContentOperator::H));
                }
            }
        }

        // Painting Operator
        let paint_op = if is_stroked && is_filled {
            ContentOperator::B
        } else if is_stroked {
            ContentOperator::S
        } else if is_filled {
            ContentOperator::F
        } else {
            ContentOperator::N
        };

        instructions.push(ContentInstruction::new(Vec::new(), paint_op));

        Ok(instructions)
    }

    fn write_instruction<W: Write>(writer: &mut W, inst: &ContentInstruction) -> PdfResult<()> {
        for op in &inst.operands {
            match op {
                ContentOperand::Integer(i) => {
                    write!(writer, "{i} ").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::Real(r) => {
                    if (r.fract()).abs() < 1e-6 {
                        write!(writer, "{:.0} ", r)
                            .map_err(|e| PdfError::Serialization(e.to_string()))?;
                    } else {
                        write!(writer, "{:.4} ", r)
                            .map_err(|e| PdfError::Serialization(e.to_string()))?;
                    }
                }
                ContentOperand::Name(n) => {
                    write!(writer, "/{n} ").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::String(s) => {
                    write!(writer, "(").map_err(|e| PdfError::Serialization(e.to_string()))?;
                    for &b in s {
                        match b {
                            b'(' => write!(writer, "\\(")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                            b')' => write!(writer, "\\)")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                            b'\\' => write!(writer, "\\\\")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                            _ => writer
                                .write_all(&[b])
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                        }
                    }
                    write!(writer, ") ").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::Array(arr) => {
                    write!(writer, "[ ").map_err(|e| PdfError::Serialization(e.to_string()))?;
                    for item in arr {
                        match item {
                            ContentOperand::Integer(i) => write!(writer, "{i} ")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                            ContentOperand::Real(r) => write!(writer, "{r:.4} ")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                            ContentOperand::Name(n) => write!(writer, "/{n} ")
                                .map_err(|e| PdfError::Serialization(e.to_string()))?,
                            _ => {}
                        }
                    }
                    write!(writer, "] ").map_err(|e| PdfError::Serialization(e.to_string()))?;
                }
                ContentOperand::Dict(_) => {}
            }
        }

        writeln!(writer, "{}", inst.operator.as_str())
            .map_err(|e| PdfError::Serialization(e.to_string()))?;
        Ok(())
    }
}
