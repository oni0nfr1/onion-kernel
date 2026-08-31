use core::{mem::align_of, ptr::NonNull};

use crate::arch::{
    common::paging::{
        Address as _, DirectMap, MapError, MappingPermissions, Page as _,
        PageMapper as CommonPageMapper, PageTablePageProvider, UnmapError,
    },
    x86_64::paging::{
        entry::{PageFlags, PageTableEntry, PdFlags, PdptFlags, PtFlags},
        level::{Level4, NonLeafTableLevel, TableLevel},
        table::PageTable,
        tlb,
    },
};

use super::address::{PAGE_SIZE, PhysicalAddress, PhysicalPage, VirtualAddress, VirtualPage};

const LEVEL2_PAGE_SIZE: u64 = 1 << 21;
const LEVEL3_PAGE_SIZE: u64 = 1 << 30;

pub struct PageMapper<D>
where
    D: DirectMap<PhysicalAddress = PhysicalAddress, VirtualAddress = VirtualAddress>,
{
    root_table: PhysicalPage,
    direct_map: D,
    max_physical_address_bits: u8,
}

impl<D> PageMapper<D>
where
    D: DirectMap<PhysicalAddress = PhysicalAddress, VirtualAddress = VirtualAddress>,
{
    /// Creates a mapper for a page-table hierarchy reachable through
    /// `direct_map`.
    ///
    /// # Safety
    ///
    /// `root_table` must identify a valid, exclusively managed Level4 table.
    /// Every page-table page reachable from it must remain allocated and must
    /// be mapped at the address produced by `direct_map` while this mapper
    /// accesses it. `max_physical_address_bits` must be the detected address
    /// width used to construct every physical page in the hierarchy.
    pub unsafe fn new(
        root_table: PhysicalPage,
        direct_map: D,
        max_physical_address_bits: u8,
    ) -> Self {
        Self {
            root_table,
            direct_map,
            max_physical_address_bits,
        }
    }

    pub fn root_table(&self) -> PhysicalPage {
        self.root_table
    }

    pub fn direct_map(&self) -> &D {
        &self.direct_map
    }

    /// Rebinds access to the same hierarchy through a different DirectMap.
    ///
    /// # Safety
    ///
    /// `direct_map` must make the root table and every reachable page-table
    /// page accessible for the entire lifetime of the returned mapper. The
    /// caller must not use this mapper until the new mapping is active.
    pub unsafe fn reconstruct_with_direct_map<N>(self, direct_map: N) -> PageMapper<N>
    where
        N: DirectMap<PhysicalAddress = PhysicalAddress, VirtualAddress = VirtualAddress>,
    {
        PageMapper {
            root_table: self.root_table,
            direct_map,
            max_physical_address_bits: self.max_physical_address_bits,
        }
    }

    fn table_pointer<L>(&self, page: PhysicalPage) -> Result<NonNull<PageTable<L>>, MapError>
    where
        L: TableLevel,
    {
        let address = self
            .direct_map
            .physical_to_virtual(page.start_address())
            .ok_or(MapError::InvalidAddress)?
            .value();
        let address = usize::try_from(address).map_err(|_| MapError::InvalidAddress)?;

        if address % align_of::<PageTable<L>>() != 0 {
            return Err(MapError::InvalidAddress);
        }

        NonNull::new(address as *mut PageTable<L>).ok_or(MapError::InvalidAddress)
    }

    fn next_table_pointer<L>(
        &self,
        entry: &PageTableEntry<L>,
    ) -> Result<NonNull<PageTable<L::Next>>, MapError>
    where
        L: NonLeafTableLevel,
    {
        let page = entry
            .physical_page(self.max_physical_address_bits)
            .ok_or(MapError::InvalidAddress)?;
        self.table_pointer(page)
    }

    fn next_or_create_table<L, P>(
        &self,
        entry: &mut PageTableEntry<L>,
        user_accessible: bool,
        table_pages: &mut P,
    ) -> Result<NonNull<PageTable<L::Next>>, MapError>
    where
        L: NonLeafTableLevel,
        P: PageTablePageProvider<PhysicalPage = PhysicalPage>,
    {
        if entry.contains_flags(PageFlags::PRESENT) {
            entry.insert_flags(PageFlags::WRITABLE);
            if user_accessible {
                entry.insert_flags(PageFlags::USER);
            }
            return self.next_table_pointer(entry);
        }

        let page = table_pages
            .allocate_page_table_page()
            .ok_or(MapError::PageTablePageUnavailable)?;
        let mut table = match self.table_pointer(page) {
            Ok(table) => table,
            Err(error) => {
                // SAFETY: `page` was just returned by this provider and has not
                // been linked into the hierarchy or exposed to another user.
                unsafe { table_pages.release_page_table_page(page) };
                return Err(error);
            }
        };

        // SAFETY: The constructor contract guarantees exclusive access to the
        // hierarchy, and the newly allocated page is not reachable yet.
        unsafe { table.as_mut().clear() };

        let mut flags = PageFlags::PRESENT | PageFlags::WRITABLE;
        if user_accessible {
            flags |= PageFlags::USER;
        }
        *entry = PageTableEntry::new(page, flags);

        Ok(table)
    }

    fn table_pointer_for_unmap<L>(
        &self,
        entry: &PageTableEntry<L>,
    ) -> Result<NonNull<PageTable<L::Next>>, UnmapError>
    where
        L: NonLeafTableLevel,
    {
        if !entry.contains_flags(PageFlags::PRESENT) {
            return Err(UnmapError::NotMapped);
        }

        self.next_table_pointer(entry)
            .map_err(|_| UnmapError::InvalidAddress)
    }

    fn translated_address(
        &self,
        entry_page: PhysicalPage,
        virtual_address: VirtualAddress,
        page_size: u64,
    ) -> Option<PhysicalAddress> {
        let base = entry_page.start_address().value();
        if base % page_size != 0 {
            return None;
        }

        let offset = virtual_address.value() & (page_size - 1);
        let address = base.checked_add(offset)?;
        PhysicalAddress::new(address, self.max_physical_address_bits)
    }

    /// Coordinates invalidation of cached translations for `address`.
    ///
    /// This is currently valid only while the kernel runs on one processor.
    /// It must provide a completed global shootdown before SMP is enabled.
    #[inline]
    fn shootdown_page(&self, address: VirtualAddress) {
        tlb::invalidate_page(address);

        // TODO: Once additional processors are enabled, send a shootdown IPI
        // to every processor that may use this address space and wait for all
        // acknowledgements before returning.
    }
}

