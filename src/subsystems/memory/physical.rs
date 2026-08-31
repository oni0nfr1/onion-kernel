use crate::{
    arch::x86_64::paging::{address::PhysicalPage, mapper::PageTablePageProvider},
    util::bitmap::Bitmap,
};

#[derive(Debug)]
#[expect(dead_code, reason = "implementation scaffold")]
pub struct PhysicalRegionDescriptor {
    start_page: PhysicalPage,
    page_count: usize,
    bitmap_offset: usize,
    bitmap_byte_count: usize,
    next_hint: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAllocatorInitError {
    NoUsablePages,
    InvalidRegion,
    InsufficientBitmapStorage,
    AddressOverflow,
    SizeOverflow,
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct BitmapPageAllocator<'a> {
    regions: &'a mut [PhysicalRegionDescriptor],
    bitmap_storage: &'a mut [u8],
    next_region_hint: usize,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<'a> BitmapPageAllocator<'a> {
    pub fn new(
        regions: &'a mut [PhysicalRegionDescriptor],
        bitmap_storage: &'a mut [u8],
    ) -> Result<Self, PhysicalAllocatorInitError> {
        todo!()
    }

    pub fn allocate(&mut self) -> Option<PhysicalPage> {
        todo!()
    }

    /// # Safety
    ///
    /// `page` must have been returned by this allocator, must still be
    /// allocated, and must no longer be used or referenced anywhere.
    pub unsafe fn deallocate(&mut self, page: PhysicalPage) {
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

impl PageTablePageProvider for BitmapPageAllocator<'_> {
    fn allocate_page_table_page(&mut self) -> Option<PhysicalPage> {
        self.allocate()
    }

    unsafe fn release_page_table_page(&mut self, page: PhysicalPage) {
        // SAFETY: The caller must uphold `PageTablePageProvider`'s release
        // contract, which is at least as strict as `deallocate`'s contract.
        unsafe { self.deallocate(page) };
    }
}
