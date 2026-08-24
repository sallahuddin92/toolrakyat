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

        // Extended Latin & Accented characters
        "exclamdown" => Some('\u{00A1}'),
        "cent" => Some('\u{00A2}'),
        "sterling" => Some('\u{00A3}'),
        "currency" => Some('\u{00A4}'),
        "yen" => Some('\u{00A5}'),
        "brokenbar" => Some('\u{00A6}'),
        "section" => Some('\u{00A7}'),
        "dieresis" => Some('\u{00A8}'),
        "copyright" => Some('\u{00A9}'),
        "ordfeminine" => Some('\u{00AA}'),
        "guillemotleft" => Some('\u{00AB}'),
        "logicalnot" => Some('\u{00AC}'),
        "registered" => Some('\u{00AE}'),
        "macron" => Some('\u{00AF}'),
        "degree" => Some('\u{00B0}'),
        "plusminus" => Some('\u{00B1}'),
        "twosuperior" => Some('\u{00B2}'),
        "threesuperior" => Some('\u{00B3}'),
        "acute" => Some('\u{00B4}'),
        "mu" | "micro" => Some('\u{00B5}'),
        "paragraph" => Some('\u{00B6}'),
        "periodcentered" => Some('\u{00B7}'),
        "cedilla" => Some('\u{00B8}'),
        "onesuperior" => Some('\u{00B9}'),
        "ordmasculine" => Some('\u{00BA}'),
        "guillemotright" => Some('\u{00BB}'),
        "onequarter" => Some('\u{00BC}'),
        "onehalf" => Some('\u{00BD}'),
        "threequarters" => Some('\u{00BE}'),
        "questiondown" => Some('\u{00BF}'),
        "Agrave" => Some('\u{00C0}'),
        "Aacute" => Some('\u{00C1}'),
        "Acircumflex" => Some('\u{00C2}'),
        "Atilde" => Some('\u{00C3}'),
        "Adieresis" => Some('\u{00C4}'),
        "Aring" => Some('\u{00C5}'),
        "AE" => Some('\u{00C6}'),
        "Ccedilla" => Some('\u{00C7}'),
        "Egrave" => Some('\u{00C8}'),
        "Eacute" => Some('\u{00C9}'),
        "Ecircumflex" => Some('\u{00CA}'),
        "Edieresis" => Some('\u{00CB}'),
        "Igrave" => Some('\u{00CC}'),
        "Iacute" => Some('\u{00CD}'),
        "Icircumflex" => Some('\u{00CE}'),
        "Idieresis" => Some('\u{00CF}'),
        "Eth" => Some('\u{00D0}'),
        "Ntilde" => Some('\u{00D1}'),
        "Ograve" => Some('\u{00D2}'),
        "Oacute" => Some('\u{00D3}'),
        "Ocircumflex" => Some('\u{00D4}'),
        "Otilde" => Some('\u{00D5}'),
        "Odieresis" => Some('\u{00D6}'),
        "multiply" => Some('\u{00D7}'),
        "Oslash" => Some('\u{00D8}'),
        "Ugrave" => Some('\u{00D9}'),
        "Uacute" => Some('\u{00DA}'),
        "Ucircumflex" => Some('\u{00DB}'),
        "Udieresis" => Some('\u{00DC}'),
        "Yacute" => Some('\u{00DD}'),
        "Thorn" => Some('\u{00DE}'),
        "germandbls" => Some('\u{00DF}'),
        "agrave" => Some('\u{00E0}'),
        "aacute" => Some('\u{00E1}'),
        "acircumflex" => Some('\u{00E2}'),
        "atilde" => Some('\u{00E3}'),
        "adieresis" => Some('\u{00E4}'),
        "aring" => Some('\u{00E5}'),
        "ae" => Some('\u{00E6}'),
        "ccedilla" => Some('\u{00E7}'),
        "egrave" => Some('\u{00E8}'),
        "eacute" => Some('\u{00E9}'),
        "ecircumflex" => Some('\u{00EA}'),
        "edieresis" => Some('\u{00EB}'),
        "igrave" => Some('\u{00EC}'),
        "iacute" => Some('\u{00ED}'),
        "icircumflex" => Some('\u{00EE}'),
        "idieresis" => Some('\u{00EF}'),
        "eth" => Some('\u{00F0}'),
        "ntilde" => Some('\u{00F1}'),
        "ograve" => Some('\u{00F2}'),
        "oacute" => Some('\u{00F3}'),
        "ocircumflex" => Some('\u{00F4}'),
        "otilde" => Some('\u{00F5}'),
        "odieresis" => Some('\u{00F6}'),
        "divide" => Some('\u{00F7}'),
        "oslash" => Some('\u{00F8}'),
        "ugrave" => Some('\u{00F9}'),
        "uacute" => Some('\u{00FA}'),
        "ucircumflex" => Some('\u{00FB}'),
        "udieresis" => Some('\u{00FC}'),
        "yacute" => Some('\u{00FD}'),
        "thorn" => Some('\u{00FE}'),
        "ydieresis" => Some('\u{00FF}'),

        // Ligatures and extended typographic glyphs
        "Amacron" => Some('\u{0100}'),
        "amacron" => Some('\u{0101}'),
        "Abreve" => Some('\u{0102}'),
        "abreve" => Some('\u{0103}'),
        "Aogonek" => Some('\u{0104}'),
        "aogonek" => Some('\u{0105}'),
        "Cacute" => Some('\u{0106}'),
        "cacute" => Some('\u{0107}'),
        "Ccaron" => Some('\u{010C}'),
        "ccaron" => Some('\u{010D}'),
        "Dcaron" => Some('\u{010E}'),
        "dcaron" => Some('\u{010F}'),
        "Emacron" => Some('\u{0112}'),
        "emacron" => Some('\u{0113}'),
        "Eogonek" => Some('\u{0118}'),
        "eogonek" => Some('\u{0119}'),
        "Ecaron" => Some('\u{011A}'),
        "ecaron" => Some('\u{011B}'),
        "Lacute" => Some('\u{0139}'),
        "lacute" => Some('\u{013A}'),
        "Lcaron" => Some('\u{013D}'),
        "lcaron" => Some('\u{013E}'),
        "Lslash" => Some('\u{0141}'),
        "lslash" => Some('\u{0142}'),
        "Nacute" => Some('\u{0143}'),
        "nacute" => Some('\u{0144}'),
        "Ncaron" => Some('\u{0147}'),
        "ncaron" => Some('\u{0148}'),
        "OE" => Some('\u{0152}'),
        "oe" => Some('\u{0153}'),
        "Racute" => Some('\u{0154}'),
        "racute" => Some('\u{0155}'),
        "Rcaron" => Some('\u{0158}'),
        "rcaron" => Some('\u{0159}'),
        "Sacute" => Some('\u{015A}'),
        "sacute" => Some('\u{015B}'),
        "Scaron" => Some('\u{0160}'),
        "scaron" => Some('\u{0161}'),
        "Scedilla" => Some('\u{015E}'),
        "scedilla" => Some('\u{015F}'),
        "Tcaron" => Some('\u{0164}'),
        "tcaron" => Some('\u{0165}'),
        "Umacron" => Some('\u{016A}'),
        "umacron" => Some('\u{016B}'),
        "Uring" => Some('\u{016E}'),
        "uring" => Some('\u{016F}'),
        "Ydieresis_sym" => Some('\u{0178}'),
        "Zacute" => Some('\u{0179}'),
        "zacute" => Some('\u{017A}'),
        "Zdotaccent" => Some('\u{017B}'),
        "zdotaccent" => Some('\u{017C}'),
        "Zcaron" => Some('\u{017D}'),
        "zcaron" => Some('\u{017E}'),
        "florin" => Some('\u{0192}'),

        // Ligatures
        "fi" => Some('\u{FB01}'),
        "fl" => Some('\u{FB02}'),
        "ff" => Some('\u{FB00}'),
        "ffi" => Some('\u{FB03}'),
        "ffl" => Some('\u{FB04}'),

        // Typographic symbols & Punctuation
        "bullet" => Some('\u{2022}'),
        "trademark" => Some('\u{2122}'),
        "endash" => Some('\u{2013}'),
        "emdash" => Some('\u{2014}'),
        "quotedblleft" => Some('\u{201C}'),
        "quotedblright" => Some('\u{201D}'),
        "quoteleft_sym" => Some('\u{2018}'),
        "quoteright_sym" => Some('\u{2019}'),
        "quotesinglbase" => Some('\u{201A}'),
        "quotedblbase" => Some('\u{201E}'),
        "dagger" => Some('\u{2020}'),
        "daggerdbl" => Some('\u{2021}'),
        "ellipsis" => Some('\u{2026}'),
        "perthousand" => Some('\u{2030}'),
        "guilsinglleft" => Some('\u{2039}'),
        "guilsinglright" => Some('\u{203A}'),
        "fraction" => Some('\u{2044}'),
        "euro" => Some('\u{20AC}'),
        "minus_sym" => Some('\u{2212}'),

        // Support standard `uniXXXX` and `uXXXX` glyph naming conventions
        name if name.starts_with("uni") && name.len() == 7 => u32::from_str_radix(&name[3..], 16)
            .ok()
            .and_then(char::from_u32),
        name if name.starts_with('u') && (name.len() == 5 || name.len() == 7) => {
            u32::from_str_radix(&name[1..], 16)
                .ok()
                .and_then(char::from_u32)
        }
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
    /// Creates a WinAnsi standard encoding table (Windows-1252 superset of ISO-8859-1).
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
        map[0x82] = Some('\u{201A}'); // quotesinglbase
        map[0x83] = Some('\u{0192}'); // florin
        map[0x84] = Some('\u{201E}'); // quotedblbase
        map[0x85] = Some('\u{2026}'); // ellipsis
        map[0x86] = Some('\u{2020}'); // dagger
        map[0x87] = Some('\u{2021}'); // daggerdbl
        map[0x88] = Some('\u{02C6}'); // circumflex
        map[0x89] = Some('\u{2030}'); // perthousand
        map[0x8A] = Some('\u{0160}'); // Scaron
        map[0x8B] = Some('\u{2039}'); // guilsinglleft
        map[0x8C] = Some('\u{0152}'); // OE
        map[0x8E] = Some('\u{017D}'); // Zcaron
        map[0x91] = Some('\u{2018}'); // quoteleft
        map[0x92] = Some('\u{2019}'); // quoteright
        map[0x93] = Some('\u{201C}'); // quotedblleft
        map[0x94] = Some('\u{201D}'); // quotedblright
        map[0x95] = Some('\u{2022}'); // bullet
        map[0x96] = Some('\u{2013}'); // endash
        map[0x97] = Some('\u{2014}'); // emdash
        map[0x98] = Some('\u{02DC}'); // tilde
        map[0x99] = Some('\u{2122}'); // trademark
        map[0x9A] = Some('\u{0161}'); // scaron
        map[0x9B] = Some('\u{203A}'); // guilsinglright
        map[0x9C] = Some('\u{0153}'); // oe
        map[0x9E] = Some('\u{017E}'); // zcaron
        map[0x9F] = Some('\u{0178}'); // Ydieresis

        Self { map }
    }

    /// Creates a StandardEncoding (PostScript Standard Encoding).
    pub fn standard_standard_encoding() -> Self {
        let mut map = [None; 256];
        for b in 0x20u8..=0x7Eu8 {
            map[b as usize] = Some(b as char);
        }
        map[0xA1] = Some('\u{00A1}'); // exclamdown
        map[0xA2] = Some('\u{00A2}'); // cent
        map[0xA3] = Some('\u{00A3}'); // sterling
        map[0xA4] = Some('\u{2044}'); // fraction
        map[0xA5] = Some('\u{00A5}'); // yen
        map[0xA6] = Some('\u{0192}'); // florin
        map[0xA7] = Some('\u{00A7}'); // section
        map[0xA8] = Some('\u{00A4}'); // currency
        map[0xA9] = Some('\'');
        map[0xAA] = Some('\u{201C}');
        map[0xAB] = Some('\u{00AB}');
        map[0xAC] = Some('\u{2039}');
        map[0xAD] = Some('\u{203A}');
        map[0xAE] = Some('\u{FB01}'); // fi
        map[0xAF] = Some('\u{FB02}'); // fl
        map[0xB1] = Some('\u{2013}'); // endash
        map[0xB2] = Some('\u{2020}'); // dagger
        map[0xB3] = Some('\u{2021}'); // daggerdbl
        map[0xB4] = Some('\u{00B7}'); // periodcentered
        map[0xB6] = Some('\u{00B6}'); // paragraph
        map[0xB7] = Some('\u{2022}'); // bullet
        map[0xB8] = Some('\u{201A}'); // quotesinglbase
        map[0xB9] = Some('\u{201E}'); // quotedblbase
        map[0xBA] = Some('\u{201D}'); // quotedblright
        map[0xBB] = Some('\u{00BB}'); // guillemotright
        map[0xBC] = Some('\u{2026}'); // ellipsis
        map[0xBD] = Some('\u{2030}'); // perthousand
        map[0xBF] = Some('\u{00BF}'); // questiondown
        map[0xC1] = Some('`');
        map[0xC2] = Some('\'');
        map[0xC3] = Some('^');
        map[0xC4] = Some('~');
        map[0xC5] = Some('-');
        map[0xC6] = Some('\u{02D8}');
        map[0xC7] = Some('\u{02D9}');
        map[0xC8] = Some('\u{00A8}');
        map[0xCA] = Some('\u{02DA}');
        map[0xCB] = Some('\u{00B8}');
        map[0xCD] = Some('\u{02DD}');
        map[0xCE] = Some('\u{02DB}');
        map[0xCF] = Some('\u{02C7}');
        map[0xD0] = Some('\u{2014}'); // emdash
        map[0xE1] = Some('\u{00C6}'); // AE
        map[0xE3] = Some('\u{00AA}'); // ordfeminine
        map[0xE8] = Some('\u{0141}'); // Lslash
        map[0xE9] = Some('\u{00D8}'); // Oslash
        map[0xEA] = Some('\u{0152}'); // OE
        map[0xEB] = Some('\u{00BA}'); // ordmasculine
        map[0xF1] = Some('\u{00E6}'); // ae
        map[0xF8] = Some('\u{0142}'); // lslash
        map[0xF9] = Some('\u{00F8}'); // oslash
        map[0xFA] = Some('\u{0153}'); // oe
        map[0xFB] = Some('\u{00DF}'); // germandbls

        Self { map }
    }

    /// Creates a MacRoman standard encoding table.
    pub fn standard_mac_roman() -> Self {
        let mut map = [None; 256];
        for b in 0x20u8..=0x7Eu8 {
            map[b as usize] = Some(b as char);
        }
        map[0x80] = Some('\u{00C4}');
        map[0x81] = Some('\u{00C5}');
        map[0x82] = Some('\u{00C7}');
        map[0x83] = Some('\u{00C9}');
        map[0x84] = Some('\u{00D1}');
        map[0x85] = Some('\u{00D6}');
        map[0x86] = Some('\u{00DC}');
        map[0x87] = Some('\u{00E1}');
        map[0x88] = Some('\u{00E0}');
        map[0x89] = Some('\u{00E2}');
        map[0x8A] = Some('\u{00E4}');
        map[0x8B] = Some('\u{00E3}');
        map[0x8C] = Some('\u{00E5}');
        map[0x8D] = Some('\u{00E7}');
        map[0x8E] = Some('\u{00E9}');
        map[0x8F] = Some('\u{00E8}');
        map[0x90] = Some('\u{00EA}');
        map[0x91] = Some('\u{00EB}');
        map[0x92] = Some('\u{00ED}');
        map[0x93] = Some('\u{00EC}');
        map[0x94] = Some('\u{00EE}');
        map[0x95] = Some('\u{00EF}');
        map[0x96] = Some('\u{00F1}');
        map[0x97] = Some('\u{00F3}');
        map[0x98] = Some('\u{00F2}');
        map[0x99] = Some('\u{00F4}');
        map[0x9A] = Some('\u{00F6}');
        map[0x9B] = Some('\u{00F5}');
        map[0x9C] = Some('\u{00FA}');
        map[0x9D] = Some('\u{00F9}');
        map[0x9E] = Some('\u{00FB}');
        map[0x9F] = Some('\u{00FC}');
        map[0xA0] = Some('\u{2020}');
        map[0xA1] = Some('\u{00B0}');
        map[0xA2] = Some('\u{00A2}');
        map[0xA3] = Some('\u{00A3}');
        map[0xA4] = Some('\u{00A7}');
        map[0xA5] = Some('\u{2022}');
        map[0xA6] = Some('\u{00B6}');
        map[0xA7] = Some('\u{00DF}');
        map[0xA8] = Some('\u{00AE}');
        map[0xA9] = Some('\u{00A9}');
        map[0xAA] = Some('\u{2122}');
        map[0xAB] = Some('\u{00B4}');
        map[0xAC] = Some('\u{00A8}');
        map[0xAD] = Some('\u{2260}');
        map[0xAE] = Some('\u{00C6}');
        map[0xAF] = Some('\u{00D8}');
        map[0xB0] = Some('\u{221E}');
        map[0xB1] = Some('\u{00B1}');
        map[0xB2] = Some('\u{2264}');
        map[0xB3] = Some('\u{2265}');
        map[0xB4] = Some('\u{00A5}');
        map[0xB5] = Some('\u{00B5}');
        map[0xB6] = Some('\u{2202}');
        map[0xB7] = Some('\u{2211}');
        map[0xB8] = Some('\u{220F}');
        map[0xB9] = Some('\u{03C0}');
        map[0xBA] = Some('\u{222B}');
        map[0xBB] = Some('\u{00AA}');
        map[0xBC] = Some('\u{00BA}');
        map[0xBD] = Some('\u{03A9}');
        map[0xBE] = Some('\u{00E6}');
        map[0xBF] = Some('\u{00F8}');
        map[0xC0] = Some('\u{00BF}');
        map[0xC1] = Some('\u{00A1}');
        map[0xC2] = Some('\u{00AC}');
        map[0xC3] = Some('\u{221A}');
        map[0xC4] = Some('\u{0192}');
        map[0xC5] = Some('\u{2248}');
        map[0xC6] = Some('\u{2206}');
        map[0xC7] = Some('\u{00AB}');
        map[0xC8] = Some('\u{00BB}');
        map[0xC9] = Some('\u{2026}');
        map[0xCA] = Some('\u{00A0}');
        map[0xCB] = Some('\u{00C0}');
        map[0xCC] = Some('\u{00C3}');
        map[0xCD] = Some('\u{00D5}');
        map[0xCE] = Some('\u{0152}');
        map[0xCF] = Some('\u{0153}');
        map[0xD0] = Some('\u{2013}');
        map[0xD1] = Some('\u{2014}');
        map[0xD2] = Some('\u{201C}');
        map[0xD3] = Some('\u{201D}');
        map[0xD4] = Some('\u{2018}');
        map[0xD5] = Some('\u{2019}');
        map[0xD6] = Some('\u{00F7}');
        map[0xD7] = Some('\u{25CA}');
        map[0xD8] = Some('\u{00FF}');
        map[0xD9] = Some('\u{0178}');
        map[0xDA] = Some('\u{2044}');
        map[0xDB] = Some('\u{20AC}');
        map[0xDC] = Some('\u{2039}');
        map[0xDD] = Some('\u{203A}');
        map[0xDE] = Some('\u{FB01}');
        map[0xDF] = Some('\u{FB02}');
        map[0xE0] = Some('\u{2021}');
        map[0xE1] = Some('\u{00B7}');
        map[0xE2] = Some('\u{201A}');
        map[0xE3] = Some('\u{201E}');
        map[0xE4] = Some('\u{2030}');
        map[0xE5] = Some('\u{00C2}');
        map[0xE6] = Some('\u{00CA}');
        map[0xE7] = Some('\u{00C1}');
        map[0xE8] = Some('\u{00CB}');
        map[0xE9] = Some('\u{00C8}');
        map[0xEA] = Some('\u{00CD}');
        map[0xEB] = Some('\u{00CE}');
        map[0xEC] = Some('\u{00CF}');
        map[0xED] = Some('\u{00CC}');
        map[0xEE] = Some('\u{00D3}');
        map[0xEF] = Some('\u{00D4}');
        map[0xF0] = Some('\u{F8FF}');
        map[0xF1] = Some('\u{00D2}');
        map[0xF2] = Some('\u{00DA}');
        map[0xF3] = Some('\u{00DB}');
        map[0xF4] = Some('\u{00D9}');
        map[0xF5] = Some('\u{0131}');
        map[0xF6] = Some('\u{02C6}');
        map[0xF7] = Some('\u{02DC}');
        map[0xF8] = Some('\u{00AF}');
        map[0xF9] = Some('\u{02D8}');
        map[0xFA] = Some('\u{02D9}');
        map[0xFB] = Some('\u{02DA}');
        map[0xFC] = Some('\u{00B8}');
        map[0xFD] = Some('\u{02DD}');
        map[0xFE] = Some('\u{02DB}');
        map[0xFF] = Some('\u{02C7}');

        Self { map }
    }

    /// Creates a PDFDocEncoding standard table.
    pub fn standard_pdf_doc() -> Self {
        Self::standard_win_ansi()
    }

    /// Parses an `/Encoding` PDF object, applying `/Differences` array on top of a base encoding.
    pub fn from_pdf_object(encoding_obj: &PdfObject) -> Self {
        match encoding_obj {
            PdfObject::Name(name) => match name.as_str() {
                "MacRomanEncoding" => Self::standard_mac_roman(),
                "StandardEncoding" => Self::standard_standard_encoding(),
                "PDFDocEncoding" => Self::standard_pdf_doc(),
                "WinAnsiEncoding" => Self::standard_win_ansi(),
                _ => Self::standard_win_ansi(),
            },
            PdfObject::Dictionary(dict) => {
                let base_name = dict.get("BaseEncoding").and_then(|v| v.as_name());
                let mut encoding = match base_name {
                    Some("MacRomanEncoding") => Self::standard_mac_roman(),
                    Some("StandardEncoding") => Self::standard_standard_encoding(),
                    Some("PDFDocEncoding") => Self::standard_pdf_doc(),
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

    /// Encodes a Unicode char into a single-byte code point if present in this table.
    pub fn encode_char(&self, ch: char) -> Option<u8> {
        // Direct exact match
        for (i, &mapped) in self.map.iter().enumerate() {
            if mapped == Some(ch) {
                return Some(i as u8);
            }
        }
        // ASCII fallback
        let code = ch as u32;
        if (0x20..=0x7E).contains(&code) {
            return Some(code as u8);
        }
        None
    }
}
