//! Common paging capabilities used by architecture-independent kernel policy.

use core::fmt::Debug;

/// Minimum common behavior of an architecture-specific address value.
pub trait Address: Copy + Debug + Eq + Ord {
    fn value(self) -> u64;
}

/// Minimum common behavior of an architecture-specific page value.
pub trait Page: Copy + Debug + Eq + Ord {
    type Address: Address;

    const SIZE: u64;

    fn start_address(self) -> Self::Address;
}

/// Operations on physical pages required by architecture-independent policy.
pub trait PhysicalPage: Page {
    /// Runtime context needed to validate physical-page arithmetic.
    ///
    /// On x86-64 this is the detected `MAXPHYADDR` width.
    type ArithmeticContext: Copy + Debug;

    /// Returns the physical page containing `address`.
    fn containing_address(address: Self::Address) -> Self;

    fn checked_add(self, page_count: usize, context: Self::ArithmeticContext) -> Option<Self>;
}

/// A non-empty, validated range of consecutive physical pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalPageRange<P>
where
    P: PhysicalPage,
{
    start: P,
    page_count: usize,
}

impl<P> PhysicalPageRange<P>
where
    P: PhysicalPage,
{
    pub fn new(start: P, page_count: usize, context: P::ArithmeticContext) -> Option<Self> {
        let last_offset = page_count.checked_sub(1)?;
        start.checked_add(last_offset, context)?;

        Some(Self { start, page_count })
    }

    pub const fn start(self) -> P {
        self.start
    }

    pub const fn page_count(self) -> usize {
        self.page_count
    }

    pub fn last(self, context: P::ArithmeticContext) -> Option<P> {
        self.start.checked_add(self.page_count - 1, context)
    }
}

/// Operations on virtual pages required by architecture-independent policy.
pub trait VirtualPage: Page {
    fn checked_add(self, page_count: usize) -> Option<Self>;

    /// Returns whether `page_count` consecutive pages beginning at `self` are
    /// valid without crossing an architecture-defined address-space hole.
    fn is_valid_range(self, page_count: usize) -> bool;
}

/// Architecture-neutral permissions requested for a leaf mapping.
///
/// A mapping is always readable. Presence and architecture-specific control
/// bits are the mapper's responsibility and are deliberately not represented
/// here.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingPermissions(u8);

impl MappingPermissions {
    const WRITABLE_BIT: u8 = 1 << 0;
    const USER_ACCESSIBLE_BIT: u8 = 1 << 1;
    const EXECUTABLE_BIT: u8 = 1 << 2;

    pub const KERNEL_READ_ONLY: Self = Self::new(false, false, false);
    pub const KERNEL_DATA: Self = Self::new(true, false, false);
    pub const KERNEL_CODE: Self = Self::new(false, false, true);
    pub const USER_READ_ONLY: Self = Self::new(false, true, false);
    pub const USER_DATA: Self = Self::new(true, true, false);
    pub const USER_CODE: Self = Self::new(false, true, true);

    pub const fn new(writable: bool, user_accessible: bool, executable: bool) -> Self {
        let mut bits = 0;

        if writable {
            bits |= Self::WRITABLE_BIT;
        }
        if user_accessible {
            bits |= Self::USER_ACCESSIBLE_BIT;
        }
        if executable {
            bits |= Self::EXECUTABLE_BIT;
        }

        Self(bits)
    }

    pub const fn is_writable(self) -> bool {
        self.0 & Self::WRITABLE_BIT != 0
    }

    pub const fn is_user_accessible(self) -> bool {
        self.0 & Self::USER_ACCESSIBLE_BIT != 0
    }

    pub const fn is_executable(self) -> bool {
        self.0 & Self::EXECUTABLE_BIT != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    AlreadyMapped,
    PageTablePageUnavailable,
    InvalidAddress,
    UnsupportedMapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    NotMapped,
    InvalidAddress,
    UnsupportedMapping,
}

/// Supplies physical pages for intermediate page tables.
pub trait PageTablePageProvider {
    type PhysicalPage: PhysicalPage;

    fn allocate_page_table_page(&mut self) -> Option<Self::PhysicalPage>;

    /// # Safety
    ///
    /// `page` must have been allocated by this provider for a page table, must
    /// no longer be referenced by any active page-table entry, and must not be
    /// accessed after this call unless it is allocated again.
    unsafe fn release_page_table_page(&mut self, page: Self::PhysicalPage);
}

/// Architecture-neutral page mapping operations.
pub trait PageMapper {
    type PhysicalAddress: Address;
    type VirtualAddress: Address;
    type PhysicalPage: PhysicalPage<Address = Self::PhysicalAddress>;
    type VirtualPage: VirtualPage<Address = Self::VirtualAddress>;

    /// # Safety
    ///
    /// `physical_page` must be valid for the requested use and may not create
    /// aliases that violate Rust's memory rules. `virtual_page` must not be in
    /// use by another owner. Every page returned by `table_pages` must be
    /// writable through the mapper's current page-table access mapping.
    unsafe fn map<P>(
        &mut self,
        virtual_page: Self::VirtualPage,
        physical_page: Self::PhysicalPage,
        permissions: MappingPermissions,
        table_pages: &mut P,
    ) -> Result<(), MapError>
    where
        P: PageTablePageProvider<PhysicalPage = Self::PhysicalPage>;

