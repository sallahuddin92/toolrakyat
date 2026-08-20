use crate::syntax::object::PdfObject;

/// Maps standard Adobe glyph names to their corresponding Unicode char.
pub fn glyph_name_to_unicode(name: &str) -> Option<char> {
    match name {
        // Standard ASCII names
        "space" => Some(' '),
        "exclam" => Some('!'),
        "quotedbl" => Some('"'),
        "numbersign" => Some('#'),
        "dollar" => Some('$'),
        "percent" => Some('%'),
        "ampersand" => Some('&'),
        "quoteright" | "quotesingle" => Some('\''),
        "parenleft" => Some('('),
        "parenright" => Some(')'),
        "asterisk" => Some('*'),
        "plus" => Some('+'),
        "comma" => Some(','),
        "hyphen" | "minus" => Some('-'),
        "period" => Some('.'),
        "slash" => Some('/'),
        "zero" => Some('0'),
        "one" => Some('1'),
        "two" => Some('2'),
        "three" => Some('3'),
        "four" => Some('4'),
        "five" => Some('5'),
        "six" => Some('6'),
        "seven" => Some('7'),
        "eight" => Some('8'),
        "nine" => Some('9'),
        "colon" => Some(':'),
        "semicolon" => Some(';'),
        "less" => Some('<'),
        "equal" => Some('='),
        "greater" => Some('>'),
        "question" => Some('?'),
        "at" => Some('@'),
        "bracketleft" => Some('['),
        "backslash" => Some('\\'),
        "bracketright" => Some(']'),
        "asciicircum" => Some('^'),
        "underscore" => Some('_'),
        "quoteleft" | "grave" => Some('`'),
        "braceleft" => Some('{'),
        "bar" => Some('|'),
        "braceright" => Some('}'),
        "asciitilde" => Some('~'),

        // Letters A-Z
        "A" => Some('A'),
        "B" => Some('B'),
        "C" => Some('C'),
        "D" => Some('D'),
        "E" => Some('E'),
        "F" => Some('F'),
        "G" => Some('G'),
        "H" => Some('H'),
        "I" => Some('I'),
        "J" => Some('J'),
        "K" => Some('K'),
        "L" => Some('L'),
        "M" => Some('M'),
        "N" => Some('N'),
        "O" => Some('O'),
        "P" => Some('P'),
        "Q" => Some('Q'),
        "R" => Some('R'),
        "S" => Some('S'),
        "T" => Some('T'),
        "U" => Some('U'),
        "V" => Some('V'),
        "W" => Some('W'),
        "X" => Some('X'),
        "Y" => Some('Y'),
        "Z" => Some('Z'),

        // Letters a-z
        "a" => Some('a'),
        "b" => Some('b'),
        "c" => Some('c'),
        "d" => Some('d'),
        "e" => Some('e'),
        "f" => Some('f'),
        "g" => Some('g'),
        "h" => Some('h'),
        "i" => Some('i'),
        "j" => Some('j'),
        "k" => Some('k'),
        "l" => Some('l'),
        "m" => Some('m'),
        "n" => Some('n'),
        "o" => Some('o'),
        "p" => Some('p'),
        "q" => Some('q'),
        "r" => Some('r'),
        "s" => Some('s'),
        "t" => Some('t'),
        "u" => Some('u'),
        "v" => Some('v'),
        "w" => Some('w'),
        "x" => Some('x'),
        "y" => Some('y'),
        "z" => Some('z'),

        // Common symbols and typographic glyphs
        "bullet" => Some('\u{2022}'),
        "copyright" => Some('\u{00A9}'),
        "registered" => Some('\u{00AE}'),
        "trademark" => Some('\u{2122}'),
        "degree" => Some('\u{00B0}'),
        "endash" => Some('\u{2013}'),
        "emdash" => Some('\u{2014}'),
        "quotedblleft" => Some('\u{201C}'),
        "quotedblright" => Some('\u{201D}'),
        "quoteleft_sym" => Some('\u{2018}'),
        "quoteright_sym" => Some('\u{2019}'),
        "euro" => Some('\u{20AC}'),
        "cent" => Some('\u{00A2}'),
        "sterling" => Some('\u{00A3}'),
        "yen" => Some('\u{00A5}'),
        "section" => Some('\u{00A7}'),
        "paragraph" => Some('\u{00B6}'),
        "germandbls" => Some('\u{00DF}'),
        "Adieresis" => Some('\u{00C4}'),
        "Odieresis" => Some('\u{00D6}'),
        "Udieresis" => Some('\u{00DC}'),
        "adieresis" => Some('\u{00E4}'),
        "odieresis" => Some('\u{00F6}'),
        "udieresis" => Some('\u{00FC}'),
        "eacute" => Some('\u{00E9}'),
        "egrave" => Some('\u{00E8}'),
        "agrave" => Some('\u{00E0}'),
        "ccedil" => Some('\u{00E7}'),
        "ntilde" => Some('\u{00F1}'),

        // Support standard `uniXXXX` glyph naming convention
        name if name.starts_with("uni") && name.len() == 7 => u32::from_str_radix(&name[3..], 16)
            .ok()
            .and_then(char::from_u32),
        _ => None,
    }
}

