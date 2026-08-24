use std::collections::BTreeMap;

/// Standard 14 PDF font width tables (in 1000ths of a unit of text space).
pub struct StandardFontMetrics;

impl StandardFontMetrics {
    /// Returns the character width for a character in the given standard font name.
    pub fn get_char_width(base_font: &str, ch: char) -> Option<f64> {
        let clean = base_font.trim_start_matches('/');
        let code = ch as u32;

        if clean.starts_with("Courier") {
            // Courier is monospace with fixed 600 width
            return Some(600.0);
        }

        if clean.starts_with("Helvetica") {
            return Some(Self::helvetica_width(clean, code));
        }

        if clean.starts_with("Times") {
            return Some(Self::times_width(clean, code));
        }

        if clean == "Symbol" {
            return Some(Self::symbol_width(code));
        }

        if clean == "ZapfDingbats" {
            return Some(Self::zapf_dingbats_width(code));
        }

        // Default proportional width
        Some(500.0)
    }

    /// Builds the standard 256-entry widths map for a standard font.
    pub fn build_widths_map(base_font: &str) -> BTreeMap<u32, f64> {
        let mut map = BTreeMap::new();
        for code in 0u32..=255 {
            if let Some(ch) = char::from_u32(code) {
                if let Some(w) = Self::get_char_width(base_font, ch) {
                    map.insert(code, w);
                }
            }
        }
        map
    }

