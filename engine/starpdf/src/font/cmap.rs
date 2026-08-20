use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::syntax::lexer::Lexer;
use crate::syntax::token::Token;

/// A parsed character code to Unicode string map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnicodeCMap {
    pub mappings: BTreeMap<u32, String>,
}

impl UnicodeCMap {
    pub fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Looks up the Unicode string for a character code.
    pub fn lookup(&self, code: u32) -> Option<&str> {
        self.mappings.get(&code).map(String::as_str)
    }

    /// Parses a `/ToUnicode` CMap stream buffer with strict resource and security bounds.
    pub fn parse(input: &[u8]) -> PdfResult<Self> {
        let mut cmap = Self::new();
        let mut lexer = Lexer::from_bytes(input);
        const MAX_MAPPINGS: usize = 65_536;

        while let Some(tok) = lexer.next_token()? {
            match tok {
                Token::Keyword(ref kw) if kw == "beginbfchar" => {
                    Self::parse_bfchar_block(&mut lexer, &mut cmap, MAX_MAPPINGS)?;
                }
                Token::Keyword(ref kw) if kw == "beginbfrange" => {
                    Self::parse_bfrange_block(&mut lexer, &mut cmap, MAX_MAPPINGS)?;
                }
                _ => {}
            }
        }

        Ok(cmap)
    }

    fn parse_bfchar_block(
        lexer: &mut Lexer<'_>,
        cmap: &mut Self,
        max_mappings: usize,
    ) -> PdfResult<()> {
        while let Some(tok) = lexer.next_token()? {
            match tok {
                Token::Keyword(ref kw) if kw == "endbfchar" => break,
                Token::HexString(src_bytes) => {
                    let src_code = decode_hex_code(&src_bytes);
                    let dst_tok = lexer.next_token()?.ok_or(PdfError::UnexpectedEof)?;
                    let dst_str = decode_dst_unicode_token(&dst_tok);

                    if cmap.mappings.len() < max_mappings {
                        cmap.mappings.insert(src_code, dst_str);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_bfrange_block(
        lexer: &mut Lexer<'_>,
        cmap: &mut Self,
        max_mappings: usize,
    ) -> PdfResult<()> {
        while let Some(tok) = lexer.next_token()? {
            match tok {
                Token::Keyword(ref kw) if kw == "endbfrange" => break,
                Token::HexString(start_bytes) => {
                    let start_code = decode_hex_code(&start_bytes);
                    let end_tok = lexer.next_token()?.ok_or(PdfError::UnexpectedEof)?;
                    let end_code = match end_tok {
                        Token::HexString(b) => decode_hex_code(&b),
                        _ => continue,
                    };

                    if end_code < start_code {
                        continue;
                    }

                    let dst_tok = lexer.next_token()?.ok_or(PdfError::UnexpectedEof)?;
                    match dst_tok {
                        Token::HexString(dst_bytes) => {
                            let mut current_dst = decode_hex_code(&dst_bytes);
                            for code in start_code..=end_code {
                                if cmap.mappings.len() >= max_mappings {
                                    break;
                                }
                                if let Some(ch) = char::from_u32(current_dst) {
                                    cmap.mappings.insert(code, ch.to_string());
                                }
                                current_dst = current_dst.saturating_add(1);
                            }
                        }
                        Token::ArrayOpen => {
                            // Array form: [ <dst1> <dst2> ... ]
                            let mut current_code = start_code;
                            while let Some(elem_tok) = lexer.next_token()? {
                                match elem_tok {
                                    Token::ArrayClose => break,
                                    Token::HexString(dst_b) => {
                                        if current_code <= end_code
                                            && cmap.mappings.len() < max_mappings
                                        {
                                            let dst_str = decode_dst_unicode_bytes(&dst_b);
                                            cmap.mappings.insert(current_code, dst_str);
                                            current_code = current_code.saturating_add(1);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[inline]
fn decode_hex_code(bytes: &[u8]) -> u32 {
    let mut code: u32 = 0;
    for &b in bytes {
        code = (code << 8) | (b as u32);
    }
    code
}

#[inline]
fn decode_dst_unicode_token(tok: &Token) -> String {
    match tok {
        Token::HexString(bytes) => decode_dst_unicode_bytes(bytes),
        Token::LiteralString(bytes) => decode_dst_unicode_bytes(bytes),
        _ => String::new(),
    }
}

#[inline]
fn decode_dst_unicode_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // 1. Check UTF-16BE (2 bytes per char or surrogate pairs)
    if bytes.len() % 2 == 0 {
        let u16_units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();

        if let Ok(s) = String::from_utf16(&u16_units) {
            return s;
        }
    }

    // 2. Fallback to direct UTF-8 or Latin-1 chars
    if let Ok(s) = std::str::from_utf8(bytes) {
        s.to_string()
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}