/// A 256-entry character code mapping table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleEncoding {
    pub map: [Option<char>; 256],
}

impl Default for SimpleEncoding {
    fn default() -> Self {
        Self::standard_win_ansi()
    }
}

impl SimpleEncoding {
    /// Creates a WinAnsi standard encoding table.
    pub fn standard_win_ansi() -> Self {
        let mut map = [None; 256];
        // 0x20 .. 0x7E standard ASCII
        for b in 0x20u8..=0x7Eu8 {
            map[b as usize] = Some(b as char);
        }
        // WinAnsi upper code points (ISO-8859-1 + Windows-1252 extensions)
        for b in 0xA0u8..=0xFFu8 {
            map[b as usize] = Some(b as char);
        }
        // Windows-1252 specific extensions in 0x80..0x9F
        map[0x80] = Some('\u{20AC}'); // Euro
        map[0x93] = Some('\u{201C}'); // ldquo
        map[0x94] = Some('\u{201D}'); // rdquo
        map[0x96] = Some('\u{2013}'); // endash
        map[0x97] = Some('\u{2014}'); // emdash
        map[0x99] = Some('\u{2122}'); // tm
        map[0x95] = Some('\u{2022}'); // bullet

        Self { map }
    }

    /// Creates a MacRoman standard encoding table.
    pub fn standard_mac_roman() -> Self {
        let mut map = [None; 256];
        for b in 0x20u8..=0x7Eu8 {
            map[b as usize] = Some(b as char);
        }
        Self { map }
    }

    /// Parses an `/Encoding` PDF object, applying `/Differences` array on top of a base encoding.
    pub fn from_pdf_object(encoding_obj: &PdfObject) -> Self {
        match encoding_obj {
            PdfObject::Name(name) => match name.as_str() {
                "MacRomanEncoding" => Self::standard_mac_roman(),
                "WinAnsiEncoding" | "StandardEncoding" | "PDFDocEncoding" => {
                    Self::standard_win_ansi()
                }
                _ => Self::standard_win_ansi(),
            },
            PdfObject::Dictionary(dict) => {
                let base_name = dict.get("BaseEncoding").and_then(|v| v.as_name());
                let mut encoding = match base_name {
                    Some("MacRomanEncoding") => Self::standard_mac_roman(),
                    _ => Self::standard_win_ansi(),
                };

                if let Some(diff_arr) = dict.get("Differences").and_then(|v| v.as_array()) {
                    Self::apply_differences(&mut encoding.map, diff_arr);
                }

                encoding
            }
            _ => Self::standard_win_ansi(),
        }
    }

    fn apply_differences(map: &mut [Option<char>; 256], diff_arr: &[PdfObject]) {
        let mut current_code: usize = 0;
        for item in diff_arr {
            match item {
                PdfObject::Integer(code) if *code >= 0 && *code <= 255 => {
                    current_code = *code as usize;
                }
                PdfObject::Name(glyph_name) => {
                    if current_code < 256 {
                        if let Some(ch) = glyph_name_to_unicode(glyph_name) {
                            map[current_code] = Some(ch);
                        }
                        current_code = current_code.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
    }

    /// Decodes a byte using this encoding table.
    #[inline]
    pub fn decode_byte(&self, byte: u8) -> char {
        self.map[byte as usize].unwrap_or(byte as char)
    }
}
