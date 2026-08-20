use std::collections::BTreeSet;

use crate::error::{PdfError, PdfResult};
use crate::filter::limits::DecompressLimits;
use crate::io::cursor::ByteCursor;
use crate::io::source::ByteSource;
use crate::syntax::lexer::Lexer;
use crate::syntax::object::PdfObject;
use crate::syntax::parser::Parser;
use crate::syntax::token::Token;
use crate::xref::stream::XrefStreamParser;
use crate::xref::table::XrefTable;

pub struct XrefResolver;

impl XrefResolver {
    /// Locates and parses the `startxref` offset from the end of the PDF stream.
    pub fn find_startxref(source: ByteSource<'_>) -> PdfResult<u64> {
        let len = source.len();
        if len < 10 {
            return Err(PdfError::InvalidXref("PDF file too short for xref".into()));
        }

        // Search within the last 2048 bytes of the file
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

    /// High-level entry point: finds startxref and parses the entire table & trailer (with default limits).
    pub fn load_xref_and_trailer(source: ByteSource<'_>) -> PdfResult<XrefTable> {
        let limits = DecompressLimits::default();
        Self::load_xref_and_trailer_with_limits(source, &limits)
    }

    /// High-level entry point with configurable security/resource limits.
    pub fn load_xref_and_trailer_with_limits(
        source: ByteSource<'_>,
        limits: &DecompressLimits,
    ) -> PdfResult<XrefTable> {
        let startxref = Self::find_startxref(source)?;
        let mut table = XrefTable::new();
        table.startxref_offset = startxref;

        let mut visited_offsets = BTreeSet::new();
        Self::parse_xref_chain_at_offset(
            source,
            startxref,
            &mut table,
            &mut visited_offsets,
            limits,
        )?;

        Ok(table)
    }

    /// Parses a single classic xref table at `xref_offset` (backward compatibility helper).
    pub fn parse_xref_table(source: ByteSource<'_>, xref_offset: u64) -> PdfResult<XrefTable> {
        let limits = DecompressLimits::default();
        let mut table = XrefTable::new();
        table.startxref_offset = xref_offset;
        let mut visited_offsets = BTreeSet::new();
        Self::parse_xref_chain_at_offset(
            source,
            xref_offset,
            &mut table,
            &mut visited_offsets,
            &limits,
        )?;
        Ok(table)
    }

    fn parse_xref_chain_at_offset(
        source: ByteSource<'_>,
        offset: u64,
        table: &mut XrefTable,
        visited_offsets: &mut BTreeSet<u64>,
        limits: &DecompressLimits,
    ) -> PdfResult<()> {
        if offset >= source.len() as u64 {
            return Err(PdfError::InvalidXref(format!(
                "xref offset {offset} exceeds file length {}",
                source.len()
            )));
        }

        if visited_offsets.contains(&offset) {
            // Prevent cyclic /Prev recursion attacks
            return Err(PdfError::InvalidXref(format!(
                "Cyclic xref chain detected at offset {offset}"
            )));
        }

        if visited_offsets.len() >= limits.max_xref_chain_depth {
            return Err(PdfError::InvalidXref(format!(
                "XRef chain depth limit ({}) exceeded",
                limits.max_xref_chain_depth
            )));
        }

        visited_offsets.insert(offset);

        let mut cursor = ByteCursor::new(source);
        cursor.set_position(offset as usize)?;
        let mut lexer = Lexer::new(cursor);

        let first_token = lexer.peek_token()?.ok_or(PdfError::UnexpectedEof)?;

        match first_token {
            Token::KeywordXref => {
                // 1. Classic ASCII XRef Table
                let _ = lexer.next_token()?; // consume 'xref'
                Self::parse_classic_xref_table(
                    source,
                    lexer,
                    table,
                    visited_offsets,
                    limits,
                    offset,
                )?;
            }
            Token::Integer(_) => {
                // 2. Modern PDF 1.5+ XRef Stream (N G obj << /Type /XRef ... >>)
                let mut parser = Parser::new(lexer);
                let (_obj_ref, obj) = parser.parse_indirect_object()?;
                match obj {
                    PdfObject::Stream(stream) => {
                        XrefStreamParser::parse_into_table(&stream, table, limits)?;

                        // Check for chained /Prev in stream dict
                        if let Some(prev) = stream.dict.get("Prev").and_then(|v| v.as_i64()) {
                            if prev > 0 {
                                Self::parse_xref_chain_at_offset(
                                    source,
                                    prev as u64,
                                    table,
                                    visited_offsets,
                                    limits,
                                )?;
                            }
                        }
                    }
                    other => {
                        return Err(PdfError::InvalidXref(format!(
                            "Expected XRef stream object at offset {offset}, found {}",
                            other.type_name()
                        )));
                    }
                }
            }
            other => {
                return Err(PdfError::InvalidXref(format!(
                    "Expected 'xref' keyword or object number at offset {offset}, found {other:?}"
                )));
            }
        }

        Ok(())
    }

    fn parse_classic_xref_table(
        source: ByteSource<'_>,
        mut lexer: Lexer<'_>,
        table: &mut XrefTable,
        visited_offsets: &mut BTreeSet<u64>,
        limits: &DecompressLimits,
        _current_offset: u64,
    ) -> PdfResult<()> {
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
                            table.entries.entry(obj_num).or_insert(
                                crate::xref::table::XrefEntry::InUse {
                                    byte_offset: offset_tok,
                                    generation: gen_tok,
                                },
                            );
                        } else {
                            table.entries.entry(obj_num).or_insert(
                                crate::xref::table::XrefEntry::Free {
                                    next_free_obj: offset_tok,
                                    generation: gen_tok,
                                },
                            );
                        }
                    }
                }
                other => {
                    return Err(PdfError::InvalidXref(format!(
                        "Unexpected token in xref table: {other:?}"
                    )));
                }
            }
        }

        // Parse trailer dictionary
        let mut parser = Parser::new(lexer);
        let trailer_obj = parser.parse_object()?;
        let current_dict = match trailer_obj {
            PdfObject::Dictionary(dict) => {
                for (k, v) in &dict {
                    table.trailer.entry(k.clone()).or_insert_with(|| v.clone());
                }
                dict
            }
            other => {
                return Err(PdfError::InvalidXref(format!(
                    "Trailer must be a dictionary, found {}",
                    other.type_name()
                )));
            }
        };

        // 1. Check for /XRefStm in current trailer (Hybrid-Reference PDFs)
        if let Some(xref_stm_offset) = current_dict.get("XRefStm").and_then(|v| v.as_i64()) {
            if xref_stm_offset > 0 {
                Self::parse_xref_chain_at_offset(
                    source,
                    xref_stm_offset as u64,
                    table,
                    visited_offsets,
                    limits,
                )?;
            }
        }

        // 2. Check for chained /Prev xref tables if incremental update
        if let Some(prev) = current_dict.get("Prev").and_then(|v| v.as_i64()) {
            if prev > 0 {
                Self::parse_xref_chain_at_offset(
                    source,
                    prev as u64,
                    table,
                    visited_offsets,
                    limits,
                )?;
            }
        }

        Ok(())
    }
}
