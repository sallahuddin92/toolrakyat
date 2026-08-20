use crate::error::{PdfError, PdfResult};
use crate::io::cursor::ByteCursor;
use crate::syntax::token::Token;

#[inline]
pub const fn is_pdf_whitespace(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

#[inline]
pub const fn is_pdf_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

#[inline]
pub const fn is_regular_char(b: u8) -> bool {
    !is_pdf_whitespace(b) && !is_pdf_delimiter(b)
}

pub struct Lexer<'a> {
    cursor: ByteCursor<'a>,
}

impl<'a> Lexer<'a> {
    pub const fn new(cursor: ByteCursor<'a>) -> Self {
        Self { cursor }
    }

    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            cursor: ByteCursor::from_bytes(bytes),
        }
    }

    pub const fn cursor(&self) -> &ByteCursor<'a> {
        &self.cursor
    }

    pub fn cursor_mut(&mut self) -> &mut ByteCursor<'a> {
        &mut self.cursor
    }

    pub fn position(&self) -> usize {
        self.cursor.position()
    }

    pub fn set_position(&mut self, pos: usize) -> PdfResult<()> {
        self.cursor.set_position(pos)
    }

    pub fn skip_whitespace_and_comments(&mut self) {
        while let Some(b) = self.cursor.peek_byte() {
            if is_pdf_whitespace(b) {
                let _ = self.cursor.read_byte();
            } else if b == b'%' {
                // Skip comment line
                let _ = self.cursor.read_byte();
                while let Some(c) = self.cursor.peek_byte() {
                    let _ = self.cursor.read_byte();
                    if c == b'\r' {
                        if self.cursor.peek_byte() == Some(b'\n') {
                            let _ = self.cursor.read_byte();
                        }
                        break;
                    } else if c == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> PdfResult<Option<Token>> {
        self.skip_whitespace_and_comments();

        let b = match self.cursor.peek_byte() {
            Some(byte) => byte,
            None => return Ok(None),
        };

        match b {
            b'[' => {
                let _ = self.cursor.read_byte();
                Ok(Some(Token::ArrayOpen))
            }
            b']' => {
                let _ = self.cursor.read_byte();
                Ok(Some(Token::ArrayClose))
            }
            b'<' => {
                let _ = self.cursor.read_byte();
                if self.cursor.peek_byte() == Some(b'<') {
                    let _ = self.cursor.read_byte();
                    Ok(Some(Token::DictOpen))
                } else {
                    self.parse_hex_string().map(|s| Some(Token::HexString(s)))
                }
            }
            b'>' => {
                let _ = self.cursor.read_byte();
                if self.cursor.peek_byte() == Some(b'>') {
                    let _ = self.cursor.read_byte();
                    Ok(Some(Token::DictClose))
                } else {
                    Err(PdfError::InvalidToken("Unexpected single '>'".into()))
                }
            }
            b'(' => {
                let _ = self.cursor.read_byte();
                self.parse_literal_string()
                    .map(|s| Some(Token::LiteralString(s)))
            }
            b'/' => {
                let _ = self.cursor.read_byte();
                self.parse_name().map(|n| Some(Token::Name(n)))
            }
            _ => self.parse_word_or_number(),
        }
    }

    pub fn peek_token(&mut self) -> PdfResult<Option<Token>> {
        let saved_pos = self.cursor.position();
        let tok = self.next_token();
        self.cursor.set_position(saved_pos)?;
        tok
    }

    fn parse_name(&mut self) -> PdfResult<String> {
        let mut name_bytes = Vec::with_capacity(32);

        while let Some(b) = self.cursor.peek_byte() {
            if is_pdf_whitespace(b) || is_pdf_delimiter(b) {
                break;
            }
            let _ = self.cursor.read_byte();

            if b == b'#' {
                // Hex character escape #xx
                let h1 = self.cursor.read_byte()?;
                let h2 = self.cursor.read_byte()?;
                let v1 = hex_val(h1).ok_or_else(|| {
                    PdfError::InvalidToken(format!("Invalid hex escape in name: #{h1:02x}{h2:02x}"))
                })?;
                let v2 = hex_val(h2).ok_or_else(|| {
                    PdfError::InvalidToken(format!("Invalid hex escape in name: #{h1:02x}{h2:02x}"))
                })?;
                name_bytes.push((v1 << 4) | v2);
            } else {
                name_bytes.push(b);
            }
        }

        // PDF names are UTF-8 or standard 8-bit text
        String::from_utf8(name_bytes.clone())
            .or_else(|_| Ok(name_bytes.iter().map(|&c| c as char).collect()))
    }

    fn parse_literal_string(&mut self) -> PdfResult<Vec<u8>> {
        let mut result = Vec::with_capacity(64);
        let mut depth = 1usize;

        while let Some(b) = self.cursor.peek_byte() {
            let _ = self.cursor.read_byte();
            match b {
                b'(' => {
                    depth += 1;
                    result.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(result);
                    }
                    result.push(b')');
                }
                b'\\' => {
                    // Escape sequence
                    let next_b = self.cursor.read_byte()?;
                    match next_b {
                        b'n' => result.push(b'\n'),
                        b'r' => result.push(b'\r'),
                        b't' => result.push(b'\t'),
                        b'b' => result.push(0x08),
                        b'f' => result.push(0x0C),
                        b'(' => result.push(b'('),
                        b')' => result.push(b')'),
                        b'\\' => result.push(b'\\'),
                        b'\r' => {
                            // Line continuation: ignore \r or \r\n
                            if self.cursor.peek_byte() == Some(b'\n') {
                                let _ = self.cursor.read_byte();
                            }
                        }
                        b'\n' => {
                            // Line continuation
                        }
                        b'0'..=b'7' => {
                            // Octal escape (up to 3 digits)
                            let mut oct_val = (next_b - b'0') as u32;
                            for _ in 0..2 {
                                if let Some(d @ b'0'..=b'7') = self.cursor.peek_byte() {
                                    let _ = self.cursor.read_byte();
                                    oct_val = (oct_val << 3) + (d - b'0') as u32;
                                } else {
                                    break;
                                }
                            }
                            result.push((oct_val & 0xFF) as u8);
                        }
                        other => result.push(other),
                    }
                }
                b'\r' => {
                    // EOL normalization: convert \r or \r\n to \n
                    if self.cursor.peek_byte() == Some(b'\n') {
                        let _ = self.cursor.read_byte();
                    }
                    result.push(b'\n');
                }
                other => {
                    result.push(other);
                }
            }
        }

        Err(PdfError::UnexpectedEof)
    }

    fn parse_hex_string(&mut self) -> PdfResult<Vec<u8>> {
        let mut result = Vec::with_capacity(32);
        let mut first_nibble: Option<u8> = None;

        while let Some(b) = self.cursor.peek_byte() {
            let _ = self.cursor.read_byte();
            if b == b'>' {
                if let Some(high) = first_nibble {
                    result.push(high << 4);
                }
                return Ok(result);
            } else if is_pdf_whitespace(b) {
                continue;
            } else if let Some(val) = hex_val(b) {
                if let Some(high) = first_nibble {
                    result.push((high << 4) | val);
                    first_nibble = None;
                } else {
                    first_nibble = Some(val);
                }
            } else {
                return Err(PdfError::InvalidToken(format!(
                    "Invalid character in hex string: {b:02x}"
                )));
            }
        }

        Err(PdfError::UnexpectedEof)
    }

    fn parse_word_or_number(&mut self) -> PdfResult<Option<Token>> {
        let mut word_bytes = Vec::with_capacity(32);

        while let Some(b) = self.cursor.peek_byte() {
            if is_pdf_whitespace(b) || is_pdf_delimiter(b) {
                break;
            }
            let _ = self.cursor.read_byte();
            word_bytes.push(b);
        }

        if word_bytes.is_empty() {
            return Ok(None);
        }

        let word = match std::str::from_utf8(&word_bytes) {
            Ok(s) => s,
            Err(_) => {
                return Err(PdfError::InvalidToken("Non-UTF-8 word token".into()));
            }
        };

        // Check keywords
        match word {
            "true" => return Ok(Some(Token::Boolean(true))),
            "false" => return Ok(Some(Token::Boolean(false))),
            "null" => return Ok(Some(Token::Null)),
            "R" => return Ok(Some(Token::KeywordR)),
            "obj" => return Ok(Some(Token::KeywordObj)),
            "endobj" => return Ok(Some(Token::KeywordEndObj)),
            "stream" => return Ok(Some(Token::KeywordStream)),
            "endstream" => return Ok(Some(Token::KeywordEndStream)),
            "xref" => return Ok(Some(Token::KeywordXref)),
            "trailer" => return Ok(Some(Token::KeywordTrailer)),
            "startxref" => return Ok(Some(Token::KeywordStartXref)),
            _ => {}
        }

        // Check integer or real number
        if let Ok(i) = word.parse::<i64>() {
            return Ok(Some(Token::Integer(i)));
        }

        if let Ok(f) = word.parse::<f64>() {
            if f.is_finite() {
                return Ok(Some(Token::Real(f)));
            }
        }

        // Generic keyword / operator
        Ok(Some(Token::Keyword(word.to_string())))
    }
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
