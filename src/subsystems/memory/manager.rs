use crate::arch::x86_64::paging::{entry::PtFlags, mapper::PageMapper};

use super::{
    physical::BitmapPageAllocator,
    virtual_region::{VirtualRegion, VirtualRegionAllocator},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatePagesError {
    VirtualAddressSpaceExhausted,
    PhysicalMemoryExhausted,
    MappingFailed,
    AddressOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleasePagesError {
    InvalidRegion,
    UnmapFailed,
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct MemoryManager<'a> {
    physical: BitmapPageAllocator<'a>,
    virtual_regions: VirtualRegionAllocator,
    mapper: PageMapper,
    hhdm_offset: u64,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<'a> MemoryManager<'a> {
    pub fn new(
        physical: BitmapPageAllocator<'a>,
        virtual_regions: VirtualRegionAllocator,
        mapper: PageMapper,
        hhdm_offset: u64,
    ) -> Self {
        todo!()
    }

    pub fn allocate_pages(
        &mut self,
        page_count: usize,
        alignment_pages: usize,
        flags: PtFlags,
    ) -> Result<VirtualRegion, AllocatePagesError> {
        todo!()
    }

    pub fn release_pages(&mut self, region: VirtualRegion) -> Result<(), ReleasePagesError> {
        todo!()
    }
}
