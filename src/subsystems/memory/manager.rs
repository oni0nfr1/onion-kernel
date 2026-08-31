use crate::arch::common::paging::{DirectMap, MappingPermissions, PageMapper};

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
pub struct MemoryManager<M, D>
where
    M: PageMapper,
    D: DirectMap<PhysicalAddress = M::PhysicalAddress, VirtualAddress = M::VirtualAddress>,
{
    physical: BitmapPageAllocator<M::PhysicalPage, D>,
    virtual_regions: VirtualRegionAllocator<M::VirtualPage>,
    mapper: M,
    direct_map: D,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<M, D> MemoryManager<M, D>
where
    M: PageMapper,
    D: DirectMap<PhysicalAddress = M::PhysicalAddress, VirtualAddress = M::VirtualAddress>,
{
    pub fn new(
        physical: BitmapPageAllocator<M::PhysicalPage, D>,
        virtual_regions: VirtualRegionAllocator<M::VirtualPage>,
        mapper: M,
        direct_map: D,
    ) -> Self {
        todo!()
    }

    pub fn allocate_pages(
        &mut self,
        page_count: usize,
        alignment_pages: usize,
        permissions: MappingPermissions,
    ) -> Result<VirtualRegion<M::VirtualPage>, AllocatePagesError> {
        todo!()
    }

    pub fn release_pages(
        &mut self,
        region: VirtualRegion<M::VirtualPage>,
    ) -> Result<(), ReleasePagesError> {
        todo!()
    }
}
