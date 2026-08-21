use crate::{
    subsystems::display::font::{BitmapFont, BitmapGlyph, FontMetrics},
    util::{read_u16_le, read_u32_le},
};

pub struct OnftFont<'data> {
    width: usize,
    height: usize,
    row_stride: usize,
    bytes_per_glyph: usize,
    glyph_data: &'data [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnftParseError {
    FileTooShort,
    InvalidMagic,
    UnsupportedVersion,
    InvalidHeaderSize,
    InvalidFlags,
    InvalidDimensions,
    InvalidRowStride,
    InvalidReserved,
    InvalidGlyphCount,
    InvalidFirstCodepoint,
    InvalidBytesPerGlyph,
    InvalidGlyphDataOffset,
    InvalidGlyphDataSize,
    InvalidFileSize,
    IntegerOverflow,
}

impl<'data> OnftFont<'data> {
    const HEADER_SIZE: usize = 40;
    const GLYPH_COUNT: usize = 128;

    pub fn parse(data: &'data [u8]) -> Result<Self, OnftParseError> {
        if data.len() < Self::HEADER_SIZE {
            return Err(OnftParseError::FileTooShort);
        }

        if data.get(0x00..0x04) != Some(b"ONFT") {
            return Err(OnftParseError::InvalidMagic);
        }

        let version = read_u16_le(data, 0x04).ok_or(OnftParseError::FileTooShort)?;
        let header_size = read_u16_le(data, 0x06).ok_or(OnftParseError::FileTooShort)?;
        let flags = read_u32_le(data, 0x08).ok_or(OnftParseError::FileTooShort)?;
        let width = read_u16_le(data, 0x0c).ok_or(OnftParseError::FileTooShort)?;
        let height = read_u16_le(data, 0x0e).ok_or(OnftParseError::FileTooShort)?;
        let row_stride = read_u16_le(data, 0x10).ok_or(OnftParseError::FileTooShort)?;
        let reserved = read_u16_le(data, 0x12).ok_or(OnftParseError::FileTooShort)?;
        let glyph_count = read_u32_le(data, 0x14).ok_or(OnftParseError::FileTooShort)?;
        let first_codepoint = read_u32_le(data, 0x18).ok_or(OnftParseError::FileTooShort)?;
        let bytes_per_glyph = read_u32_le(data, 0x1c).ok_or(OnftParseError::FileTooShort)?;
        let glyph_data_offset = read_u32_le(data, 0x20).ok_or(OnftParseError::FileTooShort)?;
        let glyph_data_size = read_u32_le(data, 0x24).ok_or(OnftParseError::FileTooShort)?;

        if version != 1 {
            return Err(OnftParseError::UnsupportedVersion);
        }

        if usize::from(header_size) != Self::HEADER_SIZE {
            return Err(OnftParseError::InvalidHeaderSize);
        }

        if flags != 0 {
            return Err(OnftParseError::InvalidFlags);
        }

        if width == 0 || height == 0 {
            return Err(OnftParseError::InvalidDimensions);
        }

        if reserved != 0 {
            return Err(OnftParseError::InvalidReserved);
        }

        if glyph_count != Self::GLYPH_COUNT as u32 {
            return Err(OnftParseError::InvalidGlyphCount);
        }

        if first_codepoint != 0 {
            return Err(OnftParseError::InvalidFirstCodepoint);
        }

        if glyph_data_offset != Self::HEADER_SIZE as u32 {
            return Err(OnftParseError::InvalidGlyphDataOffset);
        }

        let width = usize::from(width);
        let height = usize::from(height);
        let row_stride = usize::from(row_stride);

        let bytes_per_glyph =
            usize::try_from(bytes_per_glyph).map_err(|_| OnftParseError::IntegerOverflow)?;

        let glyph_data_offset =
            usize::try_from(glyph_data_offset).map_err(|_| OnftParseError::IntegerOverflow)?;

        let glyph_data_size =
            usize::try_from(glyph_data_size).map_err(|_| OnftParseError::IntegerOverflow)?;

        let minimum_row_stride = width
            .checked_add(7)
            .ok_or(OnftParseError::IntegerOverflow)?
            / 8;

        if row_stride < minimum_row_stride {
            return Err(OnftParseError::InvalidRowStride);
        }

        let minimum_bytes_per_glyph = row_stride
            .checked_mul(height)
            .ok_or(OnftParseError::IntegerOverflow)?;

        if bytes_per_glyph < minimum_bytes_per_glyph {
            return Err(OnftParseError::InvalidBytesPerGlyph);
        }

        let expected_glyph_data_size = Self::GLYPH_COUNT
            .checked_mul(bytes_per_glyph)
            .ok_or(OnftParseError::IntegerOverflow)?;

        if glyph_data_size != expected_glyph_data_size {
            return Err(OnftParseError::InvalidGlyphDataSize);
        }

        let expected_file_size = glyph_data_offset
            .checked_add(glyph_data_size)
            .ok_or(OnftParseError::IntegerOverflow)?;

        if data.len() != expected_file_size {
            return Err(OnftParseError::InvalidFileSize);
        }

        let glyph_data = data
            .get(glyph_data_offset..expected_file_size)
            .ok_or(OnftParseError::InvalidFileSize)?;

        Ok(Self {
            width,
            height,
            row_stride,
            bytes_per_glyph,
            glyph_data,
        })
    }
}

impl<'data> BitmapFont for OnftFont<'data> {
    type Glyph<'a>
        = OnftGlyph<'data>
    where
        Self: 'a;

    fn metrics(&self) -> FontMetrics {
        FontMetrics::new(self.width, self.height)
    }

    fn glyph(&self, ch: char) -> Option<Self::Glyph<'_>> {
        if !ch.is_ascii() {
            return None;
        }

        let glyph_index = ch as usize;
        let start = glyph_index.checked_mul(self.bytes_per_glyph)?;
        let end = start.checked_add(self.bytes_per_glyph)?;
        let data = self.glyph_data.get(start..end)?;

        Some(OnftGlyph {
            width: self.width,
            height: self.height,
            row_stride: self.row_stride,
            data,
        })
    }
}

pub struct OnftGlyph<'font> {
    width: usize,
    height: usize,
    row_stride: usize,
    data: &'font [u8],
}

impl<'font> BitmapGlyph for OnftGlyph<'font> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn pixel(&self, x: usize, y: usize) -> Option<bool> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let row_offset = y.checked_mul(self.row_stride)?;
        let byte_offset = row_offset.checked_add(x / 8)?;
        let byte = *self.data.get(byte_offset)?;
        let mask = 0x80_u8 >> (x % 8);

        Some(byte & mask != 0)
    }
}
