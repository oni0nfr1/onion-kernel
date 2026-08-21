use limine::framebuffer::{FRAMEBUFFER_RGB, Framebuffer as LimineFramebuffer};

use crate::util::color::Color;

type PixelPoint = crate::util::geometry::Point<usize>;
type PixelRegion = crate::util::geometry::Rect<usize>;
type PixelScale = crate::util::geometry::Scale<usize>;

pub struct Framebuffer<'a> {
    inner: &'a LimineFramebuffer,
    address: *mut u8,
    pub width: usize,
    pub height: usize,
    pitch: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferError {
    NullAddress,
    ZeroWidth,
    ZeroHeight,
    UnsupportedMemoryModel,
    UnsupportedBitsPerPixel,
    UnsupportedColorMask,
    InvalidColorMask,
    InvalidPitch,
    AddressMisaligned,
    SizeOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelDrawError {
    OutOfBounds,
}

impl<'a> Framebuffer<'a> {
    const ALLOWED_BPP: u16 = 32;
    const BYTES_PER_PIXEL: usize = (Self::ALLOWED_BPP / 8) as usize;

    pub fn new(inner: &'a LimineFramebuffer) -> Result<Self, FramebufferError> {
        let address = inner.address().cast::<u8>();

        if address.is_null() {
            return Err(FramebufferError::NullAddress);
        }

        if inner.width == 0 {
            return Err(FramebufferError::ZeroWidth);
        }

        if inner.height == 0 {
            return Err(FramebufferError::ZeroHeight);
        }

        if inner.memory_model != FRAMEBUFFER_RGB {
            return Err(FramebufferError::UnsupportedMemoryModel);
        }

        if inner.bpp != Self::ALLOWED_BPP {
            return Err(FramebufferError::UnsupportedBitsPerPixel);
        }

        let width = usize::try_from(inner.width).map_err(|_| FramebufferError::SizeOverflow)?;

        let height = usize::try_from(inner.height).map_err(|_| FramebufferError::SizeOverflow)?;

        let pitch = usize::try_from(inner.pitch).map_err(|_| FramebufferError::SizeOverflow)?;

        let minimum_pitch = width.checked_mul(4).ok_or(FramebufferError::SizeOverflow)?;

        if pitch < minimum_pitch {
            return Err(FramebufferError::InvalidPitch);
        }

        pitch
            .checked_mul(height)
            .ok_or(FramebufferError::SizeOverflow)?;

        let alignment = align_of::<u32>();

        if (address as usize) % alignment != 0 || pitch % alignment != 0 {
            return Err(FramebufferError::AddressMisaligned);
        }

        if inner.red_mask_size != 8 || inner.green_mask_size != 8 || inner.blue_mask_size != 8 {
            return Err(FramebufferError::UnsupportedColorMask);
        }

        if inner.red_mask_shift > 24 || inner.green_mask_shift > 24 || inner.blue_mask_shift > 24 {
            return Err(FramebufferError::InvalidColorMask);
        }

        let red_mask = 0xffu32 << inner.red_mask_shift;
        let green_mask = 0xffu32 << inner.green_mask_shift;
        let blue_mask = 0xffu32 << inner.blue_mask_shift;

        if red_mask & green_mask != 0 || red_mask & blue_mask != 0 || green_mask & blue_mask != 0 {
            return Err(FramebufferError::InvalidColorMask);
        }

        Ok(Self {
            inner,
            address,
            width,
            height,
            pitch,
        })
    }

    fn encode_color(&self, color: Color) -> u32 {
        ((color.red() as u32) << self.inner.red_mask_shift)
            | ((color.green() as u32) << self.inner.green_mask_shift)
            | ((color.blue() as u32) << self.inner.blue_mask_shift)
    }

    pub fn scale(&self) -> PixelScale {
        PixelScale::new(self.width, self.height)
    }

    pub fn put_pixel(&mut self, point: PixelPoint, color: Color) -> Result<(), PixelDrawError> {
        if point.x >= self.width || point.y >= self.height {
            return Err(PixelDrawError::OutOfBounds);
        }

        // SAFETY:
        // - `point.x`는 `self.width`보다 작다.
        // - `point.y`는 `self.height`보다 작다.
        unsafe {
            self.put_pixel_unchecked(point, color);
        }

        Ok(())
    }

    /// 프레임버퍼의 `point`에 해당하는 위치의 픽셀을 `color`로 칠한다.
    ///
    /// # Safety
    ///
    /// 파라미터의 `point.x`와 `point.y`는 다음 조건을 만족해야 한다.
    ///
    /// - 두 값이 각각 `self.width`, `self.height`보다 작아 `point`의 좌표가 프레임버퍼 안의 유효한 픽셀 위치여야 한다.
    #[inline]
    pub(crate) unsafe fn put_pixel_unchecked(&mut self, point: PixelPoint, color: Color) {
        debug_assert!(
            point.x < self.width,
            "put_pixel_unchecked received x={} outside framebuffer width {}",
            point.x,
            self.width,
        );
        debug_assert!(
            point.y < self.height,
            "put_pixel_unchecked received y={} outside framebuffer height {}",
            point.y,
            self.height,
        );

        let offset = point.y * self.pitch + point.x * Self::BYTES_PER_PIXEL;
        let color_encoded = self.encode_color(color);

        // SAFETY:
        // - `point.x`는 `self.width`보다 작다.
        // - `point.y`는 `self.height`보다 작다.
        unsafe {
            let pixel = self.address.add(offset).cast::<u32>();
            pixel.write_volatile(color_encoded);
        }
    }

    pub fn fill_rect(&mut self, region: PixelRegion, color: Color) -> Result<(), PixelDrawError> {
        let right = region.right();
        let bottom = region.bottom();

        if right > self.width || bottom > self.height {
            return Err(PixelDrawError::OutOfBounds);
        }

        for y in region.y()..bottom {
            for x in region.x()..right {
                // SAFETY: 위의 region 범위 체크로 region 안의 모든 픽셀이 프레임버퍼 안의 유효한 픽셀 위치임.
                unsafe {
                    self.put_pixel_unchecked(PixelPoint::new(x, y), color);
                }
            }
        }

        Ok(())
    }

    pub fn copy_rect(
        &mut self,
        source: PixelRegion,
        destination: PixelPoint,
    ) -> Result<(), PixelDrawError> {
        let source_right = source.right();
        let source_bottom = source.bottom();
        let destination_right = destination
            .x
            .checked_add(source.width())
            .ok_or(PixelDrawError::OutOfBounds)?;
        let destination_bottom = destination
            .y
            .checked_add(source.height())
            .ok_or(PixelDrawError::OutOfBounds)?;

        if source_right > self.width
            || source_bottom > self.height
            || destination_right > self.width
            || destination_bottom > self.height
        {
            return Err(PixelDrawError::OutOfBounds);
        }

        let rows_reversed = destination.y > source.y();
        let columns_reversed = destination.x > source.x();

        for row_index in 0..source.height() {
            let row = if rows_reversed {
                source.height() - 1 - row_index
            } else {
                row_index
            };

            for column_index in 0..source.width() {
                let column = if columns_reversed {
                    source.width() - 1 - column_index
                } else {
                    column_index
                };

                let source_offset =
                    (source.y() + row) * self.pitch + (source.x() + column) * Self::BYTES_PER_PIXEL;
                let destination_offset = (destination.y + row) * self.pitch
                    + (destination.x + column) * Self::BYTES_PER_PIXEL;

                // SAFETY:
                // - source와 destination 영역 전체가 위의 검사로 프레임버퍼 안에 있다.
                // - 두 주소는 Framebuffer::new에서 검증한 u32 정렬을 만족한다.
                // - 겹치는 영역은 복사 방향을 조절하여 아직 읽지 않은 픽셀을 덮어쓰지 않는다.
                unsafe {
                    let source_pixel = self.address.add(source_offset).cast::<u32>();
                    let destination_pixel = self.address.add(destination_offset).cast::<u32>();
                    destination_pixel.write_volatile(source_pixel.read_volatile());
                }
            }
        }

        Ok(())
    }

    pub fn clear(&mut self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                // SAFETY: 루프 범위가 프레임버퍼 크기로 제한되어 있다.
                unsafe {
                    self.put_pixel_unchecked(PixelPoint::new(x, y), color);
                }
            }
        }
    }
}
