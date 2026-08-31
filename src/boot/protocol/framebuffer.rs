//! Framebuffer information consumed by the framebuffer driver.

use crate::arch::common::paging::Address;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorMask {
    pub size: u8,
    pub shift: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbPixelFormat {
    pub bits_per_pixel: u16,
    pub red: ColorMask,
    pub green: ColorMask,
    pub blue: ColorMask,
}

/// Runtime framebuffer information whose address belongs to the kernel MMIO
/// arena rather than the bootloader's temporary address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferData<V>
where
    V: Address,
{
    virtual_start: V,
    byte_len: u64,
    width: usize,
    height: usize,
    pitch: usize,
    pixel_format: RgbPixelFormat,
}

impl<V> FramebufferData<V>
where
    V: Address,
{
    pub fn virtual_start(&self) -> V {
        todo!()
    }

    pub fn byte_len(&self) -> u64 {
        todo!()
    }

    pub fn width(&self) -> usize {
        todo!()
    }

    pub fn height(&self) -> usize {
        todo!()
    }

    pub fn pitch(&self) -> usize {
        todo!()
    }

    pub fn pixel_format(&self) -> RgbPixelFormat {
        todo!()
    }
}
