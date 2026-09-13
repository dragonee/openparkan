//! A little-endian cursor over a byte slice, and the error every reader returns.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("{source_name}: read past the end at offset {offset}")]
    Truncated { source_name: String, offset: usize },
    #[error("{source_name}: {message}")]
    Invalid { source_name: String, message: String },
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl FormatError {
    pub fn invalid(source_name: &str, message: impl Into<String>) -> Self {
        Self::Invalid { source_name: source_name.to_owned(), message: message.into() }
    }
}

/// Reads fixed-width little-endian values in order.
pub struct Cursor<'a> {
    data: &'a [u8],
    source: &'a str,
    pub pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8], source: &'a str) -> Self {
        Self { data, source, pos: 0 }
    }

    pub fn source(&self) -> &str {
        self.source
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], FormatError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.data.len()).ok_or_else(|| {
            FormatError::Truncated { source_name: self.source.to_owned(), offset: self.pos }
        })?;
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    pub fn u32(&mut self) -> Result<u32, FormatError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().expect("4 bytes")))
    }

    pub fn i32(&mut self) -> Result<i32, FormatError> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into().expect("4 bytes")))
    }

    pub fn f32(&mut self) -> Result<f32, FormatError> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().expect("4 bytes")))
    }

    pub fn vec3(&mut self) -> Result<[f32; 3], FormatError> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
}

/// A `u32` at `offset`, for readers that index rather than walk.
pub fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

/// Latin-1 bytes as text: every byte is the code point of the same number.
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}
