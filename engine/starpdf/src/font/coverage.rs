/// Compact coarse Unicode coverage bitmap.
/// Represents 0x000000..=0x10FFFF in 256-codepoint pages.
/// Total pages = 4352 (0x110000 / 256).
/// Total size = 4352 / 8 = 544 bytes.
pub const COARSE_UNICODE_PAGE_COUNT: usize = 4352;
pub const COARSE_COVERAGE_BITMAP_BYTES: usize = COARSE_UNICODE_PAGE_COUNT / 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoarseCoverageBitmap {
    pub bytes: Vec<u8>,
}

impl Default for CoarseCoverageBitmap {
    fn default() -> Self {
        Self {
            bytes: vec![0u8; COARSE_COVERAGE_BITMAP_BYTES],
        }
    }
}

impl CoarseCoverageBitmap {
    #[inline]
    pub fn new(bytes: Vec<u8>) -> Self {
        let mut b = bytes;
        if b.len() < COARSE_COVERAGE_BITMAP_BYTES {
            b.resize(COARSE_COVERAGE_BITMAP_BYTES, 0);
        }
        Self { bytes: b }
    }

    #[inline]
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self::new(bytes.to_vec())
    }

    #[inline]
    pub fn covers_char(&self, ch: char) -> bool {
        let code = ch as usize;
        let page_idx = code / 256;
        if page_idx >= COARSE_UNICODE_PAGE_COUNT {
            return false;
        }
        let byte_idx = page_idx / 8;
        let bit_idx = page_idx % 8;
        if byte_idx < self.bytes.len() {
            (self.bytes[byte_idx] & (1 << bit_idx)) != 0
        } else {
            false
        }
    }

    #[inline]
    pub fn covers_text(&self, text: &str) -> bool {
        text.chars().all(|ch| self.covers_char(ch))
    }

    pub fn set_char(&mut self, ch: char) {
        let code = ch as usize;
        let page_idx = code / 256;
        if page_idx < COARSE_UNICODE_PAGE_COUNT {
            let byte_idx = page_idx / 8;
            let bit_idx = page_idx % 8;
            if byte_idx < self.bytes.len() {
                self.bytes[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    pub fn build_from_chars<I: IntoIterator<Item = char>>(chars: I) -> Self {
        let mut bitmap = Self::default();
        for ch in chars {
            bitmap.set_char(ch);
        }
        bitmap
    }

    /// Standard Latin & Extended Latin + ASCII coarse coverage
    pub fn standard_latin() -> Self {
        let mut b = Self::default();
        // Pages 0 (ASCII/Latin-1), 1 (Latin Extended-A), 2 (Latin Extended-B), 32 (General Punctuation/Currency)
        let pages = [0, 1, 2, 32];
        for page in pages {
            let byte_idx = page / 8;
            let bit_idx = page % 8;
            b.bytes[byte_idx] |= 1 << bit_idx;
        }
        b
    }
}
