use super::{
    address::{PhysicalAddress, PhysicalFrame, VirtualAddress, VirtualPage},
    entry::PtFlags,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    AlreadyMapped,
    PageTableFrameUnavailable,
    InvalidAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    NotMapped,
    InvalidAddress,
}

/// Supplies zeroed frames used for intermediate page tables.
pub trait PageTableFrameProvider {
    fn allocate_page_table_frame(&mut self) -> Option<PhysicalFrame>;

    /// # Safety
    ///
    /// `frame` must have been allocated by this provider for a page table and
    /// must no longer be referenced by any active page-table entry.
    unsafe fn release_page_table_frame(&mut self, frame: PhysicalFrame);
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct PageMapper {
    root_table: PhysicalFrame,
    hhdm_offset: u64,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl PageMapper {
    pub fn new(root_table: PhysicalFrame, hhdm_offset: u64) -> Result<Self, MapError> {
        todo!()
    }

    pub fn map<P>(
        &mut self,
        page: VirtualPage,
        frame: PhysicalFrame,
        flags: PtFlags,
        table_frames: &mut P,
    ) -> Result<(), MapError>
    where
        P: PageTableFrameProvider,
    {
        todo!()
    }

    pub fn unmap(&mut self, page: VirtualPage) -> Result<PhysicalFrame, UnmapError> {
        todo!()
    }

    pub fn translate(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        todo!()
    }
}
