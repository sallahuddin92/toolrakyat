use crate::error::{PdfError, PdfResult};
use crate::io::cursor::ByteCursor;
use crate::io::source::ByteSource;
use crate::syntax::lexer::Lexer;
use crate::syntax::object::PdfObject;
use crate::syntax::parser::Parser;
use crate::syntax::token::Token;
use crate::xref::table::XrefTable;

pub struct XrefResolver;

impl XrefResolver {
    /// Locates and parses the `startxref` offset from the end of the PDF stream.
    pub fn find_startxref(source: ByteSource<'_>) -> PdfResult<u64> {
        let len = source.len();
        if len < 10 {
            return Err(PdfError::InvalidXref("PDF file too short for xref".into()));
        }

        // Standard allows search within the last 1024-4096 bytes of the file
        let search_start = len.saturating_sub(2048);
        let slice = source.get_slice_range(search_start, len)?;

        let pos_in_slice = slice
            .windows(9)
            .rposition(|w| w == b"startxref")
            .ok_or_else(|| PdfError::InvalidXref("startxref keyword not found near EOF".into()))?;

        let startxref_pos = search_start + pos_in_slice;
        let mut cursor = ByteCursor::new(source);
        cursor.set_position(startxref_pos + 9)?;

        let mut lexer = Lexer::new(cursor);
        match lexer.next_token()? {
            Some(Token::Integer(offset)) if offset >= 0 => Ok(offset as u64),
            Some(other) => Err(PdfError::InvalidXref(format!(
                "Expected integer offset after startxref, found {other:?}"
            ))),
            None => Err(PdfError::UnexpectedEof),
        }
    }

    /// Parses the entire cross-reference table and trailer dictionary starting at `xref_offset`.
    pub fn parse_xref_table(source: ByteSource<'_>, xref_offset: u64) -> PdfResult<XrefTable> {
        if xref_offset >= source.len() as u64 {
            return Err(PdfError::InvalidXref(format!(
                "xref offset {xref_offset} exceeds file length {}",
                source.len()
            )));
        }

        let mut cursor = ByteCursor::new(source);
        cursor.set_position(xref_offset as usize)?;
        let mut lexer = Lexer::new(cursor);

        match lexer.next_token()? {
            Some(Token::KeywordXref) => {}
            Some(other) => {
                return Err(PdfError::InvalidXref(format!(
                    "Expected 'xref' at offset {xref_offset}, found {other:?}"
                )))
            }
            None => return Err(PdfError::UnexpectedEof),
        }

        let mut table = XrefTable::new();
        table.startxref_offset = xref_offset;

        // Parse subsections
        loop {
            lexer.skip_whitespace_and_comments();
            let first_token = match lexer.peek_token()? {
                Some(t) => t,
                None => return Err(PdfError::UnexpectedEof),
            };

            match first_token {
                Token::KeywordTrailer => {
                    let _ = lexer.next_token()?; // consume 'trailer'
                    break;
                }
                Token::Integer(start_num) if start_num >= 0 => {
                    let _ = lexer.next_token()?; // consume start_num
                    let count = match lexer.next_token()? {
                        Some(Token::Integer(c)) if c >= 0 => c as usize,
                        Some(other) => {
                            return Err(PdfError::InvalidXref(format!(
                                "Expected entry count in xref subsection, found {other:?}"
                            )))
                        }
                        None => return Err(PdfError::UnexpectedEof),
                    };

                    let start_obj = start_num as u64;
                    for i in 0..count {
                        let obj_num = start_obj.checked_add(i as u64).ok_or_else(|| {
                            PdfError::InvalidXref("Object number overflow in xref table".into())
                        })?;

                        let offset_tok = match lexer.next_token()? {
                            Some(Token::Integer(o)) if o >= 0 => o as u64,
                            Some(other) => {
                                return Err(PdfError::InvalidXref(format!(
                                    "Expected offset for object {obj_num}, found {other:?}"
                                )))
                            }
                            None => return Err(PdfError::UnexpectedEof),
                        };

                        let gen_tok = match lexer.next_token()? {
                            Some(Token::Integer(g)) if g >= 0 && g <= u16::MAX as i64 => g as u16,
                            Some(other) => {
                                return Err(PdfError::InvalidXref(format!(
                                    "Expected generation for object {obj_num}, found {other:?}"
                                )))
                            }
                            None => return Err(PdfError::UnexpectedEof),
                        };

                        let flag_tok = match lexer.next_token()? {
                            Some(Token::Keyword(ref s)) if s == "n" || s == "f" => s.clone(),
                            Some(other) => {
                                return Err(PdfError::InvalidXref(format!(
                                    "Expected 'n' or 'f' entry flag for object {obj_num}, found {other:?}"
                                )))
                            }
                            None => return Err(PdfError::UnexpectedEof),
                        };

                        if flag_tok == "n" {
                            table.insert_in_use(obj_num, offset_tok, gen_tok);
                        } else {
                            table.insert_free(obj_num, offset_tok, gen_tok);
                        }
                    }
                }
                other => {
                    return Err(PdfError::InvalidXref(format!(
                        "Unexpected token in xref table: {other:?}"
                    )))
                }
            }
        }

        // Parse trailer dictionary
        let mut parser = Parser::new(lexer);
        let trailer_obj = parser.parse_object()?;
        match trailer_obj {
            PdfObject::Dictionary(dict) => {
                table.trailer = dict;
            }
            other => {
                return Err(PdfError::InvalidXref(format!(
                    "Trailer must be a dictionary, found {}",
                    other.type_name()
                )))
            }
        }

        // Check for chained /Prev xref tables if incremental update
        let prev_offset = table
            .trailer
            .get("Prev")
            .and_then(|v| v.as_i64())
            .and_then(|p| if p > 0 { Some(p as u64) } else { None });

        if let Some(prev) = prev_offset {
            if prev < xref_offset {
                if let Ok(prev_table) = Self::parse_xref_table(source, prev) {
                    // Merge older entries without overwriting newer ones
                    for (obj_num, entry) in prev_table.entries {
                        table.entries.entry(obj_num).or_insert(entry);
                    }
                }
            }
        }

        Ok(table)
    }

    /// High-level entry point: finds startxref and parses the entire table & trailer.
    pub fn load_xref_and_trailer(source: ByteSource<'_>) -> PdfResult<XrefTable> {
        let startxref = Self::find_startxref(source)?;
        Self::parse_xref_table(source, startxref)
    }
}
