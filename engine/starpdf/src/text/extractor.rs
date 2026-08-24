use crate::content::{ContentInstruction, ContentOperand, ContentOperator, ContentParser};
use crate::error::PdfResult;
use crate::font::font::Font;
use crate::font::resource::PageResources;
use crate::text::matrix::Matrix2D;
use crate::text::span::{PageText, TextSpan};
use crate::text::state::{GraphicsState, TextState, TextStateParameters};

pub struct TextExtractor;

impl TextExtractor {
    /// Extracts all coordinate-aware text spans from raw page content stream bytes and page resources.
    pub fn extract_from_content(
        page_index: usize,
        content_bytes: &[u8],
        resources: &PageResources,
    ) -> PdfResult<PageText> {
        Self::extract_from_streams(page_index, &[content_bytes], resources)
    }

    /// Extracts text spans across multiple content streams (for multi-stream pages).
    pub fn extract_from_streams(
        page_index: usize,
        streams: &[&[u8]],
        resources: &PageResources,
    ) -> PdfResult<PageText> {
        let mut page_text = PageText::new(page_index);
        let mut graphics_stack: Vec<(GraphicsState, TextStateParameters)> = Vec::new();
        let mut current_graphics = GraphicsState::default();
        let mut text_state = TextState::default();

        let fallback_font = Font::standard_fallback("Helvetica");

        for (stream_index, &stream_bytes) in streams.iter().enumerate() {
            let mut parser = ContentParser::from_bytes(stream_bytes);
            let instructions = parser.parse_instructions()?;

            Self::process_instructions(
                page_index,
                stream_index,
                &instructions,
                resources,
                &fallback_font,
                &mut graphics_stack,
                &mut current_graphics,
                &mut text_state,
                &mut page_text,
            );
        }

        Ok(page_text)
    }

    /// Extracts text spans from pre-parsed content stream instructions.
    pub fn extract_from_instructions(
        page_index: usize,
        instructions: &[ContentInstruction],
        resources: &PageResources,
    ) -> PdfResult<PageText> {
        let mut page_text = PageText::new(page_index);
        let mut graphics_stack: Vec<(GraphicsState, TextStateParameters)> = Vec::new();
        let mut current_graphics = GraphicsState::default();
        let mut text_state = TextState::default();

        let fallback_font = Font::standard_fallback("Helvetica");

        Self::process_instructions(
            page_index,
            0,
            instructions,
            resources,
            &fallback_font,
            &mut graphics_stack,
            &mut current_graphics,
            &mut text_state,
            &mut page_text,
        );

        Ok(page_text)
    }

