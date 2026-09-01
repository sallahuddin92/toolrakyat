use crate::content::operand::ContentOperand;
use crate::content::operator::{ContentInstruction, ContentOperator};
use crate::error::{PdfError, PdfResult};
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::syntax::lexer::{is_pdf_whitespace, Lexer};
use crate::syntax::object::{PdfObject, StreamObject};
use crate::syntax::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct ContentInstructionSource {
    pub instruction_index: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub operator_start: usize,
    pub operator_end: usize,
    pub instruction: ContentInstruction,
}

impl ContentInstructionSource {
    pub fn resource_name(&self) -> Option<&str> {
        if self.instruction.operator != ContentOperator::Do {
            return None;
        }
        self.instruction
            .operands
            .first()
            .and_then(ContentOperand::as_name)
    }
}

pub fn decode_content_stream(stream: &StreamObject) -> PdfResult<Vec<u8>> {
    match stream.dict.get("Filter") {
        None | Some(PdfObject::Null) => Ok(stream.data.clone()),
        Some(PdfObject::Name(name)) if name == "FlateDecode" || name == "Fl" => {
            FlateDecoder::decode(&stream.data, &DecompressLimits::default()).map_err(|error| {
                PdfError::MalformedInput(format!(
                    "IMAGE_CONTENT_DECODE_REFUSED: Flate content stream could not be decoded safely: {error}"
                ))
            })
        }
        Some(filter) => Err(PdfError::MalformedInput(format!(
            "IMAGE_CONTENT_FILTER_REFUSED: unsupported content stream filter {}",
            filter.type_name()
        ))),
    }
}

pub fn write_uncompressed_content(stream: &mut StreamObject, bytes: Vec<u8>) {
    stream.data = bytes;
    stream.stream_length = stream.data.len();
    stream.dict.remove("Filter");
    stream.dict.remove("DecodeParms");
    stream.dict.insert(
        "Length".to_string(),
        PdfObject::Integer(stream.data.len() as i64),
    );
}

pub fn scan_instruction_sources(bytes: &[u8]) -> PdfResult<Vec<ContentInstructionSource>> {
    let mut lexer = Lexer::from_bytes(bytes);
    let mut sources = Vec::new();
    let mut operands = Vec::new();
    let mut instruction_start = None;

    loop {
        lexer.skip_whitespace_and_comments();
        let token_start = lexer.position();
        let Some(token) = lexer.next_token()? else {
            break;
        };
        let token_end = lexer.position();
        instruction_start.get_or_insert(token_start);

        let keyword = match token {
            Token::Integer(value) => {
                operands.push(ContentOperand::Integer(value));
                None
            }
            Token::Real(value) => {
                operands.push(ContentOperand::Real(value));
                None
            }
            Token::Name(value) => {
                operands.push(ContentOperand::Name(value));
                None
            }
            Token::LiteralString(value) | Token::HexString(value) => {
                operands.push(ContentOperand::String(value));
                None
            }
            Token::Keyword(value) => Some(value),
            Token::KeywordR => Some("R".to_string()),
            Token::KeywordObj => Some("obj".to_string()),
            Token::KeywordEndObj => Some("endobj".to_string()),
            Token::KeywordStream => Some("stream".to_string()),
            Token::KeywordEndStream => Some("endstream".to_string()),
            Token::KeywordXref => Some("xref".to_string()),
            Token::KeywordTrailer => Some("trailer".to_string()),
            Token::KeywordStartXref => Some("startxref".to_string()),
            Token::ArrayOpen
            | Token::ArrayClose
            | Token::DictOpen
            | Token::DictClose
            | Token::Comment(_)
            | Token::Null
            | Token::Boolean(_) => None,
        };

        let Some(keyword) = keyword else {
            continue;
        };

        if keyword == "BI" {
            let inline_end = find_inline_image_end(bytes, token_end)?;
            let index = sources.len();
            sources.push(ContentInstructionSource {
                instruction_index: index,
                byte_start: instruction_start.take().unwrap_or(token_start),
                byte_end: inline_end,
                operator_start: token_start,
                operator_end: token_end,
                instruction: ContentInstruction::new(
                    std::mem::take(&mut operands),
                    ContentOperator::Unknown("BI".to_string()),
                ),
            });
            lexer.set_position(inline_end)?;
            continue;
        }

        let index = sources.len();
        sources.push(ContentInstructionSource {
            instruction_index: index,
            byte_start: instruction_start.take().unwrap_or(token_start),
            byte_end: token_end,
            operator_start: token_start,
            operator_end: token_end,
            instruction: ContentInstruction::new(
                std::mem::take(&mut operands),
                ContentOperator::from_keyword(&keyword),
            ),
        });
    }

    if !operands.is_empty() {
        return Err(PdfError::MalformedInput(
            "IMAGE_CONTENT_SCAN_REFUSED: trailing content operands have no operator".to_string(),
        ));
    }
    Ok(sources)
}

