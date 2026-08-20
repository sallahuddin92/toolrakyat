use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::io::cursor::ByteCursor;
use crate::io::source::ByteSource;
use crate::syntax::lexer::Lexer;
use crate::syntax::object::{ObjectRef, PdfObject, StreamObject};
use crate::syntax::token::Token;

const MAX_RECURSION_DEPTH: usize = 64;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    depth: usize,
}

impl<'a> Parser<'a> {
    pub const fn new(lexer: Lexer<'a>) -> Self {
        Self { lexer, depth: 0 }
    }

    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::from_bytes(bytes),
            depth: 0,
        }
    }

    pub const fn from_cursor(cursor: ByteCursor<'a>) -> Self {
        Self {
            lexer: Lexer::new(cursor),
            depth: 0,
        }
    }

    pub fn position(&self) -> usize {
        self.lexer.position()
    }

    pub fn set_position(&mut self, pos: usize) -> PdfResult<()> {
        self.lexer.set_position(pos)
    }

    pub fn lexer_mut(&mut self) -> &mut Lexer<'a> {
        &mut self.lexer
    }

    pub fn parse_object(&mut self) -> PdfResult<PdfObject> {
        if self.depth > MAX_RECURSION_DEPTH {
            return Err(PdfError::RecursionLimitExceeded);
        }

        let tok = match self.lexer.next_token()? {
            Some(t) => t,
            None => return Err(PdfError::UnexpectedEof),
        };

        match tok {
            Token::Null => Ok(PdfObject::Null),
            Token::Boolean(b) => Ok(PdfObject::Bool(b)),
            Token::Real(r) => Ok(PdfObject::Real(r)),
            Token::Name(n) => Ok(PdfObject::Name(n)),
            Token::LiteralString(s) => Ok(PdfObject::String(s)),
            Token::HexString(s) => Ok(PdfObject::String(s)),
            Token::Integer(num) => {
                // Check if this integer is the beginning of an indirect reference: "num gen R"
                let saved_pos = self.lexer.position();
                if let Ok(Some(Token::Integer(gen))) = self.lexer.next_token() {
                    if let Ok(Some(Token::KeywordR)) = self.lexer.next_token() {
                        if num >= 0 && gen >= 0 && gen <= u16::MAX as i64 {
                            return Ok(PdfObject::Reference(ObjectRef::new(
                                num as u64, gen as u16,
                            )));
                        }
                    }
                }
                // Rewind if not a reference
                self.lexer.set_position(saved_pos)?;
                Ok(PdfObject::Integer(num))
            }
            Token::ArrayOpen => self.parse_array(),
            Token::DictOpen => self.parse_dict_or_stream(),
            other => Err(PdfError::InvalidSyntax(format!(
                "Unexpected token {other:?} when expecting PDF object"
            ))),
        }
    }

    fn parse_array(&mut self) -> PdfResult<PdfObject> {
        self.depth += 1;
        let mut items = Vec::new();

        loop {
            self.lexer.skip_whitespace_and_comments();
            if let Some(Token::ArrayClose) = self.lexer.peek_token()? {
                let _ = self.lexer.next_token()?;
                break;
            }

            match self.parse_object() {
                Ok(obj) => items.push(obj),
                Err(PdfError::UnexpectedEof) => {
                    self.depth -= 1;
                    return Err(PdfError::InvalidSyntax("Unclosed PDF array".into()));
                }
                Err(e) => {
                    self.depth -= 1;
                    return Err(e);
                }
            }
        }

        self.depth -= 1;
        Ok(PdfObject::Array(items))
    }

    fn parse_dict_or_stream(&mut self) -> PdfResult<PdfObject> {
        self.depth += 1;
        let mut dict = BTreeMap::new();

        loop {
            self.lexer.skip_whitespace_and_comments();
            if let Some(Token::DictClose) = self.lexer.peek_token()? {
                let _ = self.lexer.next_token()?;
                break;
            }

            let key_token = match self.lexer.next_token()? {
                Some(Token::Name(key)) => key,
                Some(other) => {
                    self.depth -= 1;
                    return Err(PdfError::InvalidSyntax(format!(
                        "Expected dictionary key Name, found {other:?}"
                    )));
                }
                None => {
                    self.depth -= 1;
                    return Err(PdfError::InvalidSyntax("Unclosed PDF dictionary".into()));
                }
            };

            let val = match self.parse_object() {
                Ok(obj) => obj,
                Err(e) => {
                    self.depth -= 1;
                    return Err(e);
                }
            };

            dict.insert(key_token, val);
        }
        self.depth -= 1;

        // Check if a stream follows this dictionary
        self.lexer.skip_whitespace_and_comments();
        if let Some(Token::KeywordStream) = self.lexer.peek_token()? {
            let _ = self.lexer.next_token()?; // consume 'stream'
            return self.parse_stream_body(dict);
        }

        Ok(PdfObject::Dictionary(dict))
    }

    fn parse_stream_body(&mut self, dict: BTreeMap<String, PdfObject>) -> PdfResult<PdfObject> {
        let cursor = self.lexer.cursor_mut();

        // Standard requires CR LF or LF immediately following 'stream' keyword
        if let Some(b'\r') = cursor.peek_byte() {
            let _ = cursor.read_byte();
            if cursor.peek_byte() == Some(b'\n') {
                let _ = cursor.read_byte();
            }
        } else if let Some(b'\n') = cursor.peek_byte() {
            let _ = cursor.read_byte();
        }

        let stream_start = cursor.position();

        // Determine stream length if specified in dictionary
        let length_opt = dict.get("Length").and_then(|v| v.as_i64()).and_then(|len| {
            if len >= 0 {
                Some(len as usize)
            } else {
                None
            }
        });

        let source = cursor.source();
        let (stream_data, total_consumed) = if let Some(len) = length_opt {
            // Fast path using /Length
            if let Ok(slice) = source.get_slice(stream_start, len) {
                // Verify endstream appears shortly after
                let search_offset = stream_start + len;
                let endstream_pos = source.find_from(search_offset, b"endstream");
                if let Some(pos) = endstream_pos {
                    (slice.to_vec(), (pos + b"endstream".len()) - stream_start)
                } else {
                    // Fallback to scanning
                    Self::scan_stream_until_endstream(source, stream_start)?
                }
            } else {
                Self::scan_stream_until_endstream(source, stream_start)?
            }
        } else {
            Self::scan_stream_until_endstream(source, stream_start)?
        };

        cursor.set_position(stream_start + total_consumed)?;

        Ok(PdfObject::Stream(StreamObject {
            dict,
            data: stream_data.clone(),
            stream_offset: stream_start,
            stream_length: stream_data.len(),
        }))
    }

    fn scan_stream_until_endstream(
        source: ByteSource<'a>,
        stream_start: usize,
    ) -> PdfResult<(Vec<u8>, usize)> {
        let endstream_pos = source
            .find_from(stream_start, b"endstream")
            .ok_or_else(|| PdfError::InvalidSyntax("Stream missing 'endstream' marker".into()))?;

        let mut data_end = endstream_pos;
        // Trim trailing EOL before endstream
        if data_end > stream_start && source.get_byte(data_end - 1) == Ok(b'\n') {
            data_end -= 1;
            if data_end > stream_start && source.get_byte(data_end - 1) == Ok(b'\r') {
                data_end -= 1;
            }
        }

        let slice = source.get_slice_range(stream_start, data_end)?;
        let total_len = (endstream_pos + b"endstream".len()) - stream_start;
        Ok((slice.to_vec(), total_len))
    }

    pub fn parse_indirect_object(&mut self) -> PdfResult<(ObjectRef, PdfObject)> {
        self.lexer.skip_whitespace_and_comments();

        let num = match self.lexer.next_token()? {
            Some(Token::Integer(n)) if n >= 0 => n as u64,
            Some(other) => {
                return Err(PdfError::InvalidSyntax(format!(
                    "Expected object number integer, found {other:?}"
                )))
            }
            None => return Err(PdfError::UnexpectedEof),
        };

        let gen = match self.lexer.next_token()? {
            Some(Token::Integer(g)) if g >= 0 && g <= u16::MAX as i64 => g as u16,
            Some(other) => {
                return Err(PdfError::InvalidSyntax(format!(
                    "Expected generation number integer, found {other:?}"
                )))
            }
            None => return Err(PdfError::UnexpectedEof),
        };

        match self.lexer.next_token()? {
            Some(Token::KeywordObj) => {}
            Some(other) => {
                return Err(PdfError::InvalidSyntax(format!(
                    "Expected 'obj' keyword, found {other:?}"
                )))
            }
            None => return Err(PdfError::UnexpectedEof),
        }

        let obj = self.parse_object()?;

        // Optional 'endobj'
        self.lexer.skip_whitespace_and_comments();
        if let Some(Token::KeywordEndObj) = self.lexer.peek_token()? {
            let _ = self.lexer.next_token()?;
        }

        Ok((ObjectRef::new(num, gen), obj))
    }
}