    fn helvetica_width(font: &str, code: u32) -> f64 {
        let is_bold = font.contains("Bold");
        match code {
            0x20 | 0x21 => 278.0,
            0x22 => {
                if is_bold {
                    333.0
                } else {
                    355.0
                }
            }
            0x23 => 556.0,
            0x24 => 556.0,
            0x25 => 889.0,
            0x26 => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            }
            0x27 => {
                if is_bold {
                    238.0
                } else {
                    222.0
                }
            }
            0x28 | 0x29 => 333.0,
            0x2A => 389.0,
            0x2B | 0x3D | 0x3C | 0x3E => 584.0,
            0x2C | 0x2E => 278.0,
            0x2D => 333.0,
            0x2F => 278.0,
            0x30..=0x39 => 556.0, // digits 0-9
            0x3A | 0x3B => 278.0,
            0x3F => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            }
            0x40 => {
                if is_bold {
                    975.0
                } else {
                    1015.0
                }
            }
            0x41 => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            } // A
            0x42 => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            } // B
            0x43 => 722.0, // C
            0x44 => 722.0, // D
            0x45 => 667.0, // E
            0x46 => 611.0, // F
            0x47 => 778.0, // G
            0x48 => 722.0, // H
            0x49 => 278.0, // I
            0x4A => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // J
            0x4B => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            } // K
            0x4C => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // L
            0x4D => 833.0, // M
            0x4E => 722.0, // N
            0x4F => 778.0, // O
            0x50 => 667.0, // P
            0x51 => 778.0, // Q
            0x52 => 722.0, // R
            0x53 => 667.0, // S
            0x54 => 611.0, // T
            0x55 => 722.0, // U
            0x56 => 667.0, // V
            0x57 => 944.0, // W
            0x58 => 667.0, // X
            0x59 => 667.0, // Y
            0x5A => 611.0, // Z
            0x61 => 556.0, // a
            0x62 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // b
            0x63 => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // c
            0x64 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // d
            0x65 => 556.0, // e
            0x66 => {
                if is_bold {
                    333.0
                } else {
                    278.0
                }
            } // f
            0x67 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // g
            0x68 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // h
            0x69 => {
                if is_bold {
                    278.0
                } else {
                    222.0
                }
            } // i
            0x6A => {
                if is_bold {
                    278.0
                } else {
                    222.0
                }
            } // j
            0x6B => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // k
            0x6C => {
                if is_bold {
                    278.0
                } else {
                    222.0
                }
            } // l
            0x6D => {
                if is_bold {
                    889.0
                } else {
                    833.0
                }
            } // m
            0x6E => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // n
            0x6F => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // o
            0x70 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // p
            0x71 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // q
            0x72 => {
                if is_bold {
                    389.0
                } else {
                    333.0
                }
            } // r
            0x73 => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // s
            0x74 => {
                if is_bold {
                    333.0
                } else {
                    278.0
                }
            } // t
            0x75 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // u
            0x76 => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // v
            0x77 => {
                if is_bold {
                    778.0
                } else {
                    722.0
                }
            } // w
            0x78 => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // x
            0x79 => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // y
            0x7A => 500.0, // z
            0x2013 | 0x2014 => 500.0,
            0x2018 | 0x2019 => 222.0,
            0x201C | 0x201D => 333.0,
            0x2022 => 350.0,
            0x20AC => 556.0,
            _ => 500.0,
        }
    }

    fn times_width(font: &str, code: u32) -> f64 {
        let is_bold = font.contains("Bold");
        match code {
            0x20 => 250.0,
            0x21 => 333.0,
            0x22 => 408.0,
            0x23 => 500.0,
            0x24 => 500.0,
            0x25 => 833.0,
            0x26 => {
                if is_bold {
                    833.0
                } else {
                    778.0
                }
            }
            0x27 => 333.0,
            0x28 | 0x29 => 333.0,
            0x2A => 500.0,
            0x2B | 0x3D => 564.0,
            0x2C | 0x2E => 250.0,
            0x2D => 333.0,
            0x2F => 278.0,
            0x30..=0x39 => 500.0, // digits 0-9
            0x3A | 0x3B => 278.0,
            0x3F => {
                if is_bold {
                    500.0
                } else {
                    444.0
                }
            }
            0x40 => 921.0,
            0x41 => 722.0, // A
            0x42 => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            } // B
            0x43 => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            } // C
            0x44 => 722.0, // D
            0x45 => {
                if is_bold {
                    667.0
                } else {
                    611.0
                }
            } // E
            0x46 => {
                if is_bold {
                    611.0
                } else {
                    556.0
                }
            } // F
            0x47 => 722.0, // G
            0x48 => {
                if is_bold {
                    778.0
                } else {
                    722.0
                }
            } // H
            0x49 => {
                if is_bold {
                    389.0
                } else {
                    333.0
                }
            } // I
            0x4A => {
                if is_bold {
                    500.0
                } else {
                    389.0
                }
            } // J
            0x4B => 722.0, // K
            0x4C => {
                if is_bold {
                    667.0
                } else {
                    611.0
                }
            } // L
            0x4D => {
                if is_bold {
                    944.0
                } else {
                    889.0
                }
            } // M
            0x4E => 722.0, // N
            0x4F => 722.0, // O
            0x50 => {
                if is_bold {
                    667.0
                } else {
                    556.0
                }
            } // P
            0x51 => 722.0, // Q
            0x52 => 722.0, // R
            0x53 => {
                if is_bold {
                    556.0
                } else {
                    500.0
                }
            } // S
            0x54 => {
                if is_bold {
                    667.0
                } else {
                    611.0
                }
            } // T
            0x55 => 722.0, // U
            0x56 => 667.0, // V
            0x57 => {
                if is_bold {
                    944.0
                } else {
                    889.0
                }
            } // W
            0x58 => 667.0, // X
            0x59 => 667.0, // Y
            0x5A => 611.0, // Z
            0x61 => {
                if is_bold {
                    500.0
                } else {
                    444.0
                }
            } // a
            0x62 => 500.0, // b
            0x63 => 444.0, // c
            0x64 => 500.0, // d
            0x65 => 444.0, // e
            0x66 => {
                if is_bold {
                    333.0
                } else {
                    278.0
                }
            } // f
            0x67 => 500.0, // g
            0x68 => 500.0, // h
            0x69 => 278.0, // i
            0x6A => 278.0, // j
            0x6B => 500.0, // k
            0x6C => 278.0, // l
            0x6D => {
                if is_bold {
                    833.0
                } else {
                    778.0
                }
            } // m
            0x6E => 500.0, // n
            0x6F => 500.0, // o
            0x70 => 500.0, // p
            0x71 => 500.0, // q
            0x72 => 333.0, // r
            0x73 => {
                if is_bold {
                    444.0
                } else {
                    389.0
                }
            } // s
            0x74 => 278.0, // t
            0x75 => 500.0, // u
            0x76 => {
                if is_bold {
                    500.0
                } else {
                    444.0
                }
            } // v
            0x77 => {
                if is_bold {
                    722.0
                } else {
                    667.0
                }
            } // w
            0x78 => {
                if is_bold {
                    500.0
                } else {
                    444.0
                }
            } // x
            0x79 => {
                if is_bold {
                    500.0
                } else {
                    444.0
                }
            } // y
            0x7A => 444.0, // z
            0x2013 | 0x2014 => 500.0,
            0x2018 | 0x2019 => 250.0,
            0x201C | 0x201D => 333.0,
            0x2022 => 350.0,
            0x20AC => 500.0,
            _ => 450.0,
        }
    }

    fn symbol_width(code: u32) -> f64 {
        match code {
            0x20 => 250.0,
            _ => 500.0,
        }
    }

    fn zapf_dingbats_width(_code: u32) -> f64 {
        700.0
    }
}
