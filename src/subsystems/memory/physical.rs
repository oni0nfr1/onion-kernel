use crate::{
    arch::x86_64::paging::{address::PhysicalFrame, mapper::PageTableFrameProvider},
    util::bitmap::Bitmap,
};

#[derive(Debug)]
#[expect(dead_code, reason = "implementation scaffold")]
pub struct PhysicalRegionDescriptor {
    start: PhysicalFrame,
    frame_count: usize,
    bitmap_offset: usize,
    bitmap_byte_count: usize,
    next_hint: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAllocatorInitError {
    NoUsableFrames,
    InvalidRegion,
    InsufficientBitmapStorage,
    AddressOverflow,
    SizeOverflow,
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct BitmapFrameAllocator<'a> {
    regions: &'a mut [PhysicalRegionDescriptor],
    bitmap_storage: &'a mut [u8],
    next_region_hint: usize,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<'a> BitmapFrameAllocator<'a> {
    pub fn new(
        regions: &'a mut [PhysicalRegionDescriptor],
        bitmap_storage: &'a mut [u8],
    ) -> Result<Self, PhysicalAllocatorInitError> {
        todo!()
    }

    pub fn allocate(&mut self) -> Option<PhysicalFrame> {
        todo!()
    }

    /// # Safety
    ///
    /// `frame` must have been returned by this allocator, must still be
    /// allocated, and must no longer be used or referenced anywhere.
    pub unsafe fn deallocate(&mut self, frame: PhysicalFrame) {
        todo!()
    }

    #[expect(dead_code, reason = "implementation scaffold")]
    fn region_bitmap(&mut self, descriptor_index: usize) -> Option<RegionBitmap<'_>> {
        todo!()
    }
}

/// Describes the temporary bitmap view constructed for one physical region.
pub struct RegionBitmap<'a> {
    pub descriptor_index: usize,
    pub bitmap: Bitmap<'a>,
}

impl PageTableFrameProvider for BitmapFrameAllocator<'_> {
    fn allocate_page_table_frame(&mut self) -> Option<PhysicalFrame> {
        self.allocate()
    }

    unsafe fn release_page_table_frame(&mut self, frame: PhysicalFrame) {
        // SAFETY: The caller must uphold `PageTableFrameProvider`'s release
        // contract, which is at least as strict as `deallocate`'s contract.
        unsafe { self.deallocate(frame) };
    }
}