    fn process_instructions(
        page_index: usize,
        stream_index: usize,
        instructions: &[ContentInstruction],
        resources: &PageResources,
        fallback_font: &Font,
        graphics_stack: &mut Vec<(GraphicsState, TextStateParameters)>,
        current_graphics: &mut GraphicsState,
        text_state: &mut TextState,
        page_text: &mut PageText,
    ) {
        for (instruction_index, instr) in instructions.iter().enumerate() {
            match instr.operator {
                ContentOperator::Q => {
                    // Save graphics state
                    graphics_stack.push((current_graphics.clone(), text_state.save_parameters()));
                }
                ContentOperator::QEnd => {
                    // Restore graphics state
                    if let Some((graphics, text_parameters)) = graphics_stack.pop() {
                        *current_graphics = graphics;
                        text_state.restore_parameters(text_parameters);
                    }
                }
                ContentOperator::Cm => {
                    // Current Transformation Matrix: [a b c d e f] cm
                    if instr.operands.len() == 6 {
                        if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                            instr.operands[0].as_f64(),
                            instr.operands[1].as_f64(),
                            instr.operands[2].as_f64(),
                            instr.operands[3].as_f64(),
                            instr.operands[4].as_f64(),
                            instr.operands[5].as_f64(),
                        ) {
                            let cm = Matrix2D::new(a, b, c, d, e, f);
                            current_graphics.ctm = cm.multiply(&current_graphics.ctm);
                        }
                    }
                }
                ContentOperator::Bt => {
                    text_state.begin_text();
                }
                ContentOperator::Et => {
                    text_state.end_text();
                }
                ContentOperator::Tf => {
                    // /FontName fontSize Tf
                    if instr.operands.len() >= 2 {
                        let font_name = instr.operands[0].as_name().unwrap_or("");
                        let size = instr.operands[1].as_f64().unwrap_or(12.0);
                        text_state.set_font(font_name, size);
                    }
                }
                ContentOperator::GFill => {
                    if let Some(gray) = instr.operands.first().and_then(ContentOperand::as_f64) {
                        current_graphics.fill_color = [gray, gray, gray];
                    }
                }
                ContentOperator::RGFill => {
                    if instr.operands.len() >= 3 {
                        if let (Some(r), Some(g), Some(b)) = (
                            instr.operands[0].as_f64(),
                            instr.operands[1].as_f64(),
                            instr.operands[2].as_f64(),
                        ) {
                            current_graphics.fill_color = [r, g, b];
                        }
                    }
                }
                ContentOperator::KFill => {
                    if instr.operands.len() >= 4 {
                        if let (Some(c), Some(m), Some(y), Some(k)) = (
                            instr.operands[0].as_f64(),
                            instr.operands[1].as_f64(),
                            instr.operands[2].as_f64(),
                            instr.operands[3].as_f64(),
                        ) {
                            current_graphics.fill_color = [
                                1.0 - (c + k).min(1.0),
                                1.0 - (m + k).min(1.0),
                                1.0 - (y + k).min(1.0),
                            ];
                        }
                    }
                }
                ContentOperator::Tm => {
                    // a b c d e f Tm
                    if instr.operands.len() == 6 {
                        if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                            instr.operands[0].as_f64(),
                            instr.operands[1].as_f64(),
                            instr.operands[2].as_f64(),
                            instr.operands[3].as_f64(),
                            instr.operands[4].as_f64(),
                            instr.operands[5].as_f64(),
                        ) {
                            text_state.set_matrix(a, b, c, d, e, f);
                        }
                    }
                }
                ContentOperator::Td => {
                    // tx ty Td
                    if instr.operands.len() >= 2 {
                        let tx = instr.operands[0].as_f64().unwrap_or(0.0);
                        let ty = instr.operands[1].as_f64().unwrap_or(0.0);
                        text_state.move_text_position(tx, ty);
                    }
                }
                ContentOperator::TD => {
                    // tx ty TD
                    if instr.operands.len() >= 2 {
                        let tx = instr.operands[0].as_f64().unwrap_or(0.0);
                        let ty = instr.operands[1].as_f64().unwrap_or(0.0);
                        text_state.leading = -ty;
                        text_state.move_text_position(tx, ty);
                    }
                }
                ContentOperator::TStar => {
                    text_state.move_to_next_line();
                }
                ContentOperator::Tj => {
                    // (string) Tj
                    if let Some(operand) = instr.operands.first() {
                        if let Some(raw_bytes) = operand.as_bytes() {
                            let font = text_state
                                .font_name
                                .as_deref()
                                .and_then(|name| resources.get_font(name))
                                .unwrap_or(fallback_font);

                            Self::render_text_bytes(
                                page_index,
                                stream_index,
                                instruction_index,
                                0,
                                "Tj",
                                raw_bytes,
                                font,
                                current_graphics,
                                text_state,
                                page_text,
                            );
                        }
                    }
                }
                ContentOperator::TJ => {
                    // [ ... ] TJ
                    if let Some(operand) = instr.operands.first() {
                        if let Some(items) = operand.as_array() {
                            let font = text_state
                                .font_name
                                .as_deref()
                                .and_then(|name| resources.get_font(name))
                                .unwrap_or(fallback_font);

                            for (operand_index, item) in items.iter().enumerate() {
                                match item {
                                    ContentOperand::String(bytes) => {
                                        Self::render_text_bytes(
                                            page_index,
                                            stream_index,
                                            instruction_index,
                                            operand_index,
                                            "TJ",
                                            bytes,
                                            font,
                                            current_graphics,
                                            text_state,
                                            page_text,
                                        );
                                    }
                                    ContentOperand::Integer(adj) => {
                                        let adj_f = *adj as f64;
                                        Self::apply_tj_adjustment(adj_f, text_state);
                                    }
                                    ContentOperand::Real(adj_f) => {
                                        Self::apply_tj_adjustment(*adj_f, text_state);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn render_text_bytes(
        page_index: usize,
        stream_index: usize,
        instruction_index: usize,
        operand_index: usize,
        operator_name: &str,
        bytes: &[u8],
        font: &Font,
        graphics: &GraphicsState,
        text_state: &mut TextState,
        page_text: &mut PageText,
    ) {
        let decoded_glyphs = font.decode_bytes(bytes);
        if decoded_glyphs.is_empty() {
            return;
        }

        let effective_matrix = text_state.tm.multiply(&graphics.ctm);
        let (x, y) = effective_matrix.transform_point(0.0, 0.0);
        let rotation = effective_matrix.rotation_degrees();

        let mut span_text = String::new();
        let mut total_width_text_space = 0.0;
        let mut confidence = 1.0;

        for (glyph_str, advance_font_units) in decoded_glyphs {
            if glyph_str == "\u{FFFD}" {
                confidence = 0.5;
            }
            span_text.push_str(&glyph_str);

            let mut advance = (advance_font_units / 1000.0) * text_state.font_size;
            advance += text_state.char_spacing;
            if glyph_str == " " {
                advance += text_state.word_spacing;
            }
            advance *= text_state.horizontal_scaling / 100.0;

            total_width_text_space += advance;
        }

        let scale_x = effective_matrix.scale_x();
        let scale_y = effective_matrix.scale_y();

        let width = total_width_text_space * scale_x;
        let height = text_state.font_size * scale_y;

        let span_id =
            format!("p{page_index}_s{stream_index}_i{instruction_index}_o{operand_index}");
        let editability = if text_state.in_text_object {
            font.check_span_editability(&span_text)
        } else {
            crate::text::span::TextEditability::ReadOnlyNativeText(
                "Text outside BT...ET text block".to_string(),
            )
        };

        let is_editable = editability.is_editable();
        let refusal_reason = editability.reason();

        page_text.spans.push(TextSpan {
            page_index,
            text: span_text,
            x,
            y,
            width,
            height,
            rotation,
            font_name: font.name.clone(),
            font_size: text_state.font_size,
            confidence,
            source_object: None,
            span_id,
            stream_index,
            instruction_index,
            operand_index,
            operator_name: operator_name.to_string(),
            font_resource_name: text_state
                .font_name
                .clone()
                .unwrap_or_else(|| font.name.clone()),
            font_base_name: font.base_font.clone(),
            font_family: format!("{:?}", font.style.family),
            is_bold: font.style.is_bold,
            is_italic: font.style.is_italic,
            is_monospace: font.style.is_monospace,
            fill_color: graphics.fill_color,
            original_bytes: bytes.to_vec(),
            is_editable,
            editability_status: editability,
            refusal_reason,
        });

        // Advance text matrix horizontally
        let translation = Matrix2D::translation(total_width_text_space, 0.0);
        text_state.tm = translation.multiply(&text_state.tm);
    }

    #[inline]
    fn apply_tj_adjustment(adjustment: f64, text_state: &mut TextState) {
        // Negative number shifts cursor forward
        let delta_x =
            -(adjustment / 1000.0) * text_state.font_size * (text_state.horizontal_scaling / 100.0);
        let translation = Matrix2D::translation(delta_x, 0.0);
        text_state.tm = translation.multiply(&text_state.tm);
    }
}