impl<D> CommonPageMapper for PageMapper<D>
where
    D: DirectMap<PhysicalAddress = PhysicalAddress, VirtualAddress = VirtualAddress>,
{
    type PhysicalAddress = PhysicalAddress;
    type VirtualAddress = VirtualAddress;
    type PhysicalPage = PhysicalPage;
    type VirtualPage = VirtualPage;

    unsafe fn map<P>(
        &mut self,
        virtual_page: Self::VirtualPage,
        physical_page: Self::PhysicalPage,
        permissions: MappingPermissions,
        table_pages: &mut P,
    ) -> Result<(), MapError>
    where
        P: PageTablePageProvider<PhysicalPage = Self::PhysicalPage>,
    {
        let user_accessible = permissions.is_user_accessible();
        let mut level4 = self.table_pointer::<Level4>(self.root_table)?;

        // SAFETY: `PageMapper::new` requires exclusive ownership of the whole
        // hierarchy and `level4_index` is always in 0..512.
        let level4_entry = unsafe { level4.as_mut() }
            .entry_mut(virtual_page.level4_index())
            .ok_or(MapError::InvalidAddress)?;
        let mut level3 = self.next_or_create_table(level4_entry, user_accessible, table_pages)?;

        // SAFETY: The table pointer was obtained through the mapper's active
        // DirectMap and the index is always in 0..512.
        let level3_entry = unsafe { level3.as_mut() }
            .entry_mut(virtual_page.level3_index())
            .ok_or(MapError::InvalidAddress)?;
        if level3_entry.contains_flags(PdptFlags::PRESENT | PdptFlags::HUGE_PAGE) {
            return Err(MapError::AlreadyMapped);
        }
        let mut level2 = self.next_or_create_table(level3_entry, user_accessible, table_pages)?;

        // SAFETY: See the corresponding Level3 access above.
        let level2_entry = unsafe { level2.as_mut() }
            .entry_mut(virtual_page.level2_index())
            .ok_or(MapError::InvalidAddress)?;
        if level2_entry.contains_flags(PdFlags::PRESENT | PdFlags::HUGE_PAGE) {
            return Err(MapError::AlreadyMapped);
        }
        let mut level1 = self.next_or_create_table(level2_entry, user_accessible, table_pages)?;

        // SAFETY: See the corresponding Level3 access above.
        let level1_entry = unsafe { level1.as_mut() }
            .entry_mut(virtual_page.level1_index())
            .ok_or(MapError::InvalidAddress)?;
        if level1_entry.contains_flags(PtFlags::PRESENT) {
            return Err(MapError::AlreadyMapped);
        }

        let mut flags = PtFlags::PRESENT;
        if permissions.is_writable() {
            flags |= PtFlags::WRITABLE;
        }
        if user_accessible {
            flags |= PtFlags::USER;
        }
        if !permissions.is_executable() {
            flags |= PtFlags::NO_EXECUTE;
        }
        *level1_entry = PageTableEntry::new(physical_page, flags);

        Ok(())
    }

    unsafe fn unmap<P>(
        &mut self,
        virtual_page: Self::VirtualPage,
        table_pages: &mut P,
    ) -> Result<Self::PhysicalPage, UnmapError>
    where
        P: PageTablePageProvider<PhysicalPage = Self::PhysicalPage>,
    {
        let mut level4 = self
            .table_pointer::<Level4>(self.root_table)
            .map_err(|_| UnmapError::InvalidAddress)?;

        // SAFETY: The mapper exclusively owns a reachable and valid hierarchy.
        let level4_entry = unsafe { level4.as_mut() }
            .entry_mut(virtual_page.level4_index())
            .ok_or(UnmapError::InvalidAddress)?;
        let level3_page = level4_entry
            .physical_page(self.max_physical_address_bits)
            .ok_or(UnmapError::NotMapped)?;
        let mut level3 = self.table_pointer_for_unmap(level4_entry)?;

        // SAFETY: The pointer was read from a present non-leaf entry.
        let level3_entry = unsafe { level3.as_mut() }
            .entry_mut(virtual_page.level3_index())
            .ok_or(UnmapError::InvalidAddress)?;
        if level3_entry.contains_flags(PdptFlags::PRESENT | PdptFlags::HUGE_PAGE) {
            return Err(UnmapError::UnsupportedMapping);
        }
        let level2_page = level3_entry
            .physical_page(self.max_physical_address_bits)
            .ok_or(UnmapError::NotMapped)?;
        let mut level2 = self.table_pointer_for_unmap(level3_entry)?;

        // SAFETY: The pointer was read from a present non-huge Level3 entry.
        let level2_entry = unsafe { level2.as_mut() }
            .entry_mut(virtual_page.level2_index())
            .ok_or(UnmapError::InvalidAddress)?;
        if level2_entry.contains_flags(PdFlags::PRESENT | PdFlags::HUGE_PAGE) {
            return Err(UnmapError::UnsupportedMapping);
        }
        let level1_page = level2_entry
            .physical_page(self.max_physical_address_bits)
            .ok_or(UnmapError::NotMapped)?;
        let mut level1 = self.table_pointer_for_unmap(level2_entry)?;

        // SAFETY: The pointer was read from a present non-huge Level2 entry.
        let level1_entry = unsafe { level1.as_mut() }
            .entry_mut(virtual_page.level1_index())
            .ok_or(UnmapError::InvalidAddress)?;
        let physical_page = level1_entry
            .physical_page(self.max_physical_address_bits)
            .ok_or(UnmapError::NotMapped)?;
        level1_entry.clear();

        let release_level1 = unsafe { level1.as_ref() }.is_empty();
        if release_level1 {
            level2_entry.clear();
        }

        let release_level2 = release_level1 && unsafe { level2.as_ref() }.is_empty();
        if release_level2 {
            level3_entry.clear();
        }

        let release_level3 = release_level2 && unsafe { level3.as_ref() }.is_empty();
        if release_level3 {
            level4_entry.clear();
        }

        self.shootdown_page(virtual_page.start_address());

        if release_level1 {
            // SAFETY: The parent entry was cleared before the completed
            // shootdown, and the empty table is no longer reachable.
            unsafe { table_pages.release_page_table_page(level1_page) };
        }
        if release_level2 {
            // SAFETY: Same argument as for the released Level1 table.
            unsafe { table_pages.release_page_table_page(level2_page) };
        }
        if release_level3 {
            // SAFETY: Same argument as for the released Level1 table.
            unsafe { table_pages.release_page_table_page(level3_page) };
        }

        Ok(physical_page)
    }

    fn translate(&self, address: Self::VirtualAddress) -> Option<Self::PhysicalAddress> {
        let page_start = address.value() & !(PAGE_SIZE - 1);
        let virtual_page = VirtualPage::from_start_address(VirtualAddress::new(page_start)?)?;

        let level4 = self.table_pointer::<Level4>(self.root_table).ok()?;
        // SAFETY: The mapper owns a valid hierarchy reachable through its
        // DirectMap, and every derived table index is in 0..512.
        let level4_entry = unsafe { level4.as_ref() }.entry(virtual_page.level4_index())?;
        let level3 = self.next_table_pointer(level4_entry).ok()?;

        // SAFETY: The pointer came from a present Level4 entry.
        let level3_entry = unsafe { level3.as_ref() }.entry(virtual_page.level3_index())?;
        let level3_page = level3_entry.physical_page(self.max_physical_address_bits)?;
        if level3_entry.contains_flags(PdptFlags::HUGE_PAGE) {
            return self.translated_address(level3_page, address, LEVEL3_PAGE_SIZE);
        }
        let level2 = self.next_table_pointer(level3_entry).ok()?;

        // SAFETY: The pointer came from a present non-huge Level3 entry.
        let level2_entry = unsafe { level2.as_ref() }.entry(virtual_page.level2_index())?;
        let level2_page = level2_entry.physical_page(self.max_physical_address_bits)?;
        if level2_entry.contains_flags(PdFlags::HUGE_PAGE) {
            return self.translated_address(level2_page, address, LEVEL2_PAGE_SIZE);
        }
        let level1 = self.next_table_pointer(level2_entry).ok()?;

        // SAFETY: The pointer came from a present non-huge Level2 entry.
        let level1_entry = unsafe { level1.as_ref() }.entry(virtual_page.level1_index())?;
        let level1_page = level1_entry.physical_page(self.max_physical_address_bits)?;
        self.translated_address(level1_page, address, PAGE_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_PHYSICAL_ADDRESS_BITS: u8 = 52;

    #[derive(Debug, Clone, Copy)]
    struct IdentityDirectMap;

    impl DirectMap for IdentityDirectMap {
        type PhysicalAddress = PhysicalAddress;
        type VirtualAddress = VirtualAddress;

        fn physical_to_virtual(&self, address: PhysicalAddress) -> Option<VirtualAddress> {
            VirtualAddress::new(address.value())
        }

        fn virtual_to_physical(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
            PhysicalAddress::new(address.value(), MAX_PHYSICAL_ADDRESS_BITS)
        }
    }

    #[repr(C, align(4096))]
    #[derive(Clone, Copy)]
    struct TestPage([u8; PAGE_SIZE as usize]);

    struct TestPageProvider {
        pages: [TestPage; 3],
        next: usize,
        released: usize,
    }

    impl TestPageProvider {
        fn new() -> Self {
            Self {
                pages: [TestPage([0; PAGE_SIZE as usize]); 3],
                next: 0,
                released: 0,
            }
        }
    }

    impl PageTablePageProvider for TestPageProvider {
        type PhysicalPage = PhysicalPage;

        fn allocate_page_table_page(&mut self) -> Option<Self::PhysicalPage> {
            let page = self.pages.get_mut(self.next)?;
            self.next += 1;
            physical_page(page as *mut TestPage as usize as u64)
        }

        unsafe fn release_page_table_page(&mut self, _page: Self::PhysicalPage) {
            self.released += 1;
        }
    }

    fn physical_page(address: u64) -> Option<PhysicalPage> {
        let address = PhysicalAddress::new(address, MAX_PHYSICAL_ADDRESS_BITS)?;
        PhysicalPage::from_start_address(address)
    }

    fn virtual_page(address: u64) -> VirtualPage {
        VirtualPage::from_start_address(VirtualAddress::new(address).unwrap()).unwrap()
    }

    #[test]
    fn maps_translates_and_unmaps_a_4kib_page() {
        let mut root = PageTable::<Level4>::new();
        let root_page = physical_page(&mut root as *mut PageTable<Level4> as usize as u64).unwrap();
        let mut table_pages = TestPageProvider::new();
        // SAFETY: The identity DirectMap exposes the aligned root and provider
        // pages for the duration of this test, and the mapper has exclusive
        // access to all of them.
        let mut mapper =
            unsafe { PageMapper::new(root_page, IdentityDirectMap, MAX_PHYSICAL_ADDRESS_BITS) };

        let virtual_page = virtual_page(0x4000);
        let mapped_page = physical_page(0x20_0000).unwrap();
        // SAFETY: The synthetic physical page is used only as a translation
        // target, and the virtual page is exclusively owned by this test.
        unsafe {
            mapper
                .map(
                    virtual_page,
                    mapped_page,
                    MappingPermissions::KERNEL_DATA,
                    &mut table_pages,
                )
                .unwrap();
        }

        assert_eq!(
            mapper.translate(VirtualAddress::new(0x4abc).unwrap()),
            PhysicalAddress::new(0x20_0abc, MAX_PHYSICAL_ADDRESS_BITS),
        );
        assert_eq!(table_pages.next, 3);

        // SAFETY: No real memory is accessed through this synthetic mapping,
        // and this test is its sole owner.
        let unmapped = unsafe { mapper.unmap(virtual_page, &mut table_pages) }.unwrap();
        assert_eq!(unmapped, mapped_page);
        assert_eq!(table_pages.released, 3);
        assert_eq!(mapper.translate(VirtualAddress::new(0x4000).unwrap()), None);
    }

    #[test]
    fn rejects_a_second_mapping_of_the_same_page() {
        let mut root = PageTable::<Level4>::new();
        let root_page = physical_page(&mut root as *mut PageTable<Level4> as usize as u64).unwrap();
        let mut table_pages = TestPageProvider::new();
        // SAFETY: See `maps_translates_and_unmaps_a_4kib_page`.
        let mut mapper =
            unsafe { PageMapper::new(root_page, IdentityDirectMap, MAX_PHYSICAL_ADDRESS_BITS) };
        let virtual_page = virtual_page(0x8000);
        let first_page = physical_page(0x30_0000).unwrap();
        let second_page = physical_page(0x40_0000).unwrap();

        // SAFETY: Both calls obey the ownership requirements; the second is
        // expected to be rejected before changing the existing leaf.
        unsafe {
            mapper
                .map(
                    virtual_page,
                    first_page,
                    MappingPermissions::KERNEL_DATA,
                    &mut table_pages,
                )
                .unwrap();
            assert_eq!(
                mapper.map(
                    virtual_page,
                    second_page,
                    MappingPermissions::KERNEL_DATA,
                    &mut table_pages,
                ),
                Err(MapError::AlreadyMapped),
            );
        }

        assert_eq!(
            mapper.translate(VirtualAddress::new(0x8000).unwrap()),
            Some(first_page.start_address()),
        );
    }

    #[test]
    fn retains_shared_tables_until_their_last_mapping_is_removed() {
        let mut root = PageTable::<Level4>::new();
        let root_page = physical_page(&mut root as *mut PageTable<Level4> as usize as u64).unwrap();
        let mut table_pages = TestPageProvider::new();
        // SAFETY: See `maps_translates_and_unmaps_a_4kib_page`.
        let mut mapper =
            unsafe { PageMapper::new(root_page, IdentityDirectMap, MAX_PHYSICAL_ADDRESS_BITS) };
        let first_virtual = virtual_page(0x4000);
        let second_virtual = virtual_page(0x5000);
        let first_physical = physical_page(0x50_0000).unwrap();
        let second_physical = physical_page(0x60_0000).unwrap();

        // SAFETY: The test exclusively owns both synthetic mappings and all
        // page-table storage.
        unsafe {
            mapper
                .map(
                    first_virtual,
                    first_physical,
                    MappingPermissions::KERNEL_DATA,
                    &mut table_pages,
                )
                .unwrap();
            mapper
                .map(
                    second_virtual,
                    second_physical,
                    MappingPermissions::KERNEL_DATA,
                    &mut table_pages,
                )
                .unwrap();

            mapper.unmap(first_virtual, &mut table_pages).unwrap();
        }

        assert_eq!(table_pages.released, 0);
        assert_eq!(
            mapper.translate(VirtualAddress::new(0x5000).unwrap()),
            Some(second_physical.start_address()),
        );

        // SAFETY: The remaining mapping is still exclusively owned by this
        // test and is no longer used after this call.
        unsafe {
            mapper.unmap(second_virtual, &mut table_pages).unwrap();
        }
        assert_eq!(table_pages.released, 3);
    }
}
