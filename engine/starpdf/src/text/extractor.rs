use crate::content::{ContentInstruction, ContentOperand, ContentOperator, ContentParser};
use crate::error::PdfResult;
use crate::font::font::Font;
use crate::font::resource::PageResources;
use crate::text::matrix::Matrix2D;
use crate::text::span::{PageText, TextSpan};
use crate::text::state::{GraphicsState, TextState};

pub struct TextExtractor;

impl TextExtractor {
    /// Extracts all coordinate-aware text spans from raw page content stream bytes and page resources.
    pub fn extract_from_content(
        page_index: usize,
        content_bytes: &[u8],
        resources: &PageResources,
    ) -> PdfResult<PageText> {
        let mut parser = ContentParser::from_bytes(content_bytes);
        let instructions = parser.parse_instructions()?;
        Self::extract_from_instructions(page_index, &instructions, resources)
    }

    /// Extracts text spans from pre-parsed content stream instructions.
    pub fn extract_from_instructions(
        page_index: usize,
        instructions: &[ContentInstruction],
        resources: &PageResources,
    ) -> PdfResult<PageText> {
        let mut page_text = PageText::new(page_index);
        let mut graphics_stack: Vec<GraphicsState> = Vec::new();
        let mut current_graphics = GraphicsState::default();
        let mut text_state = TextState::default();

        let fallback_font = Font::standard_fallback("Helvetica");

        for instr in instructions {
            match instr.operator {
                ContentOperator::Q => {
                    // Save graphics state
                    graphics_stack.push(current_graphics.clone());
                }
                ContentOperator::QEnd => {
                    // Restore graphics state
                    if let Some(prev) = graphics_stack.pop() {
                        current_graphics = prev;
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
                                .unwrap_or(&fallback_font);

                            Self::render_text_bytes(
                                page_index,
                                raw_bytes,
                                font,
                                &current_graphics,
                                &mut text_state,
                                &mut page_text,
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
                                .unwrap_or(&fallback_font);

                            for item in items {
                                match item {
                                    ContentOperand::String(bytes) => {
                                        Self::render_text_bytes(
                                            page_index,
                                            bytes,
                                            font,
                                            &current_graphics,
                                            &mut text_state,
                                            &mut page_text,
                                        );
                                    }
                                    ContentOperand::Integer(adj) => {
                                        let adj_f = *adj as f64;
                                        Self::apply_tj_adjustment(adj_f, &mut text_state);
                                    }
                                    ContentOperand::Real(adj_f) => {
                                        Self::apply_tj_adjustment(*adj_f, &mut text_state);
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

        Ok(page_text)
    }

    fn render_text_bytes(
        page_index: usize,
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

        page_text.spans.push(TextSpan::new(
            page_index,
            span_text,
            x,
            y,
            width,
            height,
            rotation,
            font.name.clone(),
            text_state.font_size,
            confidence,
        ));

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