    /// # Safety
    ///
    /// No code or data may continue to access `virtual_page` after it is
    /// unmapped. The caller must perform any required cross-CPU coordination.
    /// `table_pages` must own every non-root page-table page reachable through
    /// this mapper that may become empty during the operation.
    unsafe fn unmap<P>(
        &mut self,
        virtual_page: Self::VirtualPage,
        table_pages: &mut P,
    ) -> Result<Self::PhysicalPage, UnmapError>
    where
        P: PageTablePageProvider<PhysicalPage = Self::PhysicalPage>;

    fn translate(&self, address: Self::VirtualAddress) -> Option<Self::PhysicalAddress>;
}

/// Fixed-offset address conversion within a bounded direct-map span.
pub trait DirectMap {
    type PhysicalAddress: Address;
    type VirtualAddress: Address;

    fn physical_to_virtual(&self, address: Self::PhysicalAddress) -> Option<Self::VirtualAddress>;

    fn virtual_to_physical(&self, address: Self::VirtualAddress) -> Option<Self::PhysicalAddress>;
}

/// A validated half-open range of virtual pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualPageRange<P>
where
    P: VirtualPage,
{
    start: P,
    page_count: usize,
}

impl<P> VirtualPageRange<P>
where
    P: VirtualPage,
{
    pub fn new(start: P, page_count: usize) -> Option<Self> {
        start
            .is_valid_range(page_count)
            .then_some(Self { start, page_count })
    }

    pub const fn start(self) -> P {
        self.start
    }

    pub const fn page_count(self) -> usize {
        self.page_count
    }

    /// Returns the first page after this range.
    ///
    /// `None` means that the exclusive end falls on an unrepresentable
    /// address-space boundary, such as the x86-64 canonical hole or the end of
    /// the address space.
    pub fn end(self) -> Option<P> {
        self.start.checked_add(self.page_count)
    }

    pub fn last(self) -> Option<P> {
        let last_offset = self.page_count.checked_sub(1)?;
        self.start.checked_add(last_offset)
    }

    pub const fn is_empty(self) -> bool {
        self.page_count == 0
    }
}

/// Named virtual regions required by architecture-independent kernel policy.
pub trait KernelVirtualMemoryLayout {
    type VirtualPage: VirtualPage;

    fn direct_map_region(&self) -> VirtualPageRange<Self::VirtualPage>;
    fn kernel_heap_region(&self) -> VirtualPageRange<Self::VirtualPage>;
    fn kernel_mapping_region(&self) -> VirtualPageRange<Self::VirtualPage>;
    fn mmio_region(&self) -> VirtualPageRange<Self::VirtualPage>;
    fn kernel_stack_region(&self) -> VirtualPageRange<Self::VirtualPage>;
    fn per_cpu_region(&self) -> VirtualPageRange<Self::VirtualPage>;
    fn kernel_image_region(&self) -> VirtualPageRange<Self::VirtualPage>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestAddress(u64);

    impl Address for TestAddress {
        fn value(self) -> u64 {
            self.0
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestPage(u64);

    impl Page for TestPage {
        type Address = TestAddress;

        const SIZE: u64 = 4096;

        fn start_address(self) -> Self::Address {
            TestAddress(self.0 * Self::SIZE)
        }
    }

    impl VirtualPage for TestPage {
        fn checked_add(self, page_count: usize) -> Option<Self> {
            let page_count = u64::try_from(page_count).ok()?;
            self.0.checked_add(page_count).map(Self)
        }

        fn is_valid_range(self, page_count: usize) -> bool {
            match page_count.checked_sub(1) {
                Some(last_offset) => self.checked_add(last_offset).is_some(),
                None => true,
            }
        }
    }

    #[test]
    fn exposes_mapping_permissions_by_meaning() {
        let permissions = MappingPermissions::new(true, true, false);

        assert!(permissions.is_writable());
        assert!(permissions.is_user_accessible());
        assert!(!permissions.is_executable());
    }

    #[test]
    fn validates_the_end_of_a_virtual_page_range() {
        let range = VirtualPageRange::new(TestPage(10), 3).unwrap();

        assert_eq!(range.start(), TestPage(10));
        assert_eq!(range.end(), Some(TestPage(13)));
        assert_eq!(range.last(), Some(TestPage(12)));
        assert_eq!(range.page_count(), 3);
    }

    #[test]
    fn accepts_an_empty_virtual_page_range() {
        let range = VirtualPageRange::new(TestPage(u64::MAX), 0).unwrap();

        assert!(range.is_empty());
        assert_eq!(range.end(), Some(TestPage(u64::MAX)));
        assert_eq!(range.last(), None);
    }

    #[test]
    fn accepts_a_range_ending_at_the_last_representable_page() {
        let range = VirtualPageRange::new(TestPage(u64::MAX), 1).unwrap();

        assert_eq!(range.last(), Some(TestPage(u64::MAX)));
        assert_eq!(range.end(), None);
        assert!(VirtualPageRange::new(TestPage(u64::MAX), 2).is_none());
    }
}
