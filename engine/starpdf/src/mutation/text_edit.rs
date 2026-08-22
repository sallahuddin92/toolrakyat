use std::io::Write as _;

use crate::content::{ContentInstruction, ContentOperand, ContentOperator, ContentParser};
use crate::error::{PdfError, PdfResult};
use crate::text::span::TextSpan;

/// Target specification for an exact, stable text edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditTarget {
    pub page_index: usize,
    pub stream_index: usize,
    pub instruction_index: usize,
    pub operand_index: usize,
    pub expected_original_text: Option<String>,
    pub expected_original_bytes: Option<Vec<u8>>,
    pub span_id: Option<String>,
}

impl TextEditTarget {
    pub fn new(
        page_index: usize,
        stream_index: usize,
        instruction_index: usize,
        operand_index: usize,
    ) -> Self {
        let span_id =
            format!("p{page_index}_s{stream_index}_i{instruction_index}_o{operand_index}");
        Self {
            page_index,
            stream_index,
            instruction_index,
            operand_index,
            expected_original_text: None,
            expected_original_bytes: None,
            span_id: Some(span_id),
        }
    }

    /// Constructs a target directly from an extracted TextSpan.
    pub fn from_span(span: &TextSpan) -> Self {
        Self {
            page_index: span.page_index,
            stream_index: span.stream_index,
            instruction_index: span.instruction_index,
            operand_index: span.operand_index,
            expected_original_text: Some(span.text.clone()),
            expected_original_bytes: Some(span.original_bytes.clone()),
            span_id: Some(span.span_id.clone()),
        }
    }

    /// Parses a structural span ID in the form `p{page}_s{stream}_i{instr}_o{op}`.
    pub fn from_span_id(span_id: &str) -> PdfResult<Self> {
        let parts: Vec<&str> = span_id.split('_').collect();
        if parts.len() != 4
            || !parts[0].starts_with('p')
            || !parts[1].starts_with('s')
            || !parts[2].starts_with('i')
            || !parts[3].starts_with('o')
        {
            return Err(PdfError::TargetTextNotFound(format!(
                "Invalid span ID format: '{span_id}' (expected 'p{{page}}_s{{stream}}_i{{instr}}_o{{op}}')"
            )));
        }

        let page_index: usize = parts[0][1..].parse().map_err(|_| {
            PdfError::TargetTextNotFound(format!("Invalid page index in '{span_id}'"))
        })?;
        let stream_index: usize = parts[1][1..].parse().map_err(|_| {
            PdfError::TargetTextNotFound(format!("Invalid stream index in '{span_id}'"))
        })?;
        let instruction_index: usize = parts[2][1..].parse().map_err(|_| {
            PdfError::TargetTextNotFound(format!("Invalid instruction index in '{span_id}'"))
        })?;
        let operand_index: usize = parts[3][1..].parse().map_err(|_| {
            PdfError::TargetTextNotFound(format!("Invalid operand index in '{span_id}'"))
        })?;

        Ok(Self {
            page_index,
            stream_index,
            instruction_index,
            operand_index,
            expected_original_text: None,
            expected_original_bytes: None,
            span_id: Some(span_id.to_string()),
        })
    }
}

/// Evaluated layout policy result for a proposed text replacement.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutPolicyResult {
    ExactFit,
    FitWithinOriginalBox { original_width: f64, new_width: f64 },
    WidthChanged { original_width: f64, new_width: f64 },
    UnsupportedLayout(String),
}

impl LayoutPolicyResult {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ExactFit => "EXACT_FIT",
            Self::FitWithinOriginalBox { .. } => "FIT_WITHIN_ORIGINAL_BOX",
            Self::WidthChanged { .. } => "WIDTH_CHANGED",
            Self::UnsupportedLayout(_) => "UNSUPPORTED_LAYOUT",
        }
    }

    pub const fn is_safe(&self) -> bool {
        !matches!(self, Self::UnsupportedLayout(_))
    }
}

/// Content stream editor for deterministic parsing, mutation, and serialization.
pub struct ContentStreamEditor;

