use core::{iter::Map, ptr::NonNull};

use crate::arch::x86_64::paging::{level::{NonLeafTableLevel, TableLevel}, table::PageTable};

use super::{
    address::{PhysicalAddress, PhysicalPage, VirtualAddress, VirtualPage},
    entry::PtFlags,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    AlreadyMapped,
    PageTablePageUnavailable,
    InvalidAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    NotMapped,
    InvalidAddress,
}

/// Supplies zeroed physical pages used for intermediate page tables.
pub trait PageTablePageProvider {
    fn allocate_page_table_page(&mut self) -> Option<PhysicalPage>;

    /// # Safety
    ///
    /// `page` must have been allocated by this provider for a page table and
    /// must no longer be referenced by any active page-table entry.
    unsafe fn release_page_table_page(&mut self, page: PhysicalPage);
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct PageMapper {
    root_table: PhysicalPage,
    hhdm_offset: u64,
    max_physical_address_bits: u8,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl PageMapper {
    pub unsafe fn new(
        root_table: PhysicalPage,
        hhdm_offset: u64,
        max_physical_address_bits: u8,
    ) -> Self {
        Self {
            root_table,
            hhdm_offset,
            max_physical_address_bits,
        }
    }

    pub fn map<P>(
        &mut self,
        virtual_page: VirtualPage,
        physical_page: PhysicalPage,
        flags: PtFlags,
        table_pages: &mut P,
    ) -> Result<(), MapError>
    where
        P: PageTablePageProvider,
    {
        todo!()
    }

    pub fn unmap(&mut self, virtual_page: VirtualPage) -> Result<PhysicalPage, UnmapError> {
        todo!()
    }

    pub fn translate(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        todo!()
    }

    fn virtual_to_physical(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        let translated = address.value().checked_sub(self.hhdm_offset)?;
        PhysicalAddress::new(translated, self.max_physical_address_bits)
    }

    fn physical_to_virtual(&self, address: PhysicalAddress) -> Option<VirtualAddress> {
        let translated = address.value().checked_add(self.hhdm_offset)?;
        VirtualAddress::new(translated)
    }

    fn table_pointer<L>(&self, page: PhysicalPage) -> Result<NonNull<PageTable<L>>, MapError>
    where
        L: TableLevel
    {
        todo!()
    }

    fn next_table_pointer<L>(&self, page_table: PageTable<L>) -> Result<NonNull<PageTable<L::Next>>, MapError>
    where
        L: NonLeafTableLevel,
    {
        todo!()
    }
}