fn find_inline_image_end(bytes: &[u8], after_bi: usize) -> PdfResult<usize> {
    let mut id_end = None;
    let mut index = after_bi;
    while index + 2 < bytes.len() {
        if is_pdf_whitespace(bytes[index])
            && bytes[index + 1] == b'I'
            && bytes[index + 2] == b'D'
            && bytes.get(index + 3).copied().is_some_and(is_pdf_whitespace)
        {
            id_end = Some(index + 4);
            break;
        }
        index += 1;
    }
    let data_start = id_end.ok_or_else(|| {
        PdfError::MalformedInput(
            "INLINE_IMAGE_REFUSED: inline image has no bounded ID marker".to_string(),
        )
    })?;
    index = data_start;
    while index + 2 < bytes.len() {
        if is_pdf_whitespace(bytes[index])
            && bytes[index + 1] == b'E'
            && bytes[index + 2] == b'I'
            && (index + 3 == bytes.len()
                || bytes.get(index + 3).copied().is_some_and(is_pdf_whitespace))
        {
            return Ok(index + 3);
        }
        index += 1;
    }
    Err(PdfError::MalformedInput(
        "INLINE_IMAGE_REFUSED: inline image has no bounded EI marker".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn scans_do_ranges_without_normalizing_complex_content() {
        let bytes =
            b"% keep\n/Span << /MCID 7 /Flag true >> BDC\nq\n10 0 0 10 2 3 cm\n/Im1 Do\nQ\nEMC\n";
        let sources = scan_instruction_sources(bytes).unwrap();
        let image = sources
            .iter()
            .find(|source| source.resource_name() == Some("Im1"))
            .unwrap();
        assert_eq!(&bytes[image.byte_start..image.byte_end], b"/Im1 Do");
    }

    #[test]
    fn skips_inline_image_payload_as_one_lossless_source_range() {
        let bytes = b"q BI /W 1 /H 1 /BPC 8 /CS /RGB ID \x00EI\xff\nEI Q\n/Im2 Do\n";
        let sources = scan_instruction_sources(bytes).unwrap();
        assert_eq!(sources[0].instruction.operator, ContentOperator::Q);
        assert_eq!(sources[1].instruction.operator.as_str(), "BI");
        assert_eq!(sources.last().unwrap().resource_name(), Some("Im2"));
    }

    #[test]
    fn refuses_undecodable_or_unsupported_filtered_content() {
        let flate = StreamObject {
            dict: BTreeMap::from([(
                "Filter".to_string(),
                PdfObject::Name("FlateDecode".to_string()),
            )]),
            data: b"not deflate data".to_vec(),
            stream_offset: 0,
            stream_length: 16,
        };
        assert!(decode_content_stream(&flate)
            .unwrap_err()
            .to_string()
            .contains("IMAGE_CONTENT_DECODE_REFUSED"));

        let unsupported = StreamObject {
            dict: BTreeMap::from([(
                "Filter".to_string(),
                PdfObject::Name("ASCII85Decode".to_string()),
            )]),
            data: b"ignored".to_vec(),
            stream_offset: 0,
            stream_length: 7,
        };
        assert!(decode_content_stream(&unsupported)
            .unwrap_err()
            .to_string()
            .contains("IMAGE_CONTENT_FILTER_REFUSED"));
    }
}