impl ContentStreamEditor {
    /// Replaces targeted text bytes in raw content stream data, preserving all unrelated operators and graphics state.
    pub fn replace_in_stream(
        stream_bytes: &[u8],
        target: &TextEditTarget,
        new_bytes: &[u8],
    ) -> PdfResult<Vec<u8>> {
        let mut parser = ContentParser::from_bytes(stream_bytes);
        let mut instructions = parser.parse_instructions()?;

        if target.instruction_index >= instructions.len() {
            return Err(PdfError::TargetTextNotFound(format!(
                "Instruction index {} out of bounds (total instructions: {})",
                target.instruction_index,
                instructions.len()
            )));
        }

        let instr = &mut instructions[target.instruction_index];
        match instr.operator {
            ContentOperator::Tj => {
                if target.operand_index != 0 {
                    return Err(PdfError::TargetTextNotFound(format!(
                        "Tj operator expects operand index 0, found {}",
                        target.operand_index
                    )));
                }
                if instr.operands.is_empty() {
                    return Err(PdfError::TargetTextNotFound(
                        "Tj operator missing string operand".to_string(),
                    ));
                }
                instr.operands[0] = ContentOperand::String(new_bytes.to_vec());
            }
            ContentOperator::TJ => {
                if instr.operands.is_empty() {
                    return Err(PdfError::TargetTextNotFound(
                        "TJ operator missing array operand".to_string(),
                    ));
                }
                match &mut instr.operands[0] {
                    ContentOperand::Array(items) => {
                        if target.operand_index >= items.len() {
                            return Err(PdfError::TargetTextNotFound(format!(
                                "TJ array operand index {} out of bounds (array length: {})",
                                target.operand_index,
                                items.len()
                            )));
                        }
                        items[target.operand_index] = ContentOperand::String(new_bytes.to_vec());
                    }
                    _ => {
                        return Err(PdfError::TargetTextNotFound(
                            "TJ operand is not an array".to_string(),
                        ));
                    }
                }
            }
            ref op => {
                return Err(PdfError::TargetTextNotFound(format!(
                    "Target instruction is '{op}', expected text-show operator 'Tj' or 'TJ'"
                )));
            }
        }

        Ok(Self::serialize_instructions(&instructions))
    }

    /// Replaces multiple targeted text bytes in raw content stream data atomically.
    pub fn replace_multiple_in_stream(
        stream_bytes: &[u8],
        edits: &[(&TextEditTarget, &[u8])],
    ) -> PdfResult<Vec<u8>> {
        let mut parser = ContentParser::from_bytes(stream_bytes);
        let mut instructions = parser.parse_instructions()?;

        for (target, new_bytes) in edits {
            if target.instruction_index >= instructions.len() {
                return Err(PdfError::TargetTextNotFound(format!(
                    "Instruction index {} out of bounds (total instructions: {})",
                    target.instruction_index,
                    instructions.len()
                )));
            }

            let instr = &mut instructions[target.instruction_index];
            match instr.operator {
                ContentOperator::Tj => {
                    if target.operand_index != 0 {
                        return Err(PdfError::TargetTextNotFound(format!(
                            "Tj operator expects operand index 0, found {}",
                            target.operand_index
                        )));
                    }
                    if instr.operands.is_empty() {
                        return Err(PdfError::TargetTextNotFound(
                            "Tj operator missing string operand".to_string(),
                        ));
                    }
                    instr.operands[0] = ContentOperand::String(new_bytes.to_vec());
                }
                ContentOperator::TJ => {
                    if instr.operands.is_empty() {
                        return Err(PdfError::TargetTextNotFound(
                            "TJ operator missing array operand".to_string(),
                        ));
                    }
                    match &mut instr.operands[0] {
                        ContentOperand::Array(items) => {
                            if target.operand_index >= items.len() {
                                return Err(PdfError::TargetTextNotFound(format!(
                                    "TJ array operand index {} out of bounds (array length: {})",
                                    target.operand_index,
                                    items.len()
                                )));
                            }
                            items[target.operand_index] =
                                ContentOperand::String(new_bytes.to_vec());
                        }
                        _ => {
                            return Err(PdfError::TargetTextNotFound(
                                "TJ operand is not an array".to_string(),
                            ));
                        }
                    }
                }
                ref op => {
                    return Err(PdfError::TargetTextNotFound(format!(
                        "Target instruction is '{op}', expected text-show operator 'Tj' or 'TJ'"
                    )));
                }
            }
        }

        Ok(Self::serialize_instructions(&instructions))
    }

