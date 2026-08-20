use crate::error::{PdfError, PdfResult};
use crate::io::source::ByteSource;

#[derive(Debug, Clone, Copy)]
pub struct ByteCursor<'a> {
    source: ByteSource<'a>,
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    #[inline]
    pub const fn new(source: ByteSource<'a>) -> Self {
        Self { source, pos: 0 }
    }

    #[inline]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            source: ByteSource::new(bytes),
            pos: 0,
        }
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    pub fn set_position(&mut self, new_pos: usize) -> PdfResult<()> {
        if new_pos > self.source.len() {
            return Err(PdfError::UnexpectedEof);
        }
        self.pos = new_pos;
        Ok(())
    }

    #[inline]
    pub fn remaining(&self) -> usize {
        self.source.len().saturating_sub(self.pos)
    }

    #[inline]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        self.source.get_byte(self.pos).ok()
    }

    #[inline]
    pub fn peek_ahead(&self, offset: usize) -> Option<u8> {
        self.pos
            .checked_add(offset)
            .and_then(|idx| self.source.get_byte(idx).ok())
    }

    #[inline]
    pub fn read_byte(&mut self) -> PdfResult<u8> {
        let b = self.source.get_byte(self.pos)?;
        self.pos += 1;
        Ok(b)
    }

    #[inline]
    pub fn read_bytes(&mut self, count: usize) -> PdfResult<&'a [u8]> {
        let slice = self.source.get_slice(self.pos, count)?;
        self.pos += count;
        Ok(slice)
    }

    #[inline]
    pub fn advance(&mut self, count: usize) -> PdfResult<()> {
        let new_pos = self.pos.checked_add(count).ok_or(PdfError::UnexpectedEof)?;
        if new_pos > self.source.len() {
            return Err(PdfError::UnexpectedEof);
        }
        self.pos = new_pos;
        Ok(())
    }

    #[inline]
    pub fn remaining_slice(&self) -> &'a [u8] {
        if self.pos >= self.source.len() {
            &[]
        } else {
            self.source
                .get_slice_range(self.pos, self.source.len())
                .unwrap_or(&[])
        }
    }

    #[inline]
    pub fn source(&self) -> ByteSource<'a> {
        self.source
    }
}
