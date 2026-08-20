use crate::error::{PdfError, PdfResult};

#[derive(Debug, Clone, Copy)]
pub struct ByteSource<'a> {
    data: &'a [u8],
}

impl<'a> ByteSource<'a> {
    #[inline]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub const fn as_slice(&self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub fn get_byte(&self, offset: usize) -> PdfResult<u8> {
        self.data
            .get(offset)
            .copied()
            .ok_or(PdfError::UnexpectedEof)
    }

    #[inline]
    pub fn get_slice(&self, start: usize, len: usize) -> PdfResult<&'a [u8]> {
        let end = start.checked_add(len).ok_or(PdfError::UnexpectedEof)?;
        if end > self.data.len() {
            return Err(PdfError::UnexpectedEof);
        }
        Ok(&self.data[start..end])
    }

    #[inline]
    pub fn get_slice_range(&self, start: usize, end: usize) -> PdfResult<&'a [u8]> {
        if start > end || end > self.data.len() {
            return Err(PdfError::UnexpectedEof);
        }
        Ok(&self.data[start..end])
    }

    /// Finds the first occurrence of `needle` starting from `from_offset`.
    pub fn find_from(&self, from_offset: usize, needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || from_offset >= self.data.len() {
            return None;
        }
        let slice = &self.data[from_offset..];
        slice
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|pos| from_offset + pos)
    }

    /// Finds the last occurrence of `needle` in the byte stream.
    pub fn find_last(&self, needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || needle.len() > self.data.len() {
            return None;
        }
        self.data
            .windows(needle.len())
            .rposition(|window| window == needle)
    }

    /// Finds the last occurrence of `needle` searching backwards from `before_offset`.
    pub fn find_last_before(&self, before_offset: usize, needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || before_offset == 0 {
            return None;
        }
        let search_len = before_offset.min(self.data.len());
        let slice = &self.data[..search_len];
        if needle.len() > slice.len() {
            return None;
        }
        slice
            .windows(needle.len())
            .rposition(|window| window == needle)
    }
}