    /// Deterministically serializes content stream instructions into valid PDF content stream bytes.
    pub fn serialize_instructions(instructions: &[ContentInstruction]) -> Vec<u8> {
        let mut output = Vec::with_capacity(instructions.len() * 32);

        for (i, instr) in instructions.iter().enumerate() {
            if i > 0 && !output.ends_with(b"\n") && !output.ends_with(b" ") {
                output.push(b'\n');
            }

            for operand in &instr.operands {
                Self::serialize_operand(operand, &mut output);
                output.push(b' ');
            }

            output.extend_from_slice(instr.operator.as_str().as_bytes());
            output.push(b'\n');
        }

        output
    }

    pub fn serialize_operand(operand: &ContentOperand, output: &mut Vec<u8>) {
        match operand {
            ContentOperand::Integer(val) => {
                let _ = write!(output, "{val}");
            }
            ContentOperand::Real(val) => {
                if val.fract() == 0.0 {
                    let _ = write!(output, "{val:.1}");
                } else {
                    let _ = write!(output, "{val:.4}");
                }
            }
            ContentOperand::Name(name) => {
                output.push(b'/');
                for &b in name.as_bytes() {
                    if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' {
                        output.push(b);
                    } else {
                        let _ = write!(output, "#{:02X}", b);
                    }
                }
            }
            ContentOperand::String(bytes) => {
                // If bytes contain non-ASCII or special characters, write as hex string <...>
                let has_special = bytes
                    .iter()
                    .any(|&b| !(0x20..=0x7E).contains(&b) || b == b'(' || b == b')' || b == b'\\');
                if has_special {
                    output.push(b'<');
                    for &b in bytes {
                        let _ = write!(output, "{:02X}", b);
                    }
                    output.push(b'>');
                } else {
                    output.push(b'(');
                    output.extend_from_slice(bytes);
                    output.push(b')');
                }
            }
            ContentOperand::Array(items) => {
                output.push(b'[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        output.push(b' ');
                    }
                    Self::serialize_operand(item, output);
                }
                output.push(b']');
            }
            ContentOperand::Dict(map) => {
                output.extend_from_slice(b"<<");
                for (key, val) in map {
                    output.push(b'/');
                    output.extend_from_slice(key.as_bytes());
                    output.push(b' ');
                    Self::serialize_operand(val, output);
                    output.push(b' ');
                }
                output.extend_from_slice(b">>");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_span_id() {
        let target = TextEditTarget::from_span_id("p2_s1_i42_o0").unwrap();
        assert_eq!(target.page_index, 2);
        assert_eq!(target.stream_index, 1);
        assert_eq!(target.instruction_index, 42);
        assert_eq!(target.operand_index, 0);

        assert!(TextEditTarget::from_span_id("invalid").is_err());
    }

    #[test]
    fn mutates_tj_content_stream_preserving_graphics_state() {
        let original_stream = b"q\n1 0 0 1 50 700 cm\nBT\n/F1 12 Tf\n(Original Text) Tj\nET\nQ\n";
        let target = TextEditTarget::new(0, 0, 4, 0);
        let new_bytes = b"Replaced Text";

        let modified_stream =
            ContentStreamEditor::replace_in_stream(original_stream, &target, new_bytes).unwrap();
        let modified_str = String::from_utf8_lossy(&modified_stream);

        assert!(modified_str.contains("q\n"));
        assert!(modified_str.contains("1 0 0 1 50 700 cm"));
        assert!(modified_str.contains("BT\n"));
        assert!(modified_str.contains("/F1 12 Tf"));
        assert!(modified_str.contains("(Replaced Text) Tj"));
        assert!(modified_str.contains("ET\n"));
        assert!(modified_str.contains("Q\n"));
    }

    #[test]
    fn mutates_tj_array_item_preserving_surrounding_kerning() {
        let original_stream = b"BT\n/F1 12 Tf\n[(Hello) 20 (World)] TJ\nET\n";
        let target = TextEditTarget::new(0, 0, 2, 0);
        let new_bytes = b"Greetings";

        let modified_stream =
            ContentStreamEditor::replace_in_stream(original_stream, &target, new_bytes).unwrap();
        let modified_str = String::from_utf8_lossy(&modified_stream);

        assert!(modified_str.contains("[(Greetings) 20 (World)] TJ"));
    }
}
