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
use crate::xref::table::{XrefKind, XrefRevision, XrefTable};

pub struct XrefResolver;

impl XrefResolver {
    /// Locates and parses the `startxref` offset from the end of the PDF stream.
    pub fn find_startxref(source: ByteSource<'_>) -> PdfResult<u64> {
        let len = source.len();
        if len < 10 {
            return Err(PdfError::InvalidXref("PDF file too short for xref".into()));
        }

        // 1. Search within the last 2048 bytes of the file (standard fast path)
        let search_start = len.saturating_sub(2048);
        let slice = source.get_slice_range(search_start, len)?;

        let pos_in_slice = if let Some(pos) = slice.windows(9).rposition(|w| w == b"startxref") {
            search_start + pos
        } else {
            // 2. Extended recovery search up to 65,536 bytes from EOF
            let extended_start = len.saturating_sub(65536);
            let ext_slice = source.get_slice_range(extended_start, len)?;
            let pos = ext_slice
                .windows(9)
                .rposition(|w| w == b"startxref")
                .ok_or_else(|| {
                    PdfError::InvalidXref("startxref keyword not found near EOF".into())
                })?;
            extended_start + pos
        };

        let startxref_pos = pos_in_slice;
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

        // Attempt exact offset first, then bounded drift recovery within +/- 64 bytes
        let mut effective_offset = offset;
        let mut cursor = ByteCursor::new(source);
        cursor.set_position(effective_offset as usize)?;
        let mut lexer = Lexer::new(cursor);
        let mut first_token = lexer.peek_token().ok().flatten();
        let is_valid_xref_start = match first_token {
            Some(Token::KeywordXref) => true,
            Some(Token::Integer(_)) => {
                let saved_pos = lexer.position();
                let is_obj = matches!(
                    (lexer.next_token(), lexer.next_token(), lexer.next_token()),
                    (
                        Ok(Some(Token::Integer(_))),
                        Ok(Some(Token::Integer(_))),
                        Ok(Some(Token::KeywordObj))
                    )
                );
                let _ = lexer.set_position(saved_pos);
                is_obj
            }
            _ => false,
        };

        if !is_valid_xref_start {
            // Check bounded search window [offset - 64, offset + 64]
            let start = (offset as usize).saturating_sub(64);
            let end = ((offset as usize) + 64).min(source.len());
            if let Ok(window) = source.get_slice_range(start, end) {
                if let Some(pos) = window.windows(4).position(|w| w == b"xref") {
                    effective_offset = (start + pos) as u64;
                    let mut cur = ByteCursor::new(source);
                    cur.set_position(effective_offset as usize)?;
                    lexer = Lexer::new(cur);
                    first_token = lexer.peek_token().ok().flatten();
                } else if let Some(pos) = window.windows(3).position(|w| w == b"obj") {
                    // Search backward for object start
                    let mut obj_start = start + pos;
                    while obj_start > start
                        && source
                            .get_byte(obj_start - 1)
                            .is_ok_and(|b| b.is_ascii_digit() || b.is_ascii_whitespace())
                    {
                        obj_start -= 1;
                    }
                    effective_offset = obj_start as u64;
                    let mut cur = ByteCursor::new(source);
                    cur.set_position(effective_offset as usize)?;
                    lexer = Lexer::new(cur);
                    first_token = lexer.peek_token().ok().flatten();
                }
            }
        }

        let first_token = first_token.ok_or(PdfError::UnexpectedEof)?;

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
                    effective_offset,
                )?;
            }
            Token::Integer(_) => {
                // 2. Modern PDF 1.5+ XRef Stream (N G obj << /Type /XRef ... >>)
                let mut parser = Parser::new(lexer);
                let (_obj_ref, obj) = parser.parse_indirect_object()?;
                match obj {
                    PdfObject::Stream(stream) => {
                        XrefStreamParser::parse_into_table(&stream, table, limits)?;

                        let prev_result = Self::validated_previous_offset(
                            stream.dict.get("Prev"),
                            offset,
                            source.len(),
                            "Prev",
                        );
                        let prev_offset = prev_result.as_ref().ok().copied().flatten();
                        let xrefstm_offset = Self::validated_previous_offset(
                            stream.dict.get("XRefStm"),
                            offset,
                            source.len(),
                            "XRefStm",
                        )?;
                        table.revisions.push(XrefRevision {
                            revision_index: table.revisions.len(),
                            kind: XrefKind::Stream,
                            xref_offset: offset,
                            prev_offset,
                            xrefstm_offset,
                        });

                        if let Some(xrefstm) = xrefstm_offset {
                            Self::parse_xref_chain_at_offset(
                                source,
                                xrefstm,
                                table,
                                visited_offsets,
                                limits,
                            )?;
                        }

                        Self::parse_prev_with_recovery(
                            source,
                            stream.dict.get("Prev"),
                            offset,
                            prev_result,
                            table,
                            visited_offsets,
                            limits,
                        )?;
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
        current_offset: u64,
    ) -> PdfResult<()> {
        // Parse subsections
        let mut entries_seen = 0usize;
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
                    entries_seen = entries_seen.checked_add(count).ok_or_else(|| {
                        PdfError::InvalidXref("Classic xref entry count overflow".into())
                    })?;
                    if entries_seen > limits.max_xref_entries {
                        return Err(PdfError::InvalidXref(format!(
                            "Classic xref entry count {entries_seen} exceeds security limit {}",
                            limits.max_xref_entries
                        )));
                    }

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

        let xrefstm_offset = Self::validated_previous_offset(
            current_dict.get("XRefStm"),
            current_offset,
            source.len(),
            "XRefStm",
        )?;
        let prev_result = Self::validated_previous_offset(
            current_dict.get("Prev"),
            current_offset,
            source.len(),
            "Prev",
        );
        let prev_offset = prev_result.as_ref().ok().copied().flatten();
        table.revisions.push(XrefRevision {
            revision_index: table.revisions.len(),
            kind: XrefKind::Classic,
            xref_offset: current_offset,
            prev_offset,
            xrefstm_offset,
        });

        // 1. Check for /XRefStm in current trailer (Hybrid-Reference PDFs)
        if let Some(xref_stm_offset) = xrefstm_offset {
            let before = table.revisions.len();
            Self::parse_xref_chain_at_offset(
                source,
                xref_stm_offset,
                table,
                visited_offsets,
                limits,
            )?;
            if let Some(revision) = table.revisions.get_mut(before) {
                revision.kind = XrefKind::HybridStream;
            }
        }

        // 2. Check for chained /Prev xref tables if incremental update.
        // A malformed historical link is isolated from the already parsed current section.
        Self::parse_prev_with_recovery(
            source,
            current_dict.get("Prev"),
            current_offset,
            prev_result,
            table,
            visited_offsets,
            limits,
        )?;

        Ok(())
    }

    fn validated_previous_offset(
        object: Option<&PdfObject>,
        current_offset: u64,
        source_len: usize,
        key: &str,
    ) -> PdfResult<Option<u64>> {
        let Some(value) = object else {
            return Ok(None);
        };
        let offset = value.as_i64().ok_or_else(|| {
            PdfError::InvalidXref(format!("/{key} must be a non-negative integer offset"))
        })?;
        let offset = u64::try_from(offset)
            .map_err(|_| PdfError::InvalidXref(format!("/{key} offset is negative")))?;
        if offset == 0 {
            return Ok(None);
        }
        if offset >= source_len as u64 {
            return Err(PdfError::InvalidXref(format!(
                "/{key} offset {offset} exceeds file length {source_len}"
            )));
        }
        if offset >= current_offset {
            return Err(PdfError::InvalidXref(format!(
                "/{key} offset {offset} is not before current xref offset {current_offset}"
            )));
        }
        Ok(Some(offset))
    }

    fn parse_prev_with_recovery(
        source: ByteSource<'_>,
        prev_object: Option<&PdfObject>,
        current_offset: u64,
        validation: PdfResult<Option<u64>>,
        table: &mut XrefTable,
        visited_offsets: &mut BTreeSet<u64>,
        limits: &DecompressLimits,
    ) -> PdfResult<()> {
        if prev_object.is_none() {
            return Ok(());
        }

        match validation {
            Ok(None) => Ok(()),
            Ok(Some(prev)) => {
                let mut candidate_table = table.clone();
                let mut candidate_visited = visited_offsets.clone();
                match Self::parse_xref_chain_at_offset(
                    source,
                    prev,
                    &mut candidate_table,
                    &mut candidate_visited,
                    limits,
                ) {
                    Ok(()) => {
                        *table = candidate_table;
                        *visited_offsets = candidate_visited;
                        Ok(())
                    }
                    Err(error) => {
                        table.record_malformed_prev_recovery(format!(
                            "Ignored malformed /Prev section at checked offset {prev}: {error}"
                        ));
                        Ok(())
                    }
                }
            }
            Err(validation_error) => {
                let raw_offset = prev_object.and_then(PdfObject::as_i64);
                let mut candidates = Vec::new();

                if let Some(raw) = raw_offset.and_then(|value| u64::try_from(value).ok()) {
                    if raw > 0 && raw < source.len() as u64 {
                        candidates.push(raw);
                    }

                    let scan_bytes = limits.max_xref_recovery_scan_bytes.min(source.len());
                    if scan_bytes > 0 {
                        let center = usize::try_from(raw)
                            .unwrap_or(source.len())
                            .min(source.len());
                        let half = scan_bytes / 2;
                        let start = center.saturating_sub(half);
                        let end = start.saturating_add(scan_bytes).min(source.len());
                        if let Ok(window) = source.get_slice_range(start, end) {
                            for (position, marker) in window.windows(4).enumerate() {
                                if marker == b"xref" {
                                    let candidate = (start + position) as u64;
                                    if !candidates.contains(&candidate) {
                                        candidates.push(candidate);
                                    }
                                    if candidates.len() >= 8 {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                for candidate in candidates {
                    if candidate == current_offset || visited_offsets.contains(&candidate) {
                        continue;
                    }
                    let mut candidate_table = table.clone();
                    let mut candidate_visited = visited_offsets.clone();
                    if Self::parse_xref_chain_at_offset(
                        source,
                        candidate,
                        &mut candidate_table,
                        &mut candidate_visited,
                        limits,
                    )
                    .is_ok()
                    {
                        candidate_table.record_malformed_prev_recovery(format!(
                            "Recovered malformed /Prev from xref offset {current_offset} using checked xref section {candidate}: {validation_error}"
                        ));
                        *table = candidate_table;
                        *visited_offsets = candidate_visited;
                        return Ok(());
                    }
                }

                table.record_malformed_prev_recovery(format!(
                    "Ignored malformed /Prev at xref offset {current_offset}: {validation_error}"
                ));
                Ok(())
            }
        }
    }
}
